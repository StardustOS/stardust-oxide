use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        // no_std crate so need to disable use of std
        .use_core()
        // use `cty` crate
        .ctypes_prefix("cty")
        // header to generate bindings for
        .header("wrapper.h")
        // override clang target
        .clang_arg("--target=x86_64-pc-linux-gnu")
        .derive_debug(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings");

    let out_path = PathBuf::from(
        env::var("OUT_DIR").expect("Failed to fetch value OUT_DIR environment variable"),
    );

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings to file");
}
