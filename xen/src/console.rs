//! Console utilities

use {
    crate::{
        hypercall,
        scheduler::{schedule_operation, Command},
        text_start,
        xen_sys::{
            evtchn_port_t, evtchn_send, xencons_interface, EVTCHNOP_send,
            __HYPERVISOR_event_channel_op, start_info_t, __HYPERVISOR_VIRT_START,
        },
    },
    core::{
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

        // SAFETY: sound as long as `start` is a valid reference and the OS is
        // running in an unprivileged (non-Dom0) domain
        let dom_u = unsafe { start.console.domU };

        let page_num = dom_u.mfn as isize;

        let hypervisor_virt_start = __HYPERVISOR_VIRT_START as *mut usize;

        // SAFETY:
        let console = unsafe {
            &mut *(((*hypervisor_virt_start.offset(page_num) << 12) + text_start() as usize)
                as *mut xencons_interface)
        };

        *WRITER.lock() = Some(Writer {
            console,
            console_evt: dom_u.evtchn,
        });
    }

    fn write_byte(&mut self, event: &evtchn_send, byte: u8) {
        loop {
            let data = self.console.out_prod.wrapping_sub(self.console.out_cons);

            unsafe {
                hypercall!(
                    __HYPERVISOR_event_channel_op,
                    EVTCHNOP_send,
                    event as *const evtchn_send as u64
                )
            };

            fence(Ordering::SeqCst);

            if data < 2048 {
                break;
            }
        }

        let out_prod = self.console.out_prod;
        let ring_index = (out_prod & (2047)) as usize;

        self.console.out[ring_index] = byte as i8;

        fence(Ordering::SeqCst);

        self.console.out_prod += 1;
    }

    fn write_bytes<I: Iterator<Item = u8>>(&mut self, bytes: I) {
        let event = evtchn_send {
            port: self.console_evt,
        };

        for byte in bytes {
            self.write_byte(&event, byte);

            if byte == b'\n' {
                self.write_byte(&event, b'\r');
            }
        }

        unsafe {
            hypercall!(
                __HYPERVISOR_event_channel_op,
                EVTCHNOP_send,
                &event as *const evtchn_send as u64
            )
        };
    }

    fn flush(&self) {
        while (self.console).out_cons < (self.console).out_prod {
            schedule_operation(Command::Yield);
            fence(Ordering::SeqCst);
        }
    }
}

impl<'a> fmt::Write for Writer<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.bytes();

        self.write_bytes(bytes);

        self.flush();

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
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
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
