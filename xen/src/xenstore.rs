//! XenStore interface
//!
//! "The XenStore is a storage system shared between Xen guests. It is a
//! simple hierarchical storage system, maintained by Domain 0 and accessed
//! via a shared memory page and an event channel." - The Definitive Guide
//! to the Xen Hypervisor, Chapter 8

use {
    crate::{
        events::event_channel_op,
        memory::{MachineFrameNumber, VirtualAddress},
        START_INFO,
    },
    alloc::{borrow::ToOwned, string::String, vec, vec::Vec},
    core::{
        convert::{TryFrom, TryInto},
        str,
        sync::atomic::{fence, Ordering},
    },
    lazy_static::lazy_static,
    log::{debug, error},
    spin::Mutex,
    xen_sys::{
        evtchn_port_t, evtchn_send, xenstore_domain_interface, xsd_sockmsg,
        xsd_sockmsg_type_XS_DIRECTORY, xsd_sockmsg_type_XS_READ, xsd_sockmsg_type_XS_WRITE,
        EVTCHNOP_send, XENSTORE_RING_SIZE,
    },
};

lazy_static! {
    /// Global XenStore interface
    static ref XENSTORE: Mutex<XenStore> = {
        let event_channel_port = unsafe { *START_INFO }.store_evtchn;

        // convert store_mfn to a virtual address
        let interface_ptr = VirtualAddress::from(MachineFrameNumber(
            unsafe { *START_INFO }
                .store_mfn
                .try_into()
                .expect("Failed to convert u64 to usize"),
        ))
        .0 as *mut xenstore_domain_interface;

        // convert the mutable pointer to a mutable reference with a static lifetime, using lazy_static ensures only a single mutable reference exists
        let interface =
            unsafe { interface_ptr.as_mut() }.expect("XenStore interface pointer was null");

        Mutex::new(XenStore {
            event_channel_port,
            interface,
            req_id: 0,
        })
    };
}

/// Initialize XenStore
pub fn init() {
    lazy_static::initialize(&XENSTORE);
    debug!("Initialized XenStore");
}

/// Write a key-value pair to the XenStore
pub fn write<K: AsRef<str>, V: AsRef<str>>(key: K, value: V) {
    XENSTORE.lock().write(key.as_ref(), value.as_ref())
}

/// Read a key's value from the XenStore
pub fn read<K: AsRef<str>>(key: K) -> String {
    XENSTORE.lock().read(key.as_ref())
}

/// List contents of directory
pub fn ls<K: AsRef<str>>(key: K) -> Vec<String> {
    XENSTORE.lock().ls(key.as_ref())
}

/// Read the current domain's ID
pub fn domain_id() -> u32 {
    XENSTORE.lock().domain_id()
}

/// XenStore interface
#[derive(Debug)]
struct XenStore {
    event_channel_port: evtchn_port_t,
    interface: &'static mut xenstore_domain_interface,
    req_id: u32,
}

/// Required due to the raw mutable pointer not being Send, this is safe as the virtual address it refers to is constant throughout the lifetime of the kernel
unsafe impl Send for XenStore {}

impl XenStore {
    /// Write a key-value pair to the XenStore
    fn write(&mut self, key: &str, value: &str) {
        let msg = &mut xsd_sockmsg {
            type_: xsd_sockmsg_type_XS_WRITE,
            req_id: self.req_id,
            tx_id: 0,
            len: (key.len() + value.len())
                .try_into()
                .expect("Failed to convert usize to u32"),
        };

        //TODO: validate that this is safe
        let msg_bytes = unsafe {
            core::mem::transmute::<&mut xsd_sockmsg, &mut [u8; core::mem::size_of::<xsd_sockmsg>()]>(
                msg,
            )
        };

        self.write_request(msg_bytes);
        self.write_request(key.as_bytes());
        self.write_request(value.as_bytes());

        self.notify();

        self.read_response(msg_bytes);
        self.ignore(msg.len.try_into().expect("Failed to convert u32 to usize"));

        self.req_id += 1;

        if msg.req_id != (self.req_id - 1) {
            panic!("XenStore read failed due to unexpected message request ID");
        }
    }

    /// Perform initial steps of a read operation, returning the length of value now ready to be read
    ///
    /// The read operation was split like this to allow for building operations requiring reads without allocating (e.g. domain_id)
    fn read_preamble(&mut self, key: &str) -> usize {
        let msg = &mut xsd_sockmsg {
            type_: xsd_sockmsg_type_XS_READ,
            req_id: self.req_id,
            tx_id: 0,
            len: key
                .len()
                .try_into()
                .expect("Failed to convert usize to u32"),
        };

        //TODO: validate that this is safe
        let msg_bytes = unsafe {
            core::mem::transmute::<&mut xsd_sockmsg, &mut [u8; core::mem::size_of::<xsd_sockmsg>()]>(
                msg,
            )
        };

        self.write_request(msg_bytes);
        self.write_request(key.as_bytes());

        self.notify();

        self.read_response(msg_bytes);

        let msg_len = msg.len.try_into().expect("Failed to convert u32 to usize");
        self.req_id += 1;

        if msg.req_id != (self.req_id - 1) {
            self.ignore(msg_len);
            panic!("XenStore read failed due to unexpected message request ID");
        }

        msg_len
    }

    /// Read a key's value from the XenStore
    ///
    /// Requires that the allocator be initialised before calling
    fn read(&mut self, key: &str) -> String {
        let msg_len = self.read_preamble(key);

        let mut buf = vec![0; msg_len];

        self.read_response(&mut buf);

        // remove nul terminator if it exists
        if let Some(0) = buf.last() {
            buf.truncate(buf.len() - 1);
        }

        // does not reallocate
        String::from_utf8(buf).expect("XenStore value contains invalid UTF-8")
    }

    /// List contents of directory
    fn ls(&mut self, key: &str) -> Vec<String> {
        let msg = &mut xsd_sockmsg {
            type_: xsd_sockmsg_type_XS_DIRECTORY,
            req_id: self.req_id,
            tx_id: 0,
            len: key
                .len()
                .try_into()
                .expect("Failed to convert usize to u32"),
        };

        //TODO: validate that this is safe
        let msg_bytes = unsafe {
            core::mem::transmute::<&mut xsd_sockmsg, &mut [u8; core::mem::size_of::<xsd_sockmsg>()]>(
                msg,
            )
        };

        self.write_request(msg_bytes);
        self.write_request(key.as_bytes());

        self.notify();

        self.read_response(msg_bytes);

        let msg_len = msg.len.try_into().expect("Failed to convert u32 to usize");
        self.req_id += 1;

        if msg.req_id != (self.req_id - 1) {
            self.ignore(msg_len);
            panic!("XenStore read failed due to unexpected message request ID");
        }

        let mut value = vec![0; msg_len];

        self.read_response(&mut value);

        value
            .split(|&c| c == 0)
            .map(|slice| match str::from_utf8(slice) {
                Ok(str) => str,
                Err(e) => {
                    error!(
                        "XenStore directory contained non UTF-8 key ({:?}), error: {}",
                        slice, e
                    );
                    ""
                }
            })
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect()
    }

    /// Read the current domain's ID
    fn domain_id(&mut self) -> u32 {
        // fill with newlines so that str::trim removes excess bytes
        let mut buf = [b'\n'; 4];

        let len = self.read_preamble("domid\0");
        self.read_response(&mut buf[..len]);

        // convert slice to str
        str::from_utf8(&buf)
            .expect("XenStore domid value contains invalid UTF-8")
            // remove leading and trailing whitespace
            .trim()
            // parse as u32
            .parse()
            .expect("Failed to parse XenStore domid value as u32")
    }

    fn notify(&self) {
        let mut event = evtchn_send {
            port: self.event_channel_port,
        };
        event_channel_op(EVTCHNOP_send, &mut event as *mut _ as u64);
    }

    fn ignore(&mut self, len: usize) {
        let mut buffer = [0u8; XENSTORE_RING_SIZE as usize];
        self.read_response(&mut buffer[..len]);
    }

    fn write_request(&mut self, mut message: &[u8]) {
        assert!(
            message.len()
                < XENSTORE_RING_SIZE
                    .try_into()
                    .expect("Failed to convert u32 to usize")
        );

        let mut i = self.interface.req_prod;
        let mut length = message.len();

        while length > 0 {
            let mut data;

            loop {
                data = i - self.interface.req_cons;
                fence(Ordering::SeqCst);

                if !(usize::try_from(data).expect("Failed to convert u32 to usize")
                    >= self.interface.req.len())
                {
                    break;
                }
            }

            let ring_index =
                mask_xenstore_idx(i.try_into().expect("Failed to convert u32 to usize"));
            self.interface.req[ring_index] = message[0] as i8;
            message = &message[1..];

            length -= 1;
            i += 1;
        }

        fence(Ordering::SeqCst);
        self.interface.req_prod = i;
    }

    fn read_response(&mut self, mut message: &mut [u8]) {
        let mut i = self.interface.rsp_cons;
        let mut length = message.len();

        while length > 0 {
            let mut data;

            loop {
                data = self.interface.rsp_prod - i;
                fence(Ordering::SeqCst);

                if !(data == 0) {
                    break;
                }
            }

            let ring_index =
                mask_xenstore_idx(i.try_into().expect("Failed to convert u32 to usize"));
            message[0] = self.interface.rsp[ring_index] as u8;
            message = &mut message[1..];

            length -= 1;
            i += 1;
        }

        self.interface.rsp_cons = i;
    }
}

fn mask_xenstore_idx(idx: usize) -> usize {
    idx & (usize::try_from(XENSTORE_RING_SIZE).expect("Failed to convert u32 to usize") - 1)
}
