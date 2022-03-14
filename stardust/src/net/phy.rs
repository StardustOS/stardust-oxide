use {
    super::ring::{Ring, RingFront, LAYOUT},
    alloc::{alloc::alloc, format},
    core::{convert::TryInto, ptr},
    log::trace,
    smoltcp::{
        self,
        phy::{self, DeviceCapabilities, Medium},
        time::Instant,
        wire::EthernetAddress,
    },
    xen::{
        events::{bind_event_channel, event_channel_op},
        grant_table,
        memory::VirtualAddress,
        xen_sys::{
            self, domid_t, evtchn_alloc_unbound_t, evtchn_port_t, grant_ref_t, netif_rx_sring,
            netif_tx_sring, EVTCHNOP_alloc_unbound,
        },
        xenbus::{self, MessageKind},
        xenstore, DOMID_SELF,
    },
};

#[derive(Clone, Copy)]
struct Buffer {
    page: *mut u8,
    grant_ref: grant_ref_t,
}

pub struct Device {
    rx_buffer: [u8; 1536],
    tx_buffer: [u8; 1536],
    mac: EthernetAddress,
    backend_domain: domid_t,
    event_channel_port: evtchn_port_t,

    pub tx: RingFront<netif_tx_sring>,
    tx_ring_ref: grant_ref_t,
    tx_buffers: [Buffer; 256],

    pub rx: RingFront<netif_rx_sring>,
    rx_ring_ref: grant_ref_t,
    rx_buffers: [Buffer; 256],
}

impl Device {
    pub async fn new() -> Self {
        // retrieve MAC
        let mac = {
            let mut buf = [0; 6];
            xenstore::read("device/vif/0/mac\0")
                .split(':')
                .map(|s| u8::from_str_radix(s, 16).expect("failed to convert hex byte"))
                .take(6)
                .enumerate()
                .for_each(|(i, b)| buf[i] = b);
            EthernetAddress(buf)
        };

        // get domain ID of backend
        let backend_domain = xenstore::read("device/vif/0/backend-id\0")
            .parse::<domid_t>()
            .expect("failed to parse backend-id");

        // setup event channel
        let event_channel_port = alloc_event_channel(backend_domain);

        let tx_buffers = [Buffer {
            page: ptr::null_mut(),
            grant_ref: 0,
        }; 256];

        let rx_buffers = [Buffer {
            page: ptr::null_mut(),
            grant_ref: 0,
        }; 256];

        for mut buf in rx_buffers {
            buf.page = unsafe { alloc(LAYOUT) };
        }

        let txs = Ring::<netif_tx_sring>::new();
        let rxs = Ring::<netif_rx_sring>::new();

        let tx_ring_ref =
            grant_table::grant_access(backend_domain, VirtualAddress(txs.0 as usize).into(), false);
        let rx_ring_ref =
            grant_table::grant_access(backend_domain, VirtualAddress(rxs.0 as usize).into(), false);

        let tx = txs.front();
        let rx = rxs.front();

        {
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
                        format!("{}", tx_ring_ref).as_bytes(),
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
                        format!("{}", rx_ring_ref).as_bytes(),
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
                        format!("{}", event_channel_port).as_bytes(),
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
                let rsp =
                    xenbus::request(MessageKind::Read, &[b"device/vif/0/state\0"], txn_id).await;
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

                let rsp =
                    xenbus::request(MessageKind::Read, &[b"device/vif/0/state\0"], txn_id).await;
                let state = xenbus::State::from(rsp.1.parse::<u32>().expect("failed to parse u32"));
                log::trace!("{:?} {:?}", rsp, state);
            }

            // end transaction
            let rsp = xenbus::request(MessageKind::TransactionEnd, &[b"T\0"], txn_id).await;
            log::trace!("{:?}", rsp);
        }

        log::trace!(
            "backend: {:?}\nmac: {:?}",
            xenbus::request(MessageKind::Read, &[b"device/vif/0/backend\0"], 0).await,
            xenbus::request(MessageKind::Read, &[b"device/vif/0/mac\0"], 0).await,
        );

        Self {
            rx_buffer: [0; 1536],
            tx_buffer: [0; 1536],
            mac,
            backend_domain,
            event_channel_port,
            tx,
            tx_ring_ref,
            tx_buffers,
            rx,
            rx_ring_ref,
            rx_buffers,
        }
    }

    pub fn mac(&self) -> EthernetAddress {
        self.mac
    }

    pub fn debug(&self) {
        self.tx.sring.debug();
        self.rx.sring.debug();
    }
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
        Some((
            PhyRxToken(&mut self.rx_buffer),
            PhyTxToken(&mut self.tx_buffer),
        ))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(PhyTxToken(&mut self.tx_buffer))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct PhyRxToken<'a>(&'a mut [u8]);

impl<'a> phy::RxToken for PhyRxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        // TODO: receive packet into buffer
        todo!();
        let result = f(&mut self.0);
        result
    }
}

pub struct PhyTxToken<'a>(&'a mut [u8]);

impl<'a> phy::TxToken for PhyTxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        todo!();
        let result = f(&mut self.0[..len]);
        log::debug!("tx called {}", len);
        // TODO: send packet out
        result
    }
}
