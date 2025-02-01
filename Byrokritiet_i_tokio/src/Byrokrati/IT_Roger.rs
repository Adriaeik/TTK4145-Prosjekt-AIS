//! IT_Roger Handter LAN - Altså mellom den lokale backup

use super::{konsulent, Sjefen};

use tokio::time::{sleep, Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::env;
use std::process::Command;
use socket2::{Socket, Domain, Type, Protocol};
use std::net::SocketAddr;
use std::net::IpAddr;


static BACKUP_STARTED: AtomicBool = AtomicBool::new(false);



impl Sjefen::Sjefen {
    pub async fn create_reusable_listener(&self) -> TcpListener {
        let socket_addr: SocketAddr = SocketAddr::new(self.ip, 8082);
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).expect("Failed to create socket");
        socket.set_reuse_address(true).expect("Failed to set reuse address");
        socket.bind(&socket_addr.into()).expect("Failed to bind socket");
        socket.listen(128).expect("Failed to listen on socket");

        TcpListener::from_std(socket.into()).expect("Failed to create TcpListener")
    }



    pub async fn monitor_backup(&self, last_received: Arc<Mutex<Instant>>, timeout_duration: Duration) {
        //let mut backup_timer = interval(Duration::from_secs(3));
        //backup_timer.tick().await; // Start timer

        loop {
            //backup_timer.tick().await;
            let elapsed = {
                let last = last_received.lock().await;
                last.elapsed()
            };

            //println!("Sekunder: {}", elapsed.as_secs());

            if elapsed > timeout_duration {
                println!("Backup is unresponsive. Starting a new backup...");
                //log_to_csv("Primary", "Backup Unresponsive", 0);

                // Reset BACKUP_STARTED and start a new backup
                self.start_backup_with_reset();

                let mut last = last_received.lock().await;
                *last = Instant::now();
            }
        }
    }

    pub async fn create_and_monitor_backup(&self) {
        let last_received = Arc::new(Mutex::new(Instant::now())); //Usikker på om denne kan puttes i funksjonen
        let timeout_duration = Duration::from_secs(1);
        

        self.start_backup();
        
        //Under starter backupen, og venter til den er startet riktig
        let listener = self.create_reusable_listener().await;
        

        let last_received_clone = Arc::clone(&last_received);
        
        //Følger med på om det skjer en timeout basically
        //Lager også ny backup om den feiler
        let self_clone = self.copy();
        tokio::spawn(async move {
            self_clone.monitor_backup(last_received_clone, timeout_duration).await;
        });  

        //Sender kontinuerlig worldview til backupen. Den lagrer også tiden når forrige ack skjedde
        //Så monitor_backup kan lage en ny backup på samme port om den blir inresponsive
        loop {
            //print!("Jeg lever i roger -> createandmonitorbackup");
            if let Ok((mut socket, _)) = listener.accept().await {
                socket.write_all("Worldview".as_bytes()).await.expect("Failed to send count");

                //println!("Backup acka! (er i IT_Roger, create_and_monitor_backup())");

                let mut last = last_received.lock().await;
                *last = Instant::now();

            }
            sleep(Duration::from_millis(100)).await;
        }  
    }

    pub fn start_backup(&self) {
        if !BACKUP_STARTED.load(Ordering::SeqCst) {
            let (cmd, args) = konsulent::get_terminal_command();
            let mut backup_args = args;
            backup_args.push(env::current_exe().unwrap().to_str().unwrap().to_string());
            backup_args.push("backup".to_string());


            Command::new(cmd)
                .args(backup_args)
                .spawn()
                .expect("Failed to start backup process");

            BACKUP_STARTED.store(true, Ordering::SeqCst);
            //konsulent::log_to_csv("Primary", "Backup Started", 0);
        }

    }

    pub fn start_backup_with_reset(&self) {
        BACKUP_STARTED.store(false, Ordering::SeqCst);
        self.start_backup();
    }






    pub async fn backup_connection(&self) {
        let mut last_received = Instant::now(); //Usikker på om denne kan puttes i funksjonen
        let timeout_duration = Duration::from_secs(1);
        
        let addr_string = format!("{}:{}", self.ip.to_string(), 8082);
        loop {
            match TcpStream::connect(addr_string.clone()).await {
                Ok(mut stream) => {
                    let mut buf = String::new();
                    let mut reader = BufReader::new(&mut stream);
                    if reader.read_line(&mut buf).await.is_ok() {
                        last_received = Instant::now();
                    }
                }
                Err(_) => {
                    // Retry silently
                }
            }

            if last_received.elapsed() > timeout_duration {
                self.primary_process().await;
            }

            sleep(Duration::from_millis(100)).await;
        }  

    }
}
