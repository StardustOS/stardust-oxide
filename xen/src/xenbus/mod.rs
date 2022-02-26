//! XenBus
//!
//! "provides a way of enumerating the (virtual) devices available to a given domain, and connecting to them"

use {
    crate::{
        events::event_channel_op,
        memory::{MachineFrameNumber, VirtualAddress},
        xenbus::util::mask_xenstore_idx,
        START_INFO,
    },
    alloc::{collections::BTreeMap, vec::Vec},
    core::{
        cmp,
        convert::TryInto,
        mem::size_of,
        ptr::copy_nonoverlapping,
        slice,
        sync::atomic::{fence, Ordering},
    },
    lazy_static::lazy_static,
    log::debug,
    spin::Mutex,
    xen_sys::{
        evtchn_send_t, xenstore_domain_interface, xsd_sockmsg, xsd_sockmsg_type_XS_CONTROL,
        EVTCHNOP_send, XENSTORE_RING_SIZE,
    },
};

pub use task::task;

use self::util::MessageHeader;

mod task;
mod util;

lazy_static! {
    /// Global XenBus interface
    static ref XENBUS: Mutex<XenBus> = {
        let interface = unsafe {
            &mut *(VirtualAddress::from(MachineFrameNumber(
                (*START_INFO)
                    .store_mfn
                    .try_into()
                    .expect("Failed to convert u64 to usize"),
            ))
            .0 as *mut xenstore_domain_interface)
        };

        let event_channel = unsafe { *START_INFO }.store_evtchn;

        let responses = BTreeMap::new();

        debug!("Initialized XenBus: {:p}", interface);

        Mutex::new(XenBus { interface, event_channel, responses})
    };
}

/// Initialize XenStore
pub fn init() {
    lazy_static::initialize(&XENBUS);
}

/// Associates a value with a path
pub fn request() {
    xb_write(
        &mut XENBUS.lock(),
        xsd_sockmsg_type_XS_CONTROL,
        0,
        0,
        &[b"print\0", b"hello world!", b"\0"],
    );

    let resp = XENBUS
        .lock()
        .responses
        .remove(&0)
        .map(|(msg, data)| (msg, alloc::string::String::from_utf8(data)));
    log::debug!("got response {:?}", resp);
}

#[derive(Debug)]
struct XenBus {
    interface: &'static mut xenstore_domain_interface,
    event_channel: u32,
    responses: BTreeMap<u32, (MessageHeader, Vec<u8>)>,
}

impl XenBus {
    fn notify(&self) {
        let op = evtchn_send_t {
            port: self.event_channel,
        };
        event_channel_op(EVTCHNOP_send, &op as *const _ as u64);
    }
}

fn xb_write(xenbus: &mut XenBus, type_: u32, req_id: u32, tx_id: u32, reqs: &[&[u8]]) {
    let m = xsd_sockmsg {
        type_,
        req_id,
        tx_id,
        len: reqs.iter().map(|s| s.len() as u32).sum(),
    };

    let len = m.len + size_of::<xsd_sockmsg>() as u32;

    assert!(len < XENSTORE_RING_SIZE);

    // Wait for the ring to drain to the point where we can send the message
    let mut prod = xenbus.interface.req_prod;
    if prod + len - xenbus.interface.req_cons > XENSTORE_RING_SIZE {
        // Wait for there to be space on the ring
        while xenbus.interface.req_prod + m.len - xenbus.interface.req_cons <= XENSTORE_RING_SIZE {}
        prod = xenbus.interface.req_prod;
    }

    // overflowing ring now impossible

    let header_req =
        unsafe { slice::from_raw_parts(&m as *const _ as *const u8, size_of::<xsd_sockmsg>()) };

    let mut total_off = 0;
    let mut req_off = 0;
    let mut cur_req = &header_req;
    let mut req_idx = 0;

    while total_off < len {
        let this_chunk = cmp::min(
            cur_req.len() - req_off,
            (XENSTORE_RING_SIZE - mask_xenstore_idx(prod)) as usize,
        );

        unsafe {
            copy_nonoverlapping(
                (*cur_req).as_ptr().add(req_off),
                xenbus
                    .interface
                    .req
                    .as_mut_ptr()
                    .add(mask_xenstore_idx(prod) as usize) as *mut u8,
                this_chunk as usize,
            )
        };

        prod += this_chunk as u32;
        req_off += this_chunk;
        total_off += this_chunk as u32;

        if req_off == cur_req.len() {
            if total_off < len {
                req_off = 0;
                if cur_req == &header_req {
                    cur_req = &reqs[req_idx];
                } else {
                    req_idx += 1;
                    cur_req = &reqs[req_idx];
                }
            }
        }
    }

    debug!("completed main xb-write loop");
    assert!(req_off != 0);
    //  assert!(total_off != len);
    //assert!(prod > xenbus.interface.req_cons + XENSTORE_RING_SIZE);

    fence(Ordering::SeqCst);

    xenbus.interface.req_prod += len;

    xenbus.notify();
}
