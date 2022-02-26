use {
    core::{cmp, ptr::copy_nonoverlapping},
    xen_sys::{
        xenbus_state_XenbusStateClosed, xenbus_state_XenbusStateClosing,
        xenbus_state_XenbusStateConnected, xenbus_state_XenbusStateInitWait,
        xenbus_state_XenbusStateInitialised, xenbus_state_XenbusStateInitialising,
        xenbus_state_XenbusStateReconfigured, xenbus_state_XenbusStateReconfiguring,
        xenbus_state_XenbusStateUnknown, xsd_sockmsg, xsd_sockmsg_type_XS_CONTROL,
        xsd_sockmsg_type_XS_DIRECTORY, xsd_sockmsg_type_XS_DIRECTORY_PART,
        xsd_sockmsg_type_XS_ERROR, xsd_sockmsg_type_XS_GET_DOMAIN_PATH,
        xsd_sockmsg_type_XS_GET_PERMS, xsd_sockmsg_type_XS_INTRODUCE,
        xsd_sockmsg_type_XS_IS_DOMAIN_INTRODUCED, xsd_sockmsg_type_XS_MKDIR,
        xsd_sockmsg_type_XS_READ, xsd_sockmsg_type_XS_RELEASE, xsd_sockmsg_type_XS_RESUME,
        xsd_sockmsg_type_XS_RM, xsd_sockmsg_type_XS_SET_PERMS, xsd_sockmsg_type_XS_SET_TARGET,
        xsd_sockmsg_type_XS_TRANSACTION_END, xsd_sockmsg_type_XS_TRANSACTION_START,
        xsd_sockmsg_type_XS_TYPE_COUNT, xsd_sockmsg_type_XS_UNWATCH, xsd_sockmsg_type_XS_WATCH,
        xsd_sockmsg_type_XS_WATCH_EVENT, xsd_sockmsg_type_XS_WRITE, XenbusState,
        XENSTORE_RING_SIZE,
    },
};

pub fn mask_xenstore_idx(idx: u32) -> u32 {
    idx & (XENSTORE_RING_SIZE - 1)
}

pub unsafe fn memcpy_from_ring(ring: *mut i8, destination: *mut i8, offset: usize, length: usize) {
    let c1 = cmp::min(length, XENSTORE_RING_SIZE as usize - offset);
    let c2 = length - c1;
    copy_nonoverlapping(ring.add(offset), destination, c1);
    copy_nonoverlapping(ring, destination.add(c1), c2);
}

/// State of XenBus connection
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

#[allow(non_upper_case_globals)]
const xsd_sockmsg_type_XS_RESET_WATCHES: u32 = xsd_sockmsg_type_XS_SET_TARGET + 2;

#[derive(Debug)]
pub enum Kind {
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

impl From<u32> for Kind {
    fn from(value: u32) -> Self {
        #[allow(non_upper_case_globals)]
        match value {
            xsd_sockmsg_type_XS_CONTROL => Kind::Control,
            xsd_sockmsg_type_XS_DIRECTORY => Kind::Directory,
            xsd_sockmsg_type_XS_READ => Kind::Read,
            xsd_sockmsg_type_XS_GET_PERMS => Kind::GetPerms,
            xsd_sockmsg_type_XS_WATCH => Kind::Watch,
            xsd_sockmsg_type_XS_UNWATCH => Kind::Unwatch,
            xsd_sockmsg_type_XS_TRANSACTION_START => Kind::TransactionStart,
            xsd_sockmsg_type_XS_TRANSACTION_END => Kind::TransactionEnd,
            xsd_sockmsg_type_XS_INTRODUCE => Kind::Introduce,
            xsd_sockmsg_type_XS_RELEASE => Kind::Release,
            xsd_sockmsg_type_XS_GET_DOMAIN_PATH => Kind::GetDomainPath,
            xsd_sockmsg_type_XS_WRITE => Kind::Write,
            xsd_sockmsg_type_XS_MKDIR => Kind::MakeDirectory,
            xsd_sockmsg_type_XS_RM => Kind::Remove,
            xsd_sockmsg_type_XS_SET_PERMS => Kind::SetPerms,
            xsd_sockmsg_type_XS_WATCH_EVENT => Kind::WatchEvent,
            xsd_sockmsg_type_XS_ERROR => Kind::Error,
            xsd_sockmsg_type_XS_IS_DOMAIN_INTRODUCED => Kind::IsDomainIntroduced,
            xsd_sockmsg_type_XS_RESUME => Kind::Resume,
            xsd_sockmsg_type_XS_SET_TARGET => Kind::SetTarget,
            xsd_sockmsg_type_XS_RESET_WATCHES => Kind::ResetWatches,
            xsd_sockmsg_type_XS_DIRECTORY_PART => Kind::DirectoryPart,
            xsd_sockmsg_type_XS_TYPE_COUNT => Kind::TypeCount,
            _ => Kind::Invalid,
        }
    }
}

impl From<Kind> for u32 {
    fn from(k: Kind) -> Self {
        match k {
            Kind::Control => xsd_sockmsg_type_XS_CONTROL,
            Kind::Debug => xsd_sockmsg_type_XS_CONTROL,
            Kind::Directory => xsd_sockmsg_type_XS_DIRECTORY,
            Kind::Read => xsd_sockmsg_type_XS_READ,
            Kind::GetPerms => xsd_sockmsg_type_XS_GET_PERMS,
            Kind::Watch => xsd_sockmsg_type_XS_WATCH,
            Kind::Unwatch => xsd_sockmsg_type_XS_UNWATCH,
            Kind::TransactionStart => xsd_sockmsg_type_XS_TRANSACTION_START,
            Kind::TransactionEnd => xsd_sockmsg_type_XS_TRANSACTION_END,
            Kind::Introduce => xsd_sockmsg_type_XS_INTRODUCE,
            Kind::Release => xsd_sockmsg_type_XS_RELEASE,
            Kind::GetDomainPath => xsd_sockmsg_type_XS_GET_DOMAIN_PATH,
            Kind::Write => xsd_sockmsg_type_XS_WRITE,
            Kind::MakeDirectory => xsd_sockmsg_type_XS_MKDIR,
            Kind::Remove => xsd_sockmsg_type_XS_RM,
            Kind::SetPerms => xsd_sockmsg_type_XS_SET_PERMS,
            Kind::WatchEvent => xsd_sockmsg_type_XS_WATCH_EVENT,
            Kind::Error => xsd_sockmsg_type_XS_ERROR,
            Kind::IsDomainIntroduced => xsd_sockmsg_type_XS_IS_DOMAIN_INTRODUCED,
            Kind::Resume => xsd_sockmsg_type_XS_RESUME,
            Kind::SetTarget => xsd_sockmsg_type_XS_SET_TARGET,
            Kind::ResetWatches => xsd_sockmsg_type_XS_RESET_WATCHES,
            Kind::DirectoryPart => xsd_sockmsg_type_XS_DIRECTORY_PART,
            Kind::TypeCount => xsd_sockmsg_type_XS_TYPE_COUNT,
            Kind::Invalid => 0xFFFF,
        }
    }
}

/// XenBus message header
#[derive(Debug)]
pub struct MessageHeader {
    /// Message type
    pub kind: Kind,
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
