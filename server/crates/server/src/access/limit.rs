//! The sign-in limit: failed sign-ins counted per client address in three
//! sliding windows, held only in this process.
use std::{
    collections::{HashMap, VecDeque},
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Where the limiter reads the time, so a test need not wait out a window.
pub type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

pub fn system_clock() -> Clock {
    Arc::new(Instant::now)
}

/// Each tier allows at most this many failures in any window this long.
const TIERS: [(usize, Duration); 3] = [
    (1, Duration::from_secs(1)),
    (5, Duration::from_secs(60)),
    (20, Duration::from_secs(3600)),
];
/// The longest window. A failure older than this counts against nothing.
const KEPT: Duration = TIERS[TIERS.len() - 1].1;

/// What one client address is counted under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    V4(Ipv4Addr),
    /// The top 64 bits of the address.
    V6(u64),
    Unresolved,
}

/// An address that must wait before it may try again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// Whole seconds until every full tier has room, at least 1.
    pub retry_after: u64,
}

pub struct Limiter {
    clock: Clock,
    entries: Mutex<HashMap<Key, VecDeque<Instant>>>,
}

impl Limiter {
    pub fn new(clock: Clock) -> Self {
        Self {
            clock,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Whether `key` must wait, without counting anything.
    pub fn refused(&self, key: Key) -> Option<Refusal> {
        let now = (self.clock)();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let attempts = entries.get_mut(&key)?;
        wait(attempts, now).map(|retry_after| Refusal { retry_after })
    }

    /// Count an attempt for `key` if every tier has room. The check and the
    /// count share one lock, so attempts at the same time see each other.
    pub fn admit(&self, key: Key) -> Result<(), Refusal> {
        let now = (self.clock)();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let attempts = entries.entry(key).or_default();
        if let Some(retry_after) = wait(attempts, now) {
            return Err(Refusal { retry_after });
        }
        attempts.push_back(now);
        Ok(())
    }
}

/// The whole seconds until every full tier has room, or `None` when none is
/// full. Forgets failures too old to count. `attempts` runs oldest first.
fn wait(attempts: &mut VecDeque<Instant>, now: Instant) -> Option<u64> {
    while attempts
        .front()
        .is_some_and(|&at| now.saturating_duration_since(at) >= KEPT)
    {
        attempts.pop_front();
    }
    let longest = TIERS
        .iter()
        .filter_map(|&(limit, window)| {
            let oldest = *attempts.iter().rev().nth(limit - 1)?;
            window.checked_sub(now.saturating_duration_since(oldest))
        })
        .filter(|wait| !wait.is_zero())
        .max()?;
    Some((longest.as_secs() + u64::from(longest.subsec_nanos() > 0)).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clock that moves only when the test says so.
    struct Manual(Arc<Mutex<Instant>>);

    impl Manual {
        fn advance(&self, by: Duration) {
            *self.0.lock().unwrap() += by;
        }
    }

    fn limiter() -> (Limiter, Manual) {
        let now = Arc::new(Mutex::new(Instant::now()));
        let reading = now.clone();
        let clock: Clock = Arc::new(move || *reading.lock().unwrap());
        (Limiter::new(clock), Manual(now))
    }

    const KEY: Key = Key::V4(Ipv4Addr::new(192, 168, 1, 20));

    fn seconds(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    /// `count` failures from `KEY`, `apart` from each other, leaving the clock
    /// at the last one.
    fn fail(limiter: &Limiter, clock: &Manual, count: usize, apart: Duration) {
        for n in 0..count {
            if n > 0 {
                clock.advance(apart);
            }
            limiter.admit(KEY).expect("room for a failure");
        }
    }

    fn wait(limiter: &Limiter) -> Option<u64> {
        limiter.admit(KEY).err().map(|refusal| refusal.retry_after)
    }

    #[test]
    fn a_key_with_no_failures_is_admitted() {
        let (limiter, _) = limiter();
        assert_eq!(wait(&limiter), None);
    }

    #[test]
    fn a_wait_under_a_second_rounds_up_to_one() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 1, Duration::ZERO);
        clock.advance(Duration::from_millis(999));
        assert_eq!(wait(&limiter), Some(1));
    }

    #[test]
    fn the_second_tier_has_room_once_its_window_passes() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 1, Duration::ZERO);
        clock.advance(seconds(1));
        assert_eq!(wait(&limiter), None);
    }

    #[test]
    fn the_sixth_attempt_within_a_minute_waits_for_the_minute_tier() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 5, seconds(2));
        clock.advance(seconds(2));
        assert_eq!(wait(&limiter), Some(50));
    }

    #[test]
    fn the_minute_tier_has_room_once_its_window_passes() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 5, seconds(2));
        clock.advance(seconds(61 - 8));
        assert_eq!(wait(&limiter), None);
    }

    #[test]
    fn the_twenty_first_attempt_within_an_hour_waits_for_the_hour_tier() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 20, seconds(15));
        clock.advance(seconds(15));
        assert_eq!(wait(&limiter), Some(3300));
    }

    #[test]
    fn the_wait_is_the_longest_over_full_tiers() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 5, seconds(2));
        clock.advance(Duration::from_millis(500));
        // The second tier needs 0.5 s more; the minute tier 51.5 s.
        assert_eq!(wait(&limiter), Some(52));
    }

    #[test]
    fn a_check_reports_the_wait_without_counting() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 5, seconds(2));
        clock.advance(seconds(2));
        assert_eq!(limiter.refused(KEY), Some(Refusal { retry_after: 50 }));
        assert_eq!(limiter.refused(KEY), Some(Refusal { retry_after: 50 }));
    }
}
