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

#[allow(clippy::wildcard_imports)]
use glfw_bindings::*;

use std::ffi::CString;
use std::ptr;

#[allow(unused)]
pub struct Window {
    pub running: bool,

    width: i32,
    height: i32,
    window: *mut GLFWwindow,
}

impl Window {
    pub fn new<T: Into<Vec<u8>>>(width: i32, height: i32, title: T) -> Self {
        let window = unsafe {
            glfwInit();

            glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);
            glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);

            let title_cstr = CString::new(title).unwrap();

            glfwCreateWindow(width, height, title_cstr.as_ptr(), ptr::null_mut(), ptr::null_mut())
        };

        Self {
            running: true,
            width,
            height,
            window,
        }
    }

    pub fn poll_events(&mut self) {
        unsafe {
            glfwPollEvents();

            if glfwWindowShouldClose(self.window) == 1 {
                self.running = false;
            }
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            glfwDestroyWindow(self.window);
            glfwTerminate();
        }
    }
}
