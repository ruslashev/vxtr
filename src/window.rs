use glfw_sys::*;

use std::ffi::{c_void, CString};
use std::ptr;

#[allow(unused)]
pub struct Window {
    pub running: bool,
    width: i32,
    height: i32,
    window: *mut GLFWwindow,
    events: Vec<Event>,
}

#[derive(Debug)]
pub enum Event {
    KeyPress(Key),
    KeyRelease(Key),
}

#[derive(Debug)]
#[repr(i32)]
pub enum Key {
    Escape = GLFW_KEY_ESCAPE,
    Unknown = GLFW_KEY_UNKNOWN,
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

        Window {
            running: true,
            width,
            height,
            window,
            events: Vec::new(),
        }
    }

    pub fn set_callbacks(&mut self) {
        let self_ptr = (self as *mut Self).cast::<c_void>();

        unsafe {
            glfwSetWindowUserPointer(self.window, self_ptr);
            glfwSetKeyCallback(self.window, Some(key_callback));
        }
    }

    pub fn poll_events(&mut self) -> impl Iterator<Item = Event> + '_ {
        unsafe {
            glfwPollEvents();

            if glfwWindowShouldClose(self.window) == 1 {
                self.running = false;
            }
        }

        self.events.drain(..)
    }

    pub fn as_inner(&self) -> *mut GLFWwindow {
        self.window
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

impl Key {
    fn from_i32(num: i32) -> Self {
        match num {
            GLFW_KEY_ESCAPE => Key::Escape,
            _ => Key::Unknown,
        }
    }
}

extern "C" fn key_callback(
    glfw_window: *mut GLFWwindow,
    code: i32,
    _scancode: i32,
    action: i32,
    _mods: i32,
) {
    let key = Key::from_i32(code);

    let event = if action == GLFW_PRESS {
        Event::KeyPress(key)
    } else {
        Event::KeyRelease(key)
    };

    unsafe {
        let window_ptr = glfwGetWindowUserPointer(glfw_window).cast::<Window>();

        if let Some(window) = window_ptr.as_mut() {
            window.events.push(event);
        } else {
            println!("key_callback: null events ptr");
        }
    }
}
