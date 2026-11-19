//! The hourly sweep of expired sessions.
use std::time::Duration;

/// When the sweep runs: `first` after it starts, then every `every`. Only
/// a test passes anything but [`Schedule::HOURLY`]; nothing reads it from
/// the environment.
#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    pub first: Duration,
    pub every: Duration,
}

impl Schedule {
    pub const HOURLY: Schedule = Schedule {
        first: Duration::from_secs(60),
        every: Duration::from_secs(60 * 60),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hourly_schedule_sweeps_a_minute_after_start_then_every_hour() {
        assert_eq!(Schedule::HOURLY.first, Duration::from_secs(60));
        assert_eq!(Schedule::HOURLY.every, Duration::from_secs(3600));
    }
}
