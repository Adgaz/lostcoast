use anyhow::{Context, Result};
use ash::vk;

pub struct DescriptorState {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
}

impl DescriptorState {
    pub fn world(device: &ash::Device, set_count: u32) -> Result<Self> {
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let layout = unsafe { device.create_descriptor_set_layout(&layout_info, None) }
            .context("create descriptor set layout")?;

        let sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: set_count,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: set_count,
            },
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(set_count)
            .pool_sizes(&sizes);
        let pool = unsafe { device.create_descriptor_pool(&pool_info, None) }
            .context("create descriptor pool")?;

        Ok(Self { layout, pool })
    }

    pub fn allocate(&self, device: &ash::Device, count: u32) -> Result<Vec<vk::DescriptorSet>> {
        let layouts = vec![self.layout; count as usize];
        let info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(&layouts);
        unsafe { device.allocate_descriptor_sets(&info) }.context("allocate descriptor sets")
    }

    pub fn destroy(self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

pub fn write_world_set(
    device: &ash::Device,
    set: vk::DescriptorSet,
    ubo: vk::Buffer,
    ubo_range: u64,
    view: vk::ImageView,
    sampler: vk::Sampler,
) {
    let buf_info = vk::DescriptorBufferInfo::default()
        .buffer(ubo)
        .offset(0)
        .range(ubo_range);
    let img_info = vk::DescriptorImageInfo::default()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(view)
        .sampler(sampler);
    let writes = [
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(std::slice::from_ref(&buf_info)),
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&img_info)),
    ];
    unsafe { device.update_descriptor_sets(&writes, &[]) };
}
