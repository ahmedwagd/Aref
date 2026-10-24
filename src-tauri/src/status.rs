//! Application status: the aggregate `get_app_status` reports.
//!
//! `DetectorModeSource` is the seam the detector change will fill in. Until then the
//! binary bundles no classifier, so `statistical_only` is the only truthful mode
//! (design.md §4.2).
//!
//! Two ways to read the status, and the difference is deliberate:
//!
//! * [`AppStatusService::snapshot`] is the pure composition seam. It reads nothing — the
//!   slice-1 test that composes the state twice asserts exactly that — so it reports an
//!   empty inventory.
//! * [`AppStatusService::resolve`] is what the command uses. It reads the repository,
//!   calls the **same** pure `selection::candidates` the request path uses, and derives
//!   `humanize_available` and `selected_provider_id` from that one result, so the button and
//!   the run cannot disagree (`FR-212`, `FR-312`).

use std::sync::Arc;

use crate::error::AppError;
use crate::providers::selection;
use crate::storage::providers_repo::{KeyStatus, ProvidersRepo};
use crate::storage::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorMode {
    Onnx,
    StatisticalOnly,
}

impl DetectorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DetectorMode::Onnx => "onnx",
            DetectorMode::StatisticalOnly => "statistical_only",
        }
    }
}

pub trait DetectorModeSource: Send + Sync {
    fn mode(&self) -> DetectorMode;
}

/// A fixed mode, used until a real detector query exists.
#[derive(Debug, Clone, Copy)]
pub struct ConstantDetectorMode(DetectorMode);

impl ConstantDetectorMode {
    pub fn statistical_only() -> Self {
        Self(DetectorMode::StatisticalOnly)
    }
}

impl DetectorModeSource for ConstantDetectorMode {
    fn mode(&self) -> DetectorMode {
        self.0
    }
}

/// Everything `AppStatus` needs, before the DTO mapping in `commands::dto`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppStatusSnapshot {
    pub detector_mode: DetectorMode,
    pub humanize_available: bool,
    pub configured_providers: u32,
    pub enabled_providers: u32,
    pub usable_keys: u32,
    pub selected_provider_id: Option<String>,
    pub app_version: String,
}

pub struct AppStatusService {
    mode: Arc<dyn DetectorModeSource>,
    app_version: String,
    /// `None` in the pure composition seam; `Some` once `lib.rs` has opened the database.
    repo: Option<Arc<dyn ProvidersRepo>>,
}

impl AppStatusService {
    /// The composition seam: no inventory, so every count is zero.
    pub fn new(mode: Arc<dyn DetectorModeSource>, app_version: impl Into<String>) -> Self {
        Self {
            mode,
            app_version: app_version.into(),
            repo: None,
        }
    }

    /// The wired service: [`AppStatusService::resolve`] reads this repository.
    pub fn with_repository(
        mode: Arc<dyn DetectorModeSource>,
        app_version: impl Into<String>,
        repo: Arc<dyn ProvidersRepo>,
    ) -> Self {
        Self {
            mode,
            app_version: app_version.into(),
            repo: Some(repo),
        }
    }

    /// The pure view: reads nothing, so it reports no providers.
    ///
    /// Used by the composition test (`tests/composition_inert.rs`) to prove that building
    /// the state performs no I/O. The command uses [`AppStatusService::resolve`].
    pub fn snapshot(&self) -> AppStatusSnapshot {
        AppStatusSnapshot {
            detector_mode: self.mode.mode(),
            humanize_available: false,
            configured_providers: 0,
            enabled_providers: 0,
            usable_keys: 0,
            selected_provider_id: None,
            app_version: self.app_version.clone(),
        }
    }

    /// The wired view: repository counts, then one pure selection pass.
    ///
    /// No transport is involved — `selection::candidates` takes two slices — so serving this
    /// command cannot produce a network request (`FR-312`, `NFR-203`).
    pub async fn resolve(&self) -> Result<AppStatusSnapshot, AppError> {
        let Some(repo) = &self.repo else {
            return Ok(self.snapshot());
        };
        let providers = repo
            .list_providers()
            .await
            .map_err(StorageError::into_app_error)?;
        let keys = repo
            .list_keys()
            .await
            .map_err(StorageError::into_app_error)?;

        // The same function the request path uses, so `humanize_available` and the selected
        // provider are the run's own answer rather than a second opinion.
        let candidates = selection::candidates(&providers, &keys);
        let usable_keys = keys
            .iter()
            .filter(|key| {
                key.status == KeyStatus::Valid
                    && providers
                        .iter()
                        .any(|provider| provider.id == key.provider_id && provider.enabled)
            })
            .count();

        Ok(AppStatusSnapshot {
            detector_mode: self.mode.mode(),
            humanize_available: !candidates.is_empty(),
            configured_providers: count(providers.len()),
            enabled_providers: count(providers.iter().filter(|provider| provider.enabled).count()),
            usable_keys: count(usable_keys),
            selected_provider_id: candidates
                .first()
                .map(|candidate| candidate.provider.id.clone()),
            app_version: self.app_version.clone(),
        })
    }
}

/// `usize` → `u32` without a panic path: a row count that large is already broken, and the
/// saturating value keeps the envelope printable.
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reports_the_constant_mode_and_version() {
        let service =
            AppStatusService::new(Arc::new(ConstantDetectorMode::statistical_only()), "0.1.0");
        let snapshot = service.snapshot();
        assert_eq!(snapshot.detector_mode, DetectorMode::StatisticalOnly);
        assert_eq!(snapshot.detector_mode.as_str(), "statistical_only");
        assert_eq!(snapshot.app_version, "0.1.0");
        assert!(!snapshot.humanize_available);
        assert_eq!(snapshot.selected_provider_id, None);
    }

    #[tokio::test]
    async fn an_unwired_service_resolves_to_the_pure_snapshot() {
        let service =
            AppStatusService::new(Arc::new(ConstantDetectorMode::statistical_only()), "0.1.0");
        assert_eq!(
            service.resolve().await,
            Ok(service.snapshot()),
            "no repository means no inventory, and no error"
        );
    }
}
