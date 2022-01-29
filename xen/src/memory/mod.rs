//! Memory utility functions and types
//!
//! Consists of wrapper types representing different kinds of memory locations. The following diagram describes the conversions between them:
//!
//! ```text
//! ┌──────────────────┐           ┌───────────────┐
//! │MachineFrameNumber│◀─────────▶│PageFrameNumber│
//! └──────────────────┘           └───────────────┘
//!      ▲        ▲                        ▲
//!      │        │      ┌─────────┐       │
//!      │        └──────│PageEntry│───┐   │
//!      │               └─────────┘   │   │
//!      │                             ▼   ▼
//!      │                         ┌──────────────┐       ┌──────────────┐
//!      └────────────────────────▶│VirtualAddress│◀─────▶│MachineAddress│
//!                                └──────────────┘       └──────────────┘
//!                                        ▲                      ▲
//!                                        │                      │
//!                                        │                      │
//!                                        ▼                      │
//!                                ┌───────────────┐              │
//!                                │PhysicalAddress│◀─────────────┘
//!                                └───────────────┘
//! ```

use {
    crate::{
        hypercall,
        platform::consts::{L1_PAGETABLE_SHIFT, PAGE_SIZE},
        DOMID_SELF,
    },
    core::convert::TryInto,
    log::warn,
    xen_sys::{__HYPERVISOR_memory_op, __HYPERVISOR_mmu_update, mmu_update_t},
};

pub mod page_table;
mod wrappers;

pub use wrappers::*;

/// Pointer to the beginning of the machine frame number list
///
/// Initialized with null pointer, this is probably really bad and **must** be set to the value of the `mfn_list` field of the start info structure before being used.
static mut MFN_LIST: *mut usize = core::ptr::null_mut();

/// MFN_LIST must be initialized before converting between PageFrameNumber and MachineFrameNumber
pub(crate) fn init_mfn_list(mfn_list_addr: usize) {
    unsafe { MFN_LIST = mfn_list_addr as *mut usize }
}

/// Gives a page frame number after rounding the given address to the next page frame boundary
pub fn pfn_up(phys: PhysicalAddress) -> PageFrameNumber {
    // no pointer arithmetic here, only usage of PFN_UP in mini-os is by passing the result of `to_phys` which casts to unsigned long
    PageFrameNumber((phys.0 + PAGE_SIZE - 1) >> L1_PAGETABLE_SHIFT)
}

/// Memory operation commands
enum Command {
    /// Returns the maximum machine frame number of mapped RAM in this system
    MaximumRamPage = 2,
    /// Returns the current memory reservation in pages of the specified domain
    CurrentReservation = 3,
    /// Returns the maximum memory reservation in pages of the specified domain
    MaximumReservation = 4,
}

/// Perform memory operation
unsafe fn memory_op(cmd: Command, arg: u64) -> Result<u64, hypercall::Error> {
    hypercall!(__HYPERVISOR_memory_op, cmd as u64, arg)
}

/// Gets the current number of reserved pages for the current domain
pub fn get_current_pages() -> Result<usize, hypercall::Error> {
    unsafe {
        memory_op(
            Command::CurrentReservation,
            (&DOMID_SELF) as *const _ as u64,
        )
    }
    .map(|n| n.try_into().expect("Failed to convert u64 to usize"))
}

/// Gets the maximum number of reserved pages for the current domain
pub fn get_max_pages() -> Result<usize, hypercall::Error> {
    unsafe {
        memory_op(
            Command::MaximumReservation,
            (&DOMID_SELF) as *const _ as u64,
        )
    }
    .map(|n| n.try_into().expect("Failed to convert u64 to usize"))
}

/// Gets the maximum machine frame number of mapped RAM in this system
pub fn get_max_machine_frame_number() -> MachineFrameNumber {
    let mfn = unsafe { memory_op(Command::MaximumRamPage, 0) }
        .expect("maximum_ram_page memory operation can never fail");

    MachineFrameNumber(mfn.try_into().expect("Failed to convert u64 to usize"))
}

/// Updates an entry in a page table
pub fn hypervisor_mmu_update(reqs: &[mmu_update_t]) -> Result<(), hypercall::Error> {
    let mut success_count = 0;
    unsafe {
        hypercall!(
            __HYPERVISOR_mmu_update,
            reqs.as_ptr() as u64,
            reqs.len() as u64,
            (&mut success_count) as *mut _ as u64,
            DOMID_SELF
        )
    }?;

    if success_count != reqs.len() {
        warn!(
            "MMU update had different number of successes to number of requests: {} != {}",
            success_count,
            reqs.len(),
        )
    }

    Ok(())
}
