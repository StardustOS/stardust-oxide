use {
    alloc::alloc::{alloc_zeroed, dealloc, Layout},
    core::fmt::Debug,
    core::mem::size_of,
    memoffset::offset_of,
    xen::{
        platform::consts::PAGE_SIZE,
        xen_sys::{netif_rx_sring, netif_rx_sring_entry, netif_tx_sring, netif_tx_sring_entry},
    },
};

pub const LAYOUT: Layout = unsafe { Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE) };
pub struct Ring<S: SharedRingInner>(pub *mut S);

impl<S: SharedRingInner> Ring<S> {
    pub fn new() -> Self {
        let ptr = unsafe { alloc_zeroed(LAYOUT) as *mut S };

        Self(ptr)
    }

    pub fn size(&self) -> usize {
        S::size()
    }

    /// Consume the ring and return the front half
    pub fn front(self) -> RingFront<S> {
        RingFront {
            req_prod_pvt: 0,
            rsp_cons: 0,
            nr_ents: self.size() as u32,
            sring: self,
        }
    }

    pub fn debug(&self) {
        unsafe { &*self.0 }.debug();
    }
}

impl<S: SharedRingInner> Drop for Ring<S> {
    fn drop(&mut self) {
        unsafe { dealloc(self.0 as *mut u8, LAYOUT) }
    }
}

/// Trait for Xen shared ring types
pub trait SharedRingInner {
    fn size() -> usize;
    fn debug(&self);
}

impl SharedRingInner for netif_tx_sring {
    fn size() -> usize {
        rd32(
            ((PAGE_SIZE - offset_of!(netif_tx_sring, ring)) / size_of::<netif_tx_sring_entry>())
                as u32,
        ) as usize
    }

    fn debug(&self) {
        log::trace!(
            "TX: req_prod: {}, req_event: {}, rsp_prod: {}, rsp_event: {}",
            self.req_prod,
            self.req_event,
            self.rsp_prod,
            self.rsp_event
        );
    }
}

impl SharedRingInner for netif_rx_sring {
    fn size() -> usize {
        rd32(
            ((PAGE_SIZE - offset_of!(netif_rx_sring, ring)) / size_of::<netif_rx_sring_entry>())
                as u32,
        ) as usize
    }

    fn debug(&self) {
        log::trace!(
            "RX: req_prod: {}, req_event: {}, rsp_prod: {}, rsp_event: {}",
            self.req_prod,
            self.req_event,
            self.rsp_prod,
            self.rsp_event
        );
    }
}

pub struct RingFront<S: SharedRingInner> {
    req_prod_pvt: u32,
    rsp_cons: u32,
    nr_ents: u32,
    pub sring: Ring<S>,
}

/// Round a 32-bit unsigned constant down to the nearest power of 2
fn rd2(x: u32) -> u32 {
    if x & 0x00000002 != 0 {
        0x2
    } else {
        x & 0x1
    }
}

/// Round a 32-bit unsigned constant down to the nearest power of 4
fn rd4(x: u32) -> u32 {
    if x & 0x0000000c != 0 {
        rd2(x >> 2) << 2
    } else {
        rd2(x)
    }
}

/// Round a 32-bit unsigned constant down to the nearest power of 8
fn rd8(x: u32) -> u32 {
    if x & 0x000000f0 != 0 {
        rd4(x >> 4) << 4
    } else {
        rd4(x)
    }
}

/// Round a 32-bit unsigned constant down to the nearest power of 16
fn rd16(x: u32) -> u32 {
    if x & 0x0000ff00 != 0 {
        rd8(x >> 8) << 8
    } else {
        rd8(x)
    }
}

/// Round a 32-bit unsigned constant down to the nearest power of 32
fn rd32(x: u32) -> u32 {
    if x & 0xffff0000 != 0 {
        rd16(x >> 16) << 16
    } else {
        rd16(x)
    }
}
