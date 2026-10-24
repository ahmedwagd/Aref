//! Composition root and application state.
//!
//! Deliberate absences (design.md §3.4, task 1.4): no reporting or usage-tracking plugin, no
//! crash-reporting plugin, no remote-configuration fetch; no `setup` hook that performs I/O;
//! no logging backend. `tauri-plugin-sql` is **not** registered: the database is opened
//! through `storage::db`, the same module that owns the migration guard (`FR-404`), so there
//! is no plugin-command surface for the frontend to reach.
//!
//! Two constructors, and the difference is the point:
//!
//! * [`compose_app_state`] is pure — no file, no socket, no environment variable — and the
//!   slice-1 test that calls it twice asserts exactly that (`NFR-203`).
//! * [`run`] opens the database and then builds the same state with the two services, so the
//!   provider and key commands have something to serve. The bootstrap reads no credential:
//!   `OsKeyring::new` is a unit struct, and no key is touched until a key command runs.

pub mod commands;
pub mod error;
pub mod providers;
pub mod security;
pub mod status;
pub mod storage;

use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tauri::Manager;

use crate::error::AppError;
use crate::providers::client::{ReqwestTransport, Transport};
use crate::providers::keys::KeyService;
use crate::providers::ProviderService;
use crate::security::keyring::{KeyStore, OsKeyring};
use crate::status::{AppStatusService, ConstantDetectorMode, DetectorModeSource};
use crate::storage::db;
use crate::storage::providers_repo::{ProvidersRepo, SqliteProvidersRepo};
use crate::storage::StorageError;

/// Tauri-managed state.
///
/// Constructing it reads no file, opens no socket and reads no environment variable.
pub struct AppState {
    status: AppStatusService,
    /// `None` only in the pure composition seam (`compose_app_state`); the bootstrap wires
    /// both. A command that needs one reports `E_INTERNAL` rather than pretending to serve.
    providers: Option<Arc<ProviderService>>,
    keys: Option<Arc<KeyService>>,
}

impl AppState {
    pub fn status(&self) -> &AppStatusService {
        &self.status
    }

    /// The provider service, or `E_INTERNAL` when the state was composed without one.
    pub fn providers(&self) -> Result<&Arc<ProviderService>, AppError> {
        self.providers.as_ref().ok_or_else(AppError::internal)
    }

    /// The key service, or `E_INTERNAL` when the state was composed without one.
    pub fn keys(&self) -> Result<&Arc<KeyService>, AppError> {
        self.keys.as_ref().ok_or_else(AppError::internal)
    }

    /// The wired state: what [`run`] manages.
    pub fn wired(
        status: AppStatusService,
        providers: Arc<ProviderService>,
        keys: Arc<KeyService>,
    ) -> Self {
        Self {
            status,
            providers: Some(providers),
            keys: Some(keys),
        }
    }
}

/// Pure composition: values in, state out. `tests/composition_inert.rs` calls this to
/// prove the composition root has no side effect to record.
pub fn compose_app_state(mode: Arc<dyn DetectorModeSource>, app_version: &str) -> AppState {
    AppState {
        status: AppStatusService::new(mode, app_version),
        providers: None,
        keys: None,
    }
}

/// Opens the application database through `storage::db` and builds both services.
///
/// `storage::db::migrate` is the plugin's own migrator reached through this crate's guard, so
/// no plugin is registered and the backup/restore behaviour of `design.md` §8.3 still
/// surrounds the run. Nothing here reads a key, and nothing here opens a socket.
fn bootstrap() -> Result<AppState, AppError> {
    let pool = tauri::async_runtime::block_on(open_database())?;
    let repo: Arc<dyn ProvidersRepo> = Arc::new(SqliteProvidersRepo::new(pool));
    let keystore: Arc<dyn KeyStore> = Arc::new(OsKeyring::new());
    let transport: Arc<dyn Transport> = Arc::new(ReqwestTransport::new());

    let providers = Arc::new(ProviderService::new(
        Arc::clone(&repo),
        transport,
        Arc::clone(&keystore),
    ));
    let keys = Arc::new(KeyService::new(Arc::clone(&repo), keystore));
    let status = AppStatusService::with_repository(
        Arc::new(ConstantDetectorMode::statistical_only()),
        env!("CARGO_PKG_VERSION"),
        repo,
    );
    Ok(AppState::wired(status, providers, keys))
}

/// The connection pool, created through `storage::db`'s constants and migration entry point.
///
/// `create_if_missing` is set because a first run has no file; `foreign_keys` is on so the
/// schema's `REFERENCES` clause is enforced as a second line behind `remove_provider`'s
/// explicit child-first delete (`design.md` §8.1).
async fn open_database() -> Result<SqlitePool, AppError> {
    let options = SqliteConnectOptions::from_str(db::DATABASE_URL)
        .map_err(|_| AppError::internal())?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .map_err(|_| AppError::internal())?;
    db::migrate(&pool, Path::new(db::DATABASE_FILE))
        .await
        .map_err(StorageError::into_app_error)?;
    Ok(pool)
}

/// Builds the window, wires the nine commands, and runs Tauri.
///
/// The state is built *before* the event loop so a failed bootstrap is a startup failure the
/// user sees, not a silently empty Providers screen.
pub fn run() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_status,
            commands::providers::list_providers,
            commands::providers::add_provider,
            commands::providers::update_provider,
            commands::providers::remove_provider,
            commands::providers::add_key,
            commands::providers::edit_key,
            commands::providers::remove_key,
            commands::providers::probe_local_providers
        ])
        .build(tauri::generate_context!())?;

    match bootstrap() {
        Ok(state) => {
            app.manage(state);
        }
        Err(error) => {
            // The window depends on configuration the database holds, so a failed bootstrap
            // is a startup error: main.rs prints this line and no window is shown.
            return Err(std::io::Error::other(error.code()).into());
        }
    }

    app.run(|_app, _event| {});
    Ok(())
}
