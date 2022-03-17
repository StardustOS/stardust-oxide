use {
    alloc::alloc::{alloc_zeroed, dealloc, Layout},
    core::{
        fmt::Debug,
        mem::size_of,
        sync::atomic::{fence, Ordering},
    },
    memoffset::offset_of,
    xen::{
        platform::consts::PAGE_SIZE,
        xen_sys::{netif_rx_sring, netif_rx_sring_entry, netif_tx_sring, netif_tx_sring_entry},
    },
};

pub const PAGE_LAYOUT: Layout = unsafe { Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE) };

pub struct Ring<S: 'static + RawRing> {
    pub req_prod_pvt: u32,
    pub rsp_cons: u32,
    pub nr_ents: u32,
    pub sring: &'static mut S,
}

impl<S: RawRing> Drop for Ring<S> {
    fn drop(&mut self) {
        unsafe { dealloc(self.sring as *mut _ as *mut u8, PAGE_LAYOUT) }
    }
}

impl<S: RawRing> Debug for Ring<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RingFront")
            .field("req_prod_pvt", &self.req_prod_pvt)
            .field("rsp_cons", &self.rsp_cons)
            .field("nr_ents", &self.nr_ents)
            .field("sring_req_prod", &self.sring.req_prod())
            .field("sring_req_event", &self.sring.req_event())
            .field("sring_rsp_prod", &self.sring.rsp_prod())
            .field("sring_rsp_event", &self.sring.rsp_event())
            .finish()
    }
}

impl<S: RawRing> Ring<S> {
    pub fn new() -> Self {
        Self {
            req_prod_pvt: 0,
            rsp_cons: 0,
            nr_ents: S::size() as u32,
            sring: unsafe { &mut *S::new() },
        }
    }

    pub fn size(&self) -> usize {
        S::size()
    }

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

    fn unconsumed_responses(&self) -> u32 {
        self.sring.rsp_prod() - self.rsp_cons
    }

    pub fn check_for_responses(&mut self) -> u32 {
        let todo = self.unconsumed_responses();

        if todo > 0 {
            return todo;
        }

        self.set_rsp_event(self.rsp_cons + 1);
        fence(Ordering::SeqCst);

        self.unconsumed_responses()
    }
}

/// Trait for Xen shared ring types
pub trait RawRing {
    type Element;

    fn new() -> *mut Self;

    fn size() -> usize;

    fn debug(&self);

    fn get(&mut self, index: usize) -> *mut Self::Element;

    fn req_prod(&self) -> u32;
    fn req_event(&self) -> u32;
    fn rsp_prod(&self) -> u32;
    fn rsp_event(&self) -> u32;

    fn set_req_prod(&mut self, val: u32);
    fn set_req_event(&mut self, val: u32);
    fn set_rsp_prod(&mut self, val: u32);
    fn set_rsp_event(&mut self, val: u32);
}

impl RawRing for netif_tx_sring {
    type Element = netif_tx_sring_entry;

    fn new() -> *mut Self {
        let ptr = unsafe { alloc_zeroed(PAGE_LAYOUT) as *mut netif_tx_sring };
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
    fn req_event(&self) -> u32 {
        self.req_event
    }
    fn rsp_prod(&self) -> u32 {
        self.rsp_prod
    }
    fn rsp_event(&self) -> u32 {
        self.rsp_event
    }

    fn set_req_prod(&mut self, val: u32) {
        self.req_prod = val
    }
    fn set_req_event(&mut self, val: u32) {
        self.req_event = val
    }
    fn set_rsp_prod(&mut self, val: u32) {
        self.rsp_prod = val
    }
    fn set_rsp_event(&mut self, val: u32) {
        self.rsp_event = val
    }
}

impl RawRing for netif_rx_sring {
    type Element = netif_rx_sring_entry;

    fn new() -> *mut Self {
        let ptr = unsafe { alloc_zeroed(PAGE_LAYOUT) as *mut netif_rx_sring };
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
    fn req_event(&self) -> u32 {
        self.req_event
    }
    fn rsp_prod(&self) -> u32 {
        self.rsp_prod
    }
    fn rsp_event(&self) -> u32 {
        self.rsp_event
    }

    fn set_req_prod(&mut self, val: u32) {
        self.req_prod = val
    }
    fn set_req_event(&mut self, val: u32) {
        self.req_event = val
    }
    fn set_rsp_prod(&mut self, val: u32) {
        self.rsp_prod = val
    }
    fn set_rsp_event(&mut self, val: u32) {
        self.rsp_event = val
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
