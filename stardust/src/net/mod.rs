//! Network front-end driver

use {
    core::{fmt::Write, time::Duration},
    log::debug,
    phy::Device,
    smoltcp::{
        iface::{InterfaceBuilder, NeighborCache, SocketStorage},
        socket::{TcpSocket, TcpSocketBuffer},
        time::Instant,
        wire::EthernetAddress,
    },
    xen::{platform::time::get_system_time, Delay},
};

mod phy;

pub async fn server() {
    let phy = Device::new();

    let mut neighbor_storage = [None; 16];
    let neighbor_cache = NeighborCache::new(&mut neighbor_storage[..]);

    let mut socket_storage = [SocketStorage::EMPTY; 8];

    let mut iface = InterfaceBuilder::new(phy, &mut socket_storage[..])
        .hardware_addr(EthernetAddress([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]).into())
        .neighbor_cache(neighbor_cache)
        .any_ip(true)
        .finalize();

    let mut rx_buffer = [0; 2048];
    let mut tx_buffer = [0; 2048];
    let socket = TcpSocket::new(
        TcpSocketBuffer::new(&mut rx_buffer[..]),
        TcpSocketBuffer::new(&mut tx_buffer[..]),
    );

    let handle = iface.add_socket(socket);

    debug!("starting TCP server");

    loop {
        match iface.poll(Instant::from_micros((get_system_time() >> 10) as i64)) {
            Ok(_) => {}
            Err(e) => {
                debug!("poll error: {}", e);
            }
        }

        let socket = iface.get_socket::<TcpSocket>(handle);
        if !socket.is_open() {
            socket.listen(80).unwrap();
        }

        if socket.can_send() {
            debug!("tcp:80 send greeting");
            writeln!(socket, "hello").unwrap();
            debug!("tcp:80 close");
            socket.close();
        }

        Delay::new(Duration::new(0, 100_000)).await;
    }
}
