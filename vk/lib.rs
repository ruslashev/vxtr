#![allow(
    clippy::similar_names,
    clippy::wildcard_imports,
    clippy::must_use_candidate,
    clippy::uninlined_format_args
)]

use glfw_sys::*;

mod device;
mod instance;
mod pipeline;
mod render_pass;
mod shader;
mod swapchain;
mod utils;

pub struct Instance {
    raw: VkInstance,
    surface: VkSurfaceKHR,
    glfw_window: *mut GLFWwindow,
}

pub struct Device {
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    queue_families: QueueFamilies,
    swapchain_support: SwapchainSupport,
}

pub struct Swapchain {
    raw: VkSwapchainKHR,
    format: VkFormat,
    extent: VkExtent2D,
    device: VkDevice,
}

pub struct RenderPass {
    raw: VkRenderPass,
    device: VkDevice,
}

pub struct Shader {
    module: VkShaderModule,
    stage_info: VkPipelineShaderStageCreateInfo,
    device: VkDevice,
}

pub struct PipelineLayout {
    raw: VkPipelineLayout,
    device: VkDevice,
}

pub struct Pipeline {
    raw: VkPipeline,
    device: VkDevice,
}

pub enum QueueFamily {
    Graphics,
    Compute,
    Transfer,
    SparseBinding,
    Protected,
    Present,
}

#[derive(Clone, Copy)]
pub enum ShaderType {
    Vertex,
    Fragment,
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
