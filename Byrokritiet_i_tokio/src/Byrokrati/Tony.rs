//! Tony er lagersjef. han driver også en italiensk mafia på siden
use super::IT_Roger;
use super::Sjefen;
use tokio::time::{sleep, Duration};


impl Sjefen::Sjefen {


    pub async fn backup_process(&self) {
        println!("En Tony er starta");

        let mut self_copy = self.copy();
        tokio::spawn(async move {
            self_copy.backup_connection().await;
        });

        loop {
            sleep(Duration::from_millis(100)).await; // Sover i 1 sekund
        }
    }
}


