//! Timer module for managing asynchronous timeouts in elevator control logic.
//!
//! This module defines a `Timer` struct that can be used to measure elapsed time and
//! detect timeouts in a non-blocking, asynchronous context. It supports both
//! *soft timeouts* based on elapsed wall time and *hard timeouts* that can be triggered manually.
//!
//! # Usage
//!
//! The timer is typically used in logic that requires tracking operation durations,
//! such as keeping elevator doors open for a fixed time, or detecting communication timeouts.
//!
//! # Example
//! ```rust,no_run
//! use elevator_logic::request::{choose_direction, should_stop};
//! use world_view::{ElevatorContainer, Dirn};
//! 
//! let direction_and_behaviour = choose_direction(&elevator);
//! if should_stop(&elevator) {
//!     // Open doors, reset timers
//! }
//! ```
//!
//! # Behaviour
//!
//! - A timer is *inactive* until `timer_start()` is called.
//! - When active, the timer checks elapsed time using `Instant`.
//! - The timer can be *manually forced* to timeout using `release_timer()`.
//! - `timer_timeouted()` returns `true` if either the soft or hard timeout has occurred.
//!
//! # Components
//!
//! - `Timer` struct stores state (active, elapsed, hard timeout flag).
//! - Methods include:
//!     - `timer_start()` to activate and reset timer.
//!     - `release_timer()` to force a timeout.
//!     - `get_wall_time()` to check elapsed time.
//!     - `timer_timeouted()` to evaluate timeout status.



/// A simple timer utility for managing soft and hard timeouts in asynchronous contexts.
///
/// The timer can be started and queried to check whether the timeout duration has been exceeded.
/// In addition to the regular (soft) timeout based on elapsed time, a "hard timeout" flag can be manually triggered
/// to force the timer into a timeout state regardless of elapsed time.
pub struct Timer {
    hard_timeout: bool,
    timer_active: bool,
    timeout_duration: tokio::time::Duration,
    start_time: tokio::time::Instant,
}

/// Creates and returns a new timer instance.
///
/// The timer is initially inactive and has not timed out.
///
/// # Arguments
/// * `timeout_duration` – The duration after which the timer should timeout once started.
///
/// # Returns
/// A new `Timer` instance with the specified timeout duration.
pub fn new(timeout_duration: tokio::time::Duration) -> Timer {
    Timer{
        hard_timeout: false,
        timer_active: false,
        timeout_duration: timeout_duration,
        start_time: tokio::time::Instant::now(),
    }
}
impl Timer {
    /// Starts the timer by setting it as active and resetting the start time.
    ///
    /// This also clears any manually set hard timeout.
    pub fn timer_start(&mut self) {
        self.hard_timeout = false;
        self.timer_active = true;
        self.start_time = tokio::time::Instant::now();
    }

    /// Forces the timer into a timeout state, regardless of elapsed time.
    ///
    /// This is useful for emergency shutdowns or forced exits.
    pub fn release_timer(&mut self) {
        self.hard_timeout = true;
    }

    /// Returns the duration elapsed since the timer was last started.
    ///
    /// This does not check whether the timer is active or has timed out.
    pub fn get_wall_time(&mut self) -> tokio::time::Duration {
        return tokio::time::Instant::now() - self.start_time
    }


    /// Checks if the timer has timed out.
    ///
    /// The timer is considered timed out if:
    /// - It is active and the elapsed time exceeds `timeout_duration`, or
    /// - It has been manually forced to timeout using `release_timer()`.
    ///
    /// # Returns
    /// `true` if the timer is considered to have timed out; `false` otherwise.
    pub fn timer_timeouted(&self) -> bool {
        return (self.timer_active && (tokio::time::Instant::now() - self.start_time) > self.timeout_duration) || self.hard_timeout;
    }
}

