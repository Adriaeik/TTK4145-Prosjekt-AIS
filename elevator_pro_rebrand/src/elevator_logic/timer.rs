

pub struct Timer {
    timer_active: bool,
    timeout_duration: tokio::time::Duration,
    start_time: tokio::time::Instant,
}

pub fn new(timeout_duration: tokio::time::Duration) -> Timer {
    Timer{
        timer_active: false,
        timeout_duration: timeout_duration,
        start_time: tokio::time::Instant::now(),
    }
}
impl Timer {
    pub fn timer_start(&mut self) {
        self.timer_active = true;
        self.start_time = tokio::time::Instant::now();
    }

    pub fn timer_stop(&mut self) {
        self.timer_active = false;
    }

    pub fn timer_timeouted(&self) -> bool {
        return self.timer_active && (tokio::time::Instant::now() - self.start_time) > self.timeout_duration
    }
}

