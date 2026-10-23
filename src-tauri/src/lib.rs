//! Composition root and application state.
//!
//! Deliberate absences (design.md §3.4, task 1.4): no reporting or usage-tracking
//! plugin, no crash-reporting plugin, no remote-configuration fetch; no `setup` hook
//! that performs I/O; no logging backend. Every field of `AppState` is plain data or a
//! trait object with no I/O implementation in this slice.

pub mod commands;
pub mod error;
pub mod status;

use std::sync::Arc;

use status::{AppStatusService, ConstantDetectorMode, DetectorModeSource};

/// Tauri-managed state. Constructing it reads no file, opens no socket and reads no
/// environment variable.
pub struct AppState {
    status: AppStatusService,
}

impl AppState {
    pub fn status(&self) -> &AppStatusService {
        &self.status
    }
}

/// Pure composition: values in, state out. `tests/composition_inert.rs` calls this to
/// prove the composition root has no side effect to record.
pub fn compose_app_state(mode: Arc<dyn DetectorModeSource>, app_version: &str) -> AppState {
    AppState {
        status: AppStatusService::new(mode, app_version),
    }
}

/// Builds the window and runs Tauri.
///
/// No plugin is registered and there is no `setup` hook, so the shell has no reporting
/// surface and no remote fetch (AGENTS.md §3.2, `NFR-203`).
pub fn run() -> tauri::Result<()> {
    let state = compose_app_state(
        Arc::new(ConstantDetectorMode::statistical_only()),
        env!("CARGO_PKG_VERSION"),
    );

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![commands::app::get_app_status])
        .run(tauri::generate_context!())
}
