//! The one error type that crosses IPC.
//!
//! `AppError` serializes to exactly `{ code, message_key, params }` (design.md §4.1,
//! AGENTS.md §9). `code` and `message_key` are catalog strings from SRS §7 — no new
//! code is introduced here, and `catalog()` is the single list the i18n tests compare
//! against.

use std::collections::BTreeMap;

use serde::ser::{Serialize, SerializeMap, Serializer};

/// A `params` value.
///
/// Closed on purpose (design.md §4.1): there is no object variant and no array variant,
/// so a nested payload cannot be expressed. That is what makes "rejected at the type
/// level" true rather than a convention.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    /// A catalog identifier or another non-secret scalar. Never user text (`NFR-205`).
    Text(String),
    Number(i64),
    Bool(bool),
    Null,
}

impl Serialize for ParamValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ParamValue::Text(value) => serializer.serialize_str(value),
            ParamValue::Number(value) => serializer.serialize_i64(*value),
            ParamValue::Bool(value) => serializer.serialize_bool(*value),
            ParamValue::Null => serializer.serialize_unit(),
        }
    }
}

/// `params` keys are `&'static str` so a key cannot be built from runtime data.
pub type Params = BTreeMap<&'static str, ParamValue>;

/// The four `E_OUTPUT_REJECTED` reasons the catalog fixes (SRS §7, design.md §4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputRejection {
    Empty,
    Truncated,
    Refused,
    WrongLanguage,
}

impl OutputRejection {
    pub fn as_str(self) -> &'static str {
        match self {
            OutputRejection::Empty => "empty",
            OutputRejection::Truncated => "truncated",
            OutputRejection::Refused => "refused",
            OutputRejection::WrongLanguage => "wrong_language",
        }
    }
}

/// The two `E_KEYRING_UNAVAILABLE` reasons (design.md §4.6, §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringFault {
    Unavailable,
    Unreadable,
}

impl KeyringFault {
    pub fn as_str(self) -> &'static str {
        match self {
            KeyringFault::Unavailable => "unavailable",
            KeyringFault::Unreadable => "unreadable",
        }
    }
}

/// Every branch maps to one row of SRS §7 / design.md §4.6.
///
/// The `Display` text is for logs and development only; the UI renders `message_key`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AppError {
    #[error("a required field is missing or blank")]
    InputEmpty { field: &'static str },
    #[error("the input exceeds the word limit")]
    InputTooLong { limit: u32 },
    #[error("no usable provider is configured")]
    NoProvider { reason: &'static str },
    #[error("the key was rejected")]
    KeyInvalid { provider: String, masked: String },
    #[error("the rate limit was reached")]
    RateLimited { minutes: u64 },
    #[error("the network is unreachable")]
    NetworkOffline,
    #[error("the provider request timed out")]
    ProviderTimeout,
    #[error("the response stream was interrupted")]
    StreamInterrupted,
    #[error("the provider output was rejected")]
    OutputRejected { reason: OutputRejection },
    #[error("humanization did not complete")]
    HumanizeFailed,
    #[error("the detector is running statistical-only")]
    DetectorDegraded,
    #[error("the bundled model could not be restored")]
    ModelRestoreFailed,
    #[error("the system keyring is unavailable")]
    KeyringUnavailable { reason: KeyringFault },
    #[error("an unexpected failure occurred")]
    Internal { status: Option<u16> },
}

/// `(code, message_key)` for every code this core can emit. SRS §7 verbatim.
const CATALOG: [(&str, &str); 14] = [
    ("E_INPUT_EMPTY", "error.input.empty"),
    ("E_INPUT_TOO_LONG", "error.input.too_long"),
    ("E_NO_PROVIDER", "error.provider.none"),
    ("E_KEY_INVALID", "error.provider.key_invalid"),
    ("E_RATE_LIMITED", "error.provider.rate_limited"),
    ("E_NETWORK_OFFLINE", "error.provider.network_offline"),
    ("E_PROVIDER_TIMEOUT", "error.provider.timeout"),
    ("E_STREAM_INTERRUPTED", "error.provider.stream_interrupted"),
    ("E_OUTPUT_REJECTED", "error.provider.output_rejected"),
    ("E_HUMANIZE_FAILED", "error.humanize.failed"),
    ("E_DETECTOR_DEGRADED", "notice.detector.statistical_only"),
    (
        "E_MODEL_RESTORE_FAILED",
        "error.detector.model_restore_failed",
    ),
    (
        "E_KEYRING_UNAVAILABLE",
        "error.security.keyring_unavailable",
    ),
    ("E_INTERNAL", "error.internal"),
];

impl AppError {
    /// The SRS §7 catalog, for tests and for the i18n parity check.
    pub fn catalog() -> &'static [(&'static str, &'static str)] {
        &CATALOG
    }

    /// Malformed request bodies and other unexpected failures (design.md §4.5/§4.6).
    pub fn internal() -> Self {
        AppError::Internal { status: None }
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppError::InputEmpty { .. } => "E_INPUT_EMPTY",
            AppError::InputTooLong { .. } => "E_INPUT_TOO_LONG",
            AppError::NoProvider { .. } => "E_NO_PROVIDER",
            AppError::KeyInvalid { .. } => "E_KEY_INVALID",
            AppError::RateLimited { .. } => "E_RATE_LIMITED",
            AppError::NetworkOffline => "E_NETWORK_OFFLINE",
            AppError::ProviderTimeout => "E_PROVIDER_TIMEOUT",
            AppError::StreamInterrupted => "E_STREAM_INTERRUPTED",
            AppError::OutputRejected { .. } => "E_OUTPUT_REJECTED",
            AppError::HumanizeFailed => "E_HUMANIZE_FAILED",
            AppError::DetectorDegraded => "E_DETECTOR_DEGRADED",
            AppError::ModelRestoreFailed => "E_MODEL_RESTORE_FAILED",
            AppError::KeyringUnavailable { .. } => "E_KEYRING_UNAVAILABLE",
            AppError::Internal { .. } => "E_INTERNAL",
        }
    }

    pub fn message_key(&self) -> &'static str {
        match self {
            AppError::InputEmpty { .. } => "error.input.empty",
            AppError::InputTooLong { .. } => "error.input.too_long",
            AppError::NoProvider { .. } => "error.provider.none",
            AppError::KeyInvalid { .. } => "error.provider.key_invalid",
            AppError::RateLimited { .. } => "error.provider.rate_limited",
            AppError::NetworkOffline => "error.provider.network_offline",
            AppError::ProviderTimeout => "error.provider.timeout",
            AppError::StreamInterrupted => "error.provider.stream_interrupted",
            AppError::OutputRejected { .. } => "error.provider.output_rejected",
            AppError::HumanizeFailed => "error.humanize.failed",
            AppError::DetectorDegraded => "notice.detector.statistical_only",
            AppError::ModelRestoreFailed => "error.detector.model_restore_failed",
            AppError::KeyringUnavailable { .. } => "error.security.keyring_unavailable",
            AppError::Internal { .. } => "error.internal",
        }
    }

    pub fn params(&self) -> Params {
        let mut params = Params::new();
        match self {
            AppError::InputEmpty { field } => {
                params.insert("field", ParamValue::Text((*field).to_owned()));
            }
            AppError::InputTooLong { limit } => {
                params.insert("limit", ParamValue::Number(i64::from(*limit)));
            }
            AppError::NoProvider { reason } => {
                params.insert("reason", ParamValue::Text((*reason).to_owned()));
            }
            AppError::KeyInvalid { provider, masked } => {
                params.insert("provider", ParamValue::Text(provider.clone()));
                params.insert("masked", ParamValue::Text(masked.clone()));
            }
            AppError::RateLimited { minutes } => {
                let value = i64::try_from(*minutes).unwrap_or(i64::MAX);
                params.insert("minutes", ParamValue::Number(value));
            }
            AppError::OutputRejected { reason } => {
                params.insert("reason", ParamValue::Text(reason.as_str().to_owned()));
            }
            AppError::KeyringUnavailable { reason } => {
                params.insert("reason", ParamValue::Text(reason.as_str().to_owned()));
            }
            AppError::Internal {
                status: Some(status),
            } => {
                params.insert("status", ParamValue::Number(i64::from(*status)));
            }
            AppError::NetworkOffline
            | AppError::ProviderTimeout
            | AppError::StreamInterrupted
            | AppError::HumanizeFailed
            | AppError::DetectorDegraded
            | AppError::ModelRestoreFailed
            | AppError::Internal { status: None } => {}
        }
        params
    }
}

/// Exactly three fields, in a fixed order, so a Tauri rejection *is* the envelope.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut envelope = serializer.serialize_map(Some(3))?;
        envelope.serialize_entry("code", self.code())?;
        envelope.serialize_entry("message_key", self.message_key())?;
        envelope.serialize_entry("params", &self.params())?;
        envelope.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn every_branch() -> Vec<AppError> {
        vec![
            AppError::InputEmpty { field: "secret" },
            AppError::InputTooLong { limit: 5000 },
            AppError::NoProvider { reason: "no_key" },
            AppError::KeyInvalid {
                provider: "provider-1".to_owned(),
                masked: "preview-a".to_owned(),
            },
            AppError::RateLimited { minutes: 2 },
            AppError::NetworkOffline,
            AppError::ProviderTimeout,
            AppError::StreamInterrupted,
            AppError::OutputRejected {
                reason: OutputRejection::Refused,
            },
            AppError::HumanizeFailed,
            AppError::DetectorDegraded,
            AppError::ModelRestoreFailed,
            AppError::KeyringUnavailable {
                reason: KeyringFault::Unavailable,
            },
            AppError::KeyringUnavailable {
                reason: KeyringFault::Unreadable,
            },
            AppError::Internal { status: Some(503) },
            AppError::Internal { status: None },
        ]
    }

    #[test]
    fn serializes_to_exactly_three_fields() -> Result<(), serde_json::Error> {
        for branch in every_branch() {
            let value = serde_json::to_value(&branch)?;
            let object = match value.as_object() {
                Some(object) => object,
                None => panic!("envelope is not an object"),
            };
            let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
            keys.sort();
            assert_eq!(
                keys,
                vec!["code", "message_key", "params"],
                "{}",
                branch.code()
            );
            assert_eq!(object.get("code"), Some(&json!(branch.code())));
            assert_eq!(
                object.get("message_key"),
                Some(&json!(branch.message_key()))
            );
            assert!(object.get("params").map(Value::is_object).unwrap_or(false));
        }
        Ok(())
    }

    #[test]
    fn params_hold_scalars_only() -> Result<(), serde_json::Error> {
        for branch in every_branch() {
            let params = serde_json::to_value(branch.params())?;
            for (name, value) in params.as_object().into_iter().flatten() {
                assert!(
                    value.is_string() || value.is_number() || value.is_boolean() || value.is_null(),
                    "{} produced a non-scalar param {name}",
                    branch.code()
                );
            }
        }
        Ok(())
    }

    #[test]
    fn input_empty_carries_the_dto_field_name() {
        let error = AppError::InputEmpty {
            field: "provider_id",
        };
        assert_eq!(error.code(), "E_INPUT_EMPTY");
        assert_eq!(
            error.params().get("field"),
            Some(&ParamValue::Text("provider_id".to_owned()))
        );
    }

    #[test]
    fn catalog_matches_the_branches() {
        let mut codes: Vec<&str> = every_branch().iter().map(AppError::code).collect();
        codes.sort_unstable();
        codes.dedup();
        let mut expected: Vec<&str> = AppError::catalog().iter().map(|(code, _)| *code).collect();
        expected.sort_unstable();
        assert_eq!(codes, expected);

        for branch in every_branch() {
            let found = AppError::catalog()
                .iter()
                .find(|(code, _)| *code == branch.code());
            assert_eq!(found.map(|(_, key)| *key), Some(branch.message_key()));
        }
    }

    #[test]
    fn catalog_keys_are_unique() {
        let mut keys: Vec<&str> = AppError::catalog().iter().map(|(_, key)| *key).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total);
    }
}
