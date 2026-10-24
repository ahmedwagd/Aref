//! Request and response DTOs, mirroring `src/ipc/types.ts` field for field.
//!
//! Field names are `snake_case` on the wire with no `serde(rename_all)` anywhere
//! (design.md §4.1), so the two sides are literal mirrors.
//!
//! Every request field is `Option<T>`: a **missing** field is `E_INPUT_EMPTY { field }`, a
//! **wrongly typed** field fails in `parse_body` as `E_INTERNAL`, and an absent value in
//! `update_provider` means "unchanged" (design.md §4.5, §4.2). Presence validation and the
//! mapping onto service types live here so each handler stays `parse → validate → one
//! service call → map`.
//!
//! No request or response type holds key material: the secret travels as the submitted text
//! and is converted by the service layer (`tests/redaction_boundary.rs`).

use serde::{Deserialize, Serialize};

use crate::commands::parse::required_text;
use crate::error::AppError;
use crate::providers::discovery::DiscoveredProvider;
use crate::providers::{ProviderChanges, ProviderDraft, ProviderSnapshot};
use crate::status::AppStatusSnapshot;
use crate::storage::providers_repo::{KeyMetadata, KeyStatus, LocalFlavor, ProviderKind};

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

    /// `(provider_id, label, secret)`, all presence-checked.
    ///
    /// The secret travels as the submitted text and is converted by the service layer, so no
    /// DTO holds key material (`tests/redaction_boundary.rs`).
    pub fn parts(&self) -> Result<(String, String, String), AppError> {
        Ok((
            required_text("provider_id", self.provider_id.as_deref())?,
            required_text("label", self.label.as_deref())?,
            required_text("secret", self.secret.as_deref())?,
        ))
    }

    /// Presence only. Type errors already failed in `parse_body`, and no key is
    /// rejected on length grounds (design.md §13 item 7).
    pub fn validate(&self) -> Result<(), AppError> {
        self.parts().map(|_| ())
    }
}

// ------------------------------------------------------------ provider DTOs

/// The wire view of a provider (design.md §4.2, deltas 1 and 2).
///
/// `id` is delta 1: `update_provider` and `remove_provider` take it. `keys` is delta 2: one
/// DTO for all three provider responses, so the UI can replace a row without a second
/// `list_providers` round trip. Empty array on add, never null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderView {
    pub id: String,
    pub kind: &'static str,
    pub local_flavor: Option<&'static str>,
    pub base_url: String,
    pub model_id: String,
    pub label: String,
    pub enabled: bool,
    pub keys: Vec<KeyMetadataView>,
}

impl From<&ProviderSnapshot> for ProviderView {
    fn from(snapshot: &ProviderSnapshot) -> Self {
        let provider = &snapshot.provider;
        Self {
            id: provider.id.clone(),
            kind: kind_str(provider.kind),
            local_flavor: provider.local_flavor.map(flavor_str),
            base_url: provider.base_url.clone(),
            model_id: provider.model_id.clone(),
            label: provider.label.clone(),
            enabled: provider.enabled,
            keys: snapshot.keys.iter().map(KeyMetadataView::from).collect(),
        }
    }
}

/// The wire view of one key row: the §5.3 metadata, and no secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyMetadataView {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub masked_preview: String,
    pub status: &'static str,
    pub last_error_code: Option<String>,
    pub invalid_since: Option<String>,
    pub updated_at: String,
}

impl From<&KeyMetadata> for KeyMetadataView {
    fn from(key: &KeyMetadata) -> Self {
        Self {
            id: key.id.to_string(),
            provider_id: key.provider_id.clone(),
            label: key.label.clone(),
            masked_preview: key.masked_preview.to_string(),
            status: status_str(key.status),
            last_error_code: key.last_error_code.clone(),
            invalid_since: key.invalid_since.clone(),
            updated_at: key.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListProvidersResponse {
    pub providers: Vec<ProviderView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RemoveResponse {
    pub removed: bool,
}

/// One discovery hit (design.md §4.2). `kind` is always `local`: discovery finds local
/// servers only, and the flavour comes from the port, never from the answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveredProviderView {
    pub kind: &'static str,
    pub local_flavor: &'static str,
    pub base_url: &'static str,
    pub port: u16,
}

impl From<&DiscoveredProvider> for DiscoveredProviderView {
    fn from(provider: &DiscoveredProvider) -> Self {
        Self {
            kind: "local",
            local_flavor: flavor_str(provider.flavor),
            base_url: provider.base_url,
            port: provider.port,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProbeLocalProvidersResponse {
    pub found: Vec<DiscoveredProviderView>,
}

fn kind_str(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::OpenAiCompatible => "openai_compatible",
        ProviderKind::Local => "local",
    }
}

fn flavor_str(flavor: LocalFlavor) -> &'static str {
    match flavor {
        LocalFlavor::OpenCode => "opencode",
        LocalFlavor::Ollama => "ollama",
        LocalFlavor::LmStudio => "lm_studio",
    }
}

fn status_str(status: KeyStatus) -> &'static str {
    match status {
        KeyStatus::Valid => "valid",
        KeyStatus::Invalid => "invalid",
    }
}

/// An unknown discriminator is a client that bypassed the ratified shape, so it is
/// `E_INTERNAL` — the same reading slice 2 applies to a key that vanished between the UI's
/// read and the command.
fn provider_kind(value: &str) -> Result<ProviderKind, AppError> {
    match value {
        "openai_compatible" => Ok(ProviderKind::OpenAiCompatible),
        "local" => Ok(ProviderKind::Local),
        _ => Err(AppError::internal()),
    }
}

fn local_flavor(value: &str) -> Result<LocalFlavor, AppError> {
    match value {
        "opencode" => Ok(LocalFlavor::OpenCode),
        "ollama" => Ok(LocalFlavor::Ollama),
        "lm_studio" => Ok(LocalFlavor::LmStudio),
        _ => Err(AppError::internal()),
    }
}

/// An optional text field: absent is `None` (unchanged), present-but-blank is a presence
/// failure named the same way a required field is.
fn optional_text(field: &'static str, value: Option<&str>) -> Result<Option<String>, AppError> {
    match value {
        Some(text) => Ok(Some(required_text(field, Some(text))?)),
        None => Ok(None),
    }
}

fn optional_kind(value: Option<&str>) -> Result<Option<ProviderKind>, AppError> {
    match value {
        Some(text) => Ok(Some(provider_kind(&required_text("kind", Some(text))?)?)),
        None => Ok(None),
    }
}

fn optional_flavor(value: Option<&str>) -> Result<Option<LocalFlavor>, AppError> {
    match value {
        Some(text) => Ok(Some(local_flavor(&required_text(
            "local_flavor",
            Some(text),
        )?)?)),
        None => Ok(None),
    }
}

/// The ratified `add_provider` request (design.md §4.2).
#[derive(Clone, Deserialize)]
pub struct AddProviderRequest {
    pub kind: Option<String>,
    pub local_flavor: Option<String>,
    pub base_url: Option<String>,
    pub model_id: Option<String>,
    pub label: Option<String>,
    pub enabled: Option<bool>,
}

impl AddProviderRequest {
    /// Presence validation and the mapping onto the service draft, in one step.
    ///
    /// The flavour/kind coherence is the schema's `CHECK`, checked here so the failure can
    /// name `local_flavor` instead of surfacing as a storage error.
    pub fn draft(&self) -> Result<ProviderDraft, AppError> {
        let kind_text = required_text("kind", self.kind.as_deref())?;
        let kind = provider_kind(&kind_text)?;
        let local_flavor = optional_flavor(self.local_flavor.as_deref())?;
        match (kind, local_flavor) {
            (ProviderKind::Local, None) => {
                return Err(AppError::InputEmpty {
                    field: "local_flavor",
                })
            }
            (ProviderKind::OpenAiCompatible, Some(_)) => return Err(AppError::internal()),
            _ => {}
        }
        Ok(ProviderDraft {
            kind,
            local_flavor,
            base_url: required_text("base_url", self.base_url.as_deref())?,
            model_id: required_text("model_id", self.model_id.as_deref())?,
            label: required_text("label", self.label.as_deref())?,
            enabled: self
                .enabled
                .ok_or(AppError::InputEmpty { field: "enabled" })?,
        })
    }
}

/// The ratified `update_provider` request (design.md §4.2).
///
/// `kind` is not in the ratified field list, but it is accepted here purely so a client that
/// sends it can be rejected: the service refuses any value that differs from the stored
/// kind, because an entry's kind decides whether it needs a key at all.
#[derive(Clone, Deserialize)]
pub struct UpdateProviderRequest {
    pub id: Option<String>,
    pub kind: Option<String>,
    pub local_flavor: Option<String>,
    pub base_url: Option<String>,
    pub model_id: Option<String>,
    pub label: Option<String>,
    pub enabled: Option<bool>,
}

impl UpdateProviderRequest {
    pub fn id(&self) -> Result<String, AppError> {
        required_text("id", self.id.as_deref())
    }

    /// Every present field, with absent meaning unchanged.
    pub fn changes(&self) -> Result<ProviderChanges, AppError> {
        Ok(ProviderChanges {
            kind: optional_kind(self.kind.as_deref())?,
            local_flavor: optional_flavor(self.local_flavor.as_deref())?,
            base_url: optional_text("base_url", self.base_url.as_deref())?,
            model_id: optional_text("model_id", self.model_id.as_deref())?,
            label: optional_text("label", self.label.as_deref())?,
            enabled: self.enabled,
        })
    }
}

#[derive(Clone, Deserialize)]
pub struct RemoveProviderRequest {
    pub id: Option<String>,
}

impl RemoveProviderRequest {
    pub fn id(&self) -> Result<String, AppError> {
        required_text("id", self.id.as_deref())
    }
}

/// The ratified `edit_key` request: a secret, never echoed (design.md §4.2).
#[derive(Clone, Deserialize)]
pub struct EditKeyRequest {
    pub key_id: Option<String>,
    pub secret: Option<String>,
}

impl EditKeyRequest {
    /// `(key_id, secret)`, both presence-checked. The secret is returned as submitted: a
    /// key is never rejected on length grounds (`design.md` §13 item 7).
    pub fn parts(&self) -> Result<(String, String), AppError> {
        Ok((
            required_text("key_id", self.key_id.as_deref())?,
            required_text("secret", self.secret.as_deref())?,
        ))
    }
}

#[derive(Clone, Deserialize)]
pub struct RemoveKeyRequest {
    pub key_id: Option<String>,
}

impl RemoveKeyRequest {
    pub fn key_id(&self) -> Result<crate::security::keyring::KeyId, AppError> {
        Ok(crate::security::keyring::KeyId::new(required_text(
            "key_id",
            self.key_id.as_deref(),
        )?))
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
