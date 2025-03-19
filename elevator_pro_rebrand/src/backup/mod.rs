use std::env;
use std::net::SocketAddr;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self, Write};
use socket2::{Socket, Domain, Type, Protocol};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;
use tokio::time::{sleep, Duration, timeout};

// Tilpass desse importane til prosjektet ditt:
use crate::{config, init, ip_help_functions, world_view};
use crate::print;
use crate::network::local_network;

// Global variabel for å sjå om backup-terminalen allereie er starta
static BACKUP_STARTED: AtomicBool = AtomicBool::new(false);

/// Opprett ein gjennbrukbar TcpListener med reuse_address aktivert.
pub fn create_reusable_listener(port: u16) -> TcpListener {
    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .expect("Ugyldig adresse");
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .expect("Klarte ikkje opprette socket");
    socket.set_nonblocking(true).expect("msg");
    socket.set_reuse_address(true)
        .expect("Klarte ikkje setje reuse_address");
    socket.bind(&addr.into())
        .expect("Klarte ikkje binde socketen");
    socket.listen(128)
        .expect("Klarte ikkje lytte på socketen");
    TcpListener::from_std(socket.into())
        .expect("Klarte ikkje opprette TcpListener")
}

/// Startar backup-terminalen i eit nytt terminalvindu – berre om han ikkje allereie er starta.
fn start_backup_terminal() {
    if !BACKUP_STARTED.load(Ordering::SeqCst) {
        let current_exe = env::current_exe().expect("Klarte ikkje hente ut den kjørbare fila");
        // Eksempel med gnome-terminal og --geometry for å spesifisere vindaugets storleik.
        let _child = Command::new("gnome-terminal")
            .arg("--geometry=400x24")
            .arg("--")
            .arg(current_exe.to_str().unwrap())
            .arg("backup")
            .spawn()
            .expect("Feil ved å starte backupterminalen");
        BACKUP_STARTED.store(true, Ordering::SeqCst);
    }
}

/// Handterar backup-klientar: Sender ut worldview kontinuerleg.
async fn handle_backup_client(mut stream: TcpStream, rx: watch::Receiver<Vec<u8>>) {
    loop {
        let wv = rx.borrow().clone();
        if let Err(e) = stream.write_all(&wv).await {
            eprintln!("Backup send error: {}", e);
            // Set BACKUP_STARTED til false, slik at ein ny backup-terminal kan startast
            BACKUP_STARTED.store(false, Ordering::SeqCst);
            start_backup_terminal();
            // Avslutt løkka for denne klienten for å unngå evig loop.
            break;
        }
        sleep(Duration::from_millis(1000)).await;
    }
}

// Backup-serveren: Lytter på tilkoplingar frå backup-klientar og sender ut den nyaste worldview.
/// Function to start and maintain connection to the backup-program
/// 
/// ## Parameters
/// `wv_watch_rx`: Rx on watch the worldview is being sent on in the system  
/// 
/// ## Behavior
/// - Sets up a reusable TCP listener and starts a backup program in a new terminal
/// - Continously sends the latest worldview to the backup asynchronously
/// - Continously reads the latest worldview shich will be sent
/// 
/// ## Note
/// This function is permanently blocking, and should be ran asynchronously 
pub async fn start_backup_server(wv_watch_rx: watch::Receiver<Vec<u8>>) {
    println!("Backup-serveren startar...");
    
    // Bruk ein gjennbrukbar listener.
    let listener = create_reusable_listener(config::BCU_PORT);
    let wv = world_view::get_wv(wv_watch_rx.clone());
    let (tx, rx) = watch::channel(wv.clone());
    
    // Start backup-terminalen éin gong.
    start_backup_terminal();
    
    // Task for å handtere backup-klientar.
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener
                .accept()
                .await
                .expect("Klarte ikkje akseptere backup-kopling");
            handle_backup_client(socket, rx.clone()).await;
        }
    });
    
    // Oppdater kontinuerleg worldview til backup-klientane.
    loop {
        let new_wv = world_view::get_wv(wv_watch_rx.clone());
        tx.send(new_wv).expect("Klarte ikkje sende til backup-klientane");
        sleep(Duration::from_millis(1000)).await;
    }
}

/// Backup-klienten: Koplar seg til backup-serveren, les data kontinuerleg og skriv ut worldview.
pub async fn run_as_backup() -> world_view::ElevatorContainer {
    println!("Starter backup-klient...");
    let mut current_wv = init::initialize_worldview(None).await;
    let mut retries = 0;
    
    loop {
        match timeout(
            config::MASTER_TIMEOUT,
            TcpStream::connect(format!("127.0.0.1:{}", config::BCU_PORT))
        ).await {
            Ok(Ok(mut stream)) => {
                retries = 0;
                let mut buf = vec![0u8; 1024];
                // Les data i ein løkke for kontinuerleg oppdatering
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) => {
                            eprintln!("Master koplinga vart avslutta.");
                            break;
                        },
                        Ok(n) => {
                            current_wv = buf[..n].to_vec();
                            // Rydd skjermen og sett markøren øvst
                            print!("\x1B[2J\x1B[H");

                            // Sørg for at utskrifta skjer umiddelbart
                            io::stdout().flush().unwrap();

                            print::worldview(current_wv.clone());
                        },
                        Err(e) => {
                            eprintln!("Lesefeil frå master: {}", e);
                            break;
                        }
                    }
                    sleep(Duration::from_millis(500)).await;
                }
            },
            _ => {
                retries += 1;
                eprintln!("Kunne ikkje koble til master, retry {}.", retries);
                if retries > 50 {
                    eprintln!("Master feila, promoterer backup til master!");
                    // Her kan failover-logikken setjast i gang, t.d. køyre master-logikken.
                    return world_view::extract_self_elevator_container(current_wv);
                }
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}
