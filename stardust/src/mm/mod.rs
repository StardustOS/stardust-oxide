//! Kernel memory management

use {
    core::{cmp::min, convert::TryInto},
    log::{debug, info},
    xen::{
        memory::{
            get_current_pages, get_max_pages, page_table, pfn_up, PageEntry, PageFrameNumber,
            PhysicalAddress, VirtualAddress,
        },
        platform::consts::{MAX_MEM_SIZE, PAGE_SIZE},
        sections::end,
        xen_sys::start_info_t,
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

    let (start_address, size) = unsafe { page_table::build(pt_base, start_pfn, max_pfn) };

    unsafe { allocator::init(start_address, size) };
}
