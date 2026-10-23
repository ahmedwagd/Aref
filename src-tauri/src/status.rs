//! Application status: the aggregate `get_app_status` reports.
//!
//! `DetectorModeSource` is the seam the detector change will fill in. Until then the
//! binary bundles no classifier, so `statistical_only` is the only truthful mode
//! (design.md §4.2).

use std::sync::Arc;

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
}

impl AppStatusService {
    pub fn new(mode: Arc<dyn DetectorModeSource>, app_version: impl Into<String>) -> Self {
        Self {
            mode,
            app_version: app_version.into(),
        }
    }

    pub fn snapshot(&self) -> AppStatusSnapshot {
        AppStatusSnapshot {
            detector_mode: self.mode.mode(),
            // Humanize needs a usable provider. Slice 1 stores none, so the truthful
            // answer is "not available" and the UI shows the Settings guidance.
            humanize_available: false,
            configured_providers: 0,
            enabled_providers: 0,
            usable_keys: 0,
            selected_provider_id: None,
            app_version: self.app_version.clone(),
        }
    }
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
}
