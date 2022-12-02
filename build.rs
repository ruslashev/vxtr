use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=glfw");

    let header = "\
    #define GLFW_INCLUDE_NONE\n\
    #define GLFW_INCLUDE_VULKAN\n\
    #include <GLFW/glfw3.h>";

    let output_file = PathBuf::from(env::var("OUT_DIR").unwrap()).join("glfw_bindings.rs");

    bindgen::Builder::default()
        .header_contents("wrapper.h", header)
        .derive_debug(false)
        .generate_comments(false)
        .layout_tests(false)
        .merge_extern_blocks(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("failed to generate bindings")
        .write_to_file(output_file)
        .expect("failed to write bindings");
}
