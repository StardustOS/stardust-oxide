use {
    crate::xenbus::{mask_xenstore_idx, memcpy_from_ring, XENBUS},
    alloc::vec,
    core::mem::size_of,
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
            memcpy_from_ring(
                xb.interface.rsp.as_mut_ptr(),
                &mut msg as *mut xsd_sockmsg as *mut _,
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
                memcpy_from_ring(
                    xb.interface.rsp.as_mut_ptr(),
                    data.as_mut_ptr() as *mut i8,
                    (mask_xenstore_idx(xb.interface.rsp_cons)) as usize + size_of::<xsd_sockmsg>(),
                    msg.len as usize,
                )
            };

            xb.responses.insert(msg.req_id, (msg.into(), data));
        }

        xb.interface.rsp_cons += size_of::<xsd_sockmsg>() as u32 + msg.len;

        xb.notify();

        return;
    }
}
