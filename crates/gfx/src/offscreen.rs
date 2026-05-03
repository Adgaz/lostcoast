use anyhow::{anyhow, Context, Result};
use ash::vk;
use image::{Rgba, RgbaImage};
use lostcoast_core::scene::Scene;

use crate::device::{create_headless, find_memory_type, Device};
use crate::instance::{create as create_instance, Instance, InstanceConfig};

const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

pub fn render_offscreen(
    scene: &Scene,
    _camera: [f32; 3],
    _look_at: [f32; 3],
    size: (u32, u32),
) -> Result<RgbaImage> {
    let instance = create_instance(&InstanceConfig {
        app_name: c"lostcoast-render-ref",
        want_validation: true,
        want_surface_exts: None,
    })?;
    let device = create_headless(&instance)?;
    render(&instance, &device, scene, size)
}

fn render(
    instance: &Instance,
    device: &Device,
    scene: &Scene,
    size: (u32, u32),
) -> Result<RgbaImage> {
    let (width, height) = size;
    if width == 0 || height == 0 {
        return Err(anyhow!("zero-sized render: {width}x{height}"));
    }

    let img_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(COLOR_FORMAT)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST,
        )
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let color_image =
        unsafe { device.raw.create_image(&img_info, None) }.context("create offscreen image")?;

    let mem_req = unsafe { device.raw.get_image_memory_requirements(color_image) };
    let mem_type = find_memory_type(
        &device.mem_props,
        mem_req.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    let alloc = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_req.size)
        .memory_type_index(mem_type);
    let color_mem = unsafe { device.raw.allocate_memory(&alloc, None) }
        .context("allocate offscreen image memory")?;
    unsafe { device.raw.bind_image_memory(color_image, color_mem, 0) }
        .context("bind image memory")?;

    let pixel_count = (width as u64) * (height as u64);
    let buf_size = pixel_count * 4;
    let buf_info = vk::BufferCreateInfo::default()
        .size(buf_size)
        .usage(vk::BufferUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let read_buf =
        unsafe { device.raw.create_buffer(&buf_info, None) }.context("create readback buffer")?;
    let buf_mem_req = unsafe { device.raw.get_buffer_memory_requirements(read_buf) };
    let buf_mem_type = find_memory_type(
        &device.mem_props,
        buf_mem_req.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    let buf_alloc = vk::MemoryAllocateInfo::default()
        .allocation_size(buf_mem_req.size)
        .memory_type_index(buf_mem_type);
    let buf_mem = unsafe { device.raw.allocate_memory(&buf_alloc, None) }
        .context("allocate readback memory")?;
    unsafe { device.raw.bind_buffer_memory(read_buf, buf_mem, 0) }
        .context("bind readback buffer memory")?;

    let pool_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(device.queue_family)
        .flags(vk::CommandPoolCreateFlags::TRANSIENT);
    let pool = unsafe { device.raw.create_command_pool(&pool_info, None) }
        .context("create command pool")?;
    let cmd_alloc = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let cmd = unsafe { device.raw.allocate_command_buffers(&cmd_alloc) }
        .context("allocate command buffer")?[0];

    let begin =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    unsafe { device.raw.begin_command_buffer(cmd, &begin) }.context("begin command buffer")?;

    transition(
        &device.raw,
        cmd,
        color_image,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    );

    let cc = scene.clear_color();
    let clear = vk::ClearColorValue {
        float32: [cc[0], cc[1], cc[2], 1.0],
    };
    let range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);
    unsafe {
        device.raw.cmd_clear_color_image(
            cmd,
            color_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &clear,
            std::slice::from_ref(&range),
        );
    }

    transition(
        &device.raw,
        cmd,
        color_image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
    );

    let copy = vk::BufferImageCopy::default()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(
            vk::ImageSubresourceLayers::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1),
        )
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        });
    unsafe {
        device.raw.cmd_copy_image_to_buffer(
            cmd,
            color_image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            read_buf,
            std::slice::from_ref(&copy),
        );
    }

    unsafe { device.raw.end_command_buffer(cmd) }.context("end command buffer")?;

    let submit = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));
    let fence_info = vk::FenceCreateInfo::default();
    let fence = unsafe { device.raw.create_fence(&fence_info, None) }.context("create fence")?;
    unsafe {
        device
            .raw
            .queue_submit(device.queue, std::slice::from_ref(&submit), fence)
    }
    .context("queue submit")?;
    unsafe { device.raw.wait_for_fences(&[fence], true, u64::MAX) }.context("wait fence")?;

    let mut bytes = vec![0u8; buf_size as usize];
    unsafe {
        let ptr = device
            .raw
            .map_memory(buf_mem, 0, buf_size, vk::MemoryMapFlags::empty())
            .context("map readback")? as *const u8;
        std::ptr::copy_nonoverlapping(ptr, bytes.as_mut_ptr(), buf_size as usize);
        device.raw.unmap_memory(buf_mem);
    }

    let img = RgbaImage::from_fn(width, height, |x, y| {
        let i = (y as usize * width as usize + x as usize) * 4;
        Rgba([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]])
    });

    unsafe {
        device.raw.destroy_fence(fence, None);
        device.raw.destroy_command_pool(pool, None);
        device.raw.destroy_buffer(read_buf, None);
        device.raw.free_memory(buf_mem, None);
        device.raw.destroy_image(color_image, None);
        device.raw.free_memory(color_mem, None);
    }
    let _ = instance;
    Ok(img)
}

fn transition(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    old: vk::ImageLayout,
    new: vk::ImageLayout,
) {
    let (src_stage, src_access) = stage_for(old, true);
    let (dst_stage, dst_access) = stage_for(new, false);
    let barrier = vk::ImageMemoryBarrier::default()
        .src_access_mask(src_access)
        .dst_access_mask(dst_access)
        .old_layout(old)
        .new_layout(new)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        );
    unsafe {
        device.cmd_pipeline_barrier(
            cmd,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier),
        );
    }
}

fn stage_for(layout: vk::ImageLayout, src: bool) -> (vk::PipelineStageFlags, vk::AccessFlags) {
    match layout {
        vk::ImageLayout::UNDEFINED => (
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::AccessFlags::empty(),
        ),
        vk::ImageLayout::TRANSFER_DST_OPTIMAL => (
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_WRITE,
        ),
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => (
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_READ,
        ),
        vk::ImageLayout::PRESENT_SRC_KHR => {
            if src {
                (
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags::empty(),
                )
            } else {
                (
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::AccessFlags::empty(),
                )
            }
        }
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => (
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        ),
        _ => (
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        ),
    }
}
