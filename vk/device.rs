use glfw_sys::*;

use crate::utils::{convert_to_c_ptrs, CheckVkError};
use crate::{Device, Instance, QueueFamilies, QueueFamily, Swapchain, SwapchainSupport};

use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::ptr;

impl Device {
    pub fn new(instance: &Instance, glfw_window: *mut GLFWwindow) -> Self {
        let (phys_device, queue_families, swapchain_support) = get_phys_device(instance);
        let device = create_logical_device(phys_device, &queue_families);

        println!("Chosen device name: {:?}", get_device_name(phys_device));

        Self {
            phys_device,
            device,
            queue_families,
            swapchain_support,
            glfw_window,
        }
    }

    pub fn get_queue(&self, queue_family: QueueFamily) -> Option<VkQueue> {
        let family_idx = match queue_family {
            QueueFamily::Graphics => self.queue_families.graphics?,
            QueueFamily::Compute => self.queue_families.compute?,
            QueueFamily::Transfer => self.queue_families.transfer?,
            QueueFamily::SparseBinding => self.queue_families.sparse_binding?,
            QueueFamily::Protected => self.queue_families.protected?,
            QueueFamily::Present => self.queue_families.present?,
        };

        Some(self.get_queue_for_family_idx(family_idx))
    }

    pub fn create_swapchain(
        &self,
        surface: VkSurfaceKHR,
        verbose: bool,
    ) -> Swapchain {
        let surface_format = choose_swapchain_surface_format(&self.swapchain_support.formats);
        let present_mode =
            choose_swapchain_present_mode(&self.swapchain_support.present_modes, verbose);
        let extent = choose_swapchain_extent(self.glfw_window, self.swapchain_support.capabilities);

        let max_image_count = self.swapchain_support.capabilities.maxImageCount;
        let mut image_count = self.swapchain_support.capabilities.minImageCount + 1;

        if image_count > max_image_count && max_image_count != 0 {
            image_count = max_image_count;
        }

        let gfx_idx = self.queue_families.graphics.unwrap();
        let present_idx = self.queue_families.present.unwrap();
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
            preTransform: self.swapchain_support.capabilities.currentTransform,
            compositeAlpha: VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
            presentMode: present_mode,
            clipped: 1,
            oldSwapchain: ptr::null_mut(),
            ..Default::default()
        };

        let raw = unsafe {
            let mut swapchain = MaybeUninit::<VkSwapchainKHR>::uninit();

            vkCreateSwapchainKHR(self.device, &create_info, ptr::null(), swapchain.as_mut_ptr())
                .check_err("create swapchain");

            swapchain.assume_init()
        };

        Swapchain {
            raw,
            format: surface_format.format,
            extent,
            device: self,
        }
    }

    fn get_queue_for_family_idx(&self, family_idx: u32) -> VkQueue {
        let mut queue = MaybeUninit::<VkQueue>::uninit();

        unsafe {
            vkGetDeviceQueue(self.device, family_idx, 0, queue.as_mut_ptr());
            queue.assume_init()
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            vkDestroyDevice(self.device, ptr::null());
        }
    }
}

impl Drop for Swapchain<'_> {
    fn drop(&mut self) {
        unsafe {
            vkDestroySwapchainKHR(self.device.device, self.raw, ptr::null());
        }
    }
}

fn get_phys_device(instance: &Instance) -> (VkPhysicalDevice, QueueFamilies, SwapchainSupport) {
    let devices = unsafe {
        let mut count = 0;
        vkEnumeratePhysicalDevices(instance.as_raw(), &mut count, ptr::null_mut());

        assert!(count > 0, "No Vulkan-capable GPU found");

        let mut devices = Vec::with_capacity(count as usize);
        devices.resize(count as usize, ptr::null_mut());

        vkEnumeratePhysicalDevices(instance.as_raw(), &mut count, devices.as_mut_ptr());

        devices
    };

    print_devices(&devices, false);

    choose_phys_device(&devices, instance.surface)
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

fn get_vk_api_version(version: u32) -> (u32, u32, u32, u32) {
    let variant = version >> 29;
    let major = (version >> 22) & 0x7f;
    let minor = (version >> 12) & 0x3ff;
    let patch = version & 0xfff;

    (variant, major, minor, patch)
}

fn print_device_features(f: &VkPhysicalDeviceFeatures) {
    println!("Device features:");

    let features = format!("{:#?}", f);
    let indented = features.lines().map(|line| "\t".to_owned() + line + "\n").collect::<String>();

    print!("{}", indented);
}

fn choose_phys_device(
    phys_devices: &[VkPhysicalDevice],
    surface: VkSurfaceKHR,
) -> (VkPhysicalDevice, QueueFamilies, SwapchainSupport) {
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
            if let Some((queue_families, swapchain_support)) = is_device_suitable(device, surface) {
                return (device, queue_families, swapchain_support);
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

fn is_device_suitable(
    phys_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
) -> Option<(QueueFamilies, SwapchainSupport)> {
    let queue_families = get_queue_families(phys_device, surface);

    if queue_families.graphics.is_none() || queue_families.present.is_none() {
        return None;
    }

    if !supports_required_extensions(phys_device) {
        return None;
    }

    let swapchain_support = query_swapchain_support(phys_device, surface);

    if swapchain_support.formats.is_empty() || swapchain_support.present_modes.is_empty() {
        return None;
    }

    Some((queue_families, swapchain_support))
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

    print_queue_families(&family_properties);

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

fn print_queue_families(family_properties: &[VkQueueFamilyProperties]) {
    println!("Queue families:");

    for f in family_properties {
        print!("\tFlags: ");

        if f.queueFlags & VK_QUEUE_GRAPHICS_BIT != 0 {
            print!("graphics ");
        }
        if f.queueFlags & VK_QUEUE_COMPUTE_BIT != 0 {
            print!("compute");
        }
        if f.queueFlags & VK_QUEUE_TRANSFER_BIT != 0 {
            print!("transfer");
        }
        if f.queueFlags & VK_QUEUE_SPARSE_BINDING_BIT != 0 {
            print!("sparse_binding");
        }
        if f.queueFlags & VK_QUEUE_PROTECTED_BIT != 0 {
            print!("protected");
        }

        println!();

        println!("\tCount: {}", f.queueCount);
        println!("\tTimestamp bits: {}", f.timestampValidBits);
        println!(
            "\tMin image transfer: {}x{}x{}",
            f.minImageTransferGranularity.width,
            f.minImageTransferGranularity.height,
            f.minImageTransferGranularity.depth
        );
    }
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

fn query_swapchain_support(
    phys_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
) -> SwapchainSupport {
    let mut details = SwapchainSupport::default();

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

fn print_validation_layers(layers: &[VkLayerProperties]) {
    println!("Validation layers:");

    for layer in layers {
        let cstr = unsafe { CStr::from_ptr(layer.layerName.as_ptr()) };

        println!("\t{:?}", cstr);
    }
}

fn get_device_name(phys_device: VkPhysicalDevice) -> String {
    let properties = get_device_properties(phys_device);
    let cstr = unsafe { CStr::from_ptr(properties.deviceName.as_ptr()) };

    cstr.to_str().expect("invalid device name").to_string()
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
