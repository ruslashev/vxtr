#![allow(
    clippy::similar_names,
    clippy::wildcard_imports,
    clippy::must_use_candidate,
    clippy::uninlined_format_args
)]

use std::ffi::{c_char, CString};

use glfw_sys::*;

mod instance;

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

fn convert_to_c_ptrs(cstrings: &[CString]) -> Vec<*const c_char> {
    cstrings.iter().map(|cstring| cstring.as_c_str().as_ptr()).collect()
}
