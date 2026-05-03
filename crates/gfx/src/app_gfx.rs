use anyhow::{Context, Result};
use ash::vk;
use glam::{Mat4, Vec3};
use lostcoast_core::scene::Scene;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::camera::{perspective, view};
use crate::device::{create_for_surface, Device};
use crate::instance::{create as create_instance, Instance, InstanceConfig};
use crate::pipeline::{TrianglePipeline, TrianglePush};
use crate::swapchain::{self, Swapchain};
use crate::sync::{create_frame_sync, destroy_frame_sync, FrameSync};
use crate::world_pass::{DepthTarget, WorldResources};

const FRAMES_IN_FLIGHT: usize = 2;
const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

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
    triangle: Option<TrianglePipeline>,
    world: Option<WorldResources>,
    depth: Option<DepthTarget>,
    pipeline_format: vk::Format,
    frame_idx: usize,
    extent: (u32, u32),
    pub camera_pos: Vec3,
    pub camera_target: Vec3,
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

        let pipeline_format = swapchain.format;

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
            triangle: None,
            world: None,
            depth: None,
            pipeline_format,
            frame_idx: 0,
            extent: size,
            camera_pos: Vec3::new(3.0, 3.0, 3.0),
            camera_target: Vec3::ZERO,
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
        if new.format != self.pipeline_format {
            if let Some(tp) = self.triangle.take() {
                tp.destroy(&self.device.raw);
            }
            if let Some(w) = self.world.take() {
                w.destroy(&self.device);
            }
            self.pipeline_format = new.format;
        }
        if let Some(d) = self.depth.take() {
            d.destroy(&self.device);
        }
        self.swapchain = Some(new);
        Ok(())
    }

    fn ensure_resources(&mut self, scene: &Scene) -> Result<()> {
        match scene {
            Scene::Triangle { .. } => {
                if self.triangle.is_none() {
                    self.triangle = Some(TrianglePipeline::new(
                        &self.device.raw,
                        self.pipeline_format,
                    )?);
                }
            }
            Scene::Cube { .. } => {
                if self.world.is_none() {
                    self.world = Some(WorldResources::create(
                        &self.device,
                        self.cmd_pool,
                        self.pipeline_format,
                        DEPTH_FORMAT,
                        FRAMES_IN_FLIGHT as u32,
                    )?);
                }
                if self.depth.is_none() {
                    self.depth = Some(DepthTarget::create(
                        &self.device,
                        self.extent.0,
                        self.extent.1,
                    )?);
                }
            }
            Scene::Clear { .. } => {}
        }
        Ok(())
    }

    pub fn render(&mut self, scene: &Scene, t: f32) -> Result<()> {
        self.ensure_resources(scene)?;

        let frame = self.frame_idx % FRAMES_IN_FLIGHT;
        let in_flight = self.frame_syncs[frame].in_flight;
        let image_available = self.frame_syncs[frame].image_available;
        let cmd = self.cmd_bufs[frame];

        unsafe {
            self.device
                .raw
                .wait_for_fences(std::slice::from_ref(&in_flight), true, u64::MAX)
                .context("wait fence")?;
        }

        let acquire = {
            let sc = self.swapchain.as_ref().unwrap();
            unsafe {
                sc.loader
                    .acquire_next_image(sc.raw, u64::MAX, image_available, vk::Fence::null())
            }
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
                .reset_fences(std::slice::from_ref(&in_flight))
                .context("reset fence")?;
            self.device
                .raw
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .context("reset command buffer")?;
        }

        if let Some(w) = &self.world {
            let aspect = self.extent.0 as f32 / self.extent.1.max(1) as f32;
            let view_proj = perspective(aspect) * view(self.camera_pos, self.camera_target);
            let model = Mat4::from_rotation_y(t * 0.5);
            w.update_globals(&self.device, frame, view_proj, model)?;
        }

        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.device.raw.begin_command_buffer(cmd, &begin) }
            .context("begin command buffer")?;

        let (image, view, extent) = {
            let sc = self.swapchain.as_ref().unwrap();
            (sc.images[image_idx], sc.image_views[image_idx], sc.extent)
        };
        let cycled = cycled_color(scene, t);
        record_pass(
            &self.device.raw,
            cmd,
            image,
            view,
            extent,
            cycled,
            self.triangle.as_ref(),
            self.world.as_ref(),
            self.depth.as_ref(),
            frame,
        );

        unsafe { self.device.raw.end_command_buffer(cmd) }.context("end command buffer")?;

        let render_done = self.image_render_done[image_idx];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(std::slice::from_ref(&image_available))
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(std::slice::from_ref(&cmd))
            .signal_semaphores(std::slice::from_ref(&render_done));
        unsafe {
            self.device
                .raw
                .queue_submit(self.device.queue, std::slice::from_ref(&submit), in_flight)
                .context("queue submit")?;
        }

        let sc_handle = self.swapchain.as_ref().unwrap().raw;
        let image_indices = [image_idx as u32];
        let swapchains = [sc_handle];
        let present = vk::PresentInfoKHR::default()
            .wait_semaphores(std::slice::from_ref(&render_done))
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        let result = unsafe {
            self.swapchain
                .as_ref()
                .unwrap()
                .loader
                .queue_present(self.device.queue, &present)
        };
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

#[allow(clippy::too_many_arguments)]
fn record_pass(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    view: vk::ImageView,
    extent: vk::Extent2D,
    clear: [f32; 4],
    triangle: Option<&TrianglePipeline>,
    world: Option<&WorldResources>,
    depth: Option<&DepthTarget>,
    frame_idx: usize,
) {
    image_barrier(
        device,
        cmd,
        image,
        vk::ImageAspectFlags::COLOR,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::AccessFlags::empty(),
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
    );

    if let Some(d) = depth {
        image_barrier(
            device,
            cmd,
            d.image,
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        );
    }

    if let (Some(w), Some(d)) = (world, depth) {
        w.record_pass(device, cmd, view, d.view, extent, clear, frame_idx);
    } else {
        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue { float32: clear },
        };
        let attachment = vk::RenderingAttachmentInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear_value);
        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };
        let rendering = vk::RenderingInfo::default()
            .render_area(render_area)
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&attachment));
        unsafe {
            device.cmd_begin_rendering(cmd, &rendering);
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(cmd, 0, std::slice::from_ref(&viewport));
            device.cmd_set_scissor(cmd, 0, std::slice::from_ref(&render_area));
            if let Some(tp) = triangle {
                device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, tp.pipeline);
                let push = TrianglePush {
                    tint: [1.0, 1.0, 1.0, 1.0],
                };
                device.cmd_push_constants(
                    cmd,
                    tp.layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    bytemuck::bytes_of(&push),
                );
                device.cmd_draw(cmd, 3, 1, 0, 0);
            }
            device.cmd_end_rendering(cmd);
        }
    }

    image_barrier(
        device,
        cmd,
        image,
        vk::ImageAspectFlags::COLOR,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::PRESENT_SRC_KHR,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        vk::AccessFlags::empty(),
    );
}

#[allow(clippy::too_many_arguments)]
fn image_barrier(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    aspect: vk::ImageAspectFlags,
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
                .aspect_mask(aspect)
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
            if let Some(d) = self.depth.take() {
                d.destroy(&self.device);
            }
            if let Some(w) = self.world.take() {
                w.destroy(&self.device);
            }
            if let Some(tp) = self.triangle.take() {
                tp.destroy(&self.device.raw);
            }
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
