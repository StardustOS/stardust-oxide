//! XenBus
//!
//! "provides a way of enumerating the (virtual) devices available to a given domain, and connecting to them"

use {
    crate::{
        events::event_channel_op,
        memory::{MachineFrameNumber, VirtualAddress},
        START_INFO,
    },
    alloc::{collections::BTreeMap, string::String},
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
        evtchn_send_t, xenbus_state_XenbusStateClosed, xenbus_state_XenbusStateClosing,
        xenbus_state_XenbusStateConnected, xenbus_state_XenbusStateInitWait,
        xenbus_state_XenbusStateInitialised, xenbus_state_XenbusStateInitialising,
        xenbus_state_XenbusStateReconfigured, xenbus_state_XenbusStateReconfiguring,
        xenbus_state_XenbusStateUnknown, xenstore_domain_interface, xsd_sockmsg,
        xsd_sockmsg_type_XS_CONTROL, xsd_sockmsg_type_XS_DIRECTORY,
        xsd_sockmsg_type_XS_DIRECTORY_PART, xsd_sockmsg_type_XS_ERROR,
        xsd_sockmsg_type_XS_GET_DOMAIN_PATH, xsd_sockmsg_type_XS_GET_PERMS,
        xsd_sockmsg_type_XS_INTRODUCE, xsd_sockmsg_type_XS_IS_DOMAIN_INTRODUCED,
        xsd_sockmsg_type_XS_MKDIR, xsd_sockmsg_type_XS_READ, xsd_sockmsg_type_XS_RELEASE,
        xsd_sockmsg_type_XS_RESUME, xsd_sockmsg_type_XS_RM, xsd_sockmsg_type_XS_SET_PERMS,
        xsd_sockmsg_type_XS_SET_TARGET, xsd_sockmsg_type_XS_TRANSACTION_END,
        xsd_sockmsg_type_XS_TRANSACTION_START, xsd_sockmsg_type_XS_TYPE_COUNT,
        xsd_sockmsg_type_XS_UNWATCH, xsd_sockmsg_type_XS_WATCH, xsd_sockmsg_type_XS_WATCH_EVENT,
        xsd_sockmsg_type_XS_WRITE, EVTCHNOP_send, XenbusState, XENSTORE_RING_SIZE,
    },
};

mod task;

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
pub async fn request(kind: MessageKind, data: &[&[u8]], tx_id: u32) -> (MessageHeader, String) {
    let header = MessageHeader {
        kind,
        request_id: 0,
        transaction_id: tx_id,
    };

    XENBUS.lock().write(header, data);

    // call "background" task directly to block until response is received
    task::task();

    loop {
        // assign result of removal to limit lifetime of held lock
        let resp = XENBUS.lock().responses.remove(&0);
        if let Some(r) = resp {
            return r;
        }
    }
}

#[derive(Debug)]
struct XenBus {
    interface: &'static mut xenstore_domain_interface,
    event_channel: u32,
    responses: BTreeMap<u32, (MessageHeader, String)>,
}

impl XenBus {
    fn notify(&self) {
        let op = evtchn_send_t {
            port: self.event_channel,
        };
        event_channel_op(EVTCHNOP_send, &op as *const _ as u64);
    }

    fn write(&mut self, header: MessageHeader, data: &[&[u8]]) {
        let mut m = xsd_sockmsg::from(header);
        m.len = data.iter().map(|s| s.len() as u32).sum();

        let len = m.len + size_of::<xsd_sockmsg>() as u32;

        assert!(len < XENSTORE_RING_SIZE);

        // Wait for the ring to drain to the point where we can send the message
        let mut prod = self.interface.req_prod;
        if prod + len - self.interface.req_cons > XENSTORE_RING_SIZE {
            // Wait for there to be space on the ring
            while self.interface.req_prod + m.len - self.interface.req_cons <= XENSTORE_RING_SIZE {}
            prod = self.interface.req_prod;
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
                    self.interface
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
                        cur_req = &data[req_idx];
                    } else {
                        req_idx += 1;
                        cur_req = &data[req_idx];
                    }
                }
            }
        }

        assert!(req_off != 0);
        //  assert!(total_off != len);
        //assert!(prod > xenbus.interface.req_cons + XENSTORE_RING_SIZE);

        fence(Ordering::SeqCst);

        self.interface.req_prod += len;

        self.notify();
    }
}

fn mask_xenstore_idx(idx: u32) -> u32 {
    idx & (XENSTORE_RING_SIZE - 1)
}

unsafe fn copy_from_ring<T: Copy>(ring: &[T], destination: &mut [T], offset: usize, length: usize) {
    let c1 = cmp::min(length, XENSTORE_RING_SIZE as usize - offset);
    let c2 = length - c1;

    if c1 != 0 {
        destination[..c1].copy_from_slice(&ring[offset..offset + c1]);
    }

    if c2 != 0 {
        destination[c1..].copy_from_slice(&ring[..c2]);
    }
}

/// State of XenBus connection
#[derive(Debug)]
pub enum State {
    /// Initial state of the device on the bus, before either end has been connected
    Unknown,
    /// Backend is in process of initializing itself
    Initialising,
    /// Entered by the back end while it is waiting for information before completing initialization
    InitWait,
    /// Backend is now ready for connection
    Initialised,
    /// Normal state of the bus
    Connected,
    /// Device has become unavailable, backend is no longer doing useful work
    Closing,
    /// Both halves of driver have disconnected
    Closed,
    /// Device is being reconfigured
    Reconfiguring,
    /// Device reconfiguring has finished
    Reconfigured,
}

impl From<XenbusState> for State {
    fn from(s: XenbusState) -> Self {
        #[allow(non_upper_case_globals)]
        match s {
            xenbus_state_XenbusStateUnknown => State::Unknown,
            xenbus_state_XenbusStateInitialising => State::Initialising,
            xenbus_state_XenbusStateInitWait => State::InitWait,
            xenbus_state_XenbusStateInitialised => State::Initialised,
            xenbus_state_XenbusStateConnected => State::Connected,
            xenbus_state_XenbusStateClosing => State::Closing,
            xenbus_state_XenbusStateClosed => State::Closed,
            xenbus_state_XenbusStateReconfiguring => State::Reconfiguring,
            xenbus_state_XenbusStateReconfigured => State::Reconfigured,
            _ => panic!("Unknown XenBus state during conversion"),
        }
    }
}

impl From<State> for XenbusState {
    fn from(s: State) -> Self {
        match s {
            State::Unknown => xenbus_state_XenbusStateUnknown,
            State::Initialising => xenbus_state_XenbusStateInitialising,
            State::InitWait => xenbus_state_XenbusStateInitWait,
            State::Initialised => xenbus_state_XenbusStateInitialised,
            State::Connected => xenbus_state_XenbusStateConnected,
            State::Closing => xenbus_state_XenbusStateClosing,
            State::Closed => xenbus_state_XenbusStateClosed,
            State::Reconfiguring => xenbus_state_XenbusStateReconfiguring,
            State::Reconfigured => xenbus_state_XenbusStateReconfigured,
        }
    }
}

#[allow(non_upper_case_globals)]
const xsd_sockmsg_type_XS_RESET_WATCHES: u32 = xsd_sockmsg_type_XS_SET_TARGET + 2;

/// XenBus message type
#[derive(Debug)]
#[allow(missing_docs)]
pub enum MessageKind {
    Control,
    Debug,
    Directory,
    DirectoryPart,
    Read,
    GetPerms,
    Watch,
    Unwatch,
    TransactionStart,
    TransactionEnd,
    Introduce,
    Release,
    GetDomainPath,
    Write,
    MakeDirectory,
    Remove,
    SetPerms,
    WatchEvent,
    Error,
    IsDomainIntroduced,
    Resume,
    SetTarget,
    ResetWatches,
    /// Number of valid types
    TypeCount,
    /// Guaranteed to remain an invalid type
    Invalid,
}

impl From<u32> for MessageKind {
    fn from(value: u32) -> Self {
        #[allow(non_upper_case_globals)]
        match value {
            xsd_sockmsg_type_XS_CONTROL => MessageKind::Control,
            xsd_sockmsg_type_XS_DIRECTORY => MessageKind::Directory,
            xsd_sockmsg_type_XS_READ => MessageKind::Read,
            xsd_sockmsg_type_XS_GET_PERMS => MessageKind::GetPerms,
            xsd_sockmsg_type_XS_WATCH => MessageKind::Watch,
            xsd_sockmsg_type_XS_UNWATCH => MessageKind::Unwatch,
            xsd_sockmsg_type_XS_TRANSACTION_START => MessageKind::TransactionStart,
            xsd_sockmsg_type_XS_TRANSACTION_END => MessageKind::TransactionEnd,
            xsd_sockmsg_type_XS_INTRODUCE => MessageKind::Introduce,
            xsd_sockmsg_type_XS_RELEASE => MessageKind::Release,
            xsd_sockmsg_type_XS_GET_DOMAIN_PATH => MessageKind::GetDomainPath,
            xsd_sockmsg_type_XS_WRITE => MessageKind::Write,
            xsd_sockmsg_type_XS_MKDIR => MessageKind::MakeDirectory,
            xsd_sockmsg_type_XS_RM => MessageKind::Remove,
            xsd_sockmsg_type_XS_SET_PERMS => MessageKind::SetPerms,
            xsd_sockmsg_type_XS_WATCH_EVENT => MessageKind::WatchEvent,
            xsd_sockmsg_type_XS_ERROR => MessageKind::Error,
            xsd_sockmsg_type_XS_IS_DOMAIN_INTRODUCED => MessageKind::IsDomainIntroduced,
            xsd_sockmsg_type_XS_RESUME => MessageKind::Resume,
            xsd_sockmsg_type_XS_SET_TARGET => MessageKind::SetTarget,
            xsd_sockmsg_type_XS_RESET_WATCHES => MessageKind::ResetWatches,
            xsd_sockmsg_type_XS_DIRECTORY_PART => MessageKind::DirectoryPart,
            xsd_sockmsg_type_XS_TYPE_COUNT => MessageKind::TypeCount,
            _ => MessageKind::Invalid,
        }
    }
}

impl From<MessageKind> for u32 {
    fn from(k: MessageKind) -> Self {
        match k {
            MessageKind::Control => xsd_sockmsg_type_XS_CONTROL,
            MessageKind::Debug => xsd_sockmsg_type_XS_CONTROL,
            MessageKind::Directory => xsd_sockmsg_type_XS_DIRECTORY,
            MessageKind::Read => xsd_sockmsg_type_XS_READ,
            MessageKind::GetPerms => xsd_sockmsg_type_XS_GET_PERMS,
            MessageKind::Watch => xsd_sockmsg_type_XS_WATCH,
            MessageKind::Unwatch => xsd_sockmsg_type_XS_UNWATCH,
            MessageKind::TransactionStart => xsd_sockmsg_type_XS_TRANSACTION_START,
            MessageKind::TransactionEnd => xsd_sockmsg_type_XS_TRANSACTION_END,
            MessageKind::Introduce => xsd_sockmsg_type_XS_INTRODUCE,
            MessageKind::Release => xsd_sockmsg_type_XS_RELEASE,
            MessageKind::GetDomainPath => xsd_sockmsg_type_XS_GET_DOMAIN_PATH,
            MessageKind::Write => xsd_sockmsg_type_XS_WRITE,
            MessageKind::MakeDirectory => xsd_sockmsg_type_XS_MKDIR,
            MessageKind::Remove => xsd_sockmsg_type_XS_RM,
            MessageKind::SetPerms => xsd_sockmsg_type_XS_SET_PERMS,
            MessageKind::WatchEvent => xsd_sockmsg_type_XS_WATCH_EVENT,
            MessageKind::Error => xsd_sockmsg_type_XS_ERROR,
            MessageKind::IsDomainIntroduced => xsd_sockmsg_type_XS_IS_DOMAIN_INTRODUCED,
            MessageKind::Resume => xsd_sockmsg_type_XS_RESUME,
            MessageKind::SetTarget => xsd_sockmsg_type_XS_SET_TARGET,
            MessageKind::ResetWatches => xsd_sockmsg_type_XS_RESET_WATCHES,
            MessageKind::DirectoryPart => xsd_sockmsg_type_XS_DIRECTORY_PART,
            MessageKind::TypeCount => xsd_sockmsg_type_XS_TYPE_COUNT,
            MessageKind::Invalid => 0xFFFF,
        }
    }
}

/// XenBus message header
#[derive(Debug)]
pub struct MessageHeader {
    /// Message type
    pub kind: MessageKind,
    /// Request ID
    pub request_id: u32,
    /// Transaction ID
    pub transaction_id: u32,
}

impl From<xsd_sockmsg> for MessageHeader {
    fn from(m: xsd_sockmsg) -> Self {
        Self {
            kind: m.type_.into(),
            request_id: m.req_id,
            transaction_id: m.tx_id,
        }
    }
}

impl From<MessageHeader> for xsd_sockmsg {
    fn from(m: MessageHeader) -> Self {
        Self {
            type_: m.kind.into(),
            req_id: m.request_id,
            tx_id: m.transaction_id,
            len: 0,
        }
    }
}
