use anyhow::{Context, Result};
use ash::vk;
use lostcoast_core::scene::Scene;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::device::{create_for_surface, Device};
use crate::instance::{create as create_instance, Instance, InstanceConfig};
use crate::swapchain::{self, Swapchain};
use crate::sync::{create_frame_sync, destroy_frame_sync, FrameSync};

const FRAMES_IN_FLIGHT: usize = 2;

pub struct AppGfx {
    instance: Instance,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: Device,
    swapchain: Option<Swapchain>,
    cmd_pool: vk::CommandPool,
    cmd_bufs: Vec<vk::CommandBuffer>,
    frame_syncs: Vec<FrameSync>,
    image_render_done: Vec<vk::Semaphore>,
    frame_idx: usize,
    extent: (u32, u32),
}

impl AppGfx {
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        size: (u32, u32),
    ) -> Result<Self> {
        let required_exts = ash_window::enumerate_required_extensions(display_handle)
            .context("ash_window::enumerate_required_extensions")?;

        let instance = create_instance(&InstanceConfig {
            app_name: c"lostcoast",
            want_validation: true,
            want_surface_exts: Some(required_exts),
        })?;

        let surface_loader = ash::khr::surface::Instance::new(&instance.entry, &instance.raw);
        let surface = unsafe {
            ash_window::create_surface(
                &instance.entry,
                &instance.raw,
                display_handle,
                window_handle,
                None,
            )
        }
        .context("ash_window::create_surface")?;

        let device = create_for_surface(&instance, &surface_loader, surface)?;

        let swapchain = swapchain::create(
            &instance.raw,
            &device.raw,
            device.physical,
            &surface_loader,
            surface,
            device.queue_family,
            size,
            None,
        )?;

        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(device.queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let cmd_pool = unsafe { device.raw.create_command_pool(&pool_info, None) }
            .context("create cmd pool")?;
        let alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(FRAMES_IN_FLIGHT as u32);
        let cmd_bufs = unsafe { device.raw.allocate_command_buffers(&alloc) }
            .context("allocate cmd buffers")?;

        let mut frame_syncs = Vec::with_capacity(FRAMES_IN_FLIGHT);
        for _ in 0..FRAMES_IN_FLIGHT {
            frame_syncs.push(create_frame_sync(&device.raw, true)?);
        }

        let mut image_render_done = Vec::with_capacity(swapchain.images.len());
        for _ in 0..swapchain.images.len() {
            let sem = unsafe {
                device
                    .raw
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }
            .context("create render-done semaphore")?;
            image_render_done.push(sem);
        }

        Ok(AppGfx {
            instance,
            surface_loader,
            surface,
            device,
            swapchain: Some(swapchain),
            cmd_pool,
            cmd_bufs,
            frame_syncs,
            image_render_done,
            frame_idx: 0,
            extent: size,
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.extent = (w, h);
        self.recreate_swapchain().ok();
    }

    fn recreate_swapchain(&mut self) -> Result<()> {
        unsafe { self.device.raw.device_wait_idle().ok() };
        let old = self.swapchain.take();
        let old_handle = old.as_ref().map(|s| s.raw);
        let new = swapchain::create(
            &self.instance.raw,
            &self.device.raw,
            self.device.physical,
            &self.surface_loader,
            self.surface,
            self.device.queue_family,
            self.extent,
            old_handle,
        )?;
        if let Some(o) = old {
            swapchain::destroy(&self.device.raw, o);
        }
        for sem in self.image_render_done.drain(..) {
            unsafe { self.device.raw.destroy_semaphore(sem, None) };
        }
        for _ in 0..new.images.len() {
            let sem = unsafe {
                self.device
                    .raw
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }
            .context("create render-done semaphore")?;
            self.image_render_done.push(sem);
        }
        self.swapchain = Some(new);
        Ok(())
    }

    pub fn render(&mut self, scene: &Scene, t: f32) -> Result<()> {
        let frame = self.frame_idx % FRAMES_IN_FLIGHT;
        let sync = &self.frame_syncs[frame];
        let cmd = self.cmd_bufs[frame];

        unsafe {
            self.device
                .raw
                .wait_for_fences(std::slice::from_ref(&sync.in_flight), true, u64::MAX)
                .context("wait fence")?;
        }

        let sc = self.swapchain.as_ref().unwrap();
        let acquire = unsafe {
            sc.loader
                .acquire_next_image(sc.raw, u64::MAX, sync.image_available, vk::Fence::null())
        };
        let image_idx = match acquire {
            Ok((idx, _)) => idx as usize,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain()?;
                return Ok(());
            }
            Err(e) => return Err(e).context("acquire_next_image"),
        };

        unsafe {
            self.device
                .raw
                .reset_fences(std::slice::from_ref(&sync.in_flight))
                .context("reset fence")?;
            self.device
                .raw
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .context("reset command buffer")?;
        }

        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.device.raw.begin_command_buffer(cmd, &begin) }
            .context("begin command buffer")?;

        let image = sc.images[image_idx];

        record_clear(&self.device.raw, cmd, image, cycled_color(scene, t));

        unsafe { self.device.raw.end_command_buffer(cmd) }.context("end command buffer")?;

        let render_done = self.image_render_done[image_idx];
        let wait_stages = [vk::PipelineStageFlags::TRANSFER];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(std::slice::from_ref(&sync.image_available))
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(std::slice::from_ref(&cmd))
            .signal_semaphores(std::slice::from_ref(&render_done));
        unsafe {
            self.device
                .raw
                .queue_submit(
                    self.device.queue,
                    std::slice::from_ref(&submit),
                    sync.in_flight,
                )
                .context("queue submit")?;
        }

        let image_indices = [image_idx as u32];
        let swapchains = [sc.raw];
        let present = vk::PresentInfoKHR::default()
            .wait_semaphores(std::slice::from_ref(&render_done))
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        let result = unsafe { sc.loader.queue_present(self.device.queue, &present) };
        match result {
            Ok(_) | Err(vk::Result::SUBOPTIMAL_KHR) => {}
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain()?;
            }
            Err(e) => return Err(e).context("queue_present"),
        }

        self.frame_idx = self.frame_idx.wrapping_add(1);
        Ok(())
    }
}

fn cycled_color(scene: &Scene, t: f32) -> [f32; 4] {
    let base = scene.clear_color();
    let cycle = (t * 0.5).sin() * 0.5 + 0.5;
    [
        (base[0] * 0.5 + 0.5 * cycle).clamp(0.0, 1.0),
        (base[1] * 0.5 + 0.5 * (1.0 - cycle)).clamp(0.0, 1.0),
        (base[2] * 0.5 + 0.5 * cycle).clamp(0.0, 1.0),
        1.0,
    ]
}

fn record_clear(device: &ash::Device, cmd: vk::CommandBuffer, image: vk::Image, color: [f32; 4]) {
    image_barrier(
        device,
        cmd,
        image,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::AccessFlags::empty(),
        vk::PipelineStageFlags::TRANSFER,
        vk::AccessFlags::TRANSFER_WRITE,
    );

    let cv = vk::ClearColorValue { float32: color };
    let range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);
    unsafe {
        device.cmd_clear_color_image(
            cmd,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &cv,
            std::slice::from_ref(&range),
        );
    }

    image_barrier(
        device,
        cmd,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::PRESENT_SRC_KHR,
        vk::PipelineStageFlags::TRANSFER,
        vk::AccessFlags::TRANSFER_WRITE,
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        vk::AccessFlags::empty(),
    );
}

#[allow(clippy::too_many_arguments)]
fn image_barrier(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    old: vk::ImageLayout,
    new: vk::ImageLayout,
    src_stage: vk::PipelineStageFlags,
    src_access: vk::AccessFlags,
    dst_stage: vk::PipelineStageFlags,
    dst_access: vk::AccessFlags,
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

impl Drop for AppGfx {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.raw.device_wait_idle();
            for sem in self.image_render_done.drain(..) {
                self.device.raw.destroy_semaphore(sem, None);
            }
            for s in self.frame_syncs.drain(..) {
                destroy_frame_sync(&self.device.raw, s);
            }
            self.device.raw.destroy_command_pool(self.cmd_pool, None);
            if let Some(sc) = self.swapchain.take() {
                swapchain::destroy(&self.device.raw, sc);
            }
            self.surface_loader.destroy_surface(self.surface, None);
        }
        let _ = &self.instance;
    }
}
