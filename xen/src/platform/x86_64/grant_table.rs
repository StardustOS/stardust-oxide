//! x86_64 Grant Table

use {
    crate::{
        grant_table::{operations::setup_table, Error},
        memory::{
            hypervisor_mmu_update, page_table, MachineAddress, MachineFrameNumber, PageEntry,
            PageFrameNumber, VirtualAddress,
        },
        platform::consts::{L1_MASK, L1_PROT, PAGE_PRESENT, PAGE_PSE, PAGE_SHIFT, PAGE_SIZE},
        DOMID_SELF, START_INFO,
    },
    alloc::alloc::{alloc, Layout},
    core::convert::{TryFrom, TryInto},
    xen_sys::{grant_entry_t, mmu_update_t, MMU_NORMAL_PT_UPDATE},
};

/// Initialize grant table
pub fn init<const NUM_GRANT_FRAMES: usize>() -> Result<*mut grant_entry_t, Error> {
    let mut frames = [0u64; NUM_GRANT_FRAMES];

    setup_table(DOMID_SELF, &mut frames)?;

    let va = unsafe {
        alloc(
            Layout::from_size_align(PAGE_SIZE * NUM_GRANT_FRAMES, PAGE_SIZE)
                .expect("Failed to construct Layout"),
        )
    };

    let mut mmu_updates = [mmu_update_t { ptr: 0, val: 0 }; NUM_GRANT_FRAMES];

    let mut pgt: *mut PageEntry = core::ptr::null_mut();

    mmu_updates
        .iter_mut()
        .enumerate()
        .for_each(|(i, mmu_update)| {
            let va = VirtualAddress(unsafe {
                va.offset(
                    (i * PAGE_SIZE)
                        .try_into()
                        .expect("Failed to convert usize to isize"),
                )
            } as usize);

            if pgt.is_null() || (va.0 & L1_MASK) == 0 {
                pgt = unsafe { need_pgt(va) };
            }

            mmu_update.ptr = u64::try_from(MachineAddress::from(VirtualAddress(0)).0)
                .expect("Failed to convert usize to u64")
                | u64::try_from(MMU_NORMAL_PT_UPDATE).expect("Failed to convert u32 to u64");

            mmu_update.val = ((frames[i]) << PAGE_SHIFT)
                | u64::try_from(L1_PROT).expect("Failed to convert usize to u64");
        });

    hypervisor_mmu_update(&mmu_updates)?;

    Ok(va.cast())
}

/// Returns a valid PageEntry for a given virtual address
///
/// Allocates pagetable page if PageEntry does not exist
unsafe fn need_pgt(va: VirtualAddress) -> *mut PageEntry {
    let pt_base = (*START_INFO).pt_base as *mut PageEntry;

    let mut pt_mfn = MachineFrameNumber::from(VirtualAddress(pt_base as usize));
    let mut table = pt_base;
    let mut pt_pfn: PageFrameNumber;
    let mut offset: isize;

    offset = va.l4_table_offset();
    let page = table.offset(offset);
    if (*page).0 & PAGE_PRESENT == 0 {
        let virt = VirtualAddress(alloc(
            Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).expect("Failed to construct Layout"),
        ) as usize);

        pt_pfn = PageFrameNumber::from(virt);

        page_table::new_frame(pt_base, pt_pfn, pt_mfn, offset, 3);
    }
    assert!((*page).0 & PAGE_PRESENT != 0);
    pt_mfn = MachineFrameNumber::from(*page);
    table = VirtualAddress::from(pt_mfn).0 as *mut PageEntry;

    offset = va.l3_table_offset();
    let page = table.offset(offset);
    if (*page).0 & PAGE_PRESENT == 0 {
        let virt = VirtualAddress(alloc(
            Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).expect("Failed to construct Layout"),
        ) as usize);

        pt_pfn = PageFrameNumber::from(virt);

        page_table::new_frame(pt_base, pt_pfn, pt_mfn, offset, 2);
    }
    assert!((*page).0 & PAGE_PRESENT != 0);
    pt_mfn = MachineFrameNumber::from(*page);
    table = VirtualAddress::from(pt_mfn).0 as *mut PageEntry;

    offset = va.l2_table_offset();
    let page = table.offset(offset);
    if (*page).0 & PAGE_PRESENT == 0 {
        let virt = VirtualAddress(alloc(
            Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).expect("Failed to construct Layout"),
        ) as usize);

        pt_pfn = PageFrameNumber::from(virt);

        page_table::new_frame(pt_base, pt_pfn, pt_mfn, offset, 1);
    }
    assert!((*page).0 & PAGE_PRESENT != 0);

    if (*page).0 & PAGE_PSE != 0 {
        return page;
    }

    pt_mfn = MachineFrameNumber::from(*page);
    table = VirtualAddress::from(pt_mfn).0 as *mut PageEntry;

    offset = va.l1_table_offset();
    table.offset(offset)
}
