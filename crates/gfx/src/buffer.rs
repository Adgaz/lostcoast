use anyhow::{Context, Result};
use ash::vk;

use crate::device::{find_memory_type, Device};

pub struct Buffer {
    pub raw: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: u64,
}

impl Buffer {
    pub fn create(
        device: &Device,
        size: u64,
        usage: vk::BufferUsageFlags,
        flags: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let raw = unsafe { device.raw.create_buffer(&info, None) }.context("create buffer")?;
        let req = unsafe { device.raw.get_buffer_memory_requirements(raw) };
        let mem_type = find_memory_type(&device.mem_props, req.memory_type_bits, flags)?;
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(req.size)
            .memory_type_index(mem_type);
        let memory = unsafe { device.raw.allocate_memory(&alloc, None) }
            .context("allocate buffer memory")?;
        unsafe { device.raw.bind_buffer_memory(raw, memory, 0) }.context("bind buffer memory")?;
        Ok(Self {
            raw,
            memory,
            size: req.size,
        })
    }

    pub fn write_host_visible(&self, device: &Device, bytes: &[u8]) -> Result<()> {
        unsafe {
            let ptr = device
                .raw
                .map_memory(
                    self.memory,
                    0,
                    bytes.len() as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .context("map buffer memory")? as *mut u8;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            device.raw.unmap_memory(self.memory);
        }
        Ok(())
    }

    pub fn destroy(self, device: &Device) {
        unsafe {
            device.raw.destroy_buffer(self.raw, None);
            device.raw.free_memory(self.memory, None);
        }
    }
}

pub fn upload_device_local(
    device: &Device,
    pool: vk::CommandPool,
    bytes: &[u8],
    usage: vk::BufferUsageFlags,
) -> Result<Buffer> {
    let staging = Buffer::create(
        device,
        bytes.len() as u64,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    staging.write_host_visible(device, bytes)?;

    let dst = Buffer::create(
        device,
        bytes.len() as u64,
        usage | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    one_shot(device, pool, |cmd| {
        let copy = vk::BufferCopy::default().size(bytes.len() as u64);
        unsafe {
            device
                .raw
                .cmd_copy_buffer(cmd, staging.raw, dst.raw, std::slice::from_ref(&copy));
        }
    })?;

    staging.destroy(device);
    Ok(dst)
}

pub fn one_shot<F>(device: &Device, pool: vk::CommandPool, f: F) -> Result<()>
where
    F: FnOnce(vk::CommandBuffer),
{
    let alloc = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let cmd =
        unsafe { device.raw.allocate_command_buffers(&alloc) }.context("allocate one-shot cmd")?[0];
    let begin =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    unsafe { device.raw.begin_command_buffer(cmd, &begin) }.context("begin one-shot")?;
    f(cmd);
    unsafe { device.raw.end_command_buffer(cmd) }.context("end one-shot")?;
    let submit = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));
    let fence = unsafe {
        device
            .raw
            .create_fence(&vk::FenceCreateInfo::default(), None)
    }
    .context("create one-shot fence")?;
    unsafe {
        device
            .raw
            .queue_submit(device.queue, std::slice::from_ref(&submit), fence)
            .context("submit one-shot")?;
        device
            .raw
            .wait_for_fences(&[fence], true, u64::MAX)
            .context("wait one-shot fence")?;
        device.raw.destroy_fence(fence, None);
        device
            .raw
            .free_command_buffers(pool, std::slice::from_ref(&cmd));
    }
    Ok(())
}
