use xen::{
    platform::consts::{L1_PAGETABLE_SHIFT, PAGE_SIZE},
    sections::text_start,
};

/// Converts a virtual address to physical
pub fn to_phys(address: usize) -> usize {
    address - text_start() as usize
}

/// Converts a physical address to virtual
pub fn to_virt(address: usize) -> usize {
    address + text_start() as usize
}

/// Gives a page frame number after rounding the given address to the next page frame boundary
pub fn pfn_up(address: usize) -> usize {
    (address + PAGE_SIZE - 1) >> L1_PAGETABLE_SHIFT
}
