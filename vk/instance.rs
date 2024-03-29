use glfw_sys::*;

use crate::utils::{convert_to_c_ptrs, get_validation_layers, CheckVkError};
use crate::Instance;

use std::ffi::{c_char, CStr, CString};
use std::mem::MaybeUninit;
use std::ptr;
use std::str::FromStr;

impl Instance {
    /// Create a new Vulkan Instance.
    ///
    /// # Panics
    ///
    /// Panics if `app_name` contains null byte in the middle.
    pub fn new<S>(app_name: S, app_version: (u32, u32, u32), glfw_window: *mut GLFWwindow) -> Self
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

        print_required_extensions(extension_count, extension_names);

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

        let surface = create_surface(raw, glfw_window);

        Self {
            raw,
            surface,
            glfw_window,
        }
    }

    pub fn as_raw(&self) -> VkInstance {
        self.raw
    }

    pub fn surface(&self) -> VkSurfaceKHR {
        self.surface
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            vkDestroySurfaceKHR(self.raw, self.surface, ptr::null());
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

fn print_required_extensions(count: u32, names: *mut *const c_char) {
    println!("Required extensions:");

    for i in 0..count {
        let cstr = unsafe {
            let ptr = names.add(i as usize).read();
            CStr::from_ptr(ptr)
        };

        println!("\t{:?}", cstr);
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
