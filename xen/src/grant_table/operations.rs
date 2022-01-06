//!

use {
    crate::{hypercall, DOMID_SELF},
    xen_sys::{
        GNTTABOP_copy, GNTTABOP_dump_table, GNTTABOP_map_grant_ref, GNTTABOP_query_size,
        GNTTABOP_setup_table, GNTTABOP_transfer, GNTTABOP_unmap_and_replace,
        GNTTABOP_unmap_grant_ref, __HYPERVISOR_grant_table_op, domid_t, gnttab_dump_table_t,
        gnttab_query_size_t, gnttab_setup_table_t, grant_ref_t,
    },
};

/// Grant entry consisting of a domain-grant reference pair
pub struct GrantEntry {
    ///
    domain: domid_t,
    reference: grant_ref_t,
}

/// Maps grant entry
pub fn map_grant_entry() {
    unimplemented!()
}

/// Unmaps grant entry
pub fn unmap_grant_entry() {
    unimplemented!()
}

/// Sets up grant table
pub fn setup_table() {
    unimplemented!()
}

/// Dumps contents of grant table to console
pub fn dump_table() -> Result<(), hypercall::Error> {
    let mut arg = gnttab_dump_table_t {
        // INPUT
        dom: DOMID_SELF,
        // OUTPUT
        status: 0,
    };

    unsafe { grant_table_op(GNTTABOP_dump_table, &mut arg as *mut _ as u64) }?;

    Ok(())
}

/// Transfers grant entry to foreign domain
pub fn transfer() {
    unimplemented!()
}

/// Performs hypervisor copy operation for either MFNs or grant references
pub fn copy() {
    unimplemented!()
}

/// Queries the current and maximum sizes of the shared grant table
pub fn query_size() -> Result<(u32, u32), hypercall::Error> {
    let mut arg = gnttab_query_size_t {
        dom: DOMID_SELF,
        nr_frames: 0,
        max_nr_frames: 0,
        status: 0,
    };

    unsafe { grant_table_op(GNTTABOP_query_size, &mut arg as *mut _ as u64) }?;

    Ok((arg.nr_frames, arg.max_nr_frames))
}

/// Destroys one or more grant reference mappings then replace the page table entry with one with the supplied machine address
pub fn unmap_and_replace() {
    unimplemented!()
}

/// Performs `grant_table_op` hypercall
unsafe fn grant_table_op(cmd: u32, arg_ptr: u64) -> Result<u64, hypercall::Error> {
    hypercall!(__HYPERVISOR_grant_table_op, cmd, arg_ptr, 1u64)
}
