use anyhow::{Context, Result};
use ash::vk;

use crate::buffer::{one_shot, Buffer};
use crate::device::{find_memory_type, Device};

pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub mip_levels: u32,
    pub format: vk::Format,
}

impl Texture {
    pub fn destroy(self, device: &Device) {
        unsafe {
            device.raw.destroy_sampler(self.sampler, None);
            device.raw.destroy_image_view(self.view, None);
            device.raw.destroy_image(self.image, None);
            device.raw.free_memory(self.memory, None);
        }
    }
}

pub fn checkerboard_rgba(side: u32, square: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let cx = (x / square) & 1;
            let cy = (y / square) & 1;
            let dark = (cx ^ cy) == 0;
            let (r, g, b) = if dark { (50, 50, 60) } else { (210, 210, 220) };
            out.extend_from_slice(&[r, g, b, 255]);
        }
    }
    out
}

pub fn upload_2d_with_mips(
    device: &Device,
    pool: vk::CommandPool,
    width: u32,
    height: u32,
    pixels_rgba8: &[u8],
) -> Result<Texture> {
    let format = vk::Format::R8G8B8A8_SRGB;
    let mip_levels = ((width.max(height) as f32).log2().floor() as u32) + 1;

    let img_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(mip_levels)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(
            vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
        )
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image =
        unsafe { device.raw.create_image(&img_info, None) }.context("create texture image")?;
    let req = unsafe { device.raw.get_image_memory_requirements(image) };
    let mem_type = find_memory_type(
        &device.mem_props,
        req.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    let alloc = vk::MemoryAllocateInfo::default()
        .allocation_size(req.size)
        .memory_type_index(mem_type);
    let memory =
        unsafe { device.raw.allocate_memory(&alloc, None) }.context("allocate texture memory")?;
    unsafe { device.raw.bind_image_memory(image, memory, 0) }.context("bind texture memory")?;

    let staging = Buffer::create(
        device,
        pixels_rgba8.len() as u64,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    staging.write_host_visible(device, pixels_rgba8)?;

    one_shot(device, pool, |cmd| {
        layout_transition(
            &device.raw,
            cmd,
            image,
            mip_levels,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_WRITE,
            0,
            mip_levels,
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
            device.raw.cmd_copy_buffer_to_image(
                cmd,
                staging.raw,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&copy),
            );
        }

        generate_mips(&device.raw, cmd, image, width, height, mip_levels);
    })?;

    staging.destroy(device);

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(mip_levels)
                .base_array_layer(0)
                .layer_count(1),
        );
    let view = unsafe { device.raw.create_image_view(&view_info, None) }
        .context("create texture image view")?;

    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .min_lod(0.0)
        .max_lod(mip_levels as f32);
    let sampler =
        unsafe { device.raw.create_sampler(&sampler_info, None) }.context("create sampler")?;

    Ok(Texture {
        image,
        memory,
        view,
        sampler,
        mip_levels,
        format,
    })
}

fn generate_mips(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    width: u32,
    height: u32,
    mip_levels: u32,
) {
    let mut mip_w = width as i32;
    let mut mip_h = height as i32;
    for level in 1..mip_levels {
        layout_transition(
            device,
            cmd,
            image,
            mip_levels,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_READ,
            level - 1,
            1,
        );

        let next_w = (mip_w / 2).max(1);
        let next_h = (mip_h / 2).max(1);
        let blit = vk::ImageBlit::default()
            .src_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(level - 1)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .src_offsets([
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: mip_w,
                    y: mip_h,
                    z: 1,
                },
            ])
            .dst_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(level)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .dst_offsets([
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: next_w,
                    y: next_h,
                    z: 1,
                },
            ]);
        unsafe {
            device.cmd_blit_image(
                cmd,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&blit),
                vk::Filter::LINEAR,
            );
        }

        layout_transition(
            device,
            cmd,
            image,
            mip_levels,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::AccessFlags::SHADER_READ,
            level - 1,
            1,
        );

        mip_w = next_w;
        mip_h = next_h;
    }

    layout_transition(
        device,
        cmd,
        image,
        mip_levels,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::PipelineStageFlags::TRANSFER,
        vk::AccessFlags::TRANSFER_WRITE,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::AccessFlags::SHADER_READ,
        mip_levels - 1,
        1,
    );
}

#[allow(clippy::too_many_arguments)]
fn layout_transition(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    _mip_levels: u32,
    old: vk::ImageLayout,
    new: vk::ImageLayout,
    src_stage: vk::PipelineStageFlags,
    src_access: vk::AccessFlags,
    dst_stage: vk::PipelineStageFlags,
    dst_access: vk::AccessFlags,
    base_mip: u32,
    level_count: u32,
) {
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
                .base_mip_level(base_mip)
                .level_count(level_count)
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
