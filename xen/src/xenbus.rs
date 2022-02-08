//! XenBus
//!
//! "provides a way of enumerating the (virtual) devices available to a given domain, and connecting to them"

use {
    crate::{
        memory::{update_va_mapping, MachineFrameNumber, PageEntry, TLBFlushFlags, VirtualAddress},
        platform::consts::PAGE_SIZE,
        START_INFO,
    },
    core::convert::TryInto,
    lazy_static::lazy_static,
    log::debug,
    spin::Mutex,
    xen_sys::{
        xenbus_state_XenbusStateClosed, xenbus_state_XenbusStateClosing,
        xenbus_state_XenbusStateConnected, xenbus_state_XenbusStateInitWait,
        xenbus_state_XenbusStateInitialised, xenbus_state_XenbusStateInitialising,
        xenbus_state_XenbusStateReconfigured, xenbus_state_XenbusStateReconfiguring,
        xenbus_state_XenbusStateUnknown, xenstore_domain_interface, XenbusState,
    },
};

extern "C" {
    static mut xenstore_page: [u8; PAGE_SIZE];
}

lazy_static! {
    /// Global XenBus interface
    static ref XENBUS: Mutex<XenBus> = {
        let page = VirtualAddress(unsafe { &mut xenstore_page as *mut _ } as usize);

        let store = PageEntry(
            VirtualAddress::from(MachineFrameNumber(
                unsafe { *START_INFO }
                    .store_mfn
                    .try_into()
                    .expect("Failed to convert u64 to usize"),
            ))
            .0,
        );

        update_va_mapping(page, store, TLBFlushFlags::INVLPG).expect("Failed to map XenStore page");

        let interface =
            unsafe { &mut *(&mut xenstore_page as *mut _ as *mut xenstore_domain_interface) };

        Mutex::new(XenBus { interface })
    };
}

/// Initialize XenStore
pub fn init() {
    lazy_static::initialize(&XENBUS);
    debug!("Initialized XenBus");
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

struct XenBus {
    interface: &'static mut xenstore_domain_interface,
}
