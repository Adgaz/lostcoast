use anyhow::{anyhow, Context, Result};
use ash::vk;

pub struct Swapchain {
    pub loader: ash::khr::swapchain::Device,
    pub raw: vk::SwapchainKHR,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

#[allow(clippy::too_many_arguments)]
pub fn create(
    instance: &ash::Instance,
    device: &ash::Device,
    physical: vk::PhysicalDevice,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    queue_family: u32,
    requested: (u32, u32),
    old: Option<vk::SwapchainKHR>,
) -> Result<Swapchain> {
    let caps =
        unsafe { surface_loader.get_physical_device_surface_capabilities(physical, surface) }
            .context("surface capabilities")?;
    let formats = unsafe { surface_loader.get_physical_device_surface_formats(physical, surface) }
        .context("surface formats")?;
    let _modes =
        unsafe { surface_loader.get_physical_device_surface_present_modes(physical, surface) }
            .context("surface present modes")?;

    let format = pick_format(&formats);
    let extent = pick_extent(&caps, requested);
    let image_count = pick_image_count(&caps);

    let mut min_image_count = image_count;
    if min_image_count < caps.min_image_count {
        min_image_count = caps.min_image_count;
    }

    let info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(min_image_count)
        .image_format(format.format)
        .image_color_space(format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(caps.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(vk::PresentModeKHR::FIFO)
        .clipped(true)
        .old_swapchain(old.unwrap_or(vk::SwapchainKHR::null()));

    let _ = queue_family;

    let loader = ash::khr::swapchain::Device::new(instance, device);
    let raw = unsafe { loader.create_swapchain(&info, None) }.context("create swapchain")?;
    let images = unsafe { loader.get_swapchain_images(raw) }.context("get swapchain images")?;
    let image_views = images
        .iter()
        .map(|&img| {
            let view_info = vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format.format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                );
            unsafe { device.create_image_view(&view_info, None) }.context("create image view")
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Swapchain {
        loader,
        raw,
        format: format.format,
        extent,
        images,
        image_views,
    })
}

pub fn destroy(device: &ash::Device, mut sc: Swapchain) {
    unsafe {
        for v in sc.image_views.drain(..) {
            device.destroy_image_view(v, None);
        }
        sc.loader.destroy_swapchain(sc.raw, None);
    }
}

fn pick_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .copied()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .or_else(|| formats.first().copied())
        .unwrap_or(vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_SRGB,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        })
}

fn pick_extent(caps: &vk::SurfaceCapabilitiesKHR, requested: (u32, u32)) -> vk::Extent2D {
    if caps.current_extent.width != u32::MAX {
        return caps.current_extent;
    }
    vk::Extent2D {
        width: requested
            .0
            .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
        height: requested
            .1
            .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
    }
}

fn pick_image_count(caps: &vk::SurfaceCapabilitiesKHR) -> u32 {
    let mut n = caps.min_image_count + 1;
    if caps.max_image_count > 0 && n > caps.max_image_count {
        n = caps.max_image_count;
    }
    n
}

pub fn _ensure_extent(caps: &vk::SurfaceCapabilitiesKHR, requested: (u32, u32)) -> Result<()> {
    if caps.max_image_extent.width == 0 || caps.max_image_extent.height == 0 {
        return Err(anyhow!("zero-sized surface"));
    }
    let _ = requested;
    Ok(())
}
