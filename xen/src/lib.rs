//! Interface with Xen hypervisor

#![no_std]
// Nightly required for inline assembly, tracking issue: https://github.com/rust-lang/rust/issues/72016
#![feature(asm)]
#![deny(missing_docs)]

use xen_sys::{domid_t, XENFEAT_NR_SUBMAPS};

pub use xen_sys;

pub mod console;
pub mod hypercall;
pub mod memory;
pub mod platform;
pub mod scheduler;
pub mod sections;
pub mod trap;

/// Domain ID of this domain
pub const DOMID_SELF: domid_t = 0x7FF0;

/// Allocate kernel stack in BSS
#[no_mangle]
pub static mut stack: [u8; 262144] = [0; 262144];

/// ?
#[no_mangle]
pub static mut xen_features: [u8; XENFEAT_NR_SUBMAPS as usize * 32] =
    [0; XENFEAT_NR_SUBMAPS as usize * 32];
