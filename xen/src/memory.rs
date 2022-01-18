//! Memory management utilities and balloon driver

use {
    crate::{hypercall, mm::MachineFrameNumber, DOMID_SELF},
    core::convert::TryInto,
    log::warn,
    xen_sys::{__HYPERVISOR_memory_op, __HYPERVISOR_mmu_update, mmu_update_t},
};

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
