//! Blocking time

use {
    crate::{
        console::Writer,
        events::{bind_virq, unmask_event_channel},
        hypercall,
        scheduler::{schedule_operation, Command},
    },
    log::trace,
    xen_sys::{__HYPERVISOR_set_timer_op, VIRQ_TIMER},
};

pub use crate::platform::time::get_system_time;

/// Initialise time
pub fn init() {
    let port = bind_virq(
        VIRQ_TIMER,
        |_, _, _| set_timer_op(get_system_time() + 1_000_000),
        0,
    );

    trace!("time virq port: {}", port);
    unmask_event_channel(port);
}

/// Block for the supplied number of nanoseconds
pub fn block(until: u64) {
    if get_system_time() < until {
        log::trace!("here0");
        Writer::flush();

        log::trace!("here1");
        Writer::flush();

        schedule_operation(Command::Block);

        log::trace!("here2");
        Writer::flush();

        //local_irq_disable();
        set_timer_op(0);

        log::trace!("here3");
        Writer::flush();
    }
}

fn set_timer_op(until: u64) {
    unsafe {
        hypercall!(__HYPERVISOR_set_timer_op, until).expect("failed to set timer");
    }
}
