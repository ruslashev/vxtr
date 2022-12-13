use glfw_sys::*;

use std::ffi::{c_void, CStr, CString};
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
    WindowResize(i32, i32),
}

#[derive(Debug)]
#[repr(i32)]
pub enum Key {
    Escape = GLFW_KEY_ESCAPE,
    Unknown = GLFW_KEY_UNKNOWN,
}

#[allow(unused)]
#[derive(Clone, Copy)]
pub enum Resolution {
    Windowed(i32, i32),
    Fullscreen,
    FullscreenWithRes(i32, i32),
}

impl Window {
    pub fn new<T: Into<Vec<u8>>>(resolution: Resolution, title: T) -> Self {
        let title_cstr = CString::new(title).unwrap();

        let (width, height, monitor) = unsafe {
            glfwInit();

            match resolution {
                Resolution::Windowed(width, height) => {
                    glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);
                    glfwWindowHint(GLFW_FOCUSED, GLFW_FALSE);

                    (width, height, ptr::null_mut())
                }
                Resolution::Fullscreen => {
                    let monitor = get_monitor();
                    let vidmode = get_video_mode(monitor);

                    (vidmode.width, vidmode.height, monitor)
                }
                Resolution::FullscreenWithRes(width, height) => {
                    let monitor = get_monitor();

                    (width, height, monitor)
                }
            }
        };

        let window = unsafe {
            glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);

            glfwCreateWindow(width, height, title_cstr.as_ptr(), monitor, ptr::null_mut())
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
            glfwSetWindowSizeCallback(self.window, Some(window_size_callback));
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

    pub fn block_until_event() {
        unsafe {
            glfwWaitEvents();
        }
    }

    pub fn current_time() -> f64 {
        unsafe { glfwGetTime() }
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

    push_event_to_window(glfw_window, event);
}

extern "C" fn window_size_callback(glfw_window: *mut GLFWwindow, width: i32, height: i32) {
    let event = Event::WindowResize(width, height);

    push_event_to_window(glfw_window, event);
}

fn push_event_to_window(glfw_window: *mut GLFWwindow, event: Event) {
    unsafe {
        let window_ptr = glfwGetWindowUserPointer(glfw_window).cast::<Window>();

        if let Some(window) = window_ptr.as_mut() {
            window.events.push(event);
        } else {
            println!("push_event_to_window: null window ptr, event = {:?}", event);
        }
    }
}

fn get_monitor() -> *mut GLFWmonitor {
    let monitor = unsafe { glfwGetPrimaryMonitor() };

    if monitor.is_null() {
        glfw_panic("get primary monitor");
    }

    monitor
}

fn get_video_mode(monitor: *mut GLFWmonitor) -> GLFWvidmode {
    let vidmode_ptr = unsafe { glfwGetVideoMode(monitor) };

    if vidmode_ptr.is_null() {
        glfw_panic("get video mode for monitor");
    }

    unsafe { vidmode_ptr.read() }
}

fn glfw_panic(action: &'static str) -> ! {
    let (code, string) = get_glfw_error();

    panic!("Failed to {}: code = {} error = \"{}\"", action, code, string);
}

fn get_glfw_error() -> (i32, String) {
    let mut err_ptr = ptr::null();

    unsafe {
        let code = glfwGetError(&mut err_ptr);
        let cstr = CStr::from_ptr(err_ptr);
        let string = cstr.to_str().expect("failed to convert GLFW error string").to_string();

        (code, string)
    }
}
