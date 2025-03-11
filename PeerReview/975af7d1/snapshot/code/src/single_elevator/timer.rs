use std::fmt;
use std::time::SystemTime;

pub fn get_wall_time() -> f64 {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    now.as_secs() as f64 + now.subsec_micros() as f64 * 0.000001
}

pub struct Timer {
    end_time: f64,
    active: bool,
}
impl Timer {
    pub fn new() -> Self {
        Timer {
            end_time: 0.0,
            active: false,
        }
    }

    pub fn start(&mut self, duration: f64) {
        self.end_time = get_wall_time() + duration;
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn timed_out(&self) -> bool {
        self.active && get_wall_time() > self.end_time
    }
}
impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Timer {{ end_time: {}, active: {} }}",
            self.end_time, self.active
        )
    }
}

pub enum TimerMessage {
    Start(f64),
    Stop,
    TimedOut,
}
