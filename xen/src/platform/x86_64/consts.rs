//! x86_64 constants

///
pub const PAGE_SHIFT: usize = 12;

/// Size of a page in bytes
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

/// ?
pub const L1_PAGETABLE_SHIFT: usize = PAGE_SHIFT;
