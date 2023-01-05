#![allow(
    clippy::similar_names,
    clippy::wildcard_imports,
    clippy::must_use_candidate,
    clippy::uninlined_format_args
)]

use glfw_sys::*;

mod device;
mod instance;
mod utils;

pub struct Instance {
    raw: VkInstance,
    surface: VkSurfaceKHR,
}

pub struct Device {
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    queue_families: QueueFamilies,
    swapchain_support: SwapchainSupport,
}

pub enum QueueFamily {
    Graphics,
    Compute,
    Transfer,
    SparseBinding,
    Protected,
    Present,
}

#[derive(Default)]
struct QueueFamilies {
    graphics: Option<u32>,
    compute: Option<u32>,
    transfer: Option<u32>,
    sparse_binding: Option<u32>,
    protected: Option<u32>,
    present: Option<u32>,
}

#[derive(Default)]
struct SwapchainSupport {
    capabilities: VkSurfaceCapabilitiesKHR,
    formats: Vec<VkSurfaceFormatKHR>,
    present_modes: Vec<VkPresentModeKHR>,
}
