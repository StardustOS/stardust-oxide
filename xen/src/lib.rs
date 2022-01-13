//! Interface with Xen hypervisor

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

use {
    core::convert::TryInto,
    xen_sys::{
        __HYPERVISOR_update_va_mapping, domid_t, shared_info, start_info, XENFEAT_NR_SUBMAPS,
    },
};

pub use xen_sys;

pub mod console;
pub mod events;
pub mod hypercall;
pub mod memory;
pub mod mm;
pub mod platform;
pub mod scheduler;
pub mod sections;
pub mod trap;
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

    mm::init_mfn_list(
        unsafe { *START_INFO }
            .mfn_list
            .try_into()
            .expect("Failed to convert u64 to usize"),
    );

    unsafe {
        hypercall!(
            __HYPERVISOR_update_va_mapping,
            SHARED_INFO as u64,
            (*START_INFO).shared_info | 7,
            2u64
        )
    }
    .expect("Failed to map shared info page");
}
