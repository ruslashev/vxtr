#![allow(
    clippy::similar_names,
    clippy::wildcard_imports,
    clippy::must_use_candidate,
    clippy::uninlined_format_args,
    clippy::missing_panics_doc
)]

use glfw_sys::*;

mod buffer;
mod command;
mod device;
mod instance;
mod pipeline;
mod queue;
mod render_pass;
mod shader;
mod swapchain;
mod sync;

pub mod utils;

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

pub struct Queue {
    raw: VkQueue,
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

pub struct Framebuffer {
    raw: VkFramebuffer,
    device: VkDevice,
}

pub struct ImageView {
    raw: VkImageView,
    device: VkDevice,
}

pub struct CommandPool {
    raw: VkCommandPool,
    device: VkDevice,
}

#[derive(Clone)]
pub struct CommandBuffer {
    raw: VkCommandBuffer,
    device: VkDevice,
    cmd_pool: VkCommandPool,
}

pub struct CommandBufferRecording {
    cmd_buf: VkCommandBuffer,
}

pub struct Semaphore {
    raw: VkSemaphore,
    device: VkDevice,
}

pub struct Fence {
    raw: VkFence,
    device: VkDevice,
}

pub struct Buffer {
    buffer: VkBuffer,
    memory: VkDeviceMemory,
    device: VkDevice,
}

#[derive(Clone, Copy)]
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
