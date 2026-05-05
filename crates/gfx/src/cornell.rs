use anyhow::{Context, Result};
use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};

use crate::buffer::{upload_device_local, Buffer};
use crate::camera::Globals;
use crate::device::Device;
use crate::shader;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CornellVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CornellPush {
    pub albedo: [f32; 4],
}

pub struct CornellMesh {
    pub vbuf: Buffer,
    pub ibuf: Buffer,
    pub index_count: u32,
    pub albedo: [f32; 4],
}

impl CornellMesh {
    pub fn destroy(self, device: &Device) {
        self.vbuf.destroy(device);
        self.ibuf.destroy(device);
    }
}

pub struct CornellPipeline {
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub set_layout: vk::DescriptorSetLayout,
}

impl CornellPipeline {
    pub fn new(
        device: &ash::Device,
        color_format: vk::Format,
        depth_format: vk::Format,
    ) -> Result<Self> {
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let set_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None) }
            .context("create cornell set layout")?;

        let push = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<CornellPush>() as u32);
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(&set_layout))
            .push_constant_ranges(std::slice::from_ref(&push));
        let layout = unsafe { device.create_pipeline_layout(&layout_info, None) }
            .context("create cornell pipeline layout")?;

        let vert = shader::load_module(device, shader::CORNELL_VERT_SPV)?;
        let frag = shader::load_module(device, shader::CORNELL_FRAG_SPV)?;

        let entry = c"main";
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert)
                .name(entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag)
                .name(entry),
        ];

        let bindings = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<CornellVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)];
        let attributes = [
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12),
        ];
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);
        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&blend_attachment));
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let formats = [color_format];
        let mut rendering = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&formats)
            .depth_attachment_format(depth_format);

        let info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&raster)
            .multisample_state(&multisample)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blend)
            .dynamic_state(&dynamic)
            .layout(layout)
            .push_next(&mut rendering);

        let pipelines = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&info),
                None,
            )
        }
        .map_err(|(_, e)| anyhow::anyhow!("create cornell pipeline: {e:?}"))?;

        unsafe {
            device.destroy_shader_module(vert, None);
            device.destroy_shader_module(frag, None);
        }

        Ok(Self {
            layout,
            pipeline: pipelines[0],
            set_layout,
        })
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_descriptor_set_layout(self.set_layout, None);
        }
    }
}

pub struct CornellFrame {
    pub ubo: Buffer,
    pub set: vk::DescriptorSet,
}

pub struct CornellResources {
    pub pipeline: CornellPipeline,
    pub pool: vk::DescriptorPool,
    pub frames: Vec<CornellFrame>,
    pub meshes: Vec<CornellMesh>,
}

impl CornellResources {
    pub fn create(
        device: &Device,
        cmd_pool: vk::CommandPool,
        color_format: vk::Format,
        depth_format: vk::Format,
        frame_count: u32,
    ) -> Result<Self> {
        let pipeline = CornellPipeline::new(&device.raw, color_format, depth_format)?;

        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: frame_count,
        };
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(frame_count)
            .pool_sizes(std::slice::from_ref(&pool_size));
        let pool = unsafe { device.raw.create_descriptor_pool(&pool_info, None) }
            .context("create cornell pool")?;

        let layouts = vec![pipeline.set_layout; frame_count as usize];
        let alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts);
        let sets = unsafe { device.raw.allocate_descriptor_sets(&alloc) }
            .context("allocate cornell sets")?;

        let mut frames = Vec::with_capacity(frame_count as usize);
        for set in sets {
            let ubo = Buffer::create(
                device,
                std::mem::size_of::<Globals>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            let buf_info = vk::DescriptorBufferInfo::default()
                .buffer(ubo.raw)
                .offset(0)
                .range(std::mem::size_of::<Globals>() as u64);
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buf_info));
            unsafe { device.raw.update_descriptor_sets(&[write], &[]) };
            frames.push(CornellFrame { ubo, set });
        }

        let meshes = build_cornell_meshes(device, cmd_pool)?;

        Ok(Self {
            pipeline,
            pool,
            frames,
            meshes,
        })
    }

    pub fn update_globals(&self, device: &Device, frame_idx: usize, view_proj: Mat4) -> Result<()> {
        let globals = Globals {
            view_proj: view_proj.to_cols_array_2d(),
            model: Mat4::IDENTITY.to_cols_array_2d(),
        };
        self.frames[frame_idx]
            .ubo
            .write_host_visible(device, bytemuck::bytes_of(&globals))?;
        Ok(())
    }

    pub fn record_pass(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        color_view: vk::ImageView,
        depth_view: vk::ImageView,
        extent: vk::Extent2D,
        frame_idx: usize,
    ) {
        let color_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.02, 0.02, 0.02, 1.0],
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
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                std::slice::from_ref(&self.frames[frame_idx].set),
                &[],
            );
            for mesh in &self.meshes {
                let push = CornellPush {
                    albedo: mesh.albedo,
                };
                device.cmd_push_constants(
                    cmd,
                    self.pipeline.layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    bytemuck::bytes_of(&push),
                );
                device.cmd_bind_vertex_buffers(cmd, 0, &[mesh.vbuf.raw], &[0]);
                device.cmd_bind_index_buffer(cmd, mesh.ibuf.raw, 0, vk::IndexType::UINT16);
                device.cmd_draw_indexed(cmd, mesh.index_count, 1, 0, 0, 0);
            }
            device.cmd_end_rendering(cmd);
        }
    }

    pub fn destroy(self, device: &Device) {
        for f in self.frames {
            f.ubo.destroy(device);
        }
        for m in self.meshes {
            m.destroy(device);
        }
        unsafe {
            device.raw.destroy_descriptor_pool(self.pool, None);
        }
        self.pipeline.destroy(&device.raw);
    }
}

fn quad(corners: [Vec3; 4], normal: Vec3) -> (Vec<CornellVertex>, Vec<u16>) {
    let n = normal.to_array();
    let v: Vec<CornellVertex> = corners
        .iter()
        .map(|p| CornellVertex {
            pos: p.to_array(),
            normal: n,
        })
        .collect();
    let i = vec![0, 1, 2, 0, 2, 3];
    (v, i)
}

fn box_mesh(center: Vec3, half: Vec3, rotation_y: f32) -> (Vec<CornellVertex>, Vec<u16>) {
    let rot = Mat3::from_rotation_y(rotation_y);
    let corners = [
        Vec3::new(-half.x, -half.y, -half.z),
        Vec3::new(half.x, -half.y, -half.z),
        Vec3::new(half.x, half.y, -half.z),
        Vec3::new(-half.x, half.y, -half.z),
        Vec3::new(-half.x, -half.y, half.z),
        Vec3::new(half.x, -half.y, half.z),
        Vec3::new(half.x, half.y, half.z),
        Vec3::new(-half.x, half.y, half.z),
    ];
    let world: Vec<Vec3> = corners.iter().map(|c| rot * *c + center).collect();
    let faces: [([usize; 4], Vec3); 6] = [
        ([1, 5, 6, 2], Vec3::X),
        ([4, 0, 3, 7], Vec3::NEG_X),
        ([3, 2, 6, 7], Vec3::Y),
        ([4, 5, 1, 0], Vec3::NEG_Y),
        ([5, 4, 7, 6], Vec3::Z),
        ([0, 1, 2, 3], Vec3::NEG_Z),
    ];
    let mut verts = Vec::with_capacity(24);
    let mut idx = Vec::with_capacity(36);
    for (corners_idx, axis_normal) in faces {
        let n = (rot * axis_normal).normalize();
        let base = verts.len() as u16;
        for &ci in &corners_idx {
            verts.push(CornellVertex {
                pos: world[ci].to_array(),
                normal: n.to_array(),
            });
        }
        idx.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    (verts, idx)
}

fn sphere_mesh(center: Vec3, radius: f32, segs: u32, rings: u32) -> (Vec<CornellVertex>, Vec<u16>) {
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for r in 0..=rings {
        let v = r as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;
        let y = phi.cos();
        let s = phi.sin();
        for s_idx in 0..=segs {
            let u = s_idx as f32 / segs as f32;
            let theta = u * 2.0 * std::f32::consts::PI;
            let nx = s * theta.cos();
            let nz = s * theta.sin();
            let nrm = Vec3::new(nx, y, nz);
            let pos = center + nrm * radius;
            verts.push(CornellVertex {
                pos: pos.to_array(),
                normal: nrm.to_array(),
            });
        }
    }
    let segs1 = segs + 1;
    for r in 0..rings {
        for s_idx in 0..segs {
            let a = (r * segs1 + s_idx) as u16;
            let b = (r * segs1 + s_idx + 1) as u16;
            let c = ((r + 1) * segs1 + s_idx + 1) as u16;
            let d = ((r + 1) * segs1 + s_idx) as u16;
            idx.extend_from_slice(&[a, c, b, a, d, c]);
        }
    }
    (verts, idx)
}

fn upload_mesh(
    device: &Device,
    cmd_pool: vk::CommandPool,
    verts: &[CornellVertex],
    indices: &[u16],
    albedo: [f32; 4],
) -> Result<CornellMesh> {
    let vbuf = upload_device_local(
        device,
        cmd_pool,
        bytemuck::cast_slice(verts),
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )?;
    let ibuf = upload_device_local(
        device,
        cmd_pool,
        bytemuck::cast_slice(indices),
        vk::BufferUsageFlags::INDEX_BUFFER,
    )?;
    Ok(CornellMesh {
        vbuf,
        ibuf,
        index_count: indices.len() as u32,
        albedo,
    })
}

fn build_cornell_meshes(device: &Device, cmd_pool: vk::CommandPool) -> Result<Vec<CornellMesh>> {
    let mut out = Vec::new();
    let white = [0.78, 0.78, 0.78, 1.0];
    let red = [0.78, 0.10, 0.10, 1.0];
    let green = [0.10, 0.55, 0.10, 1.0];

    let (v, i) = quad(
        [
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, -1.0),
        ],
        Vec3::Y,
    );
    out.push(upload_mesh(device, cmd_pool, &v, &i, white)?);

    let (v, i) = quad(
        [
            Vec3::new(-1.0, 2.0, -1.0),
            Vec3::new(1.0, 2.0, -1.0),
            Vec3::new(1.0, 2.0, 1.0),
            Vec3::new(-1.0, 2.0, 1.0),
        ],
        Vec3::NEG_Y,
    );
    out.push(upload_mesh(device, cmd_pool, &v, &i, white)?);

    let (v, i) = quad(
        [
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(1.0, 2.0, -1.0),
            Vec3::new(-1.0, 2.0, -1.0),
        ],
        Vec3::Z,
    );
    out.push(upload_mesh(device, cmd_pool, &v, &i, white)?);

    let (v, i) = quad(
        [
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(-1.0, 2.0, -1.0),
            Vec3::new(-1.0, 2.0, 1.0),
            Vec3::new(-1.0, 0.0, 1.0),
        ],
        Vec3::X,
    );
    out.push(upload_mesh(device, cmd_pool, &v, &i, red)?);

    let (v, i) = quad(
        [
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 2.0, 1.0),
            Vec3::new(1.0, 2.0, -1.0),
        ],
        Vec3::NEG_X,
    );
    out.push(upload_mesh(device, cmd_pool, &v, &i, green)?);

    let (v, i) = box_mesh(Vec3::new(0.35, 0.55, -0.25), Vec3::splat(0.32), 0.32);
    out.push(upload_mesh(device, cmd_pool, &v, &i, white)?);

    let (v, i) = sphere_mesh(Vec3::new(-0.4, 0.35, 0.25), 0.35, 32, 24);
    out.push(upload_mesh(
        device,
        cmd_pool,
        &v,
        &i,
        [0.85, 0.85, 0.85, 1.0],
    )?);

    Ok(out)
}
