use anyhow::Result;
use ash::vk;
use egui::{ClippedPrimitive, TexturesDelta};
use egui_ash_renderer::{DynamicRendering, Options, Renderer};

use crate::device::Device;
use crate::instance::Instance;

pub struct OverlayDraw {
    pub primitives: Vec<ClippedPrimitive>,
    pub textures_delta: TexturesDelta,
    pub pixels_per_point: f32,
}

pub struct Overlay {
    renderer: Renderer,
}

impl Overlay {
    pub fn new(
        instance: &Instance,
        device: &Device,
        color_format: vk::Format,
        in_flight_frames: usize,
    ) -> Result<Self> {
        let renderer = Renderer::with_default_allocator(
            &instance.raw,
            device.physical,
            device.raw.clone(),
            DynamicRendering {
                color_attachment_format: color_format,
                depth_attachment_format: None,
            },
            Options {
                in_flight_frames,
                enable_depth_test: false,
                enable_depth_write: false,
                srgb_framebuffer: matches!(
                    color_format,
                    vk::Format::B8G8R8A8_SRGB | vk::Format::R8G8B8A8_SRGB
                ),
            },
        )
        .map_err(|e| anyhow::anyhow!("egui renderer init: {e:?}"))?;
        Ok(Self { renderer })
    }

    pub fn upload_textures(
        &mut self,
        device: &Device,
        cmd_pool: vk::CommandPool,
        delta: &TexturesDelta,
    ) -> Result<()> {
        if delta.set.is_empty() {
            return Ok(());
        }
        self.renderer
            .set_textures(device.queue, cmd_pool, &delta.set)
            .map_err(|e| anyhow::anyhow!("egui set_textures: {e:?}"))
    }

    pub fn free_textures(&mut self, delta: &TexturesDelta) -> Result<()> {
        if delta.free.is_empty() {
            return Ok(());
        }
        self.renderer
            .free_textures(&delta.free)
            .map_err(|e| anyhow::anyhow!("egui free_textures: {e:?}"))
    }

    pub fn record(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        color_view: vk::ImageView,
        extent: vk::Extent2D,
        pixels_per_point: f32,
        primitives: &[ClippedPrimitive],
    ) -> Result<()> {
        let attachment = vk::RenderingAttachmentInfo::default()
            .image_view(color_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };
        let info = vk::RenderingInfo::default()
            .render_area(area)
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&attachment));
        unsafe { device.cmd_begin_rendering(cmd, &info) };
        self.renderer
            .cmd_draw(cmd, extent, pixels_per_point, primitives)
            .map_err(|e| anyhow::anyhow!("egui cmd_draw: {e:?}"))?;
        unsafe { device.cmd_end_rendering(cmd) };
        Ok(())
    }
}
