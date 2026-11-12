//! The sign-in limit: failed sign-ins counted per client address in three
//! sliding windows, held only in this process.
use super::forwarded::Forwarded;
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex, MutexGuard},
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
const SWEEP: Duration = Duration::from_secs(60);

/// What one client address is counted under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    V4(Ipv4Addr),
    /// The top 64 bits of the address.
    V6(u64),
    Unresolved,
}

impl Key {
    /// A request without `Forwarded` came through no trusted-proxy layer, so
    /// its address is as unknown as one the layer could not resolve.
    pub fn of(forwarded: Option<&Forwarded>) -> Self {
        match forwarded.and_then(|forwarded| forwarded.client) {
            Some(IpAddr::V4(address)) => Self::V4(address),
            Some(IpAddr::V6(address)) => Self::V6((address.to_bits() >> 64) as u64),
            None => Self::Unresolved,
        }
    }
}

/// An address that must wait before it may try again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// Whole seconds until every full tier has room, at least 1.
    pub retry_after: u64,
    /// Whether this is the first refusal since the key's last attempt that
    /// was not refused.
    pub new: bool,
}

/// One counted attempt, from the moment its password check starts. Dropped
/// unsettled, as when the client goes away mid-check, it stays counted, so
/// cancelling a request does not make a guess free.
pub struct Attempt<'a> {
    limiter: &'a Limiter,
    key: Key,
    at: Instant,
}

impl Attempt<'_> {
    /// The sign-in succeeded: forget every failure for this key.
    pub fn succeeded(self) {
        self.limiter.state().entries.remove(&self.key);
    }

    /// The attempt reached no answer about the password: stop counting it.
    pub fn released(self) {
        let mut state = self.limiter.state();
        let Some(entry) = state.entries.get_mut(&self.key) else {
            return;
        };
        // Attempts settle out of order, so the newest instant may be
        // another attempt's failure.
        if let Some(index) = entry.attempts.iter().rposition(|&at| at == self.at) {
            entry.attempts.remove(index);
        }
    }
}

pub struct Limiter {
    clock: Clock,
    state: Mutex<State>,
}

struct State {
    entries: HashMap<Key, Entry>,
    swept: Instant,
}

#[derive(Default)]
struct Entry {
    /// Counted attempts in the last hour, oldest first.
    attempts: VecDeque<Instant>,
    /// Whether the key's latest check was refused.
    refused: bool,
}

impl Limiter {
    pub fn new(clock: Clock) -> Self {
        let swept = clock();
        Self {
            clock,
            state: Mutex::new(State {
                entries: HashMap::new(),
                swept,
            }),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether `key` must wait, without counting anything.
    pub fn refused(&self, key: Key) -> Option<Refusal> {
        let now = (self.clock)();
        let mut state = self.state();
        state.entries.get_mut(&key)?.check(now)
    }

    /// Count an attempt for `key` if every tier has room. The check and the
    /// count share one lock, so attempts at the same time see each other.
    pub fn admit(&self, key: Key) -> Result<Attempt<'_>, Refusal> {
        let now = (self.clock)();
        let mut state = self.state();
        state.sweep(now);
        let entry = state.entries.entry(key).or_default();
        if let Some(refusal) = entry.check(now) {
            return Err(refusal);
        }
        entry.attempts.push_back(now);
        Ok(Attempt {
            limiter: self,
            key,
            at: now,
        })
    }

    #[cfg(test)]
    fn keys(&self) -> usize {
        self.state().entries.len()
    }
}

impl State {
    /// Drop keys with no attempt in the last hour, at most once a minute, so
    /// memory holds only addresses that each paid for a password check.
    fn sweep(&mut self, now: Instant) {
        if now.saturating_duration_since(self.swept) < SWEEP {
            return;
        }
        self.swept = now;
        self.entries.retain(|_, entry| {
            entry
                .attempts
                .back()
                .is_some_and(|&at| now.saturating_duration_since(at) < KEPT)
        });
    }
}

impl Entry {
    /// Refuse the key while any tier is full. Forgets failures too old to
    /// count.
    fn check(&mut self, now: Instant) -> Option<Refusal> {
        while self
            .attempts
            .front()
            .is_some_and(|&at| now.saturating_duration_since(at) >= KEPT)
        {
            self.attempts.pop_front();
        }
        let longest = TIERS
            .iter()
            .filter_map(|&(limit, window)| {
                let oldest = *self.attempts.iter().rev().nth(limit - 1)?;
                window.checked_sub(now.saturating_duration_since(oldest))
            })
            .filter(|wait| !wait.is_zero())
            .max();
        let Some(longest) = longest else {
            self.refused = false;
            return None;
        };
        let retry_after = (longest.as_secs() + u64::from(longest.subsec_nanos() > 0)).max(1);
        let new = !self.refused;
        self.refused = true;
        Some(Refusal { retry_after, new })
    }
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
            let _failed = limiter.admit(KEY).expect("room for a failure");
        }
    }

    fn wait(limiter: &Limiter) -> Option<u64> {
        limiter.admit(KEY).err().map(|refusal| refusal.retry_after)
    }

    fn key(client: &str) -> Key {
        Key::of(Some(&Forwarded {
            https: false,
            client: Some(client.parse().unwrap()),
        }))
    }

    #[test]
    fn an_ipv4_address_is_its_own_key() {
        assert_eq!(key("192.168.1.20"), KEY);
        assert_ne!(key("192.168.1.21"), KEY);
    }

    #[test]
    fn an_ipv6_address_keys_by_its_64_prefix() {
        assert_eq!(key("2001:db8:1:2::1"), key("2001:db8:1:2::ffff"));
        assert_eq!(key("2001:db8:1:2::1"), Key::V6(0x2001_0db8_0001_0002));
    }

    #[test]
    fn another_ipv6_64_is_another_key() {
        assert_ne!(key("2001:db8:1:2::1"), key("2001:db8:1:3::1"));
    }

    #[test]
    fn an_unresolved_client_is_the_unresolved_key() {
        let unresolved = Forwarded {
            https: false,
            client: None,
        };
        assert_eq!(Key::of(Some(&unresolved)), Key::Unresolved);
    }

    #[test]
    fn a_request_without_forwarded_is_the_unresolved_key() {
        assert_eq!(Key::of(None), Key::Unresolved);
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
        assert_eq!(
            limiter.refused(KEY),
            Some(Refusal {
                retry_after: 50,
                new: true
            })
        );
        assert_eq!(
            limiter.refused(KEY),
            Some(Refusal {
                retry_after: 50,
                new: false
            })
        );
    }

    const OTHER: Key = Key::V4(Ipv4Addr::new(192, 168, 1, 21));

    #[test]
    fn a_success_forgets_the_key_failures() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 4, seconds(2));
        clock.advance(seconds(2));
        limiter.admit(KEY).unwrap().succeeded();
        assert_eq!(wait(&limiter), None);
    }

    #[test]
    fn a_success_leaves_another_key_counted() {
        let (limiter, _) = limiter();
        let _failed = limiter.admit(OTHER).unwrap();
        limiter.admit(KEY).unwrap().succeeded();
        assert_eq!(
            limiter.refused(OTHER),
            Some(Refusal {
                retry_after: 1,
                new: true
            })
        );
    }

    #[test]
    fn a_released_attempt_stops_counting() {
        let (limiter, _) = limiter();
        limiter.admit(KEY).unwrap().released();
        assert_eq!(wait(&limiter), None);
    }

    #[test]
    fn a_release_removes_only_its_own_attempt() {
        let (limiter, clock) = limiter();
        let first = limiter.admit(KEY).unwrap();
        clock.advance(seconds(1));
        // The later attempt fails first.
        let _ = limiter.admit(KEY).unwrap();
        first.released();
        clock.advance(Duration::from_millis(500));
        assert_eq!(wait(&limiter), Some(1));
    }

    #[test]
    fn an_unsettled_attempt_stays_counted() {
        let (limiter, _) = limiter();
        let _ = limiter.admit(KEY).unwrap();
        assert_eq!(wait(&limiter), Some(1));
    }

    #[test]
    fn a_refusal_is_not_counted() {
        let (limiter, clock) = limiter();
        fail(&limiter, &clock, 5, seconds(2));
        clock.advance(seconds(2));
        assert_eq!(wait(&limiter), Some(50));
        clock.advance(seconds(61 - 10));
        assert_eq!(wait(&limiter), None);
    }

    fn is_new(refusal: Result<Attempt<'_>, Refusal>) -> bool {
        refusal.err().expect("a refusal").new
    }

    #[test]
    fn the_first_refusal_is_new() {
        let (limiter, _) = limiter();
        let _ = limiter.admit(KEY).unwrap();
        assert!(is_new(limiter.admit(KEY)));
    }

    #[test]
    fn a_later_refusal_in_the_run_is_not_new() {
        let (limiter, _) = limiter();
        let _ = limiter.admit(KEY).unwrap();
        limiter.refused(KEY);
        assert!(!is_new(limiter.admit(KEY)));
    }

    #[test]
    fn a_check_with_room_ends_the_run() {
        let (limiter, clock) = limiter();
        let _ = limiter.admit(KEY).unwrap();
        limiter.refused(KEY);
        clock.advance(seconds(1));
        // Room, as before a malformed body that is never counted.
        assert_eq!(limiter.refused(KEY), None);
        let _ = limiter.admit(KEY).unwrap();
        assert!(is_new(limiter.admit(KEY)));
    }

    const THIRD: Key = Key::V4(Ipv4Addr::new(192, 168, 1, 22));

    #[test]
    fn an_admit_a_minute_after_the_last_sweep_drops_keys_idle_for_an_hour() {
        let (limiter, clock) = limiter();
        let _ = limiter.admit(OTHER).unwrap();
        clock.advance(seconds(3600));
        let _ = limiter.admit(KEY).unwrap();
        assert_eq!(limiter.keys(), 1);
    }

    #[test]
    fn an_admit_within_a_minute_of_the_last_sweep_drops_nothing() {
        let (limiter, clock) = limiter();
        let _ = limiter.admit(OTHER).unwrap();
        clock.advance(seconds(3599));
        // Sweeps, while `OTHER` is not yet an hour old.
        let _ = limiter.admit(KEY).unwrap();
        clock.advance(seconds(59));
        let _ = limiter.admit(THIRD).unwrap();
        assert_eq!(limiter.keys(), 3);
    }
}
