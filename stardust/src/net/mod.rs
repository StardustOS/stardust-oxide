//! Network front-end driver

use {
    alloc::{collections::BTreeMap, vec, vec::Vec},
    core::fmt::Write,
    log::debug,
    phy::Device,
    smoltcp::{
        iface::{InterfaceBuilder, NeighborCache, Routes},
        socket::{TcpSocket, TcpSocketBuffer},
        time::Instant,
        wire::{IpCidr, Ipv4Address},
    },
    xen::time::get_system_time,
};

mod phy;
mod ring;

pub async fn server() {
    let phy = Device::new().await;
    let mac = phy.mac();

    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let ip_addrs = [IpCidr::new(Ipv4Address::new(192, 168, 1, 2).into(), 0)];
    let mut routes_storage = [None; 1];
    let routes = Routes::new(&mut routes_storage[..]);

    let mut iface = InterfaceBuilder::new(phy, Vec::new())
        .hardware_addr(mac.into())
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();

    let mut rx_buffer = vec![0; 2048];
    let mut tx_buffer = vec![0; 2048];
    let socket = TcpSocket::new(
        TcpSocketBuffer::new(&mut rx_buffer[..]),
        TcpSocketBuffer::new(&mut tx_buffer[..]),
    );

    let tcp_handle = iface.add_socket(socket);

    debug!("starting TCP server");

    loop {
        match iface.poll(Instant::from_micros((get_system_time() >> 10) as i64)) {
            Ok(_) => {}
            Err(e) => {
                debug!("poll error: {}", e);
            }
        }

        let socket = iface.get_socket::<TcpSocket>(tcp_handle);
        if !socket.is_open() {
            socket.listen(80).unwrap();
        }

        if socket.can_send() {
            debug!("tcp:80 send greeting");
            writeln!(socket, "hello").unwrap();
            debug!("tcp:80 close");
            socket.close();
        }
    }
}
