//! Tony er lagersjef. han driver også en italiensk mafia på siden
use super::IT_Roger;
use tokio::time::{sleep, Duration};


pub async fn backup_process(ip: &str) {
    println!("En Tony er starta");
    let ip_copy = ip.to_string();

    tokio::spawn(async move {
        IT_Roger::backup_connection( &ip_copy, "69").await;
    });

    loop {
        sleep(Duration::from_millis(100)).await; // Sover i 1 sekund
    }
}
