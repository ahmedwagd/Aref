//! The backoff curve, as pure functions (`G-13`; tasks 3.6 and 3.7).
//!
//! `G-13` fixes the whole policy: "the delay after the `n`-th failed attempt is
//! `min(cap, base * 2^(n-1))` with base 1 s and cap 30 s per wait, and the actual wait is
//! drawn with full jitter uniformly in `[0, computed]`, from an injected source …
//! A `Retry-After` that is present and parseable replaces the computed delay with the
//! stated duration rather than combining with it."
//!
//! Every branch of the request path — a 429 without a usable header, a network drop, a
//! timeout — calls [`BackoffPolicy::resolve`], so "one curve serves every backoff case" is
//! true by construction rather than by three parallel implementations. Nothing here
//! reads a clock; the wait is returned and the caller's [`Clock`](crate::providers::clock)
//! waits, which is what makes the schedule assertable offline.
//!
//! `Retry-After` is parsed as **delta-seconds only** (settled `D-2`): an HTTP-date value
//! is unusable, so the curve applies. There is no date-parsing branch and no `httpdate`
//! dependency.

use std::time::Duration;

use crate::providers::clock::Jitter;
use crate::providers::config::{BACKOFF_BASE, BACKOFF_CAP, MAX_TRANSPORT_ATTEMPTS};

/// `min(cap, base * 2^(n-1))`, with the draw applied on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffPolicy {
    pub base: Duration,
    pub cap: Duration,
    pub max_attempts: u32,
}

impl BackoffPolicy {
    /// The policy of `design.md` §6.1: the `G-13` values, and the engineering attempt cap.
    ///
    /// Injecting the policy rather than reading [`MAX_TRANSPORT_ATTEMPTS`] at each call
    /// site is what lets the humanize change pass its own budget without a second
    /// constant.
    pub fn from_config() -> Self {
        Self {
            base: BACKOFF_BASE,
            cap: BACKOFF_CAP,
            max_attempts: MAX_TRANSPORT_ATTEMPTS,
        }
    }

    /// The un-jittered delay after the `n`-th failed attempt.
    ///
    /// `n` is 1 for the first failure. Saturating throughout: a large `n` clamps to
    /// [`BackoffPolicy::cap`] instead of panicking or overflowing (`NFR-306`).
    pub fn computed(&self, n: u32) -> Duration {
        let exponent = n.max(1) - 1;
        // `checked_shl` reports a shift at or beyond the width as `None`, which is the
        // saturation case: the curve has been at its ceiling for a long time by then.
        let factor = match 1u32.checked_shl(exponent) {
            Some(factor) => factor,
            None => return self.cap,
        };
        match self.base.checked_mul(factor) {
            Some(product) => product.min(self.cap),
            None => self.cap,
        }
    }

    /// The drawn wait: full jitter in `[0, computed]`.
    pub fn wait(&self, n: u32, jitter: &dyn Jitter) -> Duration {
        jitter.draw(self.computed(n))
    }

    /// The one place a wait is produced.
    ///
    /// A present, parseable header **replaces** the drawn wait rather than adding to it,
    /// which is one line because the header arrives as an argument.
    pub fn resolve(&self, n: u32, header: Option<Duration>, jitter: &dyn Jitter) -> Duration {
        header.unwrap_or_else(|| self.wait(n, jitter))
    }
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self::from_config()
    }
}

/// A parsed `Retry-After`, and the whole minutes the UI is told about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAfter {
    pub wait: Duration,
    /// `params.minutes` for `E_RATE_LIMITED`: `max(1, ceil(seconds / 60))`.
    pub minutes: u64,
}

/// Reads a `Retry-After` field value.
///
/// Delta-seconds only, via `str::parse::<u64>` (settled `D-2`). Every other value —
/// absent, negative, non-numeric, an RFC 1123 date — yields `None`, so the caller applies
/// the computed curve. A malformed header never errors: "absent or unparseable" is one
/// branch by design, and the header is never assumed to exist (`AGENTS.md` §8).
///
/// The signature takes the field's text rather than a `HeaderMap`: this crate declares no
/// HTTP client of its own (`design.md` §3.2 rule 4 keeps one inside `providers::client`),
/// and the transport already owns the response type that carries the value.
pub fn parse_retry_after(header: Option<&str>) -> Option<RetryAfter> {
    let seconds: u64 = header?.trim().parse().ok()?;
    // `Duration::new` rather than `from_secs`: the constant scan in
    // `tests/providers_config.rs` forbids repeating a duration literal outside
    // `providers::config`, and this is a parsed value, not a constant.
    let wait = Duration::new(seconds, 0);
    Some(RetryAfter {
        wait,
        minutes: minutes_for(wait),
    })
}

/// `max(1, ceil(seconds / 60))`, the value `E_RATE_LIMITED` carries.
///
/// Rounding up is the only safe direction: a UI that offered "retry" after a rounded-down
/// wait would retry into another 429.
pub fn minutes_for(wait: Duration) -> u64 {
    let rounded_up_seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    // Integer ceiling division without `div_ceil`, which would raise the crate's minimum
    // supported Rust version for one expression.
    rounded_up_seconds
        .saturating_add(59)
        .saturating_div(60)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::clock::{FakeClock, FixedJitter, SequenceJitter, StdJitter};

    fn at(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    fn millis(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn policy() -> BackoffPolicy {
        BackoffPolicy::from_config()
    }

    // -------------------------------------------------------------- 3.6 the curve

    #[test]
    fn the_policy_carries_the_g13_base_and_cap() {
        let policy = policy();
        assert_eq!(policy.base, at(1));
        assert_eq!(policy.cap, at(30));
        assert!(policy.max_attempts >= 1, "an attempt budget must exist");
    }

    #[test]
    fn the_computed_delay_doubles_from_the_base_and_stops_at_the_cap() {
        // min(cap, base * 2^(n-1)) at n = 1..6: 1, 2, 4, 8, 16, 30.
        let expected = [at(1), at(2), at(4), at(8), at(16), at(30)];
        for (index, want) in expected.iter().enumerate() {
            let n = u32::try_from(index).unwrap_or(0) + 1;
            assert_eq!(policy().computed(n), *want, "n = {n}");
        }
    }

    #[test]
    fn a_large_attempt_number_saturates_at_the_cap() {
        for n in [7_u32, 31, 32, 33, 1_000, u32::MAX] {
            assert_eq!(policy().computed(n), at(30), "n = {n}");
        }
        // `n = 0` is not an attempt number; it is treated as the first failure rather
        // than panicking on a subtraction.
        assert_eq!(policy().computed(0), at(1));
    }

    #[test]
    fn every_draw_lies_within_the_computed_delay() {
        let jitter = StdJitter::with_seed(2026);
        for n in 1..=6 {
            let computed = policy().computed(n);
            for _ in 0..32 {
                let draw = policy().wait(n, &jitter);
                assert!(draw <= computed, "n = {n}: {draw:?} > {computed:?}");
            }
        }
    }

    #[test]
    fn the_wait_is_the_injected_draw_and_a_reproduced_schedule_needs_no_clock() {
        let clock = FakeClock::new();
        let jitter = SequenceJitter::new([millis(400), millis(1_600), millis(6_400)]);
        let mut schedule = Vec::new();
        for n in 1..=3 {
            let wait = policy().resolve(n, None, &jitter);
            // The schedule is the recorded sequence, clamped to each computed delay: at
            // n = 3 the curve is 4 s, so the 6.4 s draw is bounded by it (`G-13`).
            schedule.push(wait);
        }
        assert_eq!(schedule, vec![millis(400), millis(1_600), at(4)]);
        // Nothing above touched time: the fake clock was never asked to wait.
        assert_eq!(clock.sleep_count(), 0);
    }

    #[test]
    fn a_present_header_replaces_the_computed_delay() {
        let jitter = FixedJitter(Duration::ZERO);
        // The header's 5 s is returned unchanged even though the curve would draw 0.
        assert_eq!(policy().resolve(3, Some(at(5)), &jitter), at(5));
        // Without a header the same call draws from the curve (here: the jitter's zero).
        assert_eq!(policy().resolve(3, None, &jitter), Duration::ZERO);
        // And a draw is never the header: 5 s is not combined with the 4 s computed at
        // n = 3, which would have produced 9 s.
        assert_ne!(policy().resolve(3, Some(at(5)), &jitter), at(9));
    }

    // --------------------------------------------------------- 3.7 Retry-After

    #[test]
    fn an_absent_header_yields_none() {
        assert_eq!(parse_retry_after(None), None);
    }

    #[test]
    fn delta_seconds_are_accepted() {
        assert_eq!(
            parse_retry_after(Some("120")),
            Some(RetryAfter {
                wait: at(120),
                minutes: 2
            })
        );
        assert_eq!(
            parse_retry_after(Some("0")),
            Some(RetryAfter {
                wait: Duration::ZERO,
                minutes: 1
            }),
            "zero seconds still reports one whole minute"
        );
        // Whitespace around the value is tolerated; the value itself is not reinterpreted.
        assert_eq!(
            parse_retry_after(Some("  60 ")),
            Some(RetryAfter {
                wait: at(60),
                minutes: 1
            })
        );
        assert_eq!(
            parse_retry_after(Some("61")).map(|parsed| parsed.minutes),
            Some(2)
        );
    }

    #[test]
    fn an_unusable_value_yields_none_so_the_curve_applies() {
        for header in [
            "-5",
            "abc",
            "",
            " ",
            "1.5",
            "Wed, 21 Oct 2015 07:28:00 GMT",
            "Fri, 24 Oct 2026 10:00:00 +0200",
        ] {
            assert_eq!(parse_retry_after(Some(header)), None, "header = {header:?}");
        }
    }

    #[test]
    fn the_minutes_value_rounds_up_and_never_reaches_zero() {
        assert_eq!(minutes_for(Duration::ZERO), 1);
        assert_eq!(minutes_for(millis(1)), 1);
        assert_eq!(minutes_for(at(1)), 1);
        assert_eq!(minutes_for(at(60)), 1);
        assert_eq!(minutes_for(at(61)), 2);
        assert_eq!(minutes_for(at(120)), 2);
        assert_eq!(minutes_for(at(121)), 3);
        assert_eq!(minutes_for(at(3_600)), 60);
    }
}
