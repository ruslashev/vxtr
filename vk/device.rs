use glfw_sys::*;

use crate::utils::{convert_to_c_ptrs, CheckVkError};
use crate::{Device, Instance};

use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::ptr;

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

impl Device {
    pub fn new(instance: &Instance) -> Self {
        let phys_device = get_phys_device(instance.as_raw(), instance.surface());
        let queue_families = get_queue_families(phys_device, instance.surface());
        let device = create_logical_device(phys_device, &queue_families);

        println!("Chosen device name: {:?}", get_device_name(phys_device));

        Self {
            phys_device,
            device,
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
