//! Safe grant table operation wrappers

use core::convert::TryInto;

use {
    crate::{
        grant_table::{Error, GrantStatusError},
        hypercall, DOMID_SELF,
    },
    xen_sys::{
        GNTTABOP_dump_table, GNTTABOP_map_grant_ref, GNTTABOP_query_size,
        __HYPERVISOR_grant_table_op, domid_t, gnttab_dump_table_t, gnttab_map_grant_ref_t,
        gnttab_query_size_t, gnttab_setup_table_t, gnttab_unmap_grant_ref_t, GNTMAP_host_map,
        GNTMAP_readonly, GNTTABOP_setup_table, GNTTABOP_unmap_grant_ref,
    },
};

/// Handle to track a mapping created via a grant reference
///
/// Fields are private so that a handle cannot be constructed other than by `map_grant_entry`.
pub struct GrantHandle {
    host_addr: u64,
    handle: u32,
}

impl GrantHandle {
    /// Unmaps the mapped grant reference
    pub fn unmap(&self) -> Result<(), Error> {
        let mut arg = gnttab_unmap_grant_ref_t {
            // INPUT
            host_addr: self.host_addr,
            dev_bus_addr: 0,
            handle: self.handle,
            // OUTPUT
            status: 0,
        };

        unsafe { grant_table_op(GNTTABOP_unmap_grant_ref, &mut arg as *mut _ as u64)? };

        if arg.status != 0 {
            return Err(GrantStatusError::from(arg.status).into());
        }

        Ok(())
    }
}

/// Maps grant entry
pub unsafe fn map_grant_entry(
    address: *const u8,
    reference: u32,
    domain: domid_t,
    readonly: bool,
) -> Result<GrantHandle, Error> {
    let mut arg = gnttab_map_grant_ref_t {
        // INPUT
        host_addr: address as u64,
        flags: GNTMAP_host_map,
        ref_: reference,
        dom: domain,
        // OUTPUT
        status: 0,
        handle: 0,
        dev_bus_addr: 0,
    };

    if readonly {
        arg.flags |= GNTMAP_readonly;
    }

    grant_table_op(GNTTABOP_map_grant_ref, &mut arg as *mut _ as u64)?;

    if arg.status != 0 {
        return Err(GrantStatusError::from(arg.status).into());
    }

    Ok(GrantHandle {
        host_addr: arg.host_addr,
        handle: arg.handle,
    })
}

/// Sets up grant table
pub fn setup_table(domain: domid_t, frames: &mut [u64]) -> Result<(), Error> {
    let mut arg = gnttab_setup_table_t {
        // INPUT
        dom: domain,
        nr_frames: frames
            .len()
            .try_into()
            .expect("Failed to convert usize to u32"),
        frame_list: frames.as_mut_ptr(),
        // OUTPUT
        status: 0,
    };

    unsafe { grant_table_op(GNTTABOP_setup_table, &mut arg as *mut _ as u64)? };

    if arg.status != 0 {
        return Err(GrantStatusError::from(arg.status).into());
    }

    Ok(())
}

/// Dumps contents of grant table to console
pub fn dump_table() -> Result<(), Error> {
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
pub fn query_size() -> Result<(u32, u32), Error> {
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
