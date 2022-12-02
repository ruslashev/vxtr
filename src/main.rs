#![allow(clippy::wildcard_imports)]

#[allow(clippy::approx_constant)]
#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::default_trait_access)]
#[allow(clippy::semicolon_if_nothing_returned)]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::upper_case_acronyms)]
#[allow(clippy::used_underscore_binding)]
#[allow(clippy::useless_transmute)]
#[allow(dead_code)]
#[allow(improper_ctypes)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
mod glfw_bindings {
    include!(concat!(env!("OUT_DIR"), "/glfw_bindings.rs"));
}

use glfw_bindings::*;

use std::ffi::CString;
use std::ptr;

fn main() {
    unsafe {
        glfwInit();

        glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);

        let title = CString::new("Vulkan window").unwrap();
        let window = glfwCreateWindow(800, 600, title.as_ptr(), ptr::null_mut(), ptr::null_mut());

        let mut extension_count = 0;
        vkEnumerateInstanceExtensionProperties(
            ptr::null_mut(),
            &mut extension_count,
            ptr::null_mut(),
        );

        println!("{extension_count} extensions supported");

        while glfwWindowShouldClose(window) == 0 {
            glfwPollEvents();
        }

        glfwDestroyWindow(window);

        glfwTerminate();
    }
}
