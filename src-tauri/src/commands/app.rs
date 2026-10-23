//! Thin handlers: `parse → validate → one service call → map AppError`.
//!
//! Slice 1 registers the one command that needs no storage, no keyring and no network:
//! `get_app_status`. The other eight arrive with the slices that give them a service.

use tauri::State;

use crate::commands::dto::AppStatus;
use crate::error::AppError;
use crate::AppState;

#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, AppError> {
    Ok(state.status().snapshot().into())
}
