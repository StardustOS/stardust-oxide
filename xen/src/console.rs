//! Console utilities

use {
    crate::{
        hypercall,
        mm::{MachineFrameNumber, VirtualAddress},
        scheduler::{schedule_operation, Command},
        xen_sys::{
            evtchn_port_t, evtchn_send, xencons_interface, EVTCHNOP_send,
            __HYPERVISOR_event_channel_op, start_info_t,
        },
    },
    core::{
        convert::TryInto,
        fmt,
        sync::atomic::{fence, Ordering},
    },
    spin::Mutex,
};

/// Global Xen console writer
static WRITER: Mutex<Option<Writer>> = Mutex::new(None);

/// Xen console writer
pub struct Writer<'a> {
    console: &'a mut xencons_interface,
    console_evt: evtchn_port_t,
}

impl<'a> Writer<'a> {
    /// Initialize the global Xen console writer
    pub fn init(start: &start_info_t) {
        // this might be unnecessary, re-initializing shouldn't break anything
        // but it also shouldn't ever need to be done
        if WRITER.lock().is_some() {
            panic!("WRITER already initialized");
        }

        // SAFETY: `start` is a valid reference and the OS is
        // running in an unprivileged (non-Dom0) domain
        let dom_u = unsafe { start.console.domU };

        let console = {
            let virt = VirtualAddress::from(MachineFrameNumber(
                dom_u
                    .mfn
                    .try_into()
                    .expect("Failed to convert u64 to usize"),
            ));

            // SAFETY: will be a valid pointer for the lifetime of the instance
            unsafe { &mut *(virt.0 as *mut xencons_interface) }
        };

        *WRITER.lock() = Some(Writer {
            console,
            console_evt: dom_u.evtchn,
        });
    }

    /// Yield until all data has been written
    pub fn flush() {
        match *WRITER.lock() {
            Some(ref mut w) => {
                while (w.console).out_cons < (w.console).out_prod {
                    schedule_operation(Command::Yield);
                    fence(Ordering::SeqCst);
                }
            }
            None => panic!("WRITER not initialized"),
        }
    }

    fn write_bytes(&mut self, mut bytes: &[u8]) {
        while bytes.len() > 0 {
            let sent = self.xencons_write_bytes(bytes);
            self.event_send();
            bytes = &bytes[sent..];
        }

        self.event_send();
    }

    fn xencons_write_bytes(&mut self, bytes: &[u8]) -> usize {
        let mut sent = 0;

        let intf = &mut self.console;

        let mut prod = intf.out_prod;

        fence(Ordering::SeqCst);

        while (sent < bytes.len()) && (((prod - intf.out_cons) as usize) < intf.out.len()) {
            intf.out[((prod) & (intf.out.len() as u32 - 1)) as usize] = bytes[sent] as i8;
            prod += 1;
            sent += 1;
        }

        fence(Ordering::SeqCst);

        intf.out_prod = prod;

        sent
    }

    fn event_send(&self) {
        let mut op = evtchn_send {
            port: self.console_evt,
        };

        event_channel_op(EVTCHNOP_send, &mut op as *mut _ as u64);
    }
}

impl<'a> fmt::Write for Writer<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}

/// Prints and returns the value of a given expression for quick and dirty debugging
#[macro_export]
macro_rules! dbg {
    () => {
        $crate::println!("[{}:{}]", core::file!(), core::line!());
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::println!("[{}:{}] {} = {:#?}",
                core::file!(), core::line!(), core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}

/// Prints to the Xen console with newline and carriage return
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n\r"));
    ($($arg:tt)*) => ($crate::print!("{}\n\r", format_args!($($arg)*)));
}

/// Prints to the Xen console
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    match *WRITER.lock() {
        Some(ref mut w) => w.write_fmt(format_args!("{}\0", args)).unwrap(),
        None => panic!("WRITER not initialized"),
    }
}

fn event_channel_op(cmd: u32, op_ptr: u64) {
    let rc = unsafe { hypercall!(__HYPERVISOR_event_channel_op, cmd, op_ptr) };
    if rc != 0 {
        panic!("event channel op failed with error code: {}", rc);
    }
}
