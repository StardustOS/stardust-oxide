![Screenshot 2021-05-26 at 14 34 13](https://user-images.githubusercontent.com/8290136/119814015-d8d9bf00-bee1-11eb-877f-d61fba171e02.png)

# `stardust-oxide`

[![CI](https://github.com/StardustOS/stardust-oxide/actions/workflows/ci.yml/badge.svg)](https://github.com/StardustOS/stardust-oxide/actions/workflows/ci.yml) 
[![LoC](https://tokei.rs/b1/github/StardustOS/stardust-oxide)](https://github.com/StardustOS/stardust-oxide)

## Usage

### Requirements

Building and running `stardust-oxide` requires:

* Rust toolchain (Nightly)
* a C compiler
* Xen hypervisor and its associated headers

The recommended way to install Rust is with [rustup](https://rustup.rs). The `rust-toolchain` file in the repository root will ensure the correct toolchain version is installed and used (therefore the toolchain and components selected during installation do not matter).

The required packages can be installed on Ubuntu with:

```bash
$ sudo apt install -y clang xen-system-amd64 libxen-dev
```

The host machine must be restarted to boot into the hypervisor with the original Debian/Ubuntu installation now running as the Domain 0 virtual machine.

### Building

Executing `cargo build` in the repository root will build all three crates including the `stardust` kernel binary.

### Running

Executing `cargo run` will call the `run.sh` script which generates the configuration for the Xen virtual machine then uses `xl` to start it.

### Testing

`cargo test` must be executed in the `stardust` crate; a binary will be built and run but after bootstrapping the kernel will execute all discovered unit tests instead of booting normally.

`xen-sys` contains many tests to ensure the correctness of the generated Rust bindings but they cannot be run inside `stardust`. Since the `.cargo/config.toml` overrides the build target, flags passed to `rustc` and enables building the `core` and `compiler_builtins` crates, and this configuration applies to all crates in the workspace it is currently not possible to run the `xen-sys` tests.

## Structure

This project is a Cargo workspace consisting of three crates.

### `stardust`

Main crate that produces a kernel binary and contains the `start_kernel` entrypoint function.

### `xen`

Contains safe interfaces to the `xen-sys` that could be used to write any guest Xen kernel.

### `xen-sys`

Contains Rust bindings to the following C headers using [`bindgen`](https://github.com/rust-lang/rust-bindgen):

* `xen/arch-x86_64.h`
* `xen/arch-x86/xen-x86_64.h`
* `xen/features.h`
* `xen/sched.h`
* `xen/xen.h`
* `xen/io/console.h`

More are likely to be added in the future.
