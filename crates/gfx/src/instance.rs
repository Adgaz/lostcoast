use anyhow::{anyhow, Context, Result};
use ash::vk;
use std::ffi::{c_char, c_void, CStr, CString};

pub struct Instance {
    pub entry: ash::Entry,
    pub raw: ash::Instance,
    pub debug: Option<DebugMessenger>,
}

pub struct DebugMessenger {
    pub loader: ash::ext::debug_utils::Instance,
    pub messenger: vk::DebugUtilsMessengerEXT,
}

pub struct InstanceConfig<'a> {
    pub app_name: &'a CStr,
    pub want_validation: bool,
    pub want_surface_exts: Option<&'a [*const c_char]>,
}

pub fn create(cfg: &InstanceConfig) -> Result<Instance> {
    let entry = unsafe { ash::Entry::load() }
        .map_err(|e| anyhow!("vulkan loader: {e}; install MoltenVK / vulkan SDK"))?;

    let app_info = vk::ApplicationInfo::default()
        .application_name(cfg.app_name)
        .application_version(0)
        .engine_name(c"lostcoast")
        .engine_version(0)
        .api_version(vk::API_VERSION_1_3);

    let mut layer_names: Vec<CString> = Vec::new();
    if cfg.want_validation {
        layer_names.push(CString::new("VK_LAYER_KHRONOS_validation").unwrap());
    }
    let layer_ptrs: Vec<*const c_char> = layer_names.iter().map(|n| n.as_ptr()).collect();

    let mut ext_ptrs: Vec<*const c_char> = Vec::new();
    if cfg.want_validation {
        ext_ptrs.push(ash::ext::debug_utils::NAME.as_ptr());
    }
    ext_ptrs.push(ash::khr::portability_enumeration::NAME.as_ptr());
    ext_ptrs.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
    if let Some(extra) = cfg.want_surface_exts {
        ext_ptrs.extend_from_slice(extra);
    }

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&layer_ptrs)
        .enabled_extension_names(&ext_ptrs)
        .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

    let raw = unsafe { entry.create_instance(&create_info, None) }.context("vkCreateInstance")?;

    let debug = if cfg.want_validation {
        Some(create_debug(&entry, &raw)?)
    } else {
        None
    };

    Ok(Instance { entry, raw, debug })
}

fn create_debug(entry: &ash::Entry, instance: &ash::Instance) -> Result<DebugMessenger> {
    let loader = ash::ext::debug_utils::Instance::new(entry, instance);
    let info = vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(debug_callback));
    let messenger = unsafe { loader.create_debug_utils_messenger(&info, None) }
        .context("vkCreateDebugUtilsMessengerEXT")?;
    Ok(DebugMessenger { loader, messenger })
}

unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _types: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user: *mut c_void,
) -> vk::Bool32 {
    let msg = unsafe {
        let d = &*data;
        if d.p_message.is_null() {
            "<no message>".into()
        } else {
            CStr::from_ptr(d.p_message).to_string_lossy().into_owned()
        }
    };
    if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        eprintln!("VALIDATION ERROR: {msg}");
    } else {
        eprintln!("VALIDATION WARNING: {msg}");
    }
    vk::FALSE
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            if let Some(d) = self.debug.take() {
                d.loader.destroy_debug_utils_messenger(d.messenger, None);
            }
            self.raw.destroy_instance(None);
        }
    }
}
