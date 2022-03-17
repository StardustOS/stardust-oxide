use {
    crate::xenbus::{copy_from_ring, mask_xenstore_idx, XENBUS},
    alloc::{string::String, vec, vec::Vec},
    core::{
        mem::{size_of, ManuallyDrop},
        slice,
        sync::atomic::{fence, Ordering},
    },
    xen_sys::{xsd_sockmsg, xsd_sockmsg_type_XS_WATCH_EVENT},
};

/// XenBus background task
///
/// Usually runs in a loop processing XenBus responses and events asynchronously, currently repurposed to block until a single response is read
pub fn task() {
    let mut msg = xsd_sockmsg {
        type_: 0,
        req_id: 0,
        tx_id: 0,
        len: 0,
    };

    loop {
        let mut xb = XENBUS.lock();

        if (xb.interface.rsp_prod - xb.interface.rsp_cons) < size_of::<xsd_sockmsg>() as u32 {
            continue;
        }

        unsafe {
            copy_from_ring(
                &xb.interface.rsp,
                slice::from_raw_parts_mut(&mut msg as *mut _ as *mut _, size_of::<xsd_sockmsg>()),
                mask_xenstore_idx(xb.interface.rsp_cons) as usize,
                size_of::<xsd_sockmsg>(),
            )
        };

        if xb.interface.rsp_prod - xb.interface.rsp_cons < size_of::<xsd_sockmsg>() as u32 + msg.len
        {
            continue;
        }

        if msg.type_ == xsd_sockmsg_type_XS_WATCH_EVENT {
            unimplemented!();
        } else {
            let mut data = vec![0; msg.len as usize];

            unsafe {
                copy_from_ring(
                    &xb.interface.rsp,
                    data.as_mut_slice(),
                    mask_xenstore_idx(xb.interface.rsp_cons + size_of::<xsd_sockmsg>() as u32)
                        as usize,
                    msg.len as usize,
                )
            };

            // remove trailing null byte
            if let Some(0) = data.last() {
                data.truncate(data.len() - 1);
            }

            // convert from Vec<i8> to Vec<u8>
            let data = {
                let mut v = ManuallyDrop::new(data);

                let p = v.as_mut_ptr();
                let len = v.len();
                let cap = v.capacity();

                unsafe { Vec::from_raw_parts(p as *mut u8, len, cap) }
            };

            // convert to String
            let contents = String::from_utf8(data).expect("XenBus returned invalid UTF-8");

            xb.responses.insert(msg.req_id, (msg.into(), contents));
        }

        fence(Ordering::SeqCst);

        xb.interface.rsp_cons += size_of::<xsd_sockmsg>() as u32 + msg.len;

        fence(Ordering::SeqCst);

        xb.notify();

        return;
    }
}
