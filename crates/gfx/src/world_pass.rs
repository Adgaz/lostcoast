use anyhow::{Context, Result};
use ash::vk;
use glam::{Mat4, Vec3};

use crate::buffer::{upload_device_local, Buffer};
use crate::camera::{perspective, view, Globals};
use crate::descriptor::{write_world_set, DescriptorState};
use crate::device::{find_memory_type, Device};
use crate::mesh::{unit_cube, Mesh};
use crate::texture::{checkerboard_rgba, upload_2d_with_mips, Texture};
use crate::world_pipeline::WorldPipeline;

pub struct DepthTarget {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub format: vk::Format,
}

impl DepthTarget {
    pub fn create(device: &Device, width: u32, height: u32) -> Result<Self> {
        let format = vk::Format::D32_SFLOAT;
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let image =
            unsafe { device.raw.create_image(&info, None) }.context("create depth image")?;
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
            unsafe { device.raw.allocate_memory(&alloc, None) }.context("allocate depth memory")?;
        unsafe { device.raw.bind_image_memory(image, memory, 0) }.context("bind depth memory")?;

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );
        let view = unsafe { device.raw.create_image_view(&view_info, None) }
            .context("create depth view")?;

        Ok(Self {
            image,
            memory,
            view,
            format,
        })
    }

    pub fn destroy(self, device: &Device) {
        unsafe {
            device.raw.destroy_image_view(self.view, None);
            device.raw.destroy_image(self.image, None);
            device.raw.free_memory(self.memory, None);
        }
    }
}

pub struct WorldResources {
    pub pipeline: WorldPipeline,
    pub descriptor: DescriptorState,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    pub texture: Texture,
    pub frames: Vec<FrameResources>,
}

pub struct FrameResources {
    pub ubo: Buffer,
    pub set: vk::DescriptorSet,
}

impl WorldResources {
    pub fn create(
        device: &Device,
        pool: vk::CommandPool,
        color_format: vk::Format,
        depth_format: vk::Format,
        frame_count: u32,
    ) -> Result<Self> {
        let mesh: Mesh = unit_cube();
        let vertex_bytes: &[u8] = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes: &[u8] = bytemuck::cast_slice(&mesh.indices);
        let vertex_buffer = upload_device_local(
            device,
            pool,
            vertex_bytes,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = upload_device_local(
            device,
            pool,
            index_bytes,
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;
        let pixels = checkerboard_rgba(256, 32);
        let texture = upload_2d_with_mips(device, pool, 256, 256, &pixels)?;

        let descriptor = DescriptorState::world(&device.raw, frame_count)?;
        let pipeline =
            WorldPipeline::new(&device.raw, color_format, depth_format, descriptor.layout)?;
        let sets = descriptor.allocate(&device.raw, frame_count)?;

        let mut frames = Vec::with_capacity(frame_count as usize);
        for set in sets {
            let ubo = Buffer::create(
                device,
                std::mem::size_of::<Globals>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            write_world_set(
                &device.raw,
                set,
                ubo.raw,
                std::mem::size_of::<Globals>() as u64,
                texture.view,
                texture.sampler,
            );
            frames.push(FrameResources { ubo, set });
        }

        Ok(Self {
            pipeline,
            descriptor,
            vertex_buffer,
            index_buffer,
            index_count: mesh.indices.len() as u32,
            texture,
            frames,
        })
    }

    pub fn update_globals(
        &self,
        device: &Device,
        frame_idx: usize,
        view_proj: Mat4,
        model: Mat4,
    ) -> Result<()> {
        let globals = Globals {
            view_proj: view_proj.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
        };
        self.frames[frame_idx]
            .ubo
            .write_host_visible(device, bytemuck::bytes_of(&globals))?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_pass(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        color_view: vk::ImageView,
        depth_view: vk::ImageView,
        extent: vk::Extent2D,
        clear_color: [f32; 4],
        frame_idx: usize,
    ) {
        let color_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: clear_color,
            },
        };
        let depth_clear = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };
        let color_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(color_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(color_clear);
        let depth_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(depth_view)
            .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .clear_value(depth_clear);

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };
        let rendering = vk::RenderingInfo::default()
            .render_area(render_area)
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&color_attachment))
            .depth_attachment(&depth_attachment);

        unsafe {
            device.cmd_begin_rendering(cmd, &rendering);
            let viewport = vk::Viewport {
                x: 0.0,
                y: extent.height as f32,
                width: extent.width as f32,
                height: -(extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(cmd, 0, std::slice::from_ref(&viewport));
            device.cmd_set_scissor(cmd, 0, std::slice::from_ref(&render_area));

            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer.raw], &[0]);
            device.cmd_bind_index_buffer(cmd, self.index_buffer.raw, 0, vk::IndexType::UINT16);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                std::slice::from_ref(&self.frames[frame_idx].set),
                &[],
            );
            device.cmd_draw_indexed(cmd, self.index_count, 1, 0, 0, 0);
            device.cmd_end_rendering(cmd);
        }
    }

    pub fn destroy(self, device: &Device) {
        for f in self.frames {
            f.ubo.destroy(device);
        }
        self.texture.destroy(device);
        self.index_buffer.destroy(device);
        self.vertex_buffer.destroy(device);
        self.pipeline.destroy(&device.raw);
        self.descriptor.destroy(&device.raw);
    }
}

pub fn view_proj_for(camera: [f32; 3], look_at: [f32; 3], aspect: f32) -> Mat4 {
    let cam = Vec3::from_array(camera);
    let target = Vec3::from_array(look_at);
    perspective(aspect) * view(cam, target)
}
