

pub struct Timer {
    hard_timeout: bool,
    timer_active: bool,
    timeout_duration: tokio::time::Duration,
    start_time: tokio::time::Instant,
}

pub fn new(timeout_duration: tokio::time::Duration) -> Timer {
    Timer{
        hard_timeout: false,
        timer_active: false,
        timeout_duration: timeout_duration,
        start_time: tokio::time::Instant::now(),
    }
}
impl Timer {
    pub fn timer_start(&mut self) {
        self.hard_timeout = false;
        self.timer_active = true;
        self.start_time = tokio::time::Instant::now();
    }

    pub fn release_timer(&mut self) {
        self.hard_timeout = true;
    }

    pub fn get_wall_time(&mut self) -> tokio::time::Duration {
        return tokio::time::Instant::now() - self.start_time
    }

    pub fn timer_timeouted(&self) -> bool {
        return (self.timer_active && (tokio::time::Instant::now() - self.start_time) > self.timeout_duration) || self.hard_timeout;
    }
}

