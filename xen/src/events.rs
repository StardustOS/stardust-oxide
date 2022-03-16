//! Interface to Xen event channels

use {
    crate::{
        hypercall,
        platform::util::{init_events, synch_clear_bit, synch_set_bit},
        println, SHARED_INFO,
    },
    xen_sys::{
        __HYPERVISOR_event_channel_op, evtchn_bind_virq_t, evtchn_port_t, EVTCHNOP_bind_virq,
    },
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
pub static DEFAULT_HANDLER: fn(evtchn_port_t, *mut u8, *mut u8) =
    |port, _, _| log::warn!("received event on port {}", port);

/// Action associated with event
#[derive(Clone, Copy, Debug)]
pub struct EventAction {
    handler: fn(evtchn_port_t, *mut u8, *mut u8),
    data: usize,
    count: u32,
}

/// Initialise events, set default handler and mask all event ports
pub fn init() {
    for i in 0..unsafe { EVENT_ACTIONS.len() as u32 } {
        mask_event_channel(i);
    }

    init_events();

    unsafe { *SHARED_INFO }.vcpu_info[0].evtchn_upcall_mask = 0;
}

/// Execute event on supplied channel port
pub fn do_event(port: evtchn_port_t) {
    unsafe {
        EVENT_ACTIONS[port as usize].count += 1;
        (EVENT_ACTIONS[port as usize].handler)(port, core::ptr::null_mut(), core::ptr::null_mut())
    }
}

/// Bind an event handler to an event channel
pub fn bind_event_channel(
    port: evtchn_port_t,
    handler: fn(evtchn_port_t, *mut u8, *mut u8),
    data: usize,
) {
    let idx = port as usize;

    if unsafe { EVENT_ACTIONS[idx].handler != DEFAULT_HANDLER } {
        println!(
            "Warning: handler for port {} already registered, replacing",
            port,
        );
    }

    unsafe {
        EVENT_ACTIONS[idx].data = data;
        EVENT_ACTIONS[idx].count = 0;
        EVENT_ACTIONS[idx].handler = handler;
    }

    unmask_event_channel(port);
}

/// Bind a handler to a VIRQ
pub fn bind_virq(
    virq: u32,
    handler: fn(evtchn_port_t, *mut u8, *mut u8),
    data: usize,
) -> evtchn_port_t {
    let mut op = evtchn_bind_virq_t {
        virq,
        vcpu: 0,
        port: 0,
    };

    event_channel_op(EVTCHNOP_bind_virq, &mut op as *mut _ as u64);

    bind_event_channel(op.port, handler, data);

    op.port
}

/// Mask an event channel port
pub fn mask_event_channel(port: evtchn_port_t) {
    unsafe { synch_set_bit(port.into(), &mut (*SHARED_INFO).evtchn_mask[0]) }
}

/// Unmask an event channel port
pub fn unmask_event_channel(port: evtchn_port_t) {
    unsafe { synch_clear_bit(port.into(), &mut (*SHARED_INFO).evtchn_mask[0]) }
}

/// Clear event channel port
pub fn clear_event_channel(port: evtchn_port_t) {
    unsafe { synch_clear_bit(port.into(), &mut (*SHARED_INFO).evtchn_pending[0]) }
}

/// Event channel operation hypercall
pub fn event_channel_op(cmd: u32, op_ptr: u64) {
    unsafe { hypercall!(__HYPERVISOR_event_channel_op, cmd, op_ptr) }
        .expect("Event channel operation failed");
}
