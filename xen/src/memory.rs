//! Memory management utilities and balloon driver

use core::convert::TryInto;

use {
    crate::{hypercall, DOMID_SELF},
    log::warn,
    xen_sys::{__HYPERVISOR_memory_op, __HYPERVISOR_mmu_update, domid_t, mmu_update_t},
};

/// Memory operation commands
pub enum Command {
    /// Returns the current memory reservation in pages of the specified domain
    CurrentReservation {
        /// Domain ID
        domain: domid_t,
    },
    /// Returns the maximum memory reservation in pages of the specified domain
    MaximumReservation {
        /// Domain ID
        domain: domid_t,
    },
}

impl Command {
    fn value(&self) -> u32 {
        match self {
            Command::CurrentReservation { .. } => 3,
            Command::MaximumReservation { .. } => 4,
        }
    }
}

/// Perform memory operation
pub fn memory_op(cmd: Command) -> Result<u64, hypercall::Error> {
    match cmd {
        Command::CurrentReservation { domain } | Command::MaximumReservation { domain } => unsafe {
            hypercall!(
                __HYPERVISOR_memory_op,
                cmd.value(),
                (&domain) as *const domid_t as u64
            )
        },
    }
}

/// Get the current number of reserved pages for the current domain
pub fn get_current_pages() -> Result<usize, hypercall::Error> {
    memory_op(Command::CurrentReservation { domain: DOMID_SELF })
        .map(|n| n.try_into().expect("Failed to convert u64 to usize"))
}

/// Get the maximum number of reserved pages for the current domain
pub fn get_max_pages() -> Result<usize, hypercall::Error> {
    memory_op(Command::MaximumReservation { domain: DOMID_SELF })
        .map(|n| n.try_into().expect("Failed to convert u64 to usize"))
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
