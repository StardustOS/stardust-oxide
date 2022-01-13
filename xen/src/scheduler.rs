//! Virtual machine scheduler interface

use {
    crate::hypercall,
    xen_sys::{
        evtchn_port_t, sched_pin_override_t, sched_poll_t, sched_remote_shutdown_t,
        sched_shutdown_t, sched_watchdog_t, SCHEDOP_block, SCHEDOP_pin_override, SCHEDOP_poll,
        SCHEDOP_remote_shutdown, SCHEDOP_shutdown, SCHEDOP_shutdown_code, SCHEDOP_watchdog,
        SCHEDOP_yield, SHUTDOWN_crash, SHUTDOWN_poweroff, SHUTDOWN_reboot, SHUTDOWN_suspend,
        SHUTDOWN_watchdog, __HYPERVISOR_sched_op,
    },
};

/// Scheduling command
pub enum Command<'a> {
    /// Yield remaining of domain's quantum
    Yield,

    /// Deschedule domain until event is received
    Block,

    /// "Halt execution of this domain (all VCPUs) and notify the system controller.
    /// @arg == pointer to sched_shutdown_t structure.
    ///
    /// If the sched_shutdown_t reason is SHUTDOWN_suspend then
    /// x86 PV guests must also set RDX (EDX for 32-bit guests) to the MFN
    /// of the guest's start info page.  RDX/EDX is the third hypercall
    /// argument.
    ///
    ///  In addition, which reason is SHUTDOWN_suspend this hypercall
    ///  returns 1 if suspend was cancelled or the domain was merely
    /// checkpointed, and 0 if it is resuming in a new domain."
    Shutdown(ShutdownReason),

    /// "Poll a set of event-channel ports. Return when one or more are pending"
    Poll {
        /// Event-channel ports
        ports: &'a mut [evtchn_port_t],
        /// Nanoseconds since UNIX epoch that if reached while blocked on an event
        /// will result in a non-zero return value of the hypercall
        timeout: u64,
    },

    /// "Declare a shutdown for another domain. The main use of this function is
    /// in interpreting shutdown requests and reasons for fully-virtualized
    /// domains. A para-virtualized domain may use SCHEDOP_shutdown directly."
    RemoteShutdown {
        /// Remote domain ID
        domain_id: u16,
        /// Reason for shutdown
        reason: ShutdownReason,
    },

    /// "Latch a shutdown code, so that when the domain later shuts down it
    /// reports this code to the control tools"
    ShutdownCode(ShutdownReason),

    /// "Setup, poke and destroy a domain watchdog timer.
    /// @arg == pointer to sched_watchdog_t structure.
    /// With id == 0, setup a domain watchdog timer to cause domain shutdown
    ///               after timeout, returns watchdog id.
    /// With id != 0 and timeout == 0, destroy domain watchdog timer.
    /// With id != 0 and timeout != 0, poke watchdog timer and set new timeout."
    Watchdog {
        /// ID of watchdog timer
        id: u32,
        /// Timeout
        timeout: u32,
    },

    /// "Override the current vcpu affinity by pinning it to one physical cpu or
    /// undo this override restoring the previous affinity.
    /// @arg == pointer to sched_pin_override_t structure.
    ///
    /// A negative pcpu value will undo a previous pin override and restore the
    /// previous cpu affinity.
    /// This call is allowed for the hardware domain only and requires the cpu
    /// to be part of the domain's cpupool."
    PinOverride {
        /// Physical CPU ID to pin to
        pcpu: i32,
    },
}

/// Reasons for `Command::Shutdown`
pub enum ShutdownReason {
    /// "Domain exited normally. Clean up and kill."
    Poweroff = SHUTDOWN_poweroff as isize,
    /// "Clean up, kill, and then restart."
    Reboot = SHUTDOWN_reboot as isize,
    /// "Clean up, save suspend info, kill."
    Suspend = SHUTDOWN_suspend as isize,
    /// "Tell controller we've crashed."
    Crash = SHUTDOWN_crash as isize,
    /// "Restart because watchdog time expired."
    Watchdog = SHUTDOWN_watchdog as isize,
}

unsafe fn sched_op(cmd: u32, arg: u64) {
    hypercall!(__HYPERVISOR_sched_op, cmd, arg).expect("Failed schedule operation");
}

///
pub fn schedule_operation(cmd: Command) {
    unsafe {
        match cmd {
            Command::Yield => sched_op(SCHEDOP_yield, 0),
            Command::Block => sched_op(SCHEDOP_block, 0),
            Command::Shutdown(reason) => {
                let arg = sched_shutdown_t {
                    reason: reason as u32,
                };

                sched_op(SCHEDOP_shutdown, &arg as *const sched_shutdown_t as u64)
            }
            Command::Poll { ports, timeout } => {
                let arg = sched_poll_t {
                    ports: ports.as_mut_ptr(),
                    nr_ports: ports.len() as u32,
                    timeout,
                };

                sched_op(SCHEDOP_poll, &arg as *const sched_poll_t as u64)
            }
            Command::RemoteShutdown { domain_id, reason } => {
                let arg = sched_remote_shutdown_t {
                    domain_id,
                    reason: reason as u32,
                };

                sched_op(
                    SCHEDOP_remote_shutdown,
                    &arg as *const sched_remote_shutdown_t as u64,
                )
            }
            Command::ShutdownCode(reason) => {
                let arg = sched_shutdown_t {
                    reason: reason as u32,
                };

                sched_op(
                    SCHEDOP_shutdown_code,
                    &arg as *const sched_shutdown_t as u64,
                )
            }
            Command::Watchdog { id, timeout } => {
                let arg = sched_watchdog_t { id, timeout };

                sched_op(SCHEDOP_watchdog, &arg as *const sched_watchdog_t as u64)
            }
            Command::PinOverride { pcpu } => {
                let arg = sched_pin_override_t { pcpu };

                sched_op(
                    SCHEDOP_pin_override,
                    &arg as *const sched_pin_override_t as u64,
                )
            }
        };
    }
}
