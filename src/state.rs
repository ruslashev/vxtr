use glfw_sys::*;

use std::ffi::{c_char, c_void, CStr, CString};
use std::mem::{size_of, MaybeUninit};
use std::ptr;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[allow(clippy::cast_sign_loss)]
const SUBPASS_EXTERNAL: u32 = VK_SUBPASS_EXTERNAL as u32;

macro_rules! c_str {
    ($lit:literal) => {{
        let padded = concat!($lit, "\0").as_bytes();
        CStr::from_bytes_with_nul(padded).unwrap().as_ptr()
    }};
}

trait CheckVkError {
    fn check_err(self, action: &'static str);
}

pub struct State {
    glfw_window: *mut GLFWwindow,
    instance: VkInstance,
    phys_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
    device: VkDevice,
    gfx_queue: VkQueue,
    present_queue: VkQueue,
    swapchain: VkSwapchainKHR,
    extent: VkExtent2D,
    image_views: Vec<VkImageView>,
    render_pass: VkRenderPass,
    pipeline_layout: VkPipelineLayout,
    pipeline: VkPipeline,
    framebuffers: Vec<VkFramebuffer>,
    vertex_buffer: VkBuffer,
    vertex_buffer_memory: VkDeviceMemory,
    index_buffer: VkBuffer,
    index_buffer_memory: VkDeviceMemory,
    index_count: u32,
    command_pool: VkCommandPool,
    command_buffers: Vec<VkCommandBuffer>,
    image_available: Vec<VkSemaphore>,
    render_finished: Vec<VkSemaphore>,
    is_rendering: Vec<VkFence>,
    current_frame: usize,
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
struct SwapchainSupportDetails {
    capabilities: VkSurfaceCapabilitiesKHR,
    formats: Vec<VkSurfaceFormatKHR>,
    present_modes: Vec<VkPresentModeKHR>,
}

#[derive(Clone, Copy)]
enum ShaderType {
    Vertex,
    Fragment,
}

impl State {
    pub fn new(glfw_window: *mut GLFWwindow) -> Self {
        let instance = create_instance();
        let surface = create_surface(instance, glfw_window);
        let phys_device = get_phys_device(instance, surface);
        let queue_families = get_queue_families(phys_device, surface);
        let device = create_logical_device(phys_device, &queue_families);
        let gfx_queue = get_queue_for_family_idx(device, queue_families.graphics.unwrap());
        let present_queue = get_queue_for_family_idx(device, queue_families.present.unwrap());
        let (swapchain, image_format, extent) =
            create_swapchain(glfw_window, phys_device, device, surface, true);
        let swapchain_images = get_swapchain_images(device, swapchain);
        let image_views = create_image_views(device, &swapchain_images, image_format);
        let render_pass = create_render_pass(device, image_format);
        let pipeline_layout = create_pipeline_layout(device);
        let pipeline = create_graphics_pipeline(device, extent, render_pass, pipeline_layout);
        let framebuffers = create_framebuffers(device, &image_views, extent, render_pass);
        let command_pool = create_command_pool(device, queue_families.graphics.unwrap());
        let command_buffers = create_command_buffers(device, command_pool, MAX_FRAMES_IN_FLIGHT);
        let (image_available, render_finished, is_rendering) = create_sync_objects(device);

        let vertices: [f32; 8] = [-0.5, -0.5, 0.5, -0.5, 0.5, 0.5, -0.5, 0.5];

        let (vertex_buffer, vertex_buffer_memory) = create_buffer_of_type(
            phys_device,
            device,
            command_pool,
            gfx_queue,
            VK_BUFFER_USAGE_VERTEX_BUFFER_BIT,
            &vertices,
        );

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        let (index_buffer, index_buffer_memory) = create_buffer_of_type(
            phys_device,
            device,
            command_pool,
            gfx_queue,
            VK_BUFFER_USAGE_INDEX_BUFFER_BIT,
            &indices,
        );

        println!("Chosen device name: {:?}", get_device_name(phys_device));

        Self {
            glfw_window,
            instance,
            phys_device,
            surface,
            device,
            gfx_queue,
            present_queue,
            swapchain,
            extent,
            image_views,
            render_pass,
            pipeline_layout,
            pipeline,
            framebuffers,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            index_count: indices.len().try_into().unwrap(),
            command_pool,
            command_buffers,
            image_available,
            render_finished,
            is_rendering,
            current_frame: 0,
        }
    }

    pub fn present(&mut self) {
        let timeout = u64::MAX;

        let command_buffer = self.command_buffers[self.current_frame];
        let image_available = self.image_available[self.current_frame];
        let render_finished = self.render_finished[self.current_frame];
        let is_rendering = self.is_rendering[self.current_frame];

        let image_index = unsafe {
            vkWaitForFences(self.device, 1, &is_rendering, 1, timeout);

            let mut image_index = 0;

            let result = vkAcquireNextImageKHR(
                self.device,
                self.swapchain,
                timeout,
                image_available,
                ptr::null_mut(),
                &mut image_index,
            );

            if result == VK_ERROR_OUT_OF_DATE_KHR {
                self.recreate_swapchain();
                return;
            }

            if result != VK_SUBOPTIMAL_KHR {
                result.check_err("acquire next image");
            }

            vkResetFences(self.device, 1, &is_rendering);

            image_index
        };

        self.record_commands_to_buffer(command_buffer, self.framebuffers[image_index as usize]);

        let wait_semaphores = [image_available];
        let wait_stages = [VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT];
        let signal_semaphores = [render_finished];

        let submit_info = VkSubmitInfo {
            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            waitSemaphoreCount: 1,
            pWaitSemaphores: wait_semaphores.as_ptr(),
            pWaitDstStageMask: wait_stages.as_ptr(),
            commandBufferCount: 1,
            pCommandBuffers: &command_buffer,
            signalSemaphoreCount: 1,
            pSignalSemaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            vkQueueSubmit(self.gfx_queue, 1, &submit_info, is_rendering)
                .check_err("submit to draw queue");
        }

        let swapchains = [self.swapchain];

        let present_info = VkPresentInfoKHR {
            sType: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
            waitSemaphoreCount: 1,
            pWaitSemaphores: signal_semaphores.as_ptr(),
            swapchainCount: 1,
            pSwapchains: swapchains.as_ptr(),
            pImageIndices: &image_index,
            ..Default::default()
        };

        unsafe {
            vkQueuePresentKHR(self.present_queue, &present_info);
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn record_commands_to_buffer(&self, cmd_buffer: VkCommandBuffer, framebuffer: VkFramebuffer) {
        let begin_info = VkCommandBufferBeginInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            ..Default::default()
        };

        let clear_color = VkClearValue {
            color: VkClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let render_pass_info = VkRenderPassBeginInfo {
            sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            renderPass: self.render_pass,
            framebuffer,
            renderArea: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: self.extent,
            },
            clearValueCount: 1,
            pClearValues: &clear_color,
            ..Default::default()
        };

        let vertex_buffers = [self.vertex_buffer];
        let offsets = [0];

        unsafe {
            vkResetCommandBuffer(cmd_buffer, 0);

            vkBeginCommandBuffer(cmd_buffer, &begin_info)
                .check_err("begin recording to command buffer");

            vkCmdBeginRenderPass(cmd_buffer, &render_pass_info, VK_SUBPASS_CONTENTS_INLINE);

            vkCmdBindPipeline(cmd_buffer, VK_PIPELINE_BIND_POINT_GRAPHICS, self.pipeline);

            vkCmdBindVertexBuffers(cmd_buffer, 0, 1, vertex_buffers.as_ptr(), offsets.as_ptr());

            vkCmdBindIndexBuffer(cmd_buffer, self.index_buffer, 0, VK_INDEX_TYPE_UINT16);

            vkCmdDrawIndexed(cmd_buffer, self.index_count, 1, 0, 0, 0);

            vkCmdEndRenderPass(cmd_buffer);

            vkEndCommandBuffer(cmd_buffer).check_err("end command buffer recording");
        }
    }

    fn recreate_swapchain(&mut self) {
        unsafe {
            vkDeviceWaitIdle(self.device);
        }

        self.cleanup_swapchain();

        let (swapchain, image_format, extent) =
            create_swapchain(self.glfw_window, self.phys_device, self.device, self.surface, false);
        let swapchain_images = get_swapchain_images(self.device, swapchain);
        let image_views = create_image_views(self.device, &swapchain_images, image_format);
        let framebuffers = create_framebuffers(self.device, &image_views, extent, self.render_pass);

        self.swapchain = swapchain;
        self.extent = extent;
        self.image_views = image_views;
        self.framebuffers = framebuffers;
    }

    fn cleanup_swapchain(&self) {
        unsafe {
            for framebuffer in &self.framebuffers {
                vkDestroyFramebuffer(self.device, *framebuffer, ptr::null());
            }

            for image_view in &self.image_views {
                vkDestroyImageView(self.device, *image_view, ptr::null());
            }

            vkDestroySwapchainKHR(self.device, self.swapchain, ptr::null());
        }
    }

    pub fn handle_resize(&mut self, _width: i32, _height: i32) {
        self.recreate_swapchain();
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            vkDeviceWaitIdle(self.device);

            for sem in &self.image_available {
                vkDestroySemaphore(self.device, *sem, ptr::null());
            }

            for sem in &self.render_finished {
                vkDestroySemaphore(self.device, *sem, ptr::null());
            }

            for fence in &self.is_rendering {
                vkDestroyFence(self.device, *fence, ptr::null());
            }

            vkDestroyCommandPool(self.device, self.command_pool, ptr::null());

            self.cleanup_swapchain();

            vkDestroyBuffer(self.device, self.vertex_buffer, ptr::null());
            vkFreeMemory(self.device, self.vertex_buffer_memory, ptr::null());

            vkDestroyBuffer(self.device, self.index_buffer, ptr::null());
            vkFreeMemory(self.device, self.index_buffer_memory, ptr::null());

            vkDestroyPipeline(self.device, self.pipeline, ptr::null());
            vkDestroyPipelineLayout(self.device, self.pipeline_layout, ptr::null());
            vkDestroyRenderPass(self.device, self.render_pass, ptr::null());

            vkDestroyDevice(self.device, ptr::null());
            vkDestroySurfaceKHR(self.instance, self.surface, ptr::null());
            vkDestroyInstance(self.instance, ptr::null());
        }
    }
}

impl CheckVkError for VkResult {
    fn check_err(self, action: &'static str) {
        assert!(self == VK_SUCCESS, "Failed to {}: err = {}", action, self);
    }
}

fn get_validation_layers(verbose: bool) -> Vec<CString> {
    let supported_layers = unsafe {
        let mut count = 0;
        vkEnumerateInstanceLayerProperties(&mut count, ptr::null_mut());

        let mut layers = Vec::with_capacity(count as usize);
        layers.resize(count as usize, VkLayerProperties::default());

        vkEnumerateInstanceLayerProperties(&mut count, layers.as_mut_ptr());

        layers
    };

    if verbose {
        print_validation_layers(&supported_layers);
    }

    let required_names = vec!["VK_LAYER_KHRONOS_validation"];

    // Ensure all required validation layers are supported
    for req_name in &required_names {
        let mut supported = false;

        for supp_layer in &supported_layers {
            let cstr = unsafe { CStr::from_ptr(supp_layer.layerName.as_ptr()) };
            let name = cstr.to_str().expect("invalid layer name");

            if req_name == &name {
                supported = true;
                break;
            }
        }

        assert!(supported, "Required validation layer not found: {:?}", req_name);
    }

    required_names.into_iter().map(|name| CString::new(name).unwrap()).collect()
}

fn convert_to_c_ptrs(cstrings: &[CString]) -> Vec<*const c_char> {
    cstrings.iter().map(|cstring| cstring.as_c_str().as_ptr()).collect()
}

fn make_vk_version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 22) | (minor << 12) | patch
}

fn make_vk_api_version(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
    (variant << 29) | (major << 22) | (minor << 12) | patch
}

fn get_vk_api_version(version: u32) -> (u32, u32, u32, u32) {
    let variant = version >> 29;
    let major = (version >> 22) & 0x7f;
    let minor = (version >> 12) & 0x3ff;
    let patch = version & 0xfff;

    (variant, major, minor, patch)
}

fn create_instance() -> VkInstance {
    let app_info = VkApplicationInfo {
        sType: VK_STRUCTURE_TYPE_APPLICATION_INFO,
        pApplicationName: c_str!("lole"),
        applicationVersion: make_vk_version(1, 0, 0),
        pEngineName: c_str!("jej"),
        engineVersion: make_vk_version(1, 0, 0),
        apiVersion: make_vk_api_version(0, 1, 0, 0),
        pNext: ptr::null(),
    };

    let mut extension_count = 0;
    let extension_names = unsafe { glfwGetRequiredInstanceExtensions(&mut extension_count) };

    print_extensions(extension_count, extension_names);

    let mut create_info = VkInstanceCreateInfo {
        sType: VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
        pApplicationInfo: &app_info,
        enabledExtensionCount: extension_count,
        ppEnabledExtensionNames: extension_names,
        ..Default::default()
    };

    let layers = get_validation_layers(true);
    let c_ptrs = convert_to_c_ptrs(&layers);

    if cfg!(debug_assertions) {
        create_info.enabledLayerCount = c_ptrs.len().try_into().unwrap();
        create_info.ppEnabledLayerNames = c_ptrs.as_ptr();
    }

    let mut instance = MaybeUninit::<VkInstance>::uninit();

    unsafe {
        vkCreateInstance(&create_info, ptr::null(), instance.as_mut_ptr())
            .check_err("create instance");

        instance.assume_init()
    }
}

fn create_surface(instance: VkInstance, glfw_window: *mut GLFWwindow) -> VkSurfaceKHR {
    let mut surface = MaybeUninit::<VkSurfaceKHR>::uninit();

    unsafe {
        glfwCreateWindowSurface(instance, glfw_window, ptr::null(), surface.as_mut_ptr())
            .check_err("create window surface");
        surface.assume_init()
    }
}

fn get_phys_device(instance: VkInstance, surface: VkSurfaceKHR) -> VkPhysicalDevice {
    let devices = unsafe {
        let mut count = 0;
        vkEnumeratePhysicalDevices(instance, &mut count, ptr::null_mut());

        assert!(count > 0, "No Vulkan-capable GPU found");

        let mut devices = Vec::with_capacity(count as usize);
        devices.resize(count as usize, ptr::null_mut());

        vkEnumeratePhysicalDevices(instance, &mut count, devices.as_mut_ptr());

        devices
    };

    print_devices(&devices, false);

    choose_phys_device(&devices, surface)
}

fn choose_phys_device(
    phys_devices: &[VkPhysicalDevice],
    surface: VkSurfaceKHR,
) -> VkPhysicalDevice {
    let mut devices_and_types = Vec::with_capacity(phys_devices.len());

    for dev in phys_devices {
        let properties = get_device_properties(*dev);
        devices_and_types.push((*dev, properties.deviceType));
    }

    let type_priorities = [
        VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU,
        VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU,
        VK_PHYSICAL_DEVICE_TYPE_OTHER,
        VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU,
        VK_PHYSICAL_DEVICE_TYPE_CPU,
    ];

    for type_ in type_priorities {
        if let Some(device) = first_device_of_type(&devices_and_types, type_) {
            if is_device_suitable(device, surface) {
                return device;
            }
        }
    }

    panic!("No suitable GPU found");
}

fn first_device_of_type(
    dt: &[(VkPhysicalDevice, VkPhysicalDeviceType)],
    type_predicate: VkPhysicalDeviceType,
) -> Option<VkPhysicalDevice> {
    dt.iter().find(|(_, type_)| *type_ == type_predicate).map(|(dev, _)| *dev)
}

fn is_device_suitable(phys_device: VkPhysicalDevice, surface: VkSurfaceKHR) -> bool {
    let queue_families = get_queue_families(phys_device, surface);
    let extensions_supported = supports_required_extensions(phys_device);
    let swapchain_adequate = if extensions_supported {
        let swapchain_support = query_swapchain_support(phys_device, surface);
        !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
    } else {
        false
    };

    queue_families.graphics.is_some() && queue_families.present.is_some() && swapchain_adequate
}

fn supports_required_extensions(phys_device: VkPhysicalDevice) -> bool {
    let required_extensions = get_required_extensions();

    let mut support_found = Vec::with_capacity(required_extensions.len());
    support_found.resize(required_extensions.len(), false);

    let supported_extensions = get_supported_extensions(phys_device);

    for (i, req_ext) in required_extensions.into_iter().enumerate() {
        for supp_ext in &supported_extensions {
            let supp = unsafe { CStr::from_ptr(supp_ext.extensionName.as_ptr()) };

            if supp == req_ext.as_c_str() {
                support_found[i] = true;
            }
        }
    }

    support_found.into_iter().all(|found| found)
}

fn get_required_extensions() -> Vec<CString> {
    let required_extensions = [VK_KHR_SWAPCHAIN_EXTENSION_NAME];

    required_extensions
        .into_iter()
        .map(|arr| CString::from_vec_with_nul(arr.to_vec()).unwrap())
        .collect()
}

fn get_supported_extensions(phys_device: VkPhysicalDevice) -> Vec<VkExtensionProperties> {
    unsafe {
        let mut count = 0;
        vkEnumerateDeviceExtensionProperties(phys_device, ptr::null(), &mut count, ptr::null_mut())
            .check_err("get number of supported extensions");

        let mut extensions = Vec::with_capacity(count as usize);
        extensions.resize(count as usize, VkExtensionProperties::default());

        vkEnumerateDeviceExtensionProperties(
            phys_device,
            ptr::null(),
            &mut count,
            extensions.as_mut_ptr(),
        )
        .check_err("get supported extensions");

        extensions
    }
}

fn get_device_properties(phys_device: VkPhysicalDevice) -> VkPhysicalDeviceProperties {
    unsafe {
        let mut p = MaybeUninit::<VkPhysicalDeviceProperties>::uninit();
        vkGetPhysicalDeviceProperties(phys_device, p.as_mut_ptr());
        p.assume_init()
    }
}

fn get_device_features(phys_device: VkPhysicalDevice) -> VkPhysicalDeviceFeatures {
    unsafe {
        let mut f = MaybeUninit::<VkPhysicalDeviceFeatures>::uninit();
        vkGetPhysicalDeviceFeatures(phys_device, f.as_mut_ptr());
        f.assume_init()
    }
}

fn get_device_name(phys_device: VkPhysicalDevice) -> String {
    let properties = get_device_properties(phys_device);
    let cstr = unsafe { CStr::from_ptr(properties.deviceName.as_ptr()) };

    cstr.to_str().expect("invalid device name").to_string()
}

fn get_queue_families(phys_device: VkPhysicalDevice, surface: VkSurfaceKHR) -> QueueFamilies {
    let mut families = QueueFamilies::default();

    let family_properties = unsafe {
        let mut count = 0;
        vkGetPhysicalDeviceQueueFamilyProperties(phys_device, &mut count, ptr::null_mut());

        let mut families = Vec::with_capacity(count as usize);
        families.resize(count as usize, VkQueueFamilyProperties::default());

        vkGetPhysicalDeviceQueueFamilyProperties(phys_device, &mut count, families.as_mut_ptr());

        families
    };

    for (i, f) in family_properties.iter().enumerate() {
        let idx: u32 = i.try_into().unwrap();
        let opt = Some(idx);

        if f.queueFlags & VK_QUEUE_GRAPHICS_BIT != 0 {
            families.graphics = opt;
        }
        if f.queueFlags & VK_QUEUE_COMPUTE_BIT != 0 {
            families.compute = opt;
        }
        if f.queueFlags & VK_QUEUE_TRANSFER_BIT != 0 {
            families.transfer = opt;
        }
        if f.queueFlags & VK_QUEUE_SPARSE_BINDING_BIT != 0 {
            families.sparse_binding = opt;
        }
        if f.queueFlags & VK_QUEUE_PROTECTED_BIT != 0 {
            families.protected = opt;
        }

        let mut present_support = 0;
        unsafe {
            vkGetPhysicalDeviceSurfaceSupportKHR(phys_device, idx, surface, &mut present_support)
                .check_err("get surface presentation support");
        }

        if present_support != 0 {
            families.present = opt;
        }
    }

    families
}

fn create_logical_device(
    phys_device: VkPhysicalDevice,
    queue_families: &QueueFamilies,
) -> VkDevice {
    let queue_create_infos = get_queue_create_infos(queue_families);

    let enabled_features = VkPhysicalDeviceFeatures::default();

    let required_extensions = get_required_extensions();
    let req_exts_c_ptrs = convert_to_c_ptrs(&required_extensions);

    let mut create_info = VkDeviceCreateInfo {
        sType: VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
        pEnabledFeatures: &enabled_features,
        queueCreateInfoCount: queue_create_infos.len().try_into().unwrap(),
        pQueueCreateInfos: queue_create_infos.as_ptr(),
        enabledExtensionCount: req_exts_c_ptrs.len().try_into().unwrap(),
        ppEnabledExtensionNames: req_exts_c_ptrs.as_ptr(),
        ..Default::default()
    };

    let layers = get_validation_layers(false);
    let c_ptrs = convert_to_c_ptrs(&layers);

    if cfg!(debug_assertions) {
        create_info.enabledLayerCount = c_ptrs.len().try_into().unwrap();
        create_info.ppEnabledLayerNames = c_ptrs.as_ptr();
    }

    let mut device = MaybeUninit::<VkDevice>::uninit();

    unsafe {
        vkCreateDevice(phys_device, &create_info, ptr::null(), device.as_mut_ptr())
            .check_err("create logical device");

        device.assume_init()
    }
}

fn get_queue_create_infos(queue_families: &QueueFamilies) -> Vec<VkDeviceQueueCreateInfo> {
    let mut queue_create_infos = Vec::new();

    let mut unique_families = vec![
        queue_families.graphics.unwrap(),
        queue_families.present.unwrap(),
    ];

    unique_families.sort_unstable();
    unique_families.dedup();

    for queue_family in unique_families {
        let priority = 1.0;

        let create_info = VkDeviceQueueCreateInfo {
            sType: VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
            queueFamilyIndex: queue_family,
            queueCount: 1,
            pQueuePriorities: &priority,
            ..Default::default()
        };

        queue_create_infos.push(create_info);
    }

    queue_create_infos
}

fn get_queue_for_family_idx(device: VkDevice, family_idx: u32) -> VkQueue {
    let mut queue = MaybeUninit::<VkQueue>::uninit();

    unsafe {
        vkGetDeviceQueue(device, family_idx, 0, queue.as_mut_ptr());
        queue.assume_init()
    }
}

fn query_swapchain_support(
    phys_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
) -> SwapchainSupportDetails {
    let mut details = SwapchainSupportDetails::default();

    unsafe {
        vkGetPhysicalDeviceSurfaceCapabilitiesKHR(phys_device, surface, &mut details.capabilities)
            .check_err("get physical device surface capabilities");
    }

    details.formats = unsafe {
        let mut count = 0;
        vkGetPhysicalDeviceSurfaceFormatsKHR(phys_device, surface, &mut count, ptr::null_mut());

        let mut formats = Vec::new();

        if count > 0 {
            formats.resize(count as usize, VkSurfaceFormatKHR::default());
            vkGetPhysicalDeviceSurfaceFormatsKHR(
                phys_device,
                surface,
                &mut count,
                formats.as_mut_ptr(),
            );
        }

        formats
    };

    details.present_modes = unsafe {
        let mut count = 0;
        vkGetPhysicalDeviceSurfacePresentModesKHR(
            phys_device,
            surface,
            &mut count,
            ptr::null_mut(),
        );

        let mut modes = Vec::new();

        if count > 0 {
            modes.resize(count as usize, VkPresentModeKHR::default());
            vkGetPhysicalDeviceSurfacePresentModesKHR(
                phys_device,
                surface,
                &mut count,
                modes.as_mut_ptr(),
            );
        }

        modes
    };

    details
}

fn choose_swapchain_surface_format(formats: &[VkSurfaceFormatKHR]) -> VkSurfaceFormatKHR {
    for format in formats {
        if format.format == VK_FORMAT_B8G8R8_SRGB
            && format.colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR
        {
            return *format;
        }
    }

    formats[0]
}

fn choose_swapchain_present_mode(
    present_modes: &[VkPresentModeKHR],
    verbose: bool,
) -> VkPresentModeKHR {
    if verbose {
        print_present_modes(present_modes);
    }

    let mode_priorities = [
        VK_PRESENT_MODE_IMMEDIATE_KHR,
        VK_PRESENT_MODE_FIFO_RELAXED_KHR,
        VK_PRESENT_MODE_MAILBOX_KHR,
        VK_PRESENT_MODE_FIFO_KHR,
    ];

    for mode in mode_priorities {
        if present_modes.iter().any(|m| *m == mode) {
            return mode;
        }
    }

    VK_PRESENT_MODE_FIFO_KHR
}

fn choose_swapchain_extent(
    glfw_window: *mut GLFWwindow,
    capabilities: VkSurfaceCapabilitiesKHR,
) -> VkExtent2D {
    if capabilities.currentExtent.width != u32::MAX {
        return capabilities.currentExtent;
    }

    let mut fb_width = 0;
    let mut fb_height = 0;

    unsafe {
        glfwGetFramebufferSize(glfw_window, &mut fb_width, &mut fb_height);
    }

    let fb_width: u32 = fb_width.try_into().unwrap();
    let fb_height: u32 = fb_height.try_into().unwrap();

    let min = capabilities.minImageExtent;
    let max = capabilities.maxImageExtent;

    VkExtent2D {
        width: fb_width.clamp(min.width, max.width),
        height: fb_height.clamp(min.height, max.height),
    }
}

fn create_swapchain(
    glfw_window: *mut GLFWwindow,
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    surface: VkSurfaceKHR,
    verbose: bool,
) -> (VkSwapchainKHR, VkFormat, VkExtent2D) {
    let swapchain_support = query_swapchain_support(phys_device, surface);
    let surface_format = choose_swapchain_surface_format(&swapchain_support.formats);
    let present_mode = choose_swapchain_present_mode(&swapchain_support.present_modes, verbose);
    let extent = choose_swapchain_extent(glfw_window, swapchain_support.capabilities);

    let max_image_count = swapchain_support.capabilities.maxImageCount;
    let mut image_count = swapchain_support.capabilities.minImageCount + 1;

    if image_count > max_image_count && max_image_count != 0 {
        image_count = max_image_count;
    }

    let queue_families = get_queue_families(phys_device, surface);
    let gfx_idx = queue_families.graphics.unwrap();
    let present_idx = queue_families.present.unwrap();
    let indices = [gfx_idx, present_idx];

    let (sharing_mode, qf_idx_count, qf_indices) = if gfx_idx == present_idx {
        (VK_SHARING_MODE_EXCLUSIVE, 0, ptr::null())
    } else {
        (VK_SHARING_MODE_CONCURRENT, 2, indices.as_ptr())
    };

    let create_info = VkSwapchainCreateInfoKHR {
        sType: VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
        surface,
        minImageCount: image_count,
        imageFormat: surface_format.format,
        imageColorSpace: surface_format.colorSpace,
        imageExtent: extent,
        imageArrayLayers: 1,
        imageUsage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
        imageSharingMode: sharing_mode,
        queueFamilyIndexCount: qf_idx_count,
        pQueueFamilyIndices: qf_indices,
        preTransform: swapchain_support.capabilities.currentTransform,
        compositeAlpha: VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
        presentMode: present_mode,
        clipped: 1,
        oldSwapchain: ptr::null_mut(),
        ..Default::default()
    };

    let swapchain = unsafe {
        let mut swapchain = MaybeUninit::<VkSwapchainKHR>::uninit();

        vkCreateSwapchainKHR(device, &create_info, ptr::null(), swapchain.as_mut_ptr())
            .check_err("create swapchain");

        swapchain.assume_init()
    };

    (swapchain, surface_format.format, extent)
}

fn get_swapchain_images(device: VkDevice, swapchain: VkSwapchainKHR) -> Vec<VkImage> {
    unsafe {
        let mut count = 0;
        vkGetSwapchainImagesKHR(device, swapchain, &mut count, ptr::null_mut());

        let mut images = Vec::with_capacity(count as usize);
        images.resize(count as usize, ptr::null_mut());

        vkGetSwapchainImagesKHR(device, swapchain, &mut count, images.as_mut_ptr());

        images
    }
}

fn create_image_views(
    device: VkDevice,
    swapchain_images: &[VkImage],
    image_format: VkFormat,
) -> Vec<VkImageView> {
    let mut image_views = Vec::with_capacity(swapchain_images.len());

    for img in swapchain_images {
        let create_info = VkImageViewCreateInfo {
            sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
            image: *img,
            viewType: VK_IMAGE_VIEW_TYPE_2D,
            format: image_format,
            components: VkComponentMapping {
                r: VK_COMPONENT_SWIZZLE_IDENTITY,
                g: VK_COMPONENT_SWIZZLE_IDENTITY,
                b: VK_COMPONENT_SWIZZLE_IDENTITY,
                a: VK_COMPONENT_SWIZZLE_IDENTITY,
            },
            subresourceRange: VkImageSubresourceRange {
                aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: 0,
                layerCount: 1,
            },
            ..Default::default()
        };

        let image_view = unsafe {
            let mut view = MaybeUninit::<VkImageView>::uninit();
            vkCreateImageView(device, &create_info, ptr::null(), view.as_mut_ptr())
                .check_err("create image view");
            view.assume_init()
        };

        image_views.push(image_view);
    }

    image_views
}

fn create_render_pass(device: VkDevice, image_format: VkFormat) -> VkRenderPass {
    let color_attachment = VkAttachmentDescription {
        format: image_format,
        samples: VK_SAMPLE_COUNT_1_BIT,
        loadOp: VK_ATTACHMENT_LOAD_OP_CLEAR,
        storeOp: VK_ATTACHMENT_STORE_OP_STORE,
        stencilLoadOp: VK_ATTACHMENT_LOAD_OP_DONT_CARE,
        stencilStoreOp: VK_ATTACHMENT_STORE_OP_DONT_CARE,
        initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
        finalLayout: VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
        ..Default::default()
    };

    let color_attachment_ref = VkAttachmentReference {
        attachment: 0,
        layout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
    };

    let subpass_desc = VkSubpassDescription {
        pipelineBindPoint: VK_PIPELINE_BIND_POINT_GRAPHICS,
        colorAttachmentCount: 1,
        pColorAttachments: &color_attachment_ref,
        ..Default::default()
    };

    let subpass_dependency = VkSubpassDependency {
        srcSubpass: SUBPASS_EXTERNAL,
        dstSubpass: 0,
        srcStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        srcAccessMask: 0,
        dstStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
        ..Default::default()
    };

    let create_info = VkRenderPassCreateInfo {
        sType: VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
        attachmentCount: 1,
        pAttachments: &color_attachment,
        subpassCount: 1,
        pSubpasses: &subpass_desc,
        dependencyCount: 1,
        pDependencies: &subpass_dependency,
        ..Default::default()
    };

    unsafe {
        let mut render_pass = MaybeUninit::<VkRenderPass>::uninit();

        vkCreateRenderPass(device, &create_info, ptr::null_mut(), render_pass.as_mut_ptr())
            .check_err("create render pass");

        render_pass.assume_init()
    }
}

fn create_graphics_pipeline(
    device: VkDevice,
    extent: VkExtent2D,
    render_pass: VkRenderPass,
    pipeline_layout: VkPipelineLayout,
) -> VkPipeline {
    let vert_compiled = include_bytes!("../shaders/shader.vert.spv");
    let frag_compiled = include_bytes!("../shaders/shader.frag.spv");

    let vert_shader_mod = create_shader_module(device, vert_compiled);
    let frag_shader_mod = create_shader_module(device, frag_compiled);

    let entrypoint_main = CString::new("main").unwrap();

    let shader_stage_infos = [
        create_shader_stage_info(vert_shader_mod, ShaderType::Vertex, &entrypoint_main),
        create_shader_stage_info(frag_shader_mod, ShaderType::Fragment, &entrypoint_main),
    ];

    let binding_desc = get_binding_description();
    let attr_desc = get_attribute_description();

    let vertex_input = create_pipeline_vertex_input_info(&binding_desc, &attr_desc);

    let input_assembly = create_pipeline_input_assembly();

    let viewport = create_pipeline_viewport(extent);
    let scissor = create_pipeline_scissor(extent);
    let viewport_state = create_static_viewport_state_info(&viewport, &scissor);

    let rasterizer = create_rasterizer_info();

    let multisampling = create_multisampling_info();

    let disabled_blending = create_disabled_blending_attachment();
    let blending = create_blending_info(&disabled_blending);

    let create_info = VkGraphicsPipelineCreateInfo {
        sType: VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
        stageCount: 2,
        pStages: shader_stage_infos.as_ptr(),
        pVertexInputState: &vertex_input,
        pInputAssemblyState: &input_assembly,
        pViewportState: &viewport_state,
        pRasterizationState: &rasterizer,
        pMultisampleState: &multisampling,
        pColorBlendState: &blending,
        layout: pipeline_layout,
        renderPass: render_pass,
        subpass: 0,
        ..Default::default()
    };

    let graphics_pipeline = unsafe {
        let mut pipeline = MaybeUninit::<VkPipeline>::uninit();

        vkCreateGraphicsPipelines(
            device,
            ptr::null_mut(),
            1,
            &create_info,
            ptr::null_mut(),
            pipeline.as_mut_ptr(),
        )
        .check_err("create pipeline");

        pipeline.assume_init()
    };

    unsafe {
        vkDestroyShaderModule(device, vert_shader_mod, ptr::null_mut());
        vkDestroyShaderModule(device, frag_shader_mod, ptr::null_mut());
    }

    graphics_pipeline
}

fn create_shader_module(device: VkDevice, bytes: &[u8]) -> VkShaderModule {
    let transmuted_copy = pack_to_u32s(bytes);

    let create_info = VkShaderModuleCreateInfo {
        sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
        codeSize: bytes.len(),
        pCode: transmuted_copy.as_ptr(),
        ..Default::default()
    };

    unsafe {
        let mut shader_module = MaybeUninit::<VkShaderModule>::uninit();

        vkCreateShaderModule(device, &create_info, ptr::null_mut(), shader_module.as_mut_ptr())
            .check_err("create shader module");

        shader_module.assume_init()
    }
}

fn pack_to_u32s(bytes: &[u8]) -> Vec<u32> {
    assert!(bytes.len() % 4 == 0, "code length must be a multiple of 4");

    bytes
        .chunks_exact(4)
        .map(|chunk| match chunk {
            &[b0, b1, b2, b3] => u32::from_ne_bytes([b0, b1, b2, b3]),
            _ => unreachable!(),
        })
        .collect()
}

fn create_shader_stage_info(
    shader_module: VkShaderModule,
    sh_type: ShaderType,
    entrypoint: &CString,
) -> VkPipelineShaderStageCreateInfo {
    let stage = match &sh_type {
        ShaderType::Vertex => VK_SHADER_STAGE_VERTEX_BIT,
        ShaderType::Fragment => VK_SHADER_STAGE_FRAGMENT_BIT,
    };

    VkPipelineShaderStageCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
        stage,
        module: shader_module,
        pName: entrypoint.as_ptr(),
        ..Default::default()
    }
}

fn get_binding_description() -> VkVertexInputBindingDescription {
    let vec2_stride = 2 * size_of::<f32>();

    VkVertexInputBindingDescription {
        binding: 0,
        stride: vec2_stride.try_into().unwrap(),
        inputRate: VK_VERTEX_INPUT_RATE_VERTEX,
    }
}

fn get_attribute_description() -> VkVertexInputAttributeDescription {
    VkVertexInputAttributeDescription {
        location: 0,
        binding: 0,
        format: VK_FORMAT_R32G32_SFLOAT,
        offset: 0,
    }
}

fn create_pipeline_vertex_input_info(
    binding_desc: &VkVertexInputBindingDescription,
    attr_desc: &VkVertexInputAttributeDescription,
) -> VkPipelineVertexInputStateCreateInfo {
    VkPipelineVertexInputStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
        vertexBindingDescriptionCount: 1,
        pVertexBindingDescriptions: binding_desc,
        vertexAttributeDescriptionCount: 1,
        pVertexAttributeDescriptions: attr_desc,
        ..Default::default()
    }
}

fn create_pipeline_input_assembly() -> VkPipelineInputAssemblyStateCreateInfo {
    VkPipelineInputAssemblyStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
        topology: VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
        primitiveRestartEnable: 0,
        ..Default::default()
    }
}

fn create_pipeline_viewport(extent: VkExtent2D) -> VkViewport {
    VkViewport {
        x: 0.0,
        y: 0.0,
        width: u32_to_f32_nowarn(extent.width),
        height: u32_to_f32_nowarn(extent.height),
        minDepth: 0.0,
        maxDepth: 1.0,
    }
}

#[allow(clippy::cast_precision_loss)]
fn u32_to_f32_nowarn(x: u32) -> f32 {
    let mantissa = x & 0x007f_ffff; // 23 set bits
    mantissa as f32
}

fn create_pipeline_scissor(extent: VkExtent2D) -> VkRect2D {
    VkRect2D {
        offset: VkOffset2D { x: 0, y: 0 },
        extent,
    }
}

fn create_static_viewport_state_info(
    viewport: &VkViewport,
    scissor: &VkRect2D,
) -> VkPipelineViewportStateCreateInfo {
    VkPipelineViewportStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
        viewportCount: 1,
        pViewports: viewport as *const VkViewport,
        scissorCount: 1,
        pScissors: scissor as *const VkRect2D,
        ..Default::default()
    }
}

fn create_rasterizer_info() -> VkPipelineRasterizationStateCreateInfo {
    VkPipelineRasterizationStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
        depthClampEnable: 0,
        rasterizerDiscardEnable: 0,
        polygonMode: VK_POLYGON_MODE_FILL,
        lineWidth: 1.0,
        cullMode: VK_CULL_MODE_BACK_BIT,
        frontFace: VK_FRONT_FACE_CLOCKWISE,
        depthBiasEnable: 0,
        ..Default::default()
    }
}

fn create_multisampling_info() -> VkPipelineMultisampleStateCreateInfo {
    VkPipelineMultisampleStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
        sampleShadingEnable: 0,
        rasterizationSamples: VK_SAMPLE_COUNT_1_BIT,
        minSampleShading: 1.0,
        pSampleMask: ptr::null(),
        alphaToCoverageEnable: 0,
        alphaToOneEnable: 0,
        ..Default::default()
    }
}

fn create_disabled_blending_attachment() -> VkPipelineColorBlendAttachmentState {
    VkPipelineColorBlendAttachmentState {
        colorWriteMask: VK_COLOR_COMPONENT_R_BIT
            | VK_COLOR_COMPONENT_G_BIT
            | VK_COLOR_COMPONENT_B_BIT
            | VK_COLOR_COMPONENT_A_BIT,
        blendEnable: 0,
        ..Default::default()
    }
}

fn create_blending_info(
    attachment: &VkPipelineColorBlendAttachmentState,
) -> VkPipelineColorBlendStateCreateInfo {
    VkPipelineColorBlendStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
        logicOpEnable: 0,
        attachmentCount: 1,
        pAttachments: attachment,
        ..Default::default()
    }
}

fn create_pipeline_layout(device: VkDevice) -> VkPipelineLayout {
    let create_info = VkPipelineLayoutCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
        ..Default::default()
    };

    unsafe {
        let mut layout = MaybeUninit::<VkPipelineLayout>::uninit();

        vkCreatePipelineLayout(device, &create_info, ptr::null_mut(), layout.as_mut_ptr())
            .check_err("create pipeline layout");

        layout.assume_init()
    }
}

fn create_framebuffers(
    device: VkDevice,
    image_views: &[VkImageView],
    extent: VkExtent2D,
    render_pass: VkRenderPass,
) -> Vec<VkFramebuffer> {
    let mut framebuffers = Vec::with_capacity(image_views.len());

    for image_view in image_views {
        let create_info = VkFramebufferCreateInfo {
            sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            renderPass: render_pass,
            attachmentCount: 1,
            pAttachments: image_view,
            width: extent.width,
            height: extent.height,
            layers: 1,
            ..Default::default()
        };

        let framebuffer = unsafe {
            let mut fb = MaybeUninit::<VkFramebuffer>::uninit();

            vkCreateFramebuffer(device, &create_info, ptr::null_mut(), fb.as_mut_ptr())
                .check_err("create framebuffer");

            fb.assume_init()
        };

        framebuffers.push(framebuffer);
    }

    framebuffers
}

fn create_command_pool(device: VkDevice, graphics_queue_family: u32) -> VkCommandPool {
    let create_info = VkCommandPoolCreateInfo {
        sType: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
        flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
        queueFamilyIndex: graphics_queue_family,
        ..Default::default()
    };

    unsafe {
        let mut command_pool = MaybeUninit::<VkCommandPool>::uninit();

        vkCreateCommandPool(device, &create_info, ptr::null(), command_pool.as_mut_ptr())
            .check_err("create command pool");

        command_pool.assume_init()
    }
}

fn create_command_buffers(
    device: VkDevice,
    command_pool: VkCommandPool,
    count: usize,
) -> Vec<VkCommandBuffer> {
    let mut command_buffers = Vec::with_capacity(count);
    command_buffers.resize(count, ptr::null_mut());

    let alloc_info = VkCommandBufferAllocateInfo {
        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
        commandPool: command_pool,
        level: VK_COMMAND_BUFFER_LEVEL_PRIMARY,
        commandBufferCount: count.try_into().unwrap(),
        ..Default::default()
    };

    unsafe {
        vkAllocateCommandBuffers(device, &alloc_info, command_buffers.as_mut_ptr())
            .check_err("allocate command buffer");
    }

    command_buffers
}

fn create_sync_objects(device: VkDevice) -> (Vec<VkSemaphore>, Vec<VkSemaphore>, Vec<VkFence>) {
    let mut image_available = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut render_finished = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut is_rendering = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        image_available.push(create_semaphore(device));
        render_finished.push(create_semaphore(device));
        is_rendering.push(create_fence(device));
    }

    (image_available, render_finished, is_rendering)
}

fn create_semaphore(device: VkDevice) -> VkSemaphore {
    let create_info = VkSemaphoreCreateInfo {
        sType: VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
        ..Default::default()
    };

    unsafe {
        let mut semaphore = MaybeUninit::<VkSemaphore>::uninit();

        vkCreateSemaphore(device, &create_info, ptr::null_mut(), semaphore.as_mut_ptr())
            .check_err("create semaphore");

        semaphore.assume_init()
    }
}

fn create_fence(device: VkDevice) -> VkFence {
    let create_info = VkFenceCreateInfo {
        sType: VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
        flags: VK_FENCE_CREATE_SIGNALED_BIT,
        ..Default::default()
    };

    unsafe {
        let mut fence = MaybeUninit::<VkFence>::uninit();

        vkCreateFence(device, &create_info, ptr::null_mut(), fence.as_mut_ptr())
            .check_err("create fence");

        fence.assume_init()
    }
}

fn create_buffer(
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    size: u64,
    usage: u32,
    properties: u32,
) -> (VkBuffer, VkDeviceMemory) {
    let create_info = VkBufferCreateInfo {
        sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
        size,
        usage,
        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
        ..Default::default()
    };

    let buffer = unsafe {
        let mut buffer = MaybeUninit::<VkBuffer>::uninit();

        vkCreateBuffer(device, &create_info, ptr::null_mut(), buffer.as_mut_ptr())
            .check_err("create buffer");

        buffer.assume_init()
    };

    let mem_requirements = unsafe {
        let mut requirements = MaybeUninit::<VkMemoryRequirements>::uninit();

        vkGetBufferMemoryRequirements(device, buffer, requirements.as_mut_ptr());

        requirements.assume_init()
    };

    let memory_type = find_memory_type(phys_device, mem_requirements.memoryTypeBits, properties)
        .expect("failed to find appropriate memory type");

    let alloc_info = VkMemoryAllocateInfo {
        sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        allocationSize: mem_requirements.size,
        memoryTypeIndex: memory_type,
        ..Default::default()
    };

    let memory = unsafe {
        let mut memory = MaybeUninit::<VkDeviceMemory>::uninit();

        vkAllocateMemory(device, &alloc_info, ptr::null_mut(), memory.as_mut_ptr())
            .check_err("allocate memory");

        memory.assume_init()
    };

    unsafe {
        vkBindBufferMemory(device, buffer, memory, 0);
    }

    (buffer, memory)
}

fn find_memory_type(
    phys_device: VkPhysicalDevice,
    req_type: u32,
    req_properties: u32,
) -> Option<u32> {
    let mem_properties = unsafe {
        let mut properties = MaybeUninit::<VkPhysicalDeviceMemoryProperties>::uninit();
        vkGetPhysicalDeviceMemoryProperties(phys_device, properties.as_mut_ptr());
        properties.assume_init()
    };

    for i in 0..mem_properties.memoryTypeCount {
        if req_type & (1 << i) == 0 {
            continue;
        }

        if mem_properties.memoryTypes[i as usize].propertyFlags & req_properties == 0 {
            continue;
        }

        return Some(i);
    }

    None
}

fn create_buffer_of_type<T: Copy>(
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    command_pool: VkCommandPool,
    queue: VkQueue,
    usage: u32,
    data: &[T],
) -> (VkBuffer, VkDeviceMemory) {
    let size_bytes: u64 = (data.len() * size_of::<T>()).try_into().unwrap();

    let (staging_buffer, staging_memory) = create_buffer(
        phys_device,
        device,
        size_bytes,
        VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
        VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
    );

    upload_to_buffer_memory(device, staging_memory, data);

    let (buffer, memory) = create_buffer(
        phys_device,
        device,
        size_bytes,
        usage | VK_BUFFER_USAGE_TRANSFER_DST_BIT,
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
    );

    copy_buffers(device, command_pool, queue, staging_buffer, buffer, size_bytes);

    unsafe {
        vkDestroyBuffer(device, staging_buffer, ptr::null());
        vkFreeMemory(device, staging_memory, ptr::null());
    }

    (buffer, memory)
}

fn upload_to_buffer_memory<T: Copy>(device: VkDevice, memory: VkDeviceMemory, data: &[T]) {
    let size_bytes: u64 = (data.len() * size_of::<T>()).try_into().unwrap();

    let memory_range = VkMappedMemoryRange {
        sType: VK_STRUCTURE_TYPE_MAPPED_MEMORY_RANGE,
        memory,
        offset: 0,
        size: size_bytes,
        ..Default::default()
    };

    unsafe {
        let ptr: *mut T = ptr::null_mut();
        let mut void_ptr = ptr.cast::<c_void>();

        vkMapMemory(device, memory, 0, size_bytes, 0, &mut void_ptr).check_err("map memory");

        let out_ptr = void_ptr.cast::<T>();

        let slice = std::slice::from_raw_parts_mut(out_ptr, data.len());
        slice.copy_from_slice(data);

        vkFlushMappedMemoryRanges(device, 1, &memory_range).check_err("flush mapped memory");

        vkUnmapMemory(device, memory);
    }
}

fn copy_buffers(
    device: VkDevice,
    command_pool: VkCommandPool,
    queue: VkQueue,
    src: VkBuffer,
    dst: VkBuffer,
    size: u64,
) {
    let cmd_buffer = create_command_buffers(device, command_pool, 1)[0];

    let begin_info = VkCommandBufferBeginInfo {
        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
        flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
        ..Default::default()
    };

    let copy_region = VkBufferCopy {
        size,
        ..Default::default()
    };

    let submit_info = VkSubmitInfo {
        sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
        commandBufferCount: 1,
        pCommandBuffers: &cmd_buffer,
        ..Default::default()
    };

    unsafe {
        vkBeginCommandBuffer(cmd_buffer, &begin_info);

        vkCmdCopyBuffer(cmd_buffer, src, dst, 1, &copy_region);

        vkEndCommandBuffer(cmd_buffer);

        vkQueueSubmit(queue, 1, &submit_info, ptr::null_mut());
        vkQueueWaitIdle(queue);

        vkFreeCommandBuffers(device, command_pool, 1, &cmd_buffer);
    }
}

fn print_devices(phys_devices: &[VkPhysicalDevice], verbose: bool) {
    println!("Devices:");

    for (i, phys_device) in phys_devices.iter().enumerate() {
        let properties = get_device_properties(*phys_device);
        let features = get_device_features(*phys_device);

        print_device_properties(&properties, i, verbose);

        if verbose {
            print_device_features(&features);
        }
    }
}

fn print_device_properties(p: &VkPhysicalDeviceProperties, idx: usize, verbose: bool) {
    println!("Device {} properties:", idx);
    println!("\tAPI version: {} {:?}", p.apiVersion, get_vk_api_version(p.apiVersion));
    println!("\tDriver version: {} ({:#x})", p.driverVersion, p.driverVersion);
    println!("\tVendor ID: {} ({:#x})", p.vendorID, p.vendorID);
    println!("\tDevice ID: {} ({:#x})", p.deviceID, p.deviceID);

    let device_type = match p.deviceType {
        VK_PHYSICAL_DEVICE_TYPE_OTHER => "Other",
        VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU => "Integrated GPU",
        VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU => "Discrete GPU",
        VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU => "Virtual GPU",
        VK_PHYSICAL_DEVICE_TYPE_CPU => "CPU",
        _ => "Unknown",
    };

    println!("\tDevice type: {}", device_type);

    let name = unsafe { CStr::from_ptr(p.deviceName.as_ptr()) };

    println!("\tDevice name: {:?}", name);

    if verbose {
        let limits = format!("{:#?}", p.limits);
        let indented = limits.lines().map(|line| "\t".to_owned() + line + "\n").collect::<String>();

        println!("\tLimits:");
        print!("{}", indented);
    }
}

fn print_device_features(f: &VkPhysicalDeviceFeatures) {
    println!("Device features:");

    let features = format!("{:#?}", f);
    let indented = features.lines().map(|line| "\t".to_owned() + line + "\n").collect::<String>();

    print!("{}", indented);
}

fn print_extensions(count: u32, names: *mut *const c_char) {
    println!("Extensions:");

    for i in 0..count {
        let cstr = unsafe {
            let ptr = names.add(i as usize).read();
            CStr::from_ptr(ptr)
        };

        println!("\t{:?}", cstr);
    }
}

fn print_validation_layers(layers: &[VkLayerProperties]) {
    println!("Validation layers:");

    for layer in layers {
        let cstr = unsafe { CStr::from_ptr(layer.layerName.as_ptr()) };

        println!("\t{:?}", cstr);
    }
}

fn print_present_modes(present_modes: &[VkPresentModeKHR]) {
    println!("Present modes:");

    for mode in present_modes {
        let desc = match *mode {
            VK_PRESENT_MODE_IMMEDIATE_KHR => "Immediate",
            VK_PRESENT_MODE_MAILBOX_KHR => "Mailbox",
            VK_PRESENT_MODE_FIFO_KHR => "FIFO",
            VK_PRESENT_MODE_FIFO_RELAXED_KHR => "FIFO relaxed",
            VK_PRESENT_MODE_SHARED_DEMAND_REFRESH_KHR => "Shared on-demand refresh",
            VK_PRESENT_MODE_SHARED_CONTINUOUS_REFRESH_KHR => "Shared continuous refresh",
            _ => "Unknown",
        };

        println!("\t{}", desc);
    }
}
