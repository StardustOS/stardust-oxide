//! Interface to Xen event channels

use {
    crate::{
        hypercall,
        platform::util::{synch_clear_bit, synch_set_bit},
        println, SHARED_INFO,
    },
    core::convert::TryInto,
    xen_sys::__HYPERVISOR_event_channel_op,
};

/// Number of event channel ports
pub const NUM_EVENT_PORTS: usize = 1024;

/// Actions for each event channel
pub static mut EVENT_ACTIONS: [EventAction; NUM_EVENT_PORTS] = [EventAction {
    handler: DEFAULT_HANDLER,
    data: 0,
    count: 0,
}; NUM_EVENT_PORTS];

/// Default event handler
pub static DEFAULT_HANDLER: fn(usize, *mut u8) =
    |port, _| println!("received event on port {}", port);

/// Action associated with event
#[derive(Clone, Copy, Debug)]
pub struct EventAction {
    handler: fn(usize, *mut u8),
    data: usize,
    count: u32,
}

/// Initialise events, set default handler and mask all event ports
pub fn init() {
    for i in 0..unsafe { EVENT_ACTIONS.len() } {
        mask_event_channel(i);
    }
}

/// Bind an event handler to an event channel
pub fn bind_event_channel(port: usize, handler: fn(usize, *mut u8), data: usize) {
    if unsafe { EVENT_ACTIONS[port].handler != DEFAULT_HANDLER } {
        println!(
            "Warning: handler for port {} already registered, replacing",
            port,
        );
    }

    unsafe {
        EVENT_ACTIONS[port].data = data;
        EVENT_ACTIONS[port].count = 0;
        EVENT_ACTIONS[port].handler = handler;
    }
    unmask_event_channel(port);
}

fn mask_event_channel(port: usize) {
    unsafe {
        synch_set_bit(
            port.try_into().expect("Failed to convert usize to u64"),
            &mut (*SHARED_INFO).evtchn_mask[0],
        )
    }
}

fn unmask_event_channel(port: usize) {
    unsafe {
        synch_clear_bit(
            port.try_into().expect("failed to convert usize to u64"),
            &mut (*SHARED_INFO).evtchn_mask[0],
        )
    };
}

/// Event channel operation hypercall
pub fn event_channel_op(cmd: u32, op_ptr: u64) {
    let rc = unsafe { hypercall!(__HYPERVISOR_event_channel_op, cmd, op_ptr) };
    if rc != 0 {
        panic!("event channel op failed with error code: {}", rc);
    }
}
