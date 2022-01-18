//! Kernel memory management

use {
    core::{cmp::min, convert::TryInto, mem::size_of, ptr},
    log::{debug, info, trace},
    xen::{
        memory::{
            get_current_pages, get_max_pages, hypervisor_mmu_update, l1_table_offset,
            l2_table_offset, l3_table_offset, l4_table_offset, pfn_up, MachineFrameNumber,
            PageEntry, PageFrameNumber, PhysicalAddress, VirtualAddress,
        },
        platform::consts::{
            L1_PAGETABLE_ENTRIES, L1_PROT, MAX_MEM_SIZE, PAGETABLE_LEVELS, PAGE_MASK, PAGE_PRESENT,
            PAGE_RW, PAGE_SHIFT, PAGE_SIZE, PT_PROT,
        },
        sections::end,
        xen_sys::{mmu_update_t, start_info_t, __HYPERVISOR_VIRT_START},
    },
};

pub mod allocator;

/// Initialise kernel memory management
pub fn init(start_info: &start_info_t) {
    info!("Initalising memory management");

    if start_info.mfn_list < end() as u64 {
        panic!("MFN_LIST must be beyond end of program, this can cause corruption!")
    }

    // construct pointer to base of page table
    let pt_base = start_info.pt_base as *mut PageEntry;

    let nr_pt_frames: usize = start_info
        .nr_pt_frames
        .try_into()
        .expect("nr_pt_frames could not be converted to a usize");

    let nr_pages: usize = start_info
        .nr_pages
        .try_into()
        .expect("nr_pages could not be converted to a usize");

    // first page frame number to use in memory
    let start_pfn = PageFrameNumber(
        pfn_up(PhysicalAddress::from(VirtualAddress(pt_base as usize))).0 + nr_pt_frames,
    );

    // cannot have more pages than the maximum amount of memory on the current platform
    let max_pfn = PageFrameNumber(min(nr_pages, (MAX_MEM_SIZE / PAGE_SIZE) - 1));

    debug!("             start_pfn: {:?}", start_pfn.0);
    debug!("               max_pfn: {:?}", max_pfn.0);

    debug!(
        "current reserved pages: {}",
        get_current_pages().expect("Failed to get current reserved pages")
    );
    debug!(
        "    max reserved pages: {}",
        get_max_pages().expect("Failed to get max reserved pages")
    );

    let (start_address, size) = unsafe { build_pagetable(pt_base, start_pfn, max_pfn) };

    unsafe { allocator::init(start_address, size) };
}

/// Build the initial pagetable
unsafe fn build_pagetable(
    pt_base: *mut PageEntry,
    start_pfn: PageFrameNumber,
    max_pfn: PageFrameNumber,
) -> (VirtualAddress, usize) {
    // Page frame number in which the current page table resides
    let mut current_pt_pfn = start_pfn;

    let mut mmu_updates = [mmu_update_t { ptr: 0, val: 0 }; L1_PAGETABLE_ENTRIES + 1];
    let mut mmu_updates_index = 0;

    if max_pfn >= PageFrameNumber::from(VirtualAddress(__HYPERVISOR_VIRT_START as usize)) {
        panic!("Maximum page frame number overlaps with Xen virtual space");
    }

    let start_address = VirtualAddress::from(start_pfn);
    let end_address = VirtualAddress::from(max_pfn);

    debug!(
        "Mapping memory range {:#x} - {:#x}",
        start_address.0, end_address.0
    );

    for (address, pfn_to_map) in (start_address.0..end_address.0)
        .step_by(PAGE_SIZE)
        .map(|a| VirtualAddress(a))
        .zip((start_pfn.0..).map(|n| PageFrameNumber(n)))
    {
        // lookup L3 page entry
        let l3_page = {
            let l4_table = pt_base;
            let pt_mfn =
                MachineFrameNumber::from(VirtualAddress(pt_base as *const PageEntry as usize));
            let offset = l4_table_offset(address);
            let page = l4_table.offset(offset);

            // if not present, map new L3 page table frame
            if (*page).0 & PAGE_PRESENT == 0 {
                new_pt_frame(pt_base, current_pt_pfn, pt_mfn, offset, 3);
                current_pt_pfn.0 += 1;
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
            let offset = l3_table_offset(address);
            let page = l3_table.offset(offset);

            // if not present, map new L2 page table frame
            if (*page).0 & PAGE_PRESENT == 0 {
                new_pt_frame(pt_base, current_pt_pfn, pt_mfn, offset, 2);
                current_pt_pfn.0 += 1;
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
            let offset = l2_table_offset(address);

            let page = l2_table.offset(offset);

            // if not present, map new L1 page table frame
            if (*page).0 & PAGE_PRESENT == 0 {
                new_pt_frame(pt_base, current_pt_pfn, pt_mfn, offset, 1);
                current_pt_pfn.0 += 1;
            }

            *page
        };

        // lookup page, adding to current batch of mmu_updates if not present
        {
            let pt_mfn = MachineFrameNumber::from(l1_page);
            let l1_table = VirtualAddress::from(PhysicalAddress(
                PageFrameNumber::from(pt_mfn).0 << PAGE_SHIFT,
            ))
            .0 as *mut PageEntry;
            let offset = l1_table_offset(address);

            if ((*l1_table.offset(offset)).0 & PAGE_PRESENT) == 0 {
                mmu_updates[mmu_updates_index].ptr =
                    ((pt_mfn.0 << PAGE_SHIFT) + size_of::<PageEntry>() * offset as usize) as u64;
                mmu_updates[mmu_updates_index].val =
                    (MachineFrameNumber::from(pfn_to_map).0 << PAGE_SHIFT | L1_PROT) as u64;
                mmu_updates_index += 1;
            }
        }

        // if number of mmu_updates is equal to the number of L1 page table entries
        if mmu_updates_index == L1_PAGETABLE_ENTRIES
        // OR we have reached the maximum page frame number
            || (mmu_updates_index != 0 && pfn_to_map == max_pfn)
        // issue MMU update hypercall
        {
            hypervisor_mmu_update(&mmu_updates[..mmu_updates_index])
                .expect("PTE could not be updated");

            mmu_updates_index = 0;
        }
    }

    // usable memory begins after the page table page frames
    let heap_start = VirtualAddress::from(current_pt_pfn);
    let size = end_address.0 - heap_start.0;

    (heap_start, size)
}

/// Map a new page table frame
unsafe fn new_pt_frame(
    l4_table: *mut PageEntry,
    pt_pfn: PageFrameNumber,
    prev_l_mfn: MachineFrameNumber,
    offset: isize,
    level: usize,
) {
    let pt_page = VirtualAddress::from(pt_pfn);

    let mut mmu_updates = [mmu_update_t { ptr: 0, val: 0 }; 1];

    trace!(
        "Allocating new L{} page table frame for pfn={}, prev_l_mfn={}, offset={}",
        level,
        pt_pfn.0,
        prev_l_mfn.0,
        offset
    );

    // clear the page otherwise might fail to map it as a page table page
    ptr::write_bytes(pt_page.0 as *mut u8, 0, PAGE_SIZE);

    assert!(level >= 1 && level <= PAGETABLE_LEVELS);

    // Make PFN a page table page
    let l3_table =
        VirtualAddress::from(*l4_table.offset(l4_table_offset(pt_page))).0 as *mut PageEntry;
    let l2_table =
        VirtualAddress::from(*l3_table.offset(l3_table_offset(pt_page))).0 as *mut PageEntry;

    mmu_updates[0].ptr = (((*l2_table.offset(l2_table_offset(pt_page))).0 & PAGE_MASK)
        + size_of::<PageEntry>() * l1_table_offset(pt_page) as usize)
        as u64;

    mmu_updates[0].val = ((MachineFrameNumber::from(pt_pfn).0 << PAGE_SHIFT)
        | (PT_PROT[level - 1] & !PAGE_RW)) as u64;

    hypervisor_mmu_update(&mmu_updates).expect("PTE for new page table page could not be updated");

    // Hook the new page table page into the hierarchy
    mmu_updates[0].ptr =
        ((prev_l_mfn.0 << PAGE_SHIFT) + size_of::<PageEntry>() * offset as usize) as u64;
    mmu_updates[0].val = (MachineFrameNumber::from(pt_pfn).0 << PAGE_SHIFT | PT_PROT[level]) as u64;

    hypervisor_mmu_update(&mmu_updates).expect("PTE insertion into hierarchy failed");
}
