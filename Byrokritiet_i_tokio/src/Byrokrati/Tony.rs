//! Tony er lagersjef. han driver også en italiensk mafia på siden
use super::Sjefen;
use tokio::time::{sleep, Duration};
use tokio::sync::mpsc;


impl Sjefen::Sjefen {


    pub async fn backup_process(&mut self) {
        println!("En Tony er starta");
        let (tx_worldview, mut rx_worldview) = mpsc::channel::<String>(1);

        let tx_wv_clone = tx_worldview.clone();
        let mut self_copy = self.copy();




        tokio::spawn(async move {
            self_copy.backup_connection(tx_wv_clone).await;
        });
        






        loop {
            while let Some(melding) = rx_worldview.recv().await {
                println!("Mottatt: {}", melding);
                if let Some(b) = melding.split_once(':').map(|(_, b)| b) {
                    if b == "slave" {self.rolle = Sjefen::Rolle::SLAVE;}
                    else if b == "master" {self.rolle = Sjefen::Rolle::MASTER;}

                } else {}
            }
        }
    }
}


