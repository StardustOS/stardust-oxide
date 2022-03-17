use {
    super::ring::{RawRing, Ring, PAGE_LAYOUT},
    alloc::{alloc::alloc, format, vec, vec::Vec},
    core::{
        ptr::{self, copy_nonoverlapping},
        sync::atomic::{fence, Ordering},
    },
    smoltcp::{
        self,
        phy::{self, DeviceCapabilities, Medium},
        time::Instant,
        wire::EthernetAddress,
    },
    xen::{
        events::{bind_event_channel, event_channel_op},
        grant_table,
        memory::{MachineFrameNumber, VirtualAddress},
        xen_sys::{
            self, domid_t, evtchn_alloc_unbound_t, evtchn_port_t, evtchn_send, grant_ref_t,
            netif_rx_request, netif_rx_sring, netif_tx_request, netif_tx_sring,
            EVTCHNOP_alloc_unbound, EVTCHNOP_send, NETIF_RSP_ERROR, NETIF_RSP_NULL,
        },
        xenbus::{self, MessageKind},
        xenstore, DOMID_SELF,
    },
};

const RING_SIZE: usize = 256;

struct Freelist([usize; RING_SIZE + 1]);

impl Freelist {
    fn new() -> Self {
        Self([0; RING_SIZE + 1])
    }

    fn add(&mut self, id: usize) {
        self.0[id + 1] = self.0[0];
        self.0[0] = id;
    }

    fn get(&mut self) -> usize {
        let id = self.0[0];
        self.0[0] = self.0[id + 1];
        id
    }
}

#[derive(Clone, Copy, Debug)]
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
    tx_freelist: Freelist,

    pub rx: Ring<netif_rx_sring>,
    rx_ring_ref: grant_ref_t,
    rx_buffers: [Buffer; RING_SIZE],
}

impl Device {
    pub async fn new() -> Self {
        // retrieve MAC
        let mac = get_mac();

        let mut tx_buffers = [Buffer {
            page: ptr::null_mut(),
            grant_ref: 0,
        }; RING_SIZE];

        let mut tx_freelist = Freelist::new();

        let mut rx_buffers = [Buffer {
            page: ptr::null_mut(),
            grant_ref: 0,
        }; RING_SIZE];

        for i in 0..RING_SIZE {
            tx_freelist.add(i);
            tx_buffers[i].page = ptr::null_mut();
        }

        for i in 0..RING_SIZE {
            rx_buffers[i].page = unsafe { alloc(PAGE_LAYOUT) };
        }

        // get domain ID of backend
        let backend_domain = get_backend_domain();
        log::trace!("backend_domain {}", backend_domain);

        // setup event channel
        let event_channel_port = alloc_event_channel(backend_domain);

        let tx = Ring::<netif_tx_sring>::new();
        assert!(tx.size() == RING_SIZE);

        let rx = Ring::<netif_rx_sring>::new();
        assert!(rx.size() == RING_SIZE);

        let txs = MachineFrameNumber::from(VirtualAddress(tx.sring as *mut _ as usize));
        let rxs = MachineFrameNumber::from(VirtualAddress(rx.sring as *mut _ as usize));

        log::trace!("txs {:p} rxs {:p}", tx.sring, rx.sring);
        log::trace!("txs {:x} rxs {:x}", txs.0, rxs.0);

        let tx_ring_ref = grant_table::grant_access(0, txs, false);
        let rx_ring_ref = grant_table::grant_access(0, rxs, false);

        let mut celf = Self {
            mac,
            backend_domain,
            event_channel_port,
            tx,
            tx_ring_ref,
            tx_buffers,
            tx_freelist,
            rx,
            rx_ring_ref,
            rx_buffers,
        };

        celf.init_rx_buffers();

        celf.connect().await;

        celf
    }

    fn notify(&self) {
        let mut event = evtchn_send {
            port: self.event_channel_port,
        };
        event_channel_op(EVTCHNOP_send, &mut event as *mut _ as u64);
    }

    fn init_rx_buffers(&mut self) {
        for i in 0..RING_SIZE {
            let gref = grant_table::grant_access(
                self.backend_domain,
                VirtualAddress(self.rx_buffers[i].page as usize).into(),
                false,
            );

            self.rx_buffers[i].grant_ref = gref;

            unsafe {
                (*(self.rx.get(i) as *mut netif_rx_request)).gref = gref;
                (*(self.rx.get(i) as *mut netif_rx_request)).id = i as u16;
            }

            log::trace!("{:?}", unsafe {
                *(self.rx.get(i) as *mut netif_rx_request)
            });
        }

        for i in 0..RING_SIZE {
            log::trace!("init rx buffers {:?}", self.rx_buffers[i]);
            log::trace!("init rx_req {:?}", unsafe { (*self.rx.get(i)).req });
        }

        self.rx.req_prod_pvt = RING_SIZE as u32;

        if self.rx.push_requests() {
            self.notify();
        }

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
                    format!("{}", self.tx_ring_ref).as_bytes(),
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
                    format!("{}", self.rx_ring_ref).as_bytes(),
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
                    format!("{}", xen_sys::xenbus_state::from(xenbus::State::Connected)).as_bytes(),
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
            "backend: {:?}",
            xenbus::request(MessageKind::Read, &[b"device/vif/0/backend\0"], 0)
                .await
                .1,
        );
        log::trace!(
            "mac: {:?}",
            xenbus::request(MessageKind::Read, &[b"device/vif/0/mac\0"], 0)
                .await
                .1,
        )
    }

    pub fn mac(&self) -> EthernetAddress {
        self.mac
    }

    pub fn rx(&mut self) -> Option<Vec<u8>> {
        let mut rp: u32;
        let mut cons: u32;
        let req_prod: u32;

        let mut nr_consumed: usize;

        let mut dobreak: bool;

        let mut res = None;

        nr_consumed = 0;

        loop {
            rp = self.rx.sring.rsp_prod;
            fence(Ordering::SeqCst);

            dobreak = false;

            cons = self.rx.rsp_cons;
            loop {
                if !(cons != rp && !dobreak) {
                    break;
                }

                log::trace!("cons: {}", cons);

                let rx = unsafe { (*self.rx.get(cons as usize)).rsp };

                log::trace!("rx_response {:?}", rx);

                let id = rx.id as usize;
                assert!(id < RING_SIZE);

                let page = self.rx_buffers[id].page;
                grant_table::grant_end(self.rx_buffers[id].grant_ref);

                log::trace!("received: {} bytes at {:p}", rx.status, unsafe {
                    page.add(rx.offset as usize)
                });

                // PROCESS DATA HERE
                let mut vec = vec![0; rx.status as usize];
                unsafe {
                    copy_nonoverlapping(
                        page.add(rx.offset as usize),
                        vec.as_mut_ptr(),
                        rx.status as usize,
                    )
                };

                res = Some(vec);
                dobreak = true;

                nr_consumed += 1;
                cons += 1;
            }

            self.rx.rsp_cons = cons;

            let more = self.rx.check_for_responses();
            if !(more > 0 && !dobreak) {
                break;
            }
        }

        req_prod = self.rx.req_prod_pvt;

        for i in 0..nr_consumed {
            let id = (req_prod as usize + i) & (RING_SIZE - 1);
            let page = self.rx_buffers[id].page;

            let gref = grant_table::grant_access(
                self.backend_domain,
                VirtualAddress(page as usize).into(),
                false,
            );

            log::trace!("id: {}, page: {:p}, gref: {}", id, page, gref);

            self.rx_buffers[id].grant_ref = gref;

            unsafe {
                (*(self.rx.get(i) as *mut netif_rx_request)).gref = gref;
                (*(self.rx.get(i) as *mut netif_rx_request)).id = i as u16;
            }

            log::trace!("rx_req {:?}", unsafe { (*self.rx.get(i as usize)).req });
        }

        fence(Ordering::SeqCst);

        self.rx.req_prod_pvt = req_prod + (nr_consumed) as u32;

        if self.rx.push_requests() {
            self.notify();
        }

        res
    }

    pub fn tx(&mut self, data: &[u8]) {
        let id = self.tx_freelist.get();

        log::trace!("tx id {}", id);

        if self.tx_buffers[id].page.is_null() {
            self.tx_buffers[id].page = unsafe { alloc(PAGE_LAYOUT) };
        }

        log::trace!("tx buffers {:?}", self.tx_buffers[id]);

        let i = self.tx.req_prod_pvt;

        log::trace!("tx i {}", i);

        unsafe { copy_nonoverlapping(data.as_ptr(), self.tx_buffers[id].page, data.len()) };

        let gref = grant_table::grant_access(
            self.backend_domain,
            VirtualAddress(self.tx_buffers[id].page as usize).into(),
            true,
        );

        self.tx_buffers[id].grant_ref = gref;

        log::trace!("tx buffers {:?} gref {}", self.tx_buffers[id], gref);

        unsafe {
            (*(self.tx.get(i as usize) as *mut netif_tx_request)).gref = gref;
            (*(self.tx.get(i as usize) as *mut netif_tx_request)).offset = 0;
            (*(self.tx.get(i as usize) as *mut netif_tx_request)).size = data.len() as u16;
            (*(self.tx.get(i as usize) as *mut netif_tx_request)).flags = 0;
            (*(self.tx.get(i as usize) as *mut netif_tx_request)).id = id as u16;
        }

        log::trace!("tx request {:?}", unsafe {
            *(self.tx.get(i as usize) as *mut netif_tx_request)
        });

        self.tx.req_prod_pvt = i + 1;

        fence(Ordering::SeqCst);

        self.tx.push_requests();

        self.notify();

        self.process_transmissions();
    }

    pub fn process_transmissions(&mut self) {
        let mut cons: u32;
        let mut prod: u32;

        loop {
            prod = self.tx.sring.rsp_prod();
            fence(Ordering::SeqCst);

            cons = self.tx.rsp_cons;
            loop {
                if cons == prod {
                    break;
                }

                let txrsp = unsafe { (*self.tx.get(cons as usize)).rsp };

                if txrsp.status == NETIF_RSP_NULL as i16 {
                    log::trace!("rsp null");
                    continue;
                }

                if txrsp.status == NETIF_RSP_ERROR as i16 {
                    log::trace!("tx packet error");
                }

                log::trace!("packet status: {}", txrsp.status);

                let id = txrsp.id as usize;
                let mut buf = self.tx_buffers[id];
                grant_table::grant_end(buf.grant_ref);
                buf.grant_ref = 0;

                self.tx_freelist.add(id);

                cons += 1;
            }

            if !((cons == prod) && (prod != self.tx.sring.rsp_prod())) {
                break;
            }
        }
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

    log::trace!("net event channel: {}", op.port);

    bind_event_channel(op.port, |_, _, _| log::trace!("net event channel!"), 0);

    op.port
}

impl<'a> phy::Device<'a> for Device {
    type RxToken = PhyRxToken;

    type TxToken = PhyTxToken<'a>;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.rx().map(move |d| (PhyRxToken(d), PhyTxToken(self)))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(PhyTxToken(self))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct PhyRxToken(Vec<u8>);

impl phy::RxToken for PhyRxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let result = f(&mut self.0);
        log::trace!("rx token: {:?}", self.0);
        result
    }
}

pub struct PhyTxToken<'a>(&'a mut Device);

impl<'a> phy::TxToken for PhyTxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buf = alloc::vec![0; len];
        let result = f(&mut buf);
        self.0.tx(&buf);
        log::trace!("tx token: {:?}", buf);
        result
    }
}
