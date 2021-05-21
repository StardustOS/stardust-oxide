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

/// Returns a pointer to the start of the `.text` section
#[inline]
pub fn text_start() -> *mut u64 {
    extern "C" {
        static mut _text: u64;
    }

    unsafe { &mut _text }
}

/// Allocate kernel stack in BSS
#[no_mangle]
pub static mut stack: [u8; 16384] = [0; 16384];

/// ?
#[no_mangle]
pub static mut xen_features: [u8; XENFEAT_NR_SUBMAPS as usize * 32] =
    [0; XENFEAT_NR_SUBMAPS as usize * 32];
