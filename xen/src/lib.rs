//! Interface with Xen hypervisor

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

use {
    crate::memory::{update_va_mapping, PageEntry, TLBFlushFlags, VirtualAddress},
    core::convert::TryInto,
    xen_sys::{domid_t, shared_info, start_info, XENFEAT_NR_SUBMAPS},
};

pub use {delay::Delay, xen_sys};

pub mod console;
mod delay;
pub mod events;
pub mod grant_table;
pub mod hypercall;
pub mod memory;
pub mod platform;
pub mod scheduler;
pub mod sections;
pub mod trap;
pub mod xenbus;
pub mod xenstore;

/// Domain ID of this domain
pub const DOMID_SELF: domid_t = 0x7FF0;

/// ?
#[no_mangle]
pub static mut xen_features: [u8; XENFEAT_NR_SUBMAPS as usize * 32] =
    [0; XENFEAT_NR_SUBMAPS as usize * 32];

/// Xen static startup information
pub static mut START_INFO: *mut start_info = core::ptr::null_mut();

/// Xen dynamic global state information
pub static mut SHARED_INFO: *mut shared_info = core::ptr::null_mut();

/// Map shared info page, initialise start and shared info pointers
pub fn init_info(start_info: *mut start_info) {
    unsafe { START_INFO = start_info };

    memory::init_mfn_list(
        unsafe { *START_INFO }
            .mfn_list
            .try_into()
            .expect("Failed to convert u64 to usize"),
    );

    update_va_mapping(
        VirtualAddress(unsafe { SHARED_INFO } as usize),
        PageEntry(unsafe { (*START_INFO).shared_info } as usize | 7),
        TLBFlushFlags::INVLPG,
    )
    .expect("Failed to map shared info page");
}
