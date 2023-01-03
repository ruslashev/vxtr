use glfw_sys::*;

use std::ffi::{c_char, CStr, CString};
use std::mem::MaybeUninit;
use std::ptr;
use std::str::FromStr;

pub struct Instance {
    raw: VkInstance,
}

trait CheckVkError {
    fn check_err(self, action: &'static str);
}

impl CheckVkError for VkResult {
    fn check_err(self, action: &'static str) {
        assert!(self == VK_SUCCESS, "Failed to {}: err = {}", action, self);
    }
}

impl Instance {
    pub fn create<S>(app_name: S, app_version: (u32, u32, u32)) -> Self
    where
        S: Into<Vec<u8>>,
    {
        let name_cstr = CString::new(app_name).unwrap();
        let (app_major, app_minor, app_patch) = app_version;
        let app_version_int = make_vk_version(app_major, app_minor, app_patch);

        let ver_major = u32::from_str(env!("CARGO_PKG_VERSION_MAJOR")).unwrap();
        let ver_minor = u32::from_str(env!("CARGO_PKG_VERSION_MINOR")).unwrap();
        let ver_patch = u32::from_str(env!("CARGO_PKG_VERSION_PATCH")).unwrap();
        let engine_version_int = make_vk_version(ver_major, ver_minor, ver_patch);

        let api_version = make_vk_api_version(0, 1, 3, 0);

        let app_info = VkApplicationInfo {
            sType: VK_STRUCTURE_TYPE_APPLICATION_INFO,
            pApplicationName: name_cstr.as_ptr(),
            applicationVersion: app_version_int,
            pEngineName: ptr::null(),
            engineVersion: engine_version_int,
            apiVersion: api_version,
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

        let raw = unsafe {
            let mut instance = MaybeUninit::<VkInstance>::uninit();

            vkCreateInstance(&create_info, ptr::null(), instance.as_mut_ptr())
                .check_err("create instance");

            instance.assume_init()
        };

        Self { raw }
    }

    pub fn as_raw(&self) -> VkInstance {
        self.raw
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            vkDestroyInstance(self.raw, ptr::null());
        }
    }
}

fn make_vk_version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 22) | (minor << 12) | patch
}

fn make_vk_api_version(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
    (variant << 29) | (major << 22) | (minor << 12) | patch
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

fn convert_to_c_ptrs(cstrings: &[CString]) -> Vec<*const c_char> {
    cstrings.iter().map(|cstring| cstring.as_c_str().as_ptr()).collect()
}
