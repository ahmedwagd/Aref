//! The two events the request path emits (`FR-305`, `FR-307`; task 3.15).
//!
//! Both names and both payloads are ratified in `design.md` §4.3 and mirrored by
//! `src/ipc/types.ts` (`KeyInvalidEvent`, `RateLimitedEvent`). Field names are
//! `snake_case` with no `serde(rename_all)`, so the two sides are literal mirrors.
//!
//! `masked_preview` is the **stored** value, read from the key's metadata row, never a
//! fresh derivation (`G-27`): re-deriving it would need the raw key, which is exactly what
//! the pool does not keep. `resets_at` is RFC 3339 UTC, formatted by [`resets_at`] from the
//! injected clock — the payload never formats time of its own.
//!
//! A sink cannot fail a run. `providers://key_invalid` is a notice, the status read
//! (`get_app_status`) is the source of truth for the UI, and a listener that cannot attach
//! must not turn a failed key into a failed request.

#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::Emitter;

use crate::storage::rfc3339_utc;

/// `providers://key_invalid` (SRS §6.2).
pub const KEY_INVALID_EVENT: &str = "providers://key_invalid";

/// `providers://rate_limited` (SRS §6.2).
pub const RATE_LIMITED_EVENT: &str = "providers://rate_limited";

/// A rejected credential, named by its masked preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyInvalid {
    pub provider_id: String,
    pub key_id: String,
    pub masked_preview: String,
}

/// The pool is exhausted by 429s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RateLimited {
    pub provider_id: String,
    /// The whole seconds the caller was told to wait.
    pub retry_after_seconds: u64,
    /// RFC 3339 UTC: when that wait ends.
    pub resets_at: String,
}

/// Where the two events go.
///
/// A trait so the pool can be tested with [`RecordingSink`] and shipped with
/// [`TauriEventSink`], and so `providers::pool` never names `tauri::AppHandle`
/// (design.md §3.3).
pub trait EventSink: Send + Sync {
    fn key_invalid(&self, event: &KeyInvalid);
    fn rate_limited(&self, event: &RateLimited);
}

/// The Tauri adapter: one `emit` per event, name and payload unchanged.
pub struct TauriEventSink {
    app: tauri::AppHandle,
}

impl TauriEventSink {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
    fn key_invalid(&self, event: &KeyInvalid) {
        // A window may be gone or a listener may have failed to attach; the run is not
        // the place to report either.
        let _ = self.app.emit(KEY_INVALID_EVENT, event);
    }

    fn rate_limited(&self, event: &RateLimited) {
        let _ = self.app.emit(RATE_LIMITED_EVENT, event);
    }
}

/// The instant a wait started at `now` ends, as RFC 3339 UTC.
///
/// The clock is the injected one, so a test asserts a literal instant instead of a range.
pub fn resets_at(now: SystemTime, wait: Duration) -> String {
    rfc3339_utc(now.checked_add(wait).unwrap_or(now))
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct RecordingSink {
    key_invalid: Mutex<Vec<KeyInvalid>>,
    rate_limited: Mutex<Vec<RateLimited>>,
}

#[cfg(any(test, feature = "test-support"))]
impl RecordingSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn key_invalid_events(&self) -> Vec<KeyInvalid> {
        self.key_invalid
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    pub fn rate_limited_events(&self) -> Vec<RateLimited> {
        self.rate_limited
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl EventSink for RecordingSink {
    fn key_invalid(&self, event: &KeyInvalid) {
        self.key_invalid
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(event.clone());
    }

    fn rate_limited(&self, event: &RateLimited) {
        self.rate_limited
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(event.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::time::UNIX_EPOCH;

    #[test]
    fn the_event_names_are_the_ratified_strings() {
        assert_eq!(KEY_INVALID_EVENT, "providers://key_invalid");
        assert_eq!(RATE_LIMITED_EVENT, "providers://rate_limited");
    }

    #[test]
    fn the_key_invalid_payload_has_exactly_the_ratified_fields() -> Result<(), serde_json::Error> {
        let event = KeyInvalid {
            provider_id: "provider-1".to_owned(),
            key_id: "key-1".to_owned(),
            masked_preview: "••••••••et-a".to_owned(),
        };
        let value = serde_json::to_value(&event)?;
        assert_eq!(
            value,
            json!({
                "provider_id": "provider-1",
                "key_id": "key-1",
                "masked_preview": "••••••••et-a"
            })
        );
        Ok(())
    }

    #[test]
    fn the_rate_limited_payload_has_exactly_the_ratified_fields() -> Result<(), serde_json::Error> {
        let event = RateLimited {
            provider_id: "provider-1".to_owned(),
            retry_after_seconds: 120,
            resets_at: "2026-10-24T00:02:00Z".to_owned(),
        };
        let value = serde_json::to_value(&event)?;
        assert_eq!(
            value,
            json!({
                "provider_id": "provider-1",
                "retry_after_seconds": 120,
                "resets_at": "2026-10-24T00:02:00Z"
            })
        );
        Ok(())
    }

    #[test]
    fn resets_at_is_rfc3339_utc_and_derives_from_the_injected_clock() {
        assert_eq!(
            resets_at(UNIX_EPOCH, Duration::from_secs(120)),
            "1970-01-01T00:02:00Z"
        );
        assert_eq!(
            resets_at(UNIX_EPOCH, Duration::ZERO),
            "1970-01-01T00:00:00Z"
        );
        // A wait that cannot be added must not panic: the instant is reported unchanged.
        let last_second = UNIX_EPOCH + Duration::from_secs(253_402_300_799);
        assert_eq!(
            resets_at(last_second, Duration::from_secs(u64::MAX)),
            "9999-12-31T23:59:59Z"
        );
    }

    #[test]
    fn the_recording_sink_keeps_both_payloads_apart() {
        let sink = RecordingSink::new();
        sink.key_invalid(&KeyInvalid {
            provider_id: "provider-1".to_owned(),
            key_id: "key-1".to_owned(),
            masked_preview: "••••••••et-a".to_owned(),
        });
        sink.rate_limited(&RateLimited {
            provider_id: "provider-2".to_owned(),
            retry_after_seconds: 30,
            resets_at: "1970-01-01T00:00:30Z".to_owned(),
        });

        assert_eq!(sink.key_invalid_events().len(), 1);
        assert_eq!(sink.rate_limited_events().len(), 1);
        assert_eq!(sink.key_invalid_events()[0].key_id, "key-1");
        assert_eq!(sink.rate_limited_events()[0].retry_after_seconds, 30);
    }

    #[test]
    fn a_payload_carries_no_field_beyond_the_contract() -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(KeyInvalid {
            provider_id: "provider-1".to_owned(),
            key_id: "key-1".to_owned(),
            masked_preview: "••••••••et-a".to_owned(),
        })?;
        let object = match value.as_object() {
            Some(object) => object,
            None => panic!("the payload is not an object"),
        };
        let mut names: Vec<&str> = object.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["key_id", "masked_preview", "provider_id"]);
        assert!(matches!(
            object.get("masked_preview"),
            Some(Value::String(_))
        ));
        Ok(())
    }
}
