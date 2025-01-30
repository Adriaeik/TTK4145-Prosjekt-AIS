//! Sikrer at fine wordviewpakker blir sendt dit de skal :)



use std::net::SocketAddr;

use tokio::net::TcpSocket;

use super::Tony;

fn make_socket(addr: SocketAddr) -> TcpSocket {
    let socket = TcpSocket::new_v4().unwrap();
    socket.set_reuseaddr(true).unwrap(); // allow to reuse the addr both for connect and listen
    socket.set_reuseport(true).unwrap(); // same for the port
    socket.bind(addr).unwrap();
    socket
}

async fn is_peer_connected(addr: SocketAddr) -> bool {
    make_socket("127.0.0.1:0".parse().unwrap())
        .connect(dbg!(addr))
        .await
        .is_ok()
}

async fn main() {
    let my_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap(); //kjent
    let master_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap(); //kjent fra udp broadcast

    // let (my_addr, peer_addr) = if !is_peer_connected(my_addr).await {
    //     // we are the first
    //     (my_addr, peer_addr)
    // } else {
    //     // found a peer, swap addr
    //     (peer_addr, my_addr)
    // };

    // // hvis vi er første på nettet -> den under
    // let listener = make_socket(my_addr).listen(1024).unwrap();

    // dbg!(&listener);



    -> 
        - connect til master
        - hør på porten, gjør det som trengs med worldview
            - oppdater den på en tråd til Tony
            - tony sier hva som skjer her basert på worldview
        


    loop {
        let socket_out = make_socket(my_addr);
        tokio::select! {
                evt = listener.accept() => {
                    let (socket, addr) = evt.unwrap();
                    println!("Incoming connection from: {}", addr);
                    dbg!(socket);
                },

                _evt = socket_out.connect(peer_addr) => {}
        }
    }
}
