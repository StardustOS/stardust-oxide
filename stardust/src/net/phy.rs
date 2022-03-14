use {
    super::ring::{Ring, PAGE_LAYOUT},
    alloc::{alloc::alloc, format},
    core::ptr,
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

    pub fn receive(&mut self) {}
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
