#![allow(
    clippy::similar_names,
    clippy::wildcard_imports,
    clippy::must_use_candidate,
    clippy::uninlined_format_args
)]

use glfw_sys::*;

mod instance;
mod utils;

pub struct Instance {
    raw: VkInstance,
}
