use glfw_sys::{VkResult, VK_SUCCESS};
use std::ffi::{c_char, CString};

pub(crate) trait CheckVkError {
    fn check_err(self, action: &'static str);
}

impl CheckVkError for VkResult {
    fn check_err(self, action: &'static str) {
        assert!(self == VK_SUCCESS, "Failed to {}: err = {}", action, self);
    }
}

pub(crate) fn convert_to_c_ptrs(cstrings: &[CString]) -> Vec<*const c_char> {
    cstrings.iter().map(|cstring| cstring.as_c_str().as_ptr()).collect()
}

#[allow(clippy::cast_precision_loss)]
pub fn u32_to_f32_nowarn(x: u32) -> f32 {
    let mantissa = x & 0x007f_ffff; // 23 set bits
    mantissa as f32
}
