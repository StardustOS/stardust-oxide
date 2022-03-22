![Screenshot 2021-05-26 at 14 34 13](https://user-images.githubusercontent.com/8290136/119814015-d8d9bf00-bee1-11eb-877f-d61fba171e02.png)

# `stardust-oxide`

[![CI](https://github.com/StardustOS/stardust-oxide/actions/workflows/ci.yml/badge.svg)](https://github.com/StardustOS/stardust-oxide/actions/workflows/ci.yml) 
[![LoC](https://tokei.rs/b1/github/StardustOS/stardust-oxide)](https://github.com/StardustOS/stardust-oxide)

## Features

* [x] Console driver with [log](https://github.com/rust-lang/log) support
* [x] Page frame mapping and table generation (using [buddy_system_allocator](https://github.com/rcore-os/buddy_system_allocator) as global allocator)
* [x] Grant table creation/destruction
* [x] XenStore interface
* [x] XenBus interface
* [x] Simple async executor
* [x] Network driver using [smoltcp](https://github.com/smoltcp-rs/smoltcp/tree/774b375cb04e694199e27c7b9e36628436a4fac3) for TCP/IP stack

<img width="592" alt="Screenshot 2022-03-09 at 12 15 38" src="https://user-images.githubusercontent.com/8290136/157440273-81c14e0a-d9a4-4462-98c8-8df1a21048e3.png">

## Usage

### Requirements

Building and running `stardust-oxide` requires:

* Rust toolchain (latest nightly)
* C compiler
* Xen hypervisor and associated headers

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

## Structure

This project is a Cargo workspace consisting of three crates.

### `stardust`

Main crate that produces a kernel binary and contains the `start_kernel` entrypoint function.

### `xen`

Contains safe interfaces to the `xen-sys` that could be used to write any guest Xen kernel.

### `xen-sys`

Contains Rust bindings to the Xen C headers using [`bindgen`](https://github.com/rust-lang/rust-bindgen).
