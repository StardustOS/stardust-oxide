//! Interface for Xen Grant Tables
//!
//! "The grant table mechanism [..] allows memory pages to be transferred or shared between virtual machines"

use {
    crate::platform::{self, consts::PAGE_SIZE},
    core::{
        convert::{TryFrom, TryInto},
        mem::size_of,
        sync::atomic::{fence, Ordering},
    },
    lazy_static::lazy_static,
    spin::Mutex,
    xen_sys::{grant_entry_t, grant_ref_t, GTF_accept_transfer, GTF_permit_access, GTF_readonly},
};

pub use error::{Error, GrantStatusError};
use xen_sys::domid_t;

use crate::memory::MachineFrameNumber;

mod error;
pub mod operations;

/// Number of grant frames
const NUM_GRANT_FRAMES: usize = 4;

const NUM_RESERVED_ENTRIES: usize = 8;

const NUM_GRANT_ENTRIES: usize = (NUM_GRANT_FRAMES * PAGE_SIZE) / size_of::<grant_entry_t>();

lazy_static! {
    static ref GRANT_TABLE: Mutex<GrantTable> = Mutex::new(GrantTable::new());
}

// Required due to the raw mutable pointer to the grant table not being Send, this is safe as the virtual address it refers to is constant for the lifetime of the GrantTable
unsafe impl Send for GrantTable {}

#[derive(Debug)]
struct GrantTable {
    list: [grant_ref_t; NUM_GRANT_ENTRIES],
    table: *mut grant_entry_t,
}

impl GrantTable {
    fn new() -> Self {
        let list = [0; NUM_GRANT_ENTRIES];

        let table = platform::grant_table::init::<NUM_GRANT_FRAMES>()
            .expect("Failed platform grant table initialization");

        let mut celf = Self { list, table };

        for i in NUM_RESERVED_ENTRIES..NUM_GRANT_ENTRIES {
            celf.put_free_entry(i as u32);
        }

        log::trace!("grant table mapped at {:p}", table);

        celf
    }

    fn put_free_entry(&mut self, reference: grant_ref_t) {
        self.list[reference as usize] = self.list[0];
        self.list[0] = reference
            .try_into()
            .expect("Failed to convert usize to grant_ref_t");
    }

    fn get_free_entry(&mut self) -> grant_ref_t {
        let reference = self.list[0];
        self.list[0] =
            self.list[usize::try_from(reference).expect("Failed to convert u32 to usize")];
        reference
    }

    fn grant_access(
        &mut self,
        domain: domid_t,
        frame: MachineFrameNumber,
        readonly: bool,
    ) -> grant_ref_t {
        let reference = self.get_free_entry();
        let idx: isize = reference
            .try_into()
            .expect("Failed to convert u32 to usize");

        unsafe {
            let mut entry = *(self.table.offset(idx));
            entry.frame = frame.0.try_into().expect("Failed to convert usize to u32");
            entry.domid = domain;

            fence(Ordering::SeqCst);

            entry.flags = (GTF_permit_access | if readonly { GTF_readonly } else { 0 })
                .try_into()
                .expect("Failed to convert u32 to u16");

            log::trace!(
                "granting access {} {} {} {}",
                domain,
                frame.0,
                entry.flags,
                reference
            );
        }

        reference
    }

    fn grant_transfer(&mut self, domain: domid_t, frame: MachineFrameNumber) -> grant_ref_t {
        let reference = self.get_free_entry();

        let idx: isize = reference
            .try_into()
            .expect("Failed to convert u32 to usize");

        unsafe {
            let mut entry = *(self.table.offset(idx));
            entry.frame = frame.0.try_into().expect("Failed to convert usize to u32");
            entry.domid = domain;

            fence(Ordering::SeqCst);

            entry.flags = GTF_accept_transfer
                .try_into()
                .expect("Failed to convert u32 to u16");
        }

        reference
    }

    fn grant_end(&mut self, reference: grant_ref_t) {
        unsafe { *(self.table.offset(reference as isize)) }.flags = 0;

        self.put_free_entry(reference);
    }
}

/// Initializes grant table
pub fn init() {
    lazy_static::initialize(&GRANT_TABLE)
}

/// Grants `domain` access to the supplied frame
pub fn grant_access(domain: domid_t, frame: MachineFrameNumber, readonly: bool) -> grant_ref_t {
    GRANT_TABLE.lock().grant_access(domain, frame, readonly)
}

/// Transfers the supplied frame to `domain`
pub fn grant_transfer(domain: domid_t, frame: MachineFrameNumber) -> grant_ref_t {
    GRANT_TABLE.lock().grant_transfer(domain, frame)
}

/// Ends access to the supplied grant reference
pub fn grant_end(reference: grant_ref_t) {
    GRANT_TABLE.lock().grant_end(reference)
}
