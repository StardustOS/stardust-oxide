//! Kernel memory management

use {
    core::{
        convert::TryInto,
        mem::size_of,
        ptr::{self},
    },
    util::{
        init_mfn_list, l1_table_offset, l2_table_offset, l3_table_offset, l4_table_offset, pfn_up,
        MachineFrameNumber, PageEntry, PageFrameNumber, PhysicalAddress, VirtualAddress,
    },
    xen::{
        memory::{get_current_pages, get_max_pages, hypervisor_mmu_update},
        platform::consts::{
            L1_PAGETABLE_ENTRIES, L1_PAGETABLE_SHIFT, L1_PROT, MAX_MEM_SIZE, PAGETABLE_LEVELS,
            PAGE_MASK, PAGE_PRESENT, PAGE_RW, PAGE_SHIFT, PAGE_SIZE, PT_PROT,
        },
        println,
        sections::{edata, end, erodata, etext, text_start},
        xen_sys::{mmu_update_t, start_info_t, __HYPERVISOR_VIRT_START},
    },
};

pub mod allocator;
pub mod util;

/// Initialise kernel memory management
pub fn init(start_info: &start_info_t) {
    println!();
    println!("Initalising kernel memory management...");
    println!("                 _text: {:#x}", text_start());
    println!("                _etext: {:#x}", etext());
    println!("              _erodata: {:#x}", erodata());
    println!("                _edata: {:#x}", edata());
    println!("           stack start: {:p}", unsafe { &xen::stack });
    println!("                  _end: {:#x}", end());

    // initialize the mapping between page frame numbers and machine frame numbers
    init_mfn_list(
        start_info
            .mfn_list
            .try_into()
            .expect("mfn_list could not be converted to a usize"),
    );

    // construct pointer to base of page table
    let pt_base = start_info.pt_base as *mut PageEntry;

    let nr_pt_frames: usize = start_info
        .nr_pt_frames
        .try_into()
        .expect("nr_pt_frames could not be converted to a usize");

    let mut start_pfn = PageFrameNumber(
        pfn_up(PhysicalAddress::from(VirtualAddress(pt_base as usize))).0 + nr_pt_frames,
    );
    let mut max_pfn = PageFrameNumber(
        start_info
            .nr_pages
            .try_into()
            .expect("nr_pages could not be converted to a usize"),
    );

    if max_pfn.0 >= MAX_MEM_SIZE / PAGE_SIZE {
        max_pfn.0 = (MAX_MEM_SIZE / PAGE_SIZE) - 1;
    }

    println!("             start_pfn: {:?}", start_pfn);
    println!("               max_pfn: {:?}", max_pfn);

    println!("current reserved pages: {}", get_current_pages());
    println!("    max reserved pages: {}", get_max_pages());

    unsafe { build_pagetable(pt_base, &mut start_pfn, &mut max_pfn) };

    unsafe {
        allocator::init(
            start_pfn.0 << L1_PAGETABLE_SHIFT,
            (max_pfn.0 << L1_PAGETABLE_SHIFT) - (start_pfn.0 << L1_PAGETABLE_SHIFT),
        )
    };
}

/// Build the initial pagetable
unsafe fn build_pagetable(
    pt_base: *mut PageEntry,
    start_pfn: &mut PageFrameNumber,
    max_pfn: &mut PageFrameNumber,
) {
    let mut pt_pfn = *start_pfn;
    let mut mmu_updates = [mmu_update_t { ptr: 0, val: 0 }; L1_PAGETABLE_ENTRIES + 1];
    let mut count = 0;

    // Be conservative: even if we know there will be more pages already mapped, start the loop at the very beginning
    let mut pfn_to_map = *start_pfn;

    if *max_pfn >= PageFrameNumber::from(VirtualAddress(__HYPERVISOR_VIRT_START as usize)) {
        panic!("Maximum page frame number overlaps with Xen virtual space");
    }

    let mut start_address = VirtualAddress::from(pfn_to_map);
    let end_address = VirtualAddress::from(*max_pfn);

    println!(
        "Mapping memory range {:#x} - {:#x}",
        start_address.0, end_address.0
    );

    while start_address < end_address {
        let tab = pt_base;
        let pt_mfn = MachineFrameNumber::from(VirtualAddress(pt_base as *const PageEntry as usize));

        let mut offset = l4_table_offset(start_address);

        // Need new L3 pt frame
        if ((*tab.offset(offset)).0 & PAGE_PRESENT) == 0 {
            new_pt_frame(pt_base, &mut pt_pfn, pt_mfn, offset, 3);
        }

        let page = *tab.offset(offset);
        let pt_mfn = MachineFrameNumber::from(page);
        let tab = VirtualAddress::from(PhysicalAddress(
            PageFrameNumber::from(pt_mfn).0 << PAGE_SHIFT,
        ))
        .0 as *mut PageEntry;
        offset = l3_table_offset(start_address);

        // Need new L2 pt frame
        if ((*tab.offset(offset)).0 & PAGE_PRESENT) == 0 {
            new_pt_frame(pt_base, &mut pt_pfn, pt_mfn, offset, 2);
        }

        let page = *tab.offset(offset);
        let pt_mfn = MachineFrameNumber::from(page);
        let tab = VirtualAddress::from(PhysicalAddress(
            PageFrameNumber::from(pt_mfn).0 << PAGE_SHIFT,
        ))
        .0 as *mut PageEntry;
        offset = l2_table_offset(start_address);

        // Need new L1 pt frame
        if ((*tab.offset(offset)).0 & PAGE_PRESENT) == 0 {
            new_pt_frame(pt_base, &mut pt_pfn, pt_mfn, offset, 1);
        }

        let page = *tab.offset(offset);
        let pt_mfn = MachineFrameNumber::from(page);
        let tab = VirtualAddress::from(PhysicalAddress(
            PageFrameNumber::from(pt_mfn).0 << PAGE_SHIFT,
        ))
        .0 as *mut PageEntry;
        offset = l1_table_offset(start_address);

        if ((*tab.offset(offset)).0 & PAGE_PRESENT) == 0 {
            mmu_updates[count].ptr =
                ((pt_mfn.0 << PAGE_SHIFT) + size_of::<PageEntry>() * offset as usize) as u64;
            mmu_updates[count].val =
                (MachineFrameNumber::from(pfn_to_map).0 << PAGE_SHIFT | L1_PROT) as u64;
            count += 1;
        }

        pfn_to_map.0 += 1;

        if count == L1_PAGETABLE_ENTRIES || (count != 0 && pfn_to_map == *max_pfn) {
            let rc = hypervisor_mmu_update(&mmu_updates[..count]);
            if rc < 0 {
                panic!("PTE could not be updated, mmu_update failed with rc={}", rc);
            }
            count = 0;
        }

        start_address.0 += PAGE_SIZE;
    }

    *start_pfn = pt_pfn;
}

unsafe fn new_pt_frame(
    pt_base: *mut PageEntry,
    pt_pfn: &mut PageFrameNumber,
    prev_l_mfn: MachineFrameNumber,
    offset: isize,
    level: usize,
) {
    let pt_page = VirtualAddress::from(*pt_pfn);

    let mut mmu_updates = [mmu_update_t { ptr: 0, val: 0 }; 1];

    println!(
        "Allocating new L{} pt frame for pfn={}, prev_l_mfn={}, offset={}",
        level, pt_pfn.0, prev_l_mfn.0, offset
    );

    /* We need to clear the page, otherwise we might fail to map it
    as a page table page */
    ptr::write_bytes(pt_page.0 as *mut u8, 0, PAGE_SIZE);

    assert!(level >= 1 && level <= PAGETABLE_LEVELS);

    /* Make PFN a page table page */
    let mut tab = pt_base;
    tab = VirtualAddress::from(*tab.offset(l4_table_offset(pt_page))).0 as *mut PageEntry;
    tab = VirtualAddress::from(*tab.offset(l3_table_offset(pt_page))).0 as *mut PageEntry;

    mmu_updates[0].ptr = (((*tab.offset(l2_table_offset(pt_page))).0 & PAGE_MASK)
        + size_of::<PageEntry>() * l1_table_offset(pt_page) as usize)
        as u64;

    mmu_updates[0].val = ((MachineFrameNumber::from(*pt_pfn).0 << PAGE_SHIFT)
        | (PT_PROT[level - 1] & !PAGE_RW)) as u64;

    let rc = hypervisor_mmu_update(&mmu_updates);
    if rc < 0 {
        panic!(
            "PTE for new page table page could not be updated, mmu_update failed with rc={}",
            rc
        );
    }

    /* Hook the new page table page into the hierarchy */
    mmu_updates[0].ptr =
        ((prev_l_mfn.0 << PAGE_SHIFT) + size_of::<PageEntry>() * offset as usize) as u64;
    mmu_updates[0].val =
        (MachineFrameNumber::from(*pt_pfn).0 << PAGE_SHIFT | PT_PROT[level]) as u64;

    let rc = hypervisor_mmu_update(&mmu_updates);
    if rc < 0 {
        panic!("mmu_update failed with rc={}", rc);
    }

    pt_pfn.0 += 1;
}
