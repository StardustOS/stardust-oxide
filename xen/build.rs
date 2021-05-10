use std::{
    env,
    fs::File,
    io::{Result, Write},
    path::PathBuf,
};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=bootstrap.S");
    println!("cargo:rerun-if-changed=link.x");

    // write out linker script so can be found by linker
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("link.x"))?.write_all(include_bytes!("link.x"))?;
    println!("cargo:rustc-link-search={}", out.display());

    // build bootstrap assembly
    cc::Build::new()
        .file("bootstrap.S")
        .flag("-D__ASSEMBLY__")
        .flag("-m64")
        .flag("-DCONFIG_X86_PAE")
        .flag("-D__XEN_INTERFACE_VERSION__=0x00030203")
        .compile("bootstrap");

    Ok(())
}
