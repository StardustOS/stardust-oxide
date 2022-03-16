//! x86_64 Grant Table

use {
    crate::{
        grant_table::{operations::setup_table, Error},
        memory::{
            get_max_machine_frame_number, hypervisor_mmu_update, page_table::new_frame,
            MachineFrameNumber, PageEntry, PageFrameNumber, PhysicalAddress, VirtualAddress,
        },
        platform::consts::{L1_PAGETABLE_ENTRIES, L1_PROT, PAGE_PRESENT, PAGE_SHIFT, PAGE_SIZE},
        DOMID_SELF, START_INFO,
    },
    alloc::alloc::{alloc, Layout},
    core::mem::size_of,
    xen_sys::{grant_entry_t, mmu_update_t, __HYPERVISOR_VIRT_START},
};

const PAGE_LAYOUT: Layout = unsafe { Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE) };

/// Initialize grant table
pub fn init<const NUM_GRANT_FRAMES: usize>() -> Result<*mut grant_entry_t, Error> {
    let mut frames = [0u64; NUM_GRANT_FRAMES];

    setup_table(DOMID_SELF, &mut frames)?;

    log::trace!("setup table {:p}", frames.as_ptr());

    let mfn = get_max_machine_frame_number().0;
    let grant_table = mfn << PAGE_SHIFT;

    log::trace!(
        "grant_table: {:#x} {:#x}",
        grant_table,
        grant_table + NUM_GRANT_FRAMES * PAGE_SIZE
    );

    unsafe {
        build(
            (*START_INFO).pt_base as *mut _,
            VirtualAddress(grant_table).into(),
            VirtualAddress(grant_table + NUM_GRANT_FRAMES * PAGE_SIZE).into(),
            &frames,
        )
    };

    Ok(grant_table as *mut _)
}

unsafe fn build(
    pt_base: *mut PageEntry,
    start_pfn: PageFrameNumber,
    max_pfn: PageFrameNumber,
    frames: &[u64],
) {
    let mut pfn_counter = 0;

    let mut mmu_updates = [mmu_update_t { ptr: 0, val: 0 }; L1_PAGETABLE_ENTRIES + 1];
    let mut mmu_updates_index = 0;

    if max_pfn >= PageFrameNumber::from(VirtualAddress(__HYPERVISOR_VIRT_START as usize)) {
        panic!("Maximum page frame number overlaps with Xen virtual space");
    }

    let start_address = VirtualAddress::from(start_pfn);
    let end_address = VirtualAddress::from(max_pfn);

    log::debug!(
        "Mapping memory range {:#x} - {:#x}",
        start_address.0,
        end_address.0
    );

    for address in (start_address.0..end_address.0)
        .step_by(PAGE_SIZE)
        .map(|a| VirtualAddress(a))
    {
        log::trace!("starting loop");
        // lookup L3 page entry
        let l3_page = {
            let l4_table = pt_base;
            let pt_mfn =
                MachineFrameNumber::from(VirtualAddress(pt_base as *const PageEntry as usize));
            let offset = address.l4_table_offset();
            let page = l4_table.offset(offset);

            // if not present, map new L3 page table frame
            if (*page).0 & PAGE_PRESENT == 0 {
                let npf_pfn = MachineFrameNumber::from(VirtualAddress(alloc(PAGE_LAYOUT) as usize));
                log::trace!("npf_pfn3: {}", npf_pfn.0);
                new_frame(pt_base, npf_pfn.into(), pt_mfn, offset, 3);
            }

            *page
        };

        // lookup L2 page entry
        let l2_page = {
            let pt_mfn = MachineFrameNumber::from(l3_page);
            let l3_table = VirtualAddress::from(PhysicalAddress(
                PageFrameNumber::from(pt_mfn).0 << PAGE_SHIFT,
            ))
            .0 as *mut PageEntry;
            let offset = address.l3_table_offset();
            let page = l3_table.offset(offset);

            // if not present, map new L2 page table frame
            if (*page).0 & PAGE_PRESENT == 0 {
                let npf_pfn = MachineFrameNumber::from(VirtualAddress(alloc(PAGE_LAYOUT) as usize));
                log::trace!("npf_pfn2: {}", npf_pfn.0);
                new_frame(pt_base, npf_pfn.into(), pt_mfn, offset, 2);
            }

            *page
        };

        // lookup L1 page entry
        let l1_page = {
            let pt_mfn = MachineFrameNumber::from(l2_page);
            let l2_table = VirtualAddress::from(PhysicalAddress(
                PageFrameNumber::from(pt_mfn).0 << PAGE_SHIFT,
            ))
            .0 as *mut PageEntry;
            let offset = address.l2_table_offset();

            let page = l2_table.offset(offset);

            // if not present, map new L1 page table frame
            if (*page).0 & PAGE_PRESENT == 0 {
                let npf_pfn = MachineFrameNumber::from(VirtualAddress(alloc(PAGE_LAYOUT) as usize));
                log::trace!("npf_pfn1: {}", npf_pfn.0);
                new_frame(pt_base, npf_pfn.into(), pt_mfn, offset, 1);
            }

            *page
        };

        // lookup page, adding to current batch of mmu_updates if not present
        {
            let mfn_to_map = frames[(pfn_counter)];
            pfn_counter += 1;

            let pt_mfn = MachineFrameNumber::from(l1_page);
            let l1_table = VirtualAddress::from(PhysicalAddress(
                PageFrameNumber::from(pt_mfn).0 << PAGE_SHIFT,
            ))
            .0 as *mut PageEntry;
            let offset = address.l1_table_offset();

            if ((*l1_table.offset(offset)).0 & PAGE_PRESENT) == 0 {
                mmu_updates[mmu_updates_index].ptr =
                    ((pt_mfn.0 << PAGE_SHIFT) + size_of::<PageEntry>() * offset as usize) as u64;
                mmu_updates[mmu_updates_index].val =
                    ((mfn_to_map as usize) << PAGE_SHIFT | L1_PROT) as u64;
                mmu_updates_index += 1;
            }
        }

        // if number of mmu_updates is equal to the number of L1 page table entries
        if mmu_updates_index == L1_PAGETABLE_ENTRIES
        // OR we have reached the maximum page frame number
            || address.0 + PAGE_SIZE == end_address.0
        // issue MMU update hypercall
        {
            log::trace!("issuing update");
            hypervisor_mmu_update(&mmu_updates[..mmu_updates_index])
                .expect("PTE could not be updated");

            mmu_updates_index = 0;
        }

        log::trace!("ending loop");
    }
}
