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
}
