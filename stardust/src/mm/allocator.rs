//! Kernel memory allocator

use {
    buddy_system_allocator::LockedHeap,
    log::{error, info},
    xen::mm::VirtualAddress,
};

#[global_allocator]
pub static ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

/// Initialize allocator
pub unsafe fn init(heap_start: VirtualAddress, heap_size: usize) {
    info!(
        "Initialising allocator with heap start {:#x} and length {}",
        heap_start.0, heap_size
    );

    ALLOCATOR.lock().init(heap_start.0, heap_size);
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    error!("ALLOCATOR: {:?}", ALLOCATOR.lock());
    panic!("allocation error: {:?}", layout);
}
