use crate::bindings::*;
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
