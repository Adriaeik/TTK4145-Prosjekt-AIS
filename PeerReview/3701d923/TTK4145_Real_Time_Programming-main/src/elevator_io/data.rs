use super::driver;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

#[derive(Debug)]
pub struct CallButton {
    pub floor: u8,
    pub call: u8,
}

pub async fn call_buttons(elev: driver::Elevator, ch: mpsc::Sender<CallButton>, period: Duration) {
    let mut prev = vec![[false; 3]; elev.num_floors.into()];
    let mut interval = time::interval(period);

    loop {
        interval.tick().await; // Ensure periodic execution
        for f in 0..elev.num_floors {
            for c in 0..3 {
                let v = elev.call_button(f, c).await; // Directly returns a bool
                if v && prev[f as usize][c as usize] != v {
                    if ch.send(CallButton { floor: f, call: c }).await.is_err() {
                        eprintln!("Failed to send CallButton update");
                        return;
                    }
                }
                prev[f as usize][c as usize] = v;
            }
        }
    }
}

pub async fn floor_sensor(elev: driver::Elevator, ch: mpsc::Sender<u8>, period: Duration) {
    let mut interval = time::interval(period);

    loop {
        interval.tick().await;
        if let Some(f) = elev.floor_sensor().await {
            if ch.send(f).await.is_err() {
                eprintln!("Failed to send floor sensor update");
                let _prev = 0;
            }
            let _prev = f;
        }
    }
}

pub async fn stop_button(elev: driver::Elevator, ch: mpsc::Sender<bool>, period: Duration) {
    let mut prev = false; // Previous stop button state
    let mut interval = time::interval(period);

    loop {
        interval.tick().await;

        // Directly fetch the stop button state (returns a bool)
        let v = elev.stop_button().await;
        if v != prev {
            if ch.send(v).await.is_err() {
                eprintln!("Failed to send stop button update");
                return;
            }
            prev = v;
        }
    }
}

pub async fn obstruction(elev: driver::Elevator, ch: mpsc::Sender<bool>, period: Duration) {
    let mut prev = false; // Previous obstruction state
    let mut interval = time::interval(period);

    loop {
        interval.tick().await;

        // Directly fetch the obstruction state (returns a bool)
        let v = elev.obstruction().await;
        if v != prev {
            if ch.send(v).await.is_err() {
                eprintln!("Failed to send obstruction update");
                return;
            }
            prev = v;
        }
    }
}
