use {
    super::ring::{Ring, PAGE_LAYOUT},
    alloc::{alloc::alloc, format},
    core::{
        ptr,
        sync::atomic::{fence, Ordering},
    },
    smoltcp::{
        self,
        phy::{self, DeviceCapabilities, Medium},
        time::Instant,
        wire::EthernetAddress,
    },
    xen::{
        events::event_channel_op,
        grant_table,
        memory::VirtualAddress,
        xen_sys::{
            self, domid_t, evtchn_alloc_unbound_t, evtchn_port_t, evtchn_send, grant_ref_t,
            netif_rx_sring, netif_tx_sring, EVTCHNOP_alloc_unbound, EVTCHNOP_send,
        },
        xenbus::{self, MessageKind},
        xenstore, DOMID_SELF,
    },
};

const RING_SIZE: usize = 256;

#[derive(Clone, Copy)]
struct Buffer {
    page: *mut u8,
    grant_ref: grant_ref_t,
}

pub struct Device {
    mac: EthernetAddress,
    backend_domain: domid_t,
    event_channel_port: evtchn_port_t,

    pub tx: Ring<netif_tx_sring>,
    tx_ring_ref: grant_ref_t,
    tx_buffers: [Buffer; RING_SIZE],

    pub rx: Ring<netif_rx_sring>,
    rx_ring_ref: grant_ref_t,
    rx_buffers: [Buffer; RING_SIZE],
}

impl Device {
    pub async fn new() -> Self {
        // retrieve MAC
        let mac = get_mac();

        // get domain ID of backend
        let backend_domain = get_backend_domain();

        // setup event channel
        let event_channel_port = alloc_event_channel(backend_domain);

        let tx_buffers = [Buffer {
            page: ptr::null_mut(),
            grant_ref: 0,
        }; RING_SIZE];

        let rx_buffers = [Buffer {
            page: ptr::null_mut(),
            grant_ref: 0,
        }; RING_SIZE];

        let tx = Ring::<netif_tx_sring>::new();
        assert!(tx.size() == RING_SIZE);

        let rx = Ring::<netif_rx_sring>::new();
        assert!(rx.size() == RING_SIZE);

        let tx_ring_ref = grant_table::grant_access(
            backend_domain,
            VirtualAddress(tx.sring as *mut _ as usize).into(),
            false,
        );
        let rx_ring_ref = grant_table::grant_access(
            backend_domain,
            VirtualAddress(rx.sring as *mut _ as usize).into(),
            false,
        );

        let mut celf = Self {
            mac,
            backend_domain,
            event_channel_port,
            tx,
            tx_ring_ref,
            tx_buffers,
            rx,
            rx_ring_ref,
            rx_buffers,
        };

        celf.init_buffers();
        celf.connect().await;

        celf
    }

    fn notify(&self) {
        let mut event = evtchn_send {
            port: self.event_channel_port,
        };
        event_channel_op(EVTCHNOP_send, &mut event as *mut _ as u64);
    }

    fn init_buffers(&mut self) {
        for mut buf in self.rx_buffers {
            unsafe { buf.page = alloc(PAGE_LAYOUT) }
        }

        for i in 0..RING_SIZE {
            let mut buf = self.rx_buffers[i];
            let mut req = unsafe { (*self.rx.get(i)).req };

            let gref = grant_table::grant_access(
                self.backend_domain,
                VirtualAddress(buf.page as usize).into(),
                false,
            );

            buf.grant_ref = gref;

            req.gref = gref;
            req.id = i as u16;
        }

        self.rx.req_prod_pvt = RING_SIZE as u32;

        self.rx.push_requests();

        self.notify();

        self.rx.set_rsp_event(self.rx.rsp_cons + 1);
    }

    async fn connect(&self) {
        // start transaction
        let txn_id = xenbus::request(MessageKind::TransactionStart, &[b"\0"], 0)
            .await
            .1
            .parse::<u32>()
            .expect("Failed to parse transaction id");

        // tx ring ref
        {
            let rsp = xenbus::request(
                MessageKind::Write,
                &[
                    b"device/vif/0/tx-ring-ref\0",
                    format!("{}\0", self.tx_ring_ref).as_bytes(),
                ],
                txn_id,
            )
            .await;
            log::trace!("{:?}", rsp);
        }

        // rx ring ref
        {
            let rsp = xenbus::request(
                MessageKind::Write,
                &[
                    b"device/vif/0/rx-ring-ref\0",
                    format!("{}\0", self.rx_ring_ref).as_bytes(),
                ],
                txn_id,
            )
            .await;
            log::trace!("{:?}", rsp);
        }

        // event channel
        {
            let rsp = xenbus::request(
                MessageKind::Write,
                &[
                    b"device/vif/0/event-channel\0",
                    format!("{}", self.event_channel_port).as_bytes(),
                ],
                txn_id,
            )
            .await;
            log::trace!("{:?}", rsp);
        }

        // request rx copy
        {
            let rsp = xenbus::request(
                MessageKind::Write,
                &[b"device/vif/0/request-rx-copy\0", b"1"],
                txn_id,
            )
            .await;
            log::trace!("{:?}", rsp);
        }

        // switch state
        {
            let rsp = xenbus::request(MessageKind::Read, &[b"device/vif/0/state\0"], txn_id).await;
            let state = xenbus::State::from(rsp.1.parse::<u32>().expect("failed to parse u32"));
            log::trace!("{:?} {:?}", rsp, state);

            let rsp = xenbus::request(
                MessageKind::Write,
                &[
                    b"device/vif/0/state\0",
                    format!(
                        "{}\0",
                        xen_sys::xenbus_state::from(xenbus::State::Connected)
                    )
                    .as_bytes(),
                ],
                txn_id,
            )
            .await;
            log::trace!("{:?}", rsp);

            let rsp = xenbus::request(MessageKind::Read, &[b"device/vif/0/state\0"], txn_id).await;
            let state = xenbus::State::from(rsp.1.parse::<u32>().expect("failed to parse u32"));
            log::trace!("{:?} {:?}", rsp, state);
        }

        // end transaction
        let rsp = xenbus::request(MessageKind::TransactionEnd, &[b"T\0"], txn_id).await;
        log::trace!("{:?}", rsp);

        log::trace!(
            "backend: {:?}\nmac: {:?}",
            xenbus::request(MessageKind::Read, &[b"device/vif/0/backend\0"], 0).await,
            xenbus::request(MessageKind::Read, &[b"device/vif/0/mac\0"], 0).await,
        );
    }

    pub fn mac(&self) -> EthernetAddress {
        self.mac
    }

    pub fn receive(&mut self) {
        //     RING_IDX rp,cons,req_prod;
        //     int nr_consumed, more, i, notify;
        //     int dobreak;
        let mut rp: u32;
        let mut cons: u32;
        let mut req_prod: u32;

        let mut nr_consumed: usize;
        let mut more: usize;
        let mut i: usize;

        let mut notify: bool;
        let mut dobreak: bool;

        //     nr_consumed = 0;
        nr_consumed = 0;

        loop {
            // moretodo:
            //     rp = dev->rx.sring->rsp_prod;
            //     rmb(); /* Ensure we see queued responses up to 'rp'. */
            rp = self.rx.sring.rsp_prod;
            fence(Ordering::SeqCst);

            //     dobreak = 0;
            dobreak = false;

            //     for (cons = dev->rx.rsp_cons; cons != rp && !dobreak; nr_consumed++, cons++)
            //     {
            //         struct net_buffer* buf;
            //         unsigned char* page;
            //         int id;

            //         struct netif_rx_response *rx = RING_GET_RESPONSE(&dev->rx, cons);

            //         id = rx->id;
            //         BUG_ON(id >= NET_RX_RING_SIZE);

            //         buf = &dev->rx_buffers[id];
            //         page = (unsigned char*)buf->page;
            //         gnttab_end_access(buf->gref);

            //         if (rx->status > NETIF_RSP_NULL)
            //         {

            //         dev->netif_rx(page+rx->offset,rx->status);
            //         }
            //     }

            cons = self.rx.rsp_cons;
            loop {
                if !(cons != rp && !dobreak) {
                    break;
                }

                let rx = unsafe { (*self.rx.get(cons as usize)).rsp };
                let id = rx.id as usize;
                assert!(id < RING_SIZE);

                let buf = self.rx_buffers[id];
                grant_table::grant_end(buf.grant_ref);

                log::trace!("received: {} bytes at {:p}", rx.status, unsafe {
                    buf.page.add(rx.offset as usize)
                });

                nr_consumed += 1;
                cons += 1;
            }

            log::trace!("nr_consumed: {}", nr_consumed);

            //     dev->rx.rsp_cons=cons;

            self.rx.rsp_cons = cons;

            //     RING_FINAL_CHECK_FOR_RESPONSES(&dev->rx,more);
            //     if(more && !dobreak) goto moretodo;

            let more = self.rx.check_for_responses();
            if !(more > 0 && !dobreak) {
                break;
            }
        }

        //     req_prod = dev->rx.req_prod_pvt;
        req_prod = self.rx.req_prod_pvt;

        //     for(i=0; i<nr_consumed; i++)
        //     {
        //         int id = xennet_rxidx(req_prod + i);
        //         netif_rx_request_t *req = RING_GET_REQUEST(&dev->rx, req_prod + i);
        //         struct net_buffer* buf = &dev->rx_buffers[id];
        //         void* page = buf->page;

        //         /* We are sure to have free gnttab entries since they got released above */
        //         buf->gref = req->gref =
        //             gnttab_grant_access(dev->dom,virt_to_mfn(page),0);

        //         req->id = id;
        //     }

        for i in 0..nr_consumed {
            let id = (req_prod as usize + i) & (RING_SIZE - 1);
            let entry = self.rx.get(i);
            let mut buf = self.rx_buffers[id];

            let page = buf.page;

            let gref = grant_table::grant_access(
                self.backend_domain,
                VirtualAddress(page as usize).into(),
                false,
            );

            buf.grant_ref = gref;
            unsafe { *entry }.req.gref = gref;
            unsafe { *entry }.req.id = id as u16;
        }

        fence(Ordering::SeqCst);

        self.rx.req_prod_pvt = req_prod + (nr_consumed) as u32;

        self.rx.push_requests();
        self.notify();

        log::trace!("done");

        //     wmb();

        //     dev->rx.req_prod_pvt = req_prod + i;

        //     RING_PUSH_REQUESTS_AND_CHECK_NOTIFY(&dev->rx, notify);
        //     if (notify)
        //         notify_remote_via_evtchn(dev->evtchn);
    }
}

fn get_mac() -> EthernetAddress {
    let mut buf = [0; 6];
    let s = xenstore::read("device/vif/0/mac\0");

    log::trace!("mac: {}", s);

    s.split(':')
        .map(|s| u8::from_str_radix(s, 16).expect("failed to convert hex byte"))
        .take(6)
        .enumerate()
        .for_each(|(i, b)| buf[i] = b);

    EthernetAddress(buf)
}

fn get_backend_domain() -> domid_t {
    xenstore::read("device/vif/0/backend-id\0")
        .parse::<domid_t>()
        .expect("failed to parse backend-id")
}

fn alloc_event_channel(domain: domid_t) -> evtchn_port_t {
    let mut op = evtchn_alloc_unbound_t {
        dom: DOMID_SELF,
        remote_dom: domain,
        port: 0,
    };

    event_channel_op(EVTCHNOP_alloc_unbound, &mut op as *mut _ as u64);

    // bind_event_channel(
    //     op.port
    //         .try_into()
    //         .expect("failed to convert evtchn_port_t to usize"),
    //     |_, _| trace!("net event channel!"),
    //     0,
    // );

    op.port
}

impl<'a> phy::Device<'a> for Device {
    type RxToken = PhyRxToken<'a>;

    type TxToken = PhyTxToken<'a>;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        Some((PhyRxToken(&mut self.rx), PhyTxToken(&mut self.tx)))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(PhyTxToken(&mut self.tx))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct PhyRxToken<'a>(&'a mut Ring<netif_rx_sring>);

impl<'a> phy::RxToken for PhyRxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buf = alloc::vec![];
        let result = f(&mut buf);
        log::trace!("rx: {:?}", buf);
        result
    }
}

pub struct PhyTxToken<'a>(&'a mut Ring<netif_tx_sring>);

impl<'a> phy::TxToken for PhyTxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buf = alloc::vec![0; 1536];
        let result = f(&mut buf);
        log::trace!("tx: {:?}", buf);
        todo!()
    }
}
