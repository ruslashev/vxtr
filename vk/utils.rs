use glfw_sys::*;
use std::ffi::{c_char, CStr, CString};
use std::ptr;

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

pub(crate) fn get_validation_layers(verbose: bool) -> Vec<CString> {
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

    // return supported_layers
    //     .iter()
    //     .map(|layer| unsafe { CStr::from_ptr(layer.layerName.as_ptr()).to_owned() })
    //     .collect();

    let required_names = vec![
        // "VK_LAYER_LUNARG_api_dump",
        "VK_LAYER_MESA_device_select",
        "VK_LAYER_LUNARG_monitor",
        "VK_LAYER_KHRONOS_synchronization2",
        "VK_LAYER_KHRONOS_validation",
    ];

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
        let name = unsafe { CStr::from_ptr(layer.layerName.as_ptr()) };
        let desc = unsafe { CStr::from_ptr(layer.description.as_ptr()) };

        println!("\t{:?}: {:?}", name, desc);
    }
}
