//! Memory utility functions and types
//!
//! Consists of wrapper types representing different kinds of memory locations. The following diagram describes the conversions between them:
//!
//! ```text
//! ┌──────────────────┐           ┌───────────────┐
//! │MachineFrameNumber│◀─────────▶│PageFrameNumber│
//! └──────────────────┘           └───────────────┘
//!      ▲        ▲                        ▲
//!      │        │      ┌─────────┐       │
//!      │        └──────│PageEntry│───┐   │
//!      │               └─────────┘   │   │
//!      │                             ▼   ▼
//!      │                         ┌──────────────┐       ┌──────────────┐
//!      └────────────────────────▶│VirtualAddress│◀─────▶│MachineAddress│
//!                                └──────────────┘       └──────────────┘
//!                                        ▲                      ▲
//!                                        │                      │
//!                                        │                      │
//!                                        ▼                      │
//!                                ┌───────────────┐              │
//!                                │PhysicalAddress│◀─────────────┘
//!                                └───────────────┘
//! ```

use {
    crate::{
        hypercall,
        platform::consts::{
            L1_PAGETABLE_ENTRIES, L1_PAGETABLE_SHIFT, L2_PAGETABLE_ENTRIES, L2_PAGETABLE_SHIFT,
            L3_PAGETABLE_ENTRIES, L3_PAGETABLE_SHIFT, L4_PAGETABLE_ENTRIES, L4_PAGETABLE_SHIFT,
            PADDR_MASK, PAGE_MASK, PAGE_SHIFT, PAGE_SIZE,
        },
        sections::text_start,
        xen_sys::__HYPERVISOR_VIRT_START,
        DOMID_SELF,
    },
    core::convert::TryInto,
    log::warn,
    xen_sys::{__HYPERVISOR_memory_op, __HYPERVISOR_mmu_update, mmu_update_t},
};

/// Pointer to the beginning of the machine frame number list
///
/// Initialized with null pointer, this is probably really bad and **must** be set to the value of the `mfn_list` field of the start info structure before being used.
static mut MFN_LIST: *mut usize = core::ptr::null_mut();

/// MFN_LIST must be initialized before converting between PageFrameNumber and MachineFrameNumber
pub(crate) fn init_mfn_list(mfn_list_addr: usize) {
    unsafe { MFN_LIST = mfn_list_addr as *mut usize }
}

/// Page Entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageEntry(pub usize);

/// Number for page frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageFrameNumber(pub usize);

// mfn_to_pfn
impl From<MachineFrameNumber> for PageFrameNumber {
    fn from(mfn: MachineFrameNumber) -> Self {
        Self(unsafe {
            *(__HYPERVISOR_VIRT_START as *const usize).offset(
                mfn.0
                    .try_into()
                    .expect("MachineFrameNumber could not be converted to an isize"),
            )
        })
    }
}

// virt_to_pfn
impl From<VirtualAddress> for PageFrameNumber {
    fn from(virt: VirtualAddress) -> Self {
        // convert to physical then shift down to the previous page frame boundary
        Self(PhysicalAddress::from(virt).0 >> L1_PAGETABLE_SHIFT)
    }
}

/// Number of a page in the machine's address space
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MachineFrameNumber(pub usize);

// pfn_to_mfn
impl From<PageFrameNumber> for MachineFrameNumber {
    fn from(pfn: PageFrameNumber) -> Self {
        Self(unsafe {
            *MFN_LIST.offset(
                pfn.0
                    .try_into()
                    .expect("PageFrameNumber could not be converted to an isize"),
            )
        })
    }
}

// virt_to_mfn
impl From<VirtualAddress> for MachineFrameNumber {
    fn from(virt: VirtualAddress) -> Self {
        Self::from(PageFrameNumber::from(virt))
    }
}

// pte_to_mfn
impl From<PageEntry> for MachineFrameNumber {
    fn from(pte: PageEntry) -> Self {
        Self(((pte.0) & (PADDR_MASK & PAGE_MASK)) >> L1_PAGETABLE_SHIFT)
    }
}

/// Virtual address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(pub usize);

// to_virt
impl From<PhysicalAddress> for VirtualAddress {
    fn from(phys: PhysicalAddress) -> Self {
        // no pointer arithmetic necessary here
        Self(phys.0 + text_start())
    }
}

// mach_to_virt
impl From<MachineAddress> for VirtualAddress {
    fn from(mach: MachineAddress) -> Self {
        Self::from(PhysicalAddress::from(mach))
    }
}

// mfn_to_virt
impl From<MachineFrameNumber> for VirtualAddress {
    fn from(mfn: MachineFrameNumber) -> Self {
        Self::from(PhysicalAddress(PageFrameNumber::from(mfn).0 << PAGE_SHIFT))
    }
}

// pfn_to_virt
impl From<PageFrameNumber> for VirtualAddress {
    fn from(pfn: PageFrameNumber) -> Self {
        Self::from(PhysicalAddress(pfn.0 << PAGE_SHIFT))
    }
}

// pte_to_virt
impl From<PageEntry> for VirtualAddress {
    fn from(pte: PageEntry) -> Self {
        Self::from(PhysicalAddress(
            PageFrameNumber::from(MachineFrameNumber::from(pte)).0 << PAGE_SHIFT,
        ))
    }
}

/// Pseudo-Physical address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(pub usize);

// to_phys
impl From<VirtualAddress> for PhysicalAddress {
    fn from(virt: VirtualAddress) -> Self {
        // no pointer arithmetic necessary here
        Self(virt.0 - text_start())
    }
}

// machine_to_phys
impl From<MachineAddress> for PhysicalAddress {
    fn from(mach: MachineAddress) -> Self {
        let pfn = PageFrameNumber::from(MachineFrameNumber(mach.0 >> PAGE_SHIFT));
        Self((pfn.0 << PAGE_SHIFT) | (mach.0 & !PAGE_MASK))
    }
}

/// Machine address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MachineAddress(pub usize);

// phys_to_machine
impl From<PhysicalAddress> for MachineAddress {
    fn from(phys: PhysicalAddress) -> Self {
        let mfn = MachineFrameNumber::from(PageFrameNumber(phys.0 >> PAGE_SHIFT));
        Self((mfn.0 << PAGE_SHIFT) | (phys.0 & !PAGE_MASK))
    }
}

// virt_to_mach
impl From<VirtualAddress> for MachineAddress {
    fn from(virt: VirtualAddress) -> Self {
        Self::from(PhysicalAddress::from(virt))
    }
}

/// Gives a page frame number after rounding the given address to the next page frame boundary
pub fn pfn_up(phys: PhysicalAddress) -> PageFrameNumber {
    // no pointer arithmetic here, only usage of PFN_UP in mini-os is by passing the result of `to_phys` which casts to unsigned long
    PageFrameNumber((phys.0 + PAGE_SIZE - 1) >> L1_PAGETABLE_SHIFT)
}

/// Given a virtual address get an entry offset into an L1 page table
pub fn l1_table_offset(address: VirtualAddress) -> isize {
    ((address.0 >> L1_PAGETABLE_SHIFT) & (L1_PAGETABLE_ENTRIES - 1))
        .try_into()
        .expect("Could not convert page table offset to isize")
}

/// Given a virtual address get an entry offset into an L2 page table
pub fn l2_table_offset(address: VirtualAddress) -> isize {
    ((address.0 >> L2_PAGETABLE_SHIFT) & (L2_PAGETABLE_ENTRIES - 1))
        .try_into()
        .expect("Could not convert page table offset to isize")
}

/// Given a virtual address get an entry offset into an L3 page table
pub fn l3_table_offset(address: VirtualAddress) -> isize {
    ((address.0 >> L3_PAGETABLE_SHIFT) & (L3_PAGETABLE_ENTRIES - 1))
        .try_into()
        .expect("Could not convert page table offset to isize")
}

/// Given a virtual address get an entry offset into an L4 page table
pub fn l4_table_offset(address: VirtualAddress) -> isize {
    ((address.0 >> L4_PAGETABLE_SHIFT) & (L4_PAGETABLE_ENTRIES - 1))
        .try_into()
        .expect("Could not convert page table offset to isize")
}

/// Memory operation commands
enum Command {
    /// Returns the maximum machine frame number of mapped RAM in this system
    MaximumRamPage = 2,
    /// Returns the current memory reservation in pages of the specified domain
    CurrentReservation = 3,
    /// Returns the maximum memory reservation in pages of the specified domain
    MaximumReservation = 4,
}

/// Perform memory operation
unsafe fn memory_op(cmd: Command, arg: u64) -> Result<u64, hypercall::Error> {
    hypercall!(__HYPERVISOR_memory_op, cmd as u64, arg)
}

/// Gets the current number of reserved pages for the current domain
pub fn get_current_pages() -> Result<usize, hypercall::Error> {
    unsafe {
        memory_op(
            Command::CurrentReservation,
            (&DOMID_SELF) as *const _ as u64,
        )
    }
    .map(|n| n.try_into().expect("Failed to convert u64 to usize"))
}

/// Gets the maximum number of reserved pages for the current domain
pub fn get_max_pages() -> Result<usize, hypercall::Error> {
    unsafe {
        memory_op(
            Command::MaximumReservation,
            (&DOMID_SELF) as *const _ as u64,
        )
    }
    .map(|n| n.try_into().expect("Failed to convert u64 to usize"))
}

/// Gets the maximum machine frame number of mapped RAM in this system
pub fn get_max_machine_frame_number() -> MachineFrameNumber {
    let mfn = unsafe { memory_op(Command::MaximumRamPage, 0) }
        .expect("maximum_ram_page memory operation can never fail");

    MachineFrameNumber(mfn.try_into().expect("Failed to convert u64 to usize"))
}

/// Updates an entry in a page table
pub fn hypervisor_mmu_update(reqs: &[mmu_update_t]) -> Result<(), hypercall::Error> {
    let mut success_count = 0;
    unsafe {
        hypercall!(
            __HYPERVISOR_mmu_update,
            reqs.as_ptr() as u64,
            reqs.len() as u64,
            (&mut success_count) as *mut _ as u64,
            DOMID_SELF
        )
    }?;

    if success_count != reqs.len() {
        warn!(
            "MMU update had different number of successes to number of requests: {} != {}",
            success_count,
            reqs.len(),
        )
    }

    Ok(())
}
