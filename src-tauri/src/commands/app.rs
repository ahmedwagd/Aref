//! Thin handlers: `parse → validate → one service call → map AppError`.
//!
//! `get_app_status` is the one command that needs no keyring and no network: it reads the
//! repository through the status service and calls the same pure `selection::candidates` the
//! request path uses, so `humanize_available` and the selected provider cannot disagree with
//! what a run would do (`FR-212`, `FR-312`).

use tauri::State;

use crate::commands::dto::AppStatus;
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, AppError> {
    Ok(state.status().resolve().await?.into())
}
