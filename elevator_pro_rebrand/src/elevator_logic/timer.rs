//! Timer module for managing asynchronous timeouts in elevator control logic.
//!
//! This module defines two core components:
//! - [`Timer`]: A general-purpose timer that supports both soft (elapsed time) and hard (manual) timeouts.
//! - [`ElevatorTimers`]: A struct that bundles all timers used in the elevator FSM, including door timeout,
//!   cab call priority grace period, and general error detection.
//!
//! These components are used throughout the elevator state machine to control behavior based on timeouts,
//! such as how long to keep doors open, how long to prioritize internal cab calls, or when to enter an error state
//! due to inactivity or unresponsiveness.
//!
//! # Usage
//! Timers are used in `fsm.rs` to manage:
//! - Door open duration (`door` timer)
//! - Grace period for cab button prioritization (`cab_priority` timer)
//! - Communication or logic errors (`error` timer)
//!
//! # Example (using Timer standalone)
//! ```rust,no_run
//! use tokio::time::Duration;
//! use crate::elevator_logic::timer::Timer;
//!
//! let mut door_timer = Timer::new(Duration::from_secs(3));
//! door_timer.timer_start();
//!
//! // In FSM loop:
//! if door_timer.timer_timeouted() {
//!     // Trigger door close logic
//! }
//! ```
//!
//! # ElevatorTimers Usage
//! ElevatorTimers simplifies timer management by grouping all related timers into one struct:
//!
//! ```rust,no_run
//! let mut timers = ElevatorTimers::new(
//!     Duration::from_secs(3),   // door
//!     Duration::from_secs(10),  // cab priority
//!     Duration::from_secs(7),   // error
//! );
//!
//! timers.door.timer_start();
//! if timers.cab_priority.timer_timeouted() {
//!     // Prioritization window over
//! }
//! ```
//!
//! # Timer Behavior
//! - A timer is **inactive** until [`timer_start()`](Timer::timer_start) is called.
//! - Once active, it compares current time with the internal start time.
//! - The timer can be **manually forced** to timeout using [`release_timer()`](Timer::release_timer).
//! - A call to [`timer_timeouted()`](Timer::timer_timeouted) returns `true` if either a soft or hard timeout has occurred.
//!
//! # Related
//! Used heavily in the [`fsm`](crate::elevator_logic::fsm) module.


use tokio::time::Duration;

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

impl Timer {
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


/// Collection of timers used in the elevator's finite state machine (FSM).
///
/// This struct encapsulates all timers that track different timeout conditions
/// such as door closing, inside call priority window, and general error state.
/// Also includes state tracking related to inside call grace period.
pub struct ElevatorTimers {
    /// Timer for automatic door closing.
    pub door: Timer,

    /// Timer that provides a short grace period to prioritize inside (cab) calls
    /// after a passenger enters the elevator.
    ///
    /// When the elevator stops at a floor due to a hall request (e.g. someone pressed "up"),
    /// this timer is started to give the passenger a few seconds to press a cab button
    /// (e.g. "Floor 3"). During this grace period, the FSM prioritizes inside orders
    /// in the direction the elevator was called.
    ///
    /// After the timer expires, the elevator becomes available for other external requests.
    pub cab_priority: Timer,

    /// Timer for tracking long-term inactivity or error conditions.
    pub error: Timer,

    /// Tracks whether the `cab_priority` timer had timed out in the previous iteration.
    pub prev_cab_priority_timeout: bool,
}

impl ElevatorTimers {
    /// Creates a new `ElevatorTimers` instance with custom durations.
    ///
    /// # Parameters
    /// - `door_duration`: Duration before automatically closing the elevator door.
    /// - `cab_priority_duration`: Grace period for prioritizing cab calls after stopping.
    /// - `error_duration`: Duration before considering the elevator to be in an error state.
    ///
    /// # Returns
    /// An initialized `ElevatorTimers` struct with the specified timeout settings.
    pub fn new(
        door_duration: Duration,
        cab_priority_duration: Duration,
        error_duration: Duration,
    ) -> Self {
        ElevatorTimers {
            door: Timer::new(door_duration),
            cab_priority: Timer::new(cab_priority_duration),
            error: Timer::new(error_duration),
            prev_cab_priority_timeout: false,
        }
    }
}
