use {
    alloc::alloc::{alloc_zeroed, dealloc, Layout},
    core::{
        mem::size_of,
        sync::atomic::{fence, Ordering},
    },
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
        Self(S::new())
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

    pub fn get(&mut self, index: usize) -> *mut S::Element {
        unsafe { &mut *self.0 }.get(index)
    }

    pub fn req_prod(&self) -> u32 {
        unsafe { &*self.0 }.req_prod()
    }

    pub fn set_req_prod(&mut self, val: u32) {
        unsafe { &mut *self.0 }.set_req_prod(val)
    }

    pub fn req_event(&self) -> u32 {
        unsafe { &*self.0 }.req_event()
    }

    pub fn set_rsp_event(&mut self, val: u32) {
        unsafe { &mut *self.0 }.set_rsp_event(val)
    }
}

impl<S: SharedRingInner> Drop for Ring<S> {
    fn drop(&mut self) {
        unsafe { dealloc(self.0 as *mut u8, LAYOUT) }
    }
}

/// Trait for Xen shared ring types
pub trait SharedRingInner {
    type Element;

    fn new() -> *mut Self;

    fn size() -> usize;

    fn debug(&self);

    fn get(&mut self, index: usize) -> *mut Self::Element;

    fn req_prod(&self) -> u32;

    fn set_req_prod(&mut self, val: u32);

    fn req_event(&self) -> u32;

    fn set_rsp_event(&mut self, val: u32);
}

impl SharedRingInner for netif_tx_sring {
    type Element = netif_tx_sring_entry;

    fn new() -> *mut Self {
        let ptr = unsafe { alloc_zeroed(LAYOUT) as *mut netif_tx_sring };
        unsafe {
            (*ptr).req_event = 1;
            (*ptr).rsp_event = 1;
        }
        ptr
    }

    fn size() -> usize {
        rd32(
            ((PAGE_SIZE - offset_of!(netif_tx_sring, ring)) / size_of::<netif_tx_sring_entry>())
                as u32,
        ) as usize
    }

    fn debug(&self) {
        log::trace!(
            "tx: {} {} {} {}",
            self.req_prod,
            self.req_event,
            self.rsp_prod,
            self.rsp_event
        );
    }

    fn get(&mut self, index: usize) -> *mut Self::Element {
        unsafe { self.ring.as_mut_ptr().add(index) }
    }

    fn req_prod(&self) -> u32 {
        self.req_prod
    }

    fn set_req_prod(&mut self, val: u32) {
        self.req_prod = val
    }

    fn req_event(&self) -> u32 {
        self.req_event
    }

    fn set_rsp_event(&mut self, val: u32) {
        self.rsp_event = val
    }
}

impl SharedRingInner for netif_rx_sring {
    type Element = netif_rx_sring_entry;

    fn new() -> *mut Self {
        let ptr = unsafe { alloc_zeroed(LAYOUT) as *mut netif_rx_sring };
        unsafe {
            (*ptr).req_event = 1;
            (*ptr).rsp_event = 1;
        }
        ptr
    }

    fn size() -> usize {
        rd32(
            ((PAGE_SIZE - offset_of!(netif_rx_sring, ring)) / size_of::<netif_rx_sring_entry>())
                as u32,
        ) as usize
    }

    fn debug(&self) {
        log::trace!(
            "rx: {} {} {} {}",
            self.req_prod,
            self.req_event,
            self.rsp_prod,
            self.rsp_event
        );
    }

    fn get(&mut self, index: usize) -> *mut Self::Element {
        unsafe { self.ring.as_mut_ptr().add(index) }
    }

    fn req_prod(&self) -> u32 {
        self.req_prod
    }

    fn set_req_prod(&mut self, val: u32) {
        self.req_prod = val
    }

    fn req_event(&self) -> u32 {
        self.req_event
    }

    fn set_rsp_event(&mut self, val: u32) {
        self.rsp_event = val
    }
}

pub struct RingFront<S: SharedRingInner> {
    pub req_prod_pvt: u32,
    pub rsp_cons: u32,
    pub nr_ents: u32,
    pub sring: Ring<S>,
}

impl<S: SharedRingInner> RingFront<S> {
    pub fn get(&mut self, idx: usize) -> *mut S::Element {
        self.sring.get(idx & (S::size() - 1))
    }

    pub fn push_requests(&mut self) -> bool {
        let old = self.sring.req_prod();
        let new = self.req_prod_pvt;

        fence(Ordering::SeqCst);

        self.sring.set_req_prod(new);

        fence(Ordering::SeqCst);

        (new - self.sring.req_event()) < (new - old)
    }

    pub fn set_rsp_event(&mut self, val: u32) {
        self.sring.set_rsp_event(val)
    }
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
