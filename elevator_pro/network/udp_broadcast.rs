use elevator_pro::config;


pub async fn start_udp_broadcaster() {
    let addr: &str = &format!("{}:{}", config::BC_ADDR, config::DUMMY_PORT); //🎯 
    let addr2: &str = &format!("{}:0", config::BC_LISTEN_ADDR);

    let broadcast_addr: SocketAddr = addr.parse().expect("ugyldig adresse"); // UDP-broadcast adresse
    let socket_addr: SocketAddr = addr2.parse().expect("Ugyldig adresse");
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
    
    
    socket.set_reuse_address(true)?;
    socket.set_broadcast(true)?;
    socket.bind(&socket_addr.into())?;
    let udp_socket = UdpSocket::from_std(socket.into())?;

    loop{
        if min_id == lavest_id_i_wv {
            //Broadcast egen WV
        }
    }
}