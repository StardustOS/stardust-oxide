//! Network front-end driver

use {
    alloc::{collections::BTreeMap, vec, vec::Vec},
    core::{fmt::Write, time::Duration},
    log::debug,
    phy::Device,
    smoltcp::{
        iface::{InterfaceBuilder, NeighborCache},
        socket::{TcpSocket, TcpSocketBuffer},
        time::Instant,
    },
    xen::{platform::time::get_system_time, Delay},
};

mod phy;
mod ring;

pub async fn server() {
    let phy = Device::new().await;
    let mac = phy.mac();

    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let mut iface = InterfaceBuilder::new(phy, Vec::new())
        .hardware_addr(mac.into())
        .neighbor_cache(neighbor_cache)
        .any_ip(true)
        .finalize();

    let mut rx_buffer = vec![0; 2048];
    let mut tx_buffer = vec![0; 2048];
    let socket = TcpSocket::new(
        TcpSocketBuffer::new(&mut rx_buffer[..]),
        TcpSocketBuffer::new(&mut tx_buffer[..]),
    );

    let handle = iface.add_socket(socket);

    debug!("starting TCP server");

    loop {
        iface.device().debug();

        Delay::new(Duration::new(0, 100_000_000)).await;

        iface.device_mut().receive();

        // match iface.poll(Instant::from_micros((get_system_time() >> 10) as i64)) {
        //     Ok(_) => {}
        //     Err(e) => {
        //         debug!("poll error: {}", e);
        //     }
        // }

        // let socket = iface.get_socket::<TcpSocket>(handle);
        // if !socket.is_open() {
        //     socket.listen(80).unwrap();
        // }

        // if socket.can_send() {
        //     debug!("tcp:80 send greeting");
        //     writeln!(socket, "hello").unwrap();
        //     debug!("tcp:80 close");
        //     socket.close();
        // }
    }
}
