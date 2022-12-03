use glfw_sys::*;

use std::ffi::{c_char, CStr, CString};
use std::mem::MaybeUninit;
use std::ptr;

use crate::window::Window;

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
    window: Window,
    instance: VkInstance,
}

impl State {
    pub fn new(window: Window) -> Self {
        let instance = create_instance();

        Self { window, instance }
    }

    pub fn main_loop(&mut self) {
        while self.window.running {
            self.window.poll_events();
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            vkDestroyInstance(self.instance, ptr::null());
        }
    }
}

impl CheckVkError for VkResult {
    fn check_err(self, action: &'static str) {
        assert!(self == VK_SUCCESS, "Failed to {}: err = {}", action, self);
    }
}

fn get_validation_layers() -> Vec<CString> {
    let supported_layers = unsafe {
        let mut count = 0;
        vkEnumerateInstanceLayerProperties(&mut count, ptr::null_mut());

        let mut layers = Vec::with_capacity(count as usize);
        layers.resize(count as usize, VkLayerProperties::default());

        vkEnumerateInstanceLayerProperties(&mut count, layers.as_mut_ptr());

        layers
    };

    let required_names = vec!["VK_LAYER_KHRONOS_validation"];

    print_validation_layers(&supported_layers);

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

fn make_vk_version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 22) | (minor << 12) | patch
}

fn make_vk_api_version(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
    (variant << 29) | (major << 22) | (minor << 12) | patch
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

    let layers = get_validation_layers();
    let c_ptrs =
        layers.iter().map(|cstring| cstring.as_c_str().as_ptr()).collect::<Vec<*const c_char>>();

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
