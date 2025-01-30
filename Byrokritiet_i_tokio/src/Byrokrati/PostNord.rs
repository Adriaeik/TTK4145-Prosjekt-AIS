//! Sikrer at fine wordviewpakker blir sendt dit de skal :)



use std::net::SocketAddr;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::broadcast;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::sync::Arc;


pub async fn publiser_nyhetsbrev(self_ip: &str) -> tokio::io::Result<()> {

    let self_port = "6969"; //Vet ikke helt om denne burde være en standard?


    let listener = TcpListener::bind(format!("{}:{}", self_ip, self_port)).await?;
    println!("Nyhetsbrev oppretta på {}:{}", self_ip, self_port);

    let (tx, _) = broadcast::channel::<String>(3); //Kunne vel i teorien vært 1
    let tx = Arc::new(tx);



    // Håndter alle innkommende tilkoblinger
    loop {
        //Må legge til:
        //Les nyeste worldview fra rx????




        let (mut socket, _) = listener.accept().await?;  // Nå kan vi kalle accept() på listeneren
        let mut tx = Arc::clone(&tx); // Klon senderen for bruk i ny oppgave

        // Start en ny oppgave for hver klient
        tokio::spawn(async move {
            let rx = tx.subscribe(); // Opprett en ny receiver for hver klient
            if let Err(e) = send_nyhetsbrev(socket, rx).await {
                eprintln!("Feil i kommunikasjon med klient: {}", e);
            }
        });
    }

}


async fn send_nyhetsbrev(mut socket: TcpStream, mut rx: broadcast::Receiver<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Håndter kommunikasjonen med klienten her.
    
    let mut buf = [0; 10];
    loop {
        tokio::select! {
            // Lytt på meldinger fra broadcast-kanalen
            msg = rx.recv() => {
                match msg {
                    Ok(message) => {
                        // Send melding til klienten
                        if let Err(e) = socket.write_all(message.as_bytes()).await {
                            eprintln!("Feil ved sending til klient i send_nyhetsbrev(): {}", e);
                            return Err(Box::new(e));
                        }
                    }
                    Err(e) => {
                        eprintln!("Feil ved mottak fra broadcast-kanal: {}", e);
                        return Err(Box::new(e));
                    }
                }
            }
            
            // Les ack
            result = socket.read(&mut buf) => {
                match result {
                    Ok(0) => {
                        println!("Koblingen er lukket av klienten.");
                        return Ok(()); // Klienten har koblet fra
                    }
                    Ok(_) => {
                        // Her kan du behandle innkommende data fra klienten om nødvendig.
                        // Eksempel: logge data som ble mottatt
                        println!("Mottok fra klient: {}", String::from_utf8_lossy(&buf));
                    }
                    Err(e) => {
                        eprintln!("Feil ved lesing fra klient: {}", e);
                        return Err(Box::new(e));
                    }
                }
            }
        }
    }
}



pub async fn abboner_master_nyhetsbrev(master_ip: SocketAddr) -> tokio::io::Result<()> {
    // let my_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap(); //kjent
    // let master_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap(); //kjent fra udp broadcast

    let master_ip = "69.69.69.69:420";
    //les inn string til master ip fra channel her først
    let master_addr: SocketAddr = master_ip.parse().expect("Ugyldig IP-adresse til master (postnord.rs, abboner_master_nyhetsbrev)");

    let mut stream = TcpStream::connect(master_addr).await?;
    let mut buf = [0; 1024];

    loop {
        let bytes_read = stream.read(&mut buf).await?;
        if bytes_read == 0 {
            println!("Serveren stengte tilkoblingen.");
            break;
        }
        let message = String::from_utf8_lossy(&buf[..bytes_read]);
        println!("Melding fra server: {}", message);
    }

    Ok(())

    // -> 
    //     - connect til master
    //     - hør på porten, gjør det som trengs med worldview
    //         - oppdater den på en tråd til Tony
    //         - tony sier hva som skjer her basert på worldview
        



}
