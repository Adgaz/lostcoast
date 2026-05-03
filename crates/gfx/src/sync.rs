use anyhow::{Context, Result};
use ash::vk;

pub struct FrameSync {
    pub image_available: vk::Semaphore,
    pub render_finished: vk::Semaphore,
    pub in_flight: vk::Fence,
}

pub fn create_frame_sync(device: &ash::Device, signaled: bool) -> Result<FrameSync> {
    let sem_info = vk::SemaphoreCreateInfo::default();
    let fence_info = vk::FenceCreateInfo::default().flags(if signaled {
        vk::FenceCreateFlags::SIGNALED
    } else {
        vk::FenceCreateFlags::empty()
    });
    let image_available =
        unsafe { device.create_semaphore(&sem_info, None) }.context("create_semaphore")?;
    let render_finished =
        unsafe { device.create_semaphore(&sem_info, None) }.context("create_semaphore")?;
    let in_flight = unsafe { device.create_fence(&fence_info, None) }.context("create_fence")?;
    Ok(FrameSync {
        image_available,
        render_finished,
        in_flight,
    })
}

pub fn destroy_frame_sync(device: &ash::Device, s: FrameSync) {
    unsafe {
        device.destroy_semaphore(s.image_available, None);
        device.destroy_semaphore(s.render_finished, None);
        device.destroy_fence(s.in_flight, None);
    }
}
