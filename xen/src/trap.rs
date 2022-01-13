//! Trap handling

use {crate::hypercall, xen_sys::__HYPERVISOR_set_trap_table};

/// Information for trap handler
#[repr(C)]
pub struct TrapInfo {
    /// Exception vector
    pub vector: u8,
    /// 0-3 privilege level, 4 clear event enable
    pub flags: u8,
    /// Code selector
    pub cs: u16,
    /// Handler function pointer
    pub address: *const (),
}

// *const () is not sync
unsafe impl Sync for TrapInfo {}

/// Registers a trap handler table
pub fn set_trap_table(table: &'static [TrapInfo]) {
    unsafe { hypercall!(__HYPERVISOR_set_trap_table, table.as_ptr() as u64) }
        .expect("Failed to set trap table");
}
