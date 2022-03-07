use smoltcp::{
    self,
    phy::{self, DeviceCapabilities, Medium},
    time::Instant,
};

pub struct Device {
    rx_buffer: [u8; 1536],
    tx_buffer: [u8; 1536],
}

impl Device {
    pub fn new() -> Self {
        Self {
            rx_buffer: [0; 1536],
            tx_buffer: [0; 1536],
        }
    }
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
        let result = f(&mut self.0[..len]);
        log::debug!("tx called {}", len);
        // TODO: send packet out
        result
    }
}
