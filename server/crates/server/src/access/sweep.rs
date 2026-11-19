//! The hourly sweep of expired sessions.
use super::Log;
use sidereal_core::database::Database;
use std::time::Duration;
use tokio::{
    task::JoinHandle,
    time::{interval_at, Instant, MissedTickBehavior},
};

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

/// Delete expired sessions on `schedule`, in a task of its own, so no
/// request waits on a sweep. A late sweep, after an outage or a suspended
/// host, is one sweep, not a burst of catch-up ones.
pub fn sweep(database: Database, schedule: Schedule, log: Log) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticks = interval_at(Instant::now() + schedule.first, schedule.every);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            match database.sweep_sessions().await {
                Ok(0) => {}
                Ok(1) => log.line("session sweep: deleted 1 expired session"),
                Ok(deleted) => log.line(&format!(
                    "session sweep: deleted {deleted} expired sessions"
                )),
                Err(_) => log.line(
                    "database unavailable: cannot sweep expired sessions; trying again in an hour",
                ),
            }
        }
    })
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
