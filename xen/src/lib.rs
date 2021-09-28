//! Interface with Xen hypervisor

#![no_std]
// Nightly required for inline assembly, tracking issue: https://github.com/rust-lang/rust/issues/72016
#![feature(asm)]
#![deny(missing_docs)]

use xen_sys::XENFEAT_NR_SUBMAPS;

pub use xen_sys;

pub mod console;
pub mod hypercall;
pub mod platform;
pub mod scheduler;
pub mod sections;

/// Allocate kernel stack in BSS
#[no_mangle]
pub static mut stack: [u8; 65536] = [0; 65536];

/// ?
#[no_mangle]
pub static mut xen_features: [u8; XENFEAT_NR_SUBMAPS as usize * 32] =
    [0; XENFEAT_NR_SUBMAPS as usize * 32];
