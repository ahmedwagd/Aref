//! Request and response DTOs, mirroring `src/ipc/types.ts` field for field.
//!
//! Field names are `snake_case` on the wire with no `serde(rename_all)` anywhere
//! (design.md §4.1), so the two sides are literal mirrors.
//!
//! Slice 1 needs one DTO pair (`AppStatus`) and one ratified request (`AddKeyRequest`),
//! which is what the parse layer's per-field check is proved against. The remaining
//! seven request/response types land with the commands that use them.

use serde::{Deserialize, Serialize};

use crate::commands::parse::required_text;
use crate::error::AppError;
use crate::status::AppStatusSnapshot;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderSummary {
    pub configured: u32,
    pub enabled: u32,
    pub usable_keys: u32,
    pub selected_provider_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppStatus {
    pub detector_mode: &'static str,
    pub humanize_available: bool,
    pub provider_summary: ProviderSummary,
    pub app_version: String,
}

impl From<AppStatusSnapshot> for AppStatus {
    fn from(snapshot: AppStatusSnapshot) -> Self {
        Self {
            detector_mode: snapshot.detector_mode.as_str(),
            humanize_available: snapshot.humanize_available,
            provider_summary: ProviderSummary {
                configured: snapshot.configured_providers,
                enabled: snapshot.enabled_providers,
                usable_keys: snapshot.usable_keys,
                selected_provider_id: snapshot.selected_provider_id,
            },
            app_version: snapshot.app_version,
        }
    }
}

/// The ratified `add_key` request (design.md §4.2).
///
/// Every field is `Option<T>` so a missing field and a wrongly typed one stay two
/// separately testable conditions (design.md §4.5).
///
/// No `Debug` derive: the struct holds a secret, and a `Debug` line is exactly how key
/// material reaches a log (`NFR-201`, AGENTS.md §3.3).
#[derive(Clone, Deserialize)]
pub struct AddKeyRequest {
    pub provider_id: Option<String>,
    pub label: Option<String>,
    pub secret: Option<String>,
}

impl AddKeyRequest {
    /// The DTO field names, in the order the UI presents them.
    pub const REQUIRED_FIELDS: [&'static str; 3] = ["provider_id", "label", "secret"];

    /// Presence only. Type errors already failed in `parse_body`, and no key is
    /// rejected on length grounds (design.md §13 item 7).
    pub fn validate(&self) -> Result<(), AppError> {
        required_text("provider_id", self.provider_id.as_deref())?;
        required_text("label", self.label.as_deref())?;
        required_text("secret", self.secret.as_deref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{AppStatusService, ConstantDetectorMode};
    use std::sync::Arc;

    #[test]
    fn app_status_maps_every_snapshot_field() {
        let service =
            AppStatusService::new(Arc::new(ConstantDetectorMode::statistical_only()), "0.1.0");
        let dto = AppStatus::from(service.snapshot());
        assert_eq!(dto.detector_mode, "statistical_only");
        assert!(!dto.humanize_available);
        assert_eq!(dto.provider_summary.configured, 0);
        assert_eq!(dto.provider_summary.enabled, 0);
        assert_eq!(dto.provider_summary.usable_keys, 0);
        assert_eq!(dto.provider_summary.selected_provider_id, None);
        assert_eq!(dto.app_version, "0.1.0");
    }

    #[test]
    fn add_key_validation_names_the_dto_fields() {
        let complete = AddKeyRequest {
            provider_id: Some("provider-1".to_owned()),
            label: Some("personal".to_owned()),
            secret: Some("fixture-secret-a".to_owned()),
        };
        assert!(complete.validate().is_ok());
        assert_eq!(
            AddKeyRequest::REQUIRED_FIELDS,
            ["provider_id", "label", "secret"]
        );
    }

    #[test]
    fn app_status_serializes_with_snake_case_field_names() -> Result<(), serde_json::Error> {
        let service =
            AppStatusService::new(Arc::new(ConstantDetectorMode::statistical_only()), "0.1.0");
        let value = serde_json::to_value(AppStatus::from(service.snapshot()))?;
        let object = match value.as_object() {
            Some(object) => object,
            None => panic!("AppStatus is not an object"),
        };
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "app_version",
                "detector_mode",
                "humanize_available",
                "provider_summary"
            ]
        );
        Ok(())
    }
}
