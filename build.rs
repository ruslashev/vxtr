use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=glfw");
    println!("cargo:rustc-link-lib=vulkan");

    let header = "\
    #define GLFW_INCLUDE_NONE\n\
    #define GLFW_INCLUDE_VULKAN\n\
    #include <GLFW/glfw3.h>";

    let output_file = PathBuf::from(env::var("OUT_DIR").unwrap()).join("glfw_bindings.rs");

    bindgen::Builder::default()
        .header_contents("wrapper.h", header)
        .derive_debug(false)
        .derive_default(true)
        .generate_comments(false)
        .layout_tests(false)
        .merge_extern_blocks(true)
        .parse_callbacks(Box::new(SignedIntMacros))
        .prepend_enum_name(false)
        .generate()
        .expect("failed to generate bindings")
        .write_to_file(output_file)
        .expect("failed to write bindings");
}

/// A modification to `bindgen::CargoCallbacks` that selects integer macros' type to be `i32`
/// when possible. Otherwise they would be `u32`, which is incompatible with `int`s (`i32`).
#[derive(Debug)]
pub struct SignedIntMacros;

impl bindgen::callbacks::ParseCallbacks for SignedIntMacros {
    fn int_macro(&self, _name: &str, value: i64) -> Option<bindgen::callbacks::IntKind> {
        if value >= i32::MIN.into() && value <= i32::MAX.into() {
            Some(bindgen::callbacks::IntKind::I32)
        } else {
            None // default
        }
    }
}
