//! Kernel memory allocator

use {
    linked_list_allocator::LockedHeap,
    xen::{mm::VirtualAddress, println},
};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize allocator
pub unsafe fn init(heap_start: VirtualAddress, heap_size: usize) {
    println!(
        "Initialising allocator with heap start {:#x} and length {}",
        heap_start.0, heap_size
    );

    ALLOCATOR.lock().init(heap_start.0, heap_size);
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
