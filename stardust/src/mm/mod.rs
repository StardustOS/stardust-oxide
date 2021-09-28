//! Kernel memory management

use {
    core::convert::TryInto,
    util::{pfn_up, to_phys},
    xen::{
        println,
        sections::{edata, end, erodata, etext, text_start},
        xen_sys::start_info_t,
    },
};

mod allocator;
mod util;

/// Initialise kernel memory management
pub fn init(start_info: &start_info_t) {
    println!();
    println!("Initalising kernel memory management...");
    println!("          _text: {:#x}", text_start());
    println!("         _etext: {:#x}", etext());
    println!("       _erodata: {:#x}", erodata());
    println!("         _edata: {:#x}", edata());
    println!("    stack start: {:p}", unsafe { &xen::stack });
    println!("           _end: {:#x}", end());

    let pt_base: usize = start_info
        .pt_base
        .try_into()
        .expect("pt_base could not be converted to a usize");

    let nr_pt_frames: usize = start_info
        .nr_pt_frames
        .try_into()
        .expect("pt_base could not be converted to a usize");

    let start_pfn = pfn_up(to_phys(pt_base)) + nr_pt_frames + 3;
    let max_pfn = start_info
        .nr_pages
        .try_into()
        .expect("nr_pages could not be converted to a usize");

    println!("      start_pfn: {}", start_pfn);
    println!("        max_pfn: {}", max_pfn);

    build_pagetable(start_pfn, max_pfn);

    allocator::init();
}

/// Build the initial pagetable
fn build_pagetable(start_pfn: usize, max_pfn: usize) {
    todo!();
}
