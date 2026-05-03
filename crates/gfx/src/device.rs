use anyhow::{anyhow, Context, Result};
use ash::vk;
use std::ffi::{c_char, CStr};

use crate::instance::Instance;

pub struct Device {
    pub physical: vk::PhysicalDevice,
    pub raw: ash::Device,
    pub queue_family: u32,
    pub queue: vk::Queue,
    pub mem_props: vk::PhysicalDeviceMemoryProperties,
}

pub fn create_headless(instance: &Instance) -> Result<Device> {
    create(instance, None)
}

pub fn create_for_surface(
    instance: &Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<Device> {
    create(instance, Some((surface_loader, surface)))
}

fn create(
    instance: &Instance,
    surface: Option<(&ash::khr::surface::Instance, vk::SurfaceKHR)>,
) -> Result<Device> {
    let physical_devices = unsafe { instance.raw.enumerate_physical_devices() }
        .context("enumerate_physical_devices")?;
    if physical_devices.is_empty() {
        return Err(anyhow!("no Vulkan physical devices"));
    }

    let mut chosen: Option<(vk::PhysicalDevice, u32)> = None;
    for &pd in &physical_devices {
        let qfp = unsafe { instance.raw.get_physical_device_queue_family_properties(pd) };
        for (i, qf) in qfp.iter().enumerate() {
            if !qf.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                continue;
            }
            if let Some((sl, surf)) = surface {
                let supports = unsafe {
                    sl.get_physical_device_surface_support(pd, i as u32, surf)
                        .unwrap_or(false)
                };
                if !supports {
                    continue;
                }
            }
            chosen = Some((pd, i as u32));
            break;
        }
        if chosen.is_some() {
            break;
        }
    }
    let (physical, queue_family) =
        chosen.ok_or_else(|| anyhow!("no graphics queue family with surface support"))?;

    let mut device_exts: Vec<*const c_char> = Vec::new();
    if surface.is_some() {
        device_exts.push(ash::khr::swapchain::NAME.as_ptr());
    }
    device_exts.push(ash::khr::dynamic_rendering::NAME.as_ptr());
    device_exts.push(ash::khr::synchronization2::NAME.as_ptr());

    let supported_exts = unsafe { instance.raw.enumerate_device_extension_properties(physical) }
        .context("enumerate_device_extension_properties")?;
    let portability_subset = c"VK_KHR_portability_subset";
    let has_portability = supported_exts.iter().any(|p| {
        let n = unsafe { CStr::from_ptr(p.extension_name.as_ptr()) };
        n == portability_subset
    });
    if has_portability {
        device_exts.push(portability_subset.as_ptr());
    }

    let priorities = [1.0_f32];
    let queue_infos = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family)
        .queue_priorities(&priorities)];

    let mut sync2 = vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true);
    let mut dyn_rendering =
        vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);

    let features = vk::PhysicalDeviceFeatures::default();
    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_exts)
        .enabled_features(&features)
        .push_next(&mut sync2)
        .push_next(&mut dyn_rendering);

    let raw = unsafe { instance.raw.create_device(physical, &create_info, None) }
        .context("vkCreateDevice")?;
    let queue = unsafe { raw.get_device_queue(queue_family, 0) };
    let mem_props = unsafe { instance.raw.get_physical_device_memory_properties(physical) };

    Ok(Device {
        physical,
        raw,
        queue_family,
        queue,
        mem_props,
    })
}

pub fn find_memory_type(
    mem_props: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> Result<u32> {
    for i in 0..mem_props.memory_type_count {
        if (type_bits & (1 << i)) != 0
            && mem_props.memory_types[i as usize]
                .property_flags
                .contains(flags)
        {
            return Ok(i);
        }
    }
    Err(anyhow!(
        "no memory type for bits=0x{type_bits:x} flags={flags:?}"
    ))
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let _ = self.raw.device_wait_idle();
            self.raw.destroy_device(None);
        }
    }
}
