//! Console utilities

extern "C" {
    static mut console: *mut xencons_interface;
    static mut console_evt: evtchn_port_t;
}

use {
    core::{
        fmt,
        sync::atomic::{fence, Ordering},
    },
    spin::Mutex,
    xen::{
        platform::x86_64::hypercall::hypercall2,
        xen_sys::{
            evtchn_port_t, evtchn_send, xencons_interface, EVTCHNOP_send, SCHEDOP_yield,
            __HYPERVISOR_event_channel_op, __HYPERVISOR_sched_op,
        },
    },
};

static WRITER: Mutex<Option<Writer>> = Mutex::new(Some(Writer));

/// Xen console writer
struct Writer;

impl Writer {
    fn write_byte(&mut self, event: &evtchn_send, byte: u8) {
        loop {
            let prod = unsafe { (*console).out_prod };
            let cons = unsafe { (*console).out_cons };
            let data = prod.wrapping_sub(cons);

            unsafe {
                hypercall2(
                    __HYPERVISOR_event_channel_op,
                    u64::from(EVTCHNOP_send),
                    event as *const evtchn_send as u64,
                )
            };

            fence(Ordering::SeqCst);

            if !(data >= 2048) {
                break;
            }
        }

        let out_prod = unsafe { (*console).out_prod };
        let ring_index = (out_prod & (2047)) as usize;

        unsafe { (*console).out[ring_index] = byte as i8 };

        fence(Ordering::SeqCst);

        unsafe { (*console).out_prod += 1 };
    }

    fn write_bytes<I: Iterator<Item = u8>>(&mut self, bytes: I) {
        let event = evtchn_send {
            port: unsafe { console_evt },
        };

        for byte in bytes {
            self.write_byte(&event, byte);

            if byte == b'\n' {
                self.write_byte(&event, b'\r');
            }
        }

        unsafe {
            hypercall2(
                __HYPERVISOR_event_channel_op,
                u64::from(EVTCHNOP_send),
                &event as *const evtchn_send as u64,
            )
        };
    }

    fn flush(&self) {
        unsafe {
            while (*console).out_cons < (*console).out_prod {
                hypercall2(__HYPERVISOR_sched_op, u64::from(SCHEDOP_yield), 0);
                fence(Ordering::SeqCst);
            }
        };
    }
}

impl fmt::Write for Writer {
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
