//! x86_64 constants

///
pub const PAGE_SHIFT: usize = 12;

/// Size of a page in bytes
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

///
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

/// Width of physical address in bits
pub const PADDR_BITS: usize = 52;

/// Width of virtual address in bits
pub const VADDR_BITS: usize = 48;

/// Physical address mask
pub const PADDR_MASK: usize = (1 << PADDR_BITS) - 1;

/// Virtual address mask
pub const VADDR_MASK: usize = (1 << VADDR_BITS) - 1;

///
pub const L1_PAGETABLE_SHIFT: usize = PAGE_SHIFT;
///
pub const L2_PAGETABLE_SHIFT: usize = 21;
///
pub const L3_PAGETABLE_SHIFT: usize = 30;
///
pub const L4_PAGETABLE_SHIFT: usize = 39;

/// Number of L1 pagetable entries
pub const L1_PAGETABLE_ENTRIES: usize = 512;
/// Number of L2 pagetable entries
pub const L2_PAGETABLE_ENTRIES: usize = 512;
/// Number of L3 pagetable entries
pub const L3_PAGETABLE_ENTRIES: usize = 512;
/// Number of L4 pagetable entries
pub const L4_PAGETABLE_ENTRIES: usize = 512;
/// Number of pagetable levels
pub const PAGETABLE_LEVELS: usize = 4;

///
pub const PAGE_PRESENT: usize = 0x001;
///
pub const PAGE_RW: usize = 0x002;
///
pub const PAGE_USER: usize = 0x004;
///
pub const PAGE_PWT: usize = 0x008;
///
pub const PAGE_PCD: usize = 0x010;
///
pub const PAGE_ACCESSED: usize = 0x020;
///
pub const PAGE_DIRTY: usize = 0x040;
///
pub const PAGE_PAT: usize = 0x080;
///
pub const PAGE_PSE: usize = 0x080;
///
pub const PAGE_GLOBAL: usize = 0x100;

/// L1 page flags
pub const L1_PROT: usize = PAGE_PRESENT | PAGE_RW | PAGE_ACCESSED | PAGE_USER;

/// L1 page flags read-only
pub const L1_PROT_RO: usize = PAGE_PRESENT | PAGE_ACCESSED | PAGE_USER;

/// L2 page flags
pub const L2_PROT: usize = PAGE_PRESENT | PAGE_RW | PAGE_ACCESSED | PAGE_DIRTY | PAGE_USER;

/// L3 page flags
pub const L3_PROT: usize = PAGE_PRESENT | PAGE_RW | PAGE_ACCESSED | PAGE_DIRTY | PAGE_USER;

/// L4 page flags
pub const L4_PROT: usize = PAGE_PRESENT | PAGE_RW | PAGE_ACCESSED | PAGE_DIRTY | PAGE_USER;

/// Make pt_pfn a new 'level' page table frame and hook it into the page table at offset in previous level MFN (pref_l_mfn). pt_pfn is a guest PFN.
pub const PT_PROT: [usize; 4] = [L1_PROT, L2_PROT, L3_PROT, L4_PROT];

/// Maximum amount of memory available on x86_64
pub const MAX_MEM_SIZE: usize = 512 << 30;
