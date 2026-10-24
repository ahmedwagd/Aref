//! Time and jitter, injected (design.md §6.2; task 3.5).
//!
//! Both are traits so that a retry schedule is a **pure function of the attempt number
//! and an injected draw**, never of the wall clock: a test asks for the schedule and gets
//! it, with no sleep, no flake and no network (`NFR-305`).
//!
//! [`SystemClock`] and [`StdJitter`] are the production halves. `StdJitter` is a
//! ~15-line SplitMix64 seeded from the system time rather than the `rand` crate: jitter
//! needs uniformity, not cryptographic quality, and the dependency budget is not spent on
//! it (`AGENTS.md` §4). **It is not a source of randomness for anything security-relevant**
//! — a token, a nonce, or a key must never be derived from it.
//!
//! `FakeClock`, `FixedJitter`, and `SequenceJitter` are test doubles that live in the
//! library rather than behind `cfg(test)` because an integration test drives the real
//! pool through them, exactly as it drives it through `ScriptedTransport`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A future a [`Clock`] returns from [`Clock::sleep`].
///
/// Boxed so the trait stays object-safe without an `async-trait` dependency, and `Send`
/// so a caller can hold it across an `.await` inside a spawned task.
pub type ClockFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// The two things the request path needs from time.
pub trait Clock: Send + Sync {
    /// The current instant, for `providers://rate_limited`'s `resets_at`.
    fn now(&self) -> SystemTime;

    /// Waits for `duration`. The production implementation really waits; a fake records
    /// the request and returns immediately.
    fn sleep(&self, duration: Duration) -> ClockFuture<'_>;
}

/// The real clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) -> ClockFuture<'_> {
        // `tokio::time::sleep` borrows nothing from `self`, so this is a real timer in
        // the application and is simply never constructed by a test that wants a schedule.
        Box::pin(tokio::time::sleep(duration))
    }
}

#[derive(Debug)]
struct FakeClockState {
    now: SystemTime,
    sleeps: Vec<Duration>,
}

/// Records every requested duration and advances instead of waiting.
///
/// `now()` moves forward by exactly the durations asked for, so an event that stamps
/// `now + wait` is assertable as a literal instant rather than an interval.
#[derive(Debug)]
pub struct FakeClock {
    state: Mutex<FakeClockState>,
}

impl FakeClock {
    /// A clock that starts at the Unix epoch.
    ///
    /// The epoch rather than "now" so every derived timestamp is a stable literal in a
    /// test; nothing in the request path reads a wall clock of its own.
    pub fn new() -> Self {
        Self::starting_at(UNIX_EPOCH)
    }

    /// A clock that starts at `start`.
    pub fn starting_at(start: SystemTime) -> Self {
        Self {
            state: Mutex::new(FakeClockState {
                now: start,
                sleeps: Vec::new(),
            }),
        }
    }

    /// Every duration requested so far, in request order.
    pub fn sleeps(&self) -> Vec<Duration> {
        self.lock().sleeps.clone()
    }

    /// How many sleep calls were made.
    pub fn sleep_count(&self) -> usize {
        self.lock().sleeps.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, FakeClockState> {
        // No `unwrap` outside tests (AGENTS.md §9): every critical section here is one
        // push and one assignment, so a panic cannot leave the state half-written and
        // the poisoned guard is taken as-is.
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> SystemTime {
        self.lock().now
    }

    fn sleep(&self, duration: Duration) -> ClockFuture<'_> {
        {
            let mut state = self.lock();
            state.sleeps.push(duration);
            // A clock near the end of its range must not panic: the schedule is what the
            // test asserts, the instant is a convenience.
            state.now = state.now.checked_add(duration).unwrap_or(state.now);
        }
        Box::pin(std::future::ready(()))
    }
}

/// A draw for the full-jitter wait (`G-13`).
pub trait Jitter: Send + Sync {
    /// A value in `[0, upper]`.
    ///
    /// The bound is part of the contract, not a suggestion: a caller computes
    /// `min(cap, base * 2^(n-1))` and hands it over, and a draw above it would break
    /// `G-13`'s curve without failing anywhere. Every implementation clamps.
    fn draw(&self, upper: Duration) -> Duration;
}

/// SplitMix64 — the production jitter.
///
/// Seeded from the system time plus a process-local counter, so two instances created in
/// the same nanosecond do not share a stream. **Not** a source of randomness for anything
/// security-relevant; see the module note.
#[derive(Debug)]
pub struct StdJitter {
    state: Mutex<u64>,
}

impl StdJitter {
    /// A generator seeded from the system clock.
    pub fn from_system_time() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};

        static SEQUENCE: AtomicU64 = AtomicU64::new(0);

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos() as u64)
            .unwrap_or_default();
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Self::with_seed(nanos ^ sequence.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    /// A generator with an explicit seed, so the sequence itself is testable.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            state: Mutex::new(seed),
        }
    }

    fn next_u64(&self) -> u64 {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = *state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }
}

impl Default for StdJitter {
    fn default() -> Self {
        Self::from_system_time()
    }
}

impl Jitter for StdJitter {
    fn draw(&self, upper: Duration) -> Duration {
        if upper.is_zero() {
            return Duration::ZERO;
        }
        let span = upper.as_nanos();
        // 64 random bits scaled onto `[0, span)` in 128-bit arithmetic: the product
        // saturates instead of wrapping, and the result is strictly below `span`.
        let picked = span.saturating_mul(u128::from(self.next_u64())) / (1u128 << 64);
        Duration::from_nanos(picked.min(u128::from(u64::MAX)) as u64)
    }
}

/// A jitter source that always draws the same value, clamped to the bound.
///
/// The clamp is what keeps `G-13`'s "draw in `[0, computed]`" true for every
/// implementation instead of only for the production one.
#[derive(Debug, Clone, Copy)]
pub struct FixedJitter(pub Duration);

impl Jitter for FixedJitter {
    fn draw(&self, upper: Duration) -> Duration {
        self.0.min(upper)
    }
}

#[derive(Debug)]
struct SequenceState {
    index: usize,
    draws: usize,
}

/// A jitter source that yields a scripted sequence, then repeats its last value.
///
/// Repeating rather than cycling keeps a schedule assertion readable: past the end, every
/// further draw is the same value, so an off-by-one in the number of retries shows up as
/// a repeated wait rather than as a silently plausible cycle.
#[derive(Debug)]
pub struct SequenceJitter {
    values: Vec<Duration>,
    state: Mutex<SequenceState>,
}

impl SequenceJitter {
    pub fn new(values: impl IntoIterator<Item = Duration>) -> Self {
        Self {
            values: values.into_iter().collect(),
            state: Mutex::new(SequenceState { index: 0, draws: 0 }),
        }
    }

    /// How many draws have been served.
    pub fn draws(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .draws
    }
}

impl Jitter for SequenceJitter {
    fn draw(&self, upper: Duration) -> Duration {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let value = self
            .values
            .get(state.index)
            .or_else(|| self.values.last())
            .copied()
            .unwrap_or(Duration::ZERO);
        state.draws += 1;
        // Saturating at the last index, so an exhausted sequence repeats (see above).
        if state.index + 1 < self.values.len() {
            state.index += 1;
        }
        value.min(upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn at(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    #[tokio::test]
    async fn the_fake_clock_records_every_requested_duration_and_advances() {
        let clock = FakeClock::new();
        assert_eq!(clock.now(), UNIX_EPOCH);

        for wait in [at(1), Duration::from_millis(400), at(30)] {
            clock.sleep(wait).await;
        }

        assert_eq!(
            clock.sleeps(),
            vec![at(1), Duration::from_millis(400), at(30)],
            "the schedule is recorded exactly as requested"
        );
        assert_eq!(clock.sleep_count(), 3);
        assert_eq!(clock.now(), UNIX_EPOCH + Duration::from_millis(31_400));
    }

    #[tokio::test]
    async fn a_fake_clock_that_starts_elsewhere_derives_the_instant_from_it() {
        let start = UNIX_EPOCH + at(1_000);
        let clock = FakeClock::starting_at(start);
        clock.sleep(at(5)).await;
        assert_eq!(clock.now(), start + at(5));
    }

    #[test]
    fn the_std_jitter_is_a_pure_function_of_its_seed() {
        let first = StdJitter::with_seed(7);
        let second = StdJitter::with_seed(7);
        let a: Vec<Duration> = (0..5).map(|_| first.draw(at(30))).collect();
        let b: Vec<Duration> = (0..5).map(|_| second.draw(at(30))).collect();
        assert_eq!(a, b, "the same seed must produce the same draws");
        assert!(
            a.windows(2).any(|pair| pair[0] != pair[1]),
            "the stream must not be constant"
        );
    }

    #[test]
    fn every_jitter_respects_the_bound_including_zero() {
        let sources: Vec<Box<dyn Jitter>> = vec![
            Box::new(StdJitter::with_seed(11)),
            Box::new(FixedJitter(at(5))),
            Box::new(SequenceJitter::new([at(9), at(1)])),
        ];
        for source in sources {
            for upper in [Duration::ZERO, Duration::from_millis(1), at(30)] {
                for _ in 0..20 {
                    let draw = source.draw(upper);
                    assert!(draw <= upper, "{draw:?} exceeded {upper:?}");
                }
            }
        }
    }

    #[test]
    fn a_fixed_draw_above_the_bound_is_clamped_instead_of_breaking_the_curve() {
        assert_eq!(FixedJitter(at(30)).draw(at(4)), at(4));
        assert_eq!(FixedJitter(at(2)).draw(at(4)), at(2));
    }

    #[test]
    fn a_scripted_sequence_repeats_its_last_value_and_clamps() {
        let jitter = SequenceJitter::new([at(1), at(2)]);
        assert_eq!(jitter.draw(at(30)), at(1));
        assert_eq!(jitter.draw(at(30)), at(2));
        assert_eq!(jitter.draw(at(30)), at(2), "exhausted: the last value");
        assert_eq!(jitter.draw(at(1)), at(1), "clamped to the bound");
        assert_eq!(jitter.draws(), 4);
    }

    #[test]
    fn an_empty_sequence_draws_zero() {
        let jitter = SequenceJitter::new([]);
        assert_eq!(jitter.draw(at(5)), Duration::ZERO);
    }

    #[test]
    fn both_clock_implementations_are_usable_as_trait_objects() {
        let clocks: [Arc<dyn Clock>; 2] = [Arc::new(SystemClock), Arc::new(FakeClock::new())];
        assert_eq!(clocks.len(), 2);
    }
}
