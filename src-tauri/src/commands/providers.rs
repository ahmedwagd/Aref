//! The provider and key command surface (SRS §6.1, `design.md` §4.2; task 3.17).
//!
//! Every handler is exactly `parse → validate → one service call → map` (`design.md` §3.2
//! rule 2): the raw body is parsed by [`parse_body`] so a missing field is
//! `E_INPUT_EMPTY { field }` rather than a Tauri deserialization string, the DTO's own
//! method performs presence validation and the mapping onto a service type, and the handler
//! makes one service call. There is no SQL here, no HTTP, and no branch on provider kind.
//!
//! `probe_local_providers` is the only caller of the discovery probe in the crate (task
//! 3.12), and it is reachable only from the UI: no startup path, no timer, and no other
//! command calls it (`FR-304`, `NFR-206`).

use tauri::ipc::Request;
use tauri::State;

use crate::commands::dto::{
    AddKeyRequest, AddProviderRequest, DiscoveredProviderView, EditKeyRequest, KeyMetadataView,
    ListProvidersResponse, ProbeLocalProvidersResponse, ProviderView, RemoveKeyRequest,
    RemoveProviderRequest, RemoveResponse, UpdateProviderRequest,
};
use crate::commands::parse::parse_body;
use crate::error::AppError;
use crate::providers::keys::Removed;
use crate::AppState;

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> Result<ListProvidersResponse, AppError> {
    let snapshots = state.providers()?.list_providers().await?;
    Ok(ListProvidersResponse {
        providers: snapshots.iter().map(ProviderView::from).collect(),
    })
}

#[tauri::command]
pub async fn add_provider(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<ProviderView, AppError> {
    let body: AddProviderRequest = parse_body(request.body())?;
    let draft = body.draft()?;
    let created = state.providers()?.add_provider(draft).await?;
    Ok(ProviderView::from(&created))
}

#[tauri::command]
pub async fn update_provider(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<ProviderView, AppError> {
    let body: UpdateProviderRequest = parse_body(request.body())?;
    let id = body.id()?;
    let changes = body.changes()?;
    let updated = state.providers()?.update_provider(&id, changes).await?;
    Ok(ProviderView::from(&updated))
}

#[tauri::command]
pub async fn remove_provider(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<RemoveResponse, AppError> {
    let body: RemoveProviderRequest = parse_body(request.body())?;
    let id = body.id()?;
    let removed = state.providers()?.remove_provider(&id).await?;
    Ok(RemoveResponse {
        removed: removed.removed,
    })
}

#[tauri::command]
pub async fn add_key(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<KeyMetadataView, AppError> {
    let body: AddKeyRequest = parse_body(request.body())?;
    let (provider_id, label, secret) = body.parts()?;
    let metadata = state.keys()?.add(&provider_id, &label, secret).await?;
    Ok(KeyMetadataView::from(&metadata))
}

#[tauri::command]
pub async fn edit_key(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<KeyMetadataView, AppError> {
    let body: EditKeyRequest = parse_body(request.body())?;
    let (key_id, secret) = body.parts()?;
    let metadata = state
        .keys()?
        .edit(&crate::security::keyring::KeyId::new(key_id), secret)
        .await?;
    Ok(KeyMetadataView::from(&metadata))
}

#[tauri::command]
pub async fn remove_key(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<RemoveResponse, AppError> {
    let body: RemoveKeyRequest = parse_body(request.body())?;
    let key_id = body.key_id()?;
    let Removed { removed } = state.keys()?.remove(&key_id).await?;
    Ok(RemoveResponse { removed })
}

/// Probes the two loopback ports and reports the hits, in `PROBE_TARGETS` order.
///
/// The one discovery call site in the crate. A probe creates, enables, and selects nothing:
/// adding a discovered entry stays a separate `add_provider` action, which keeps the *effect*
/// user-initiated as well as the probe (`FR-304`).
#[tauri::command]
pub async fn probe_local_providers(
    state: State<'_, AppState>,
) -> Result<ProbeLocalProvidersResponse, AppError> {
    let found = crate::providers::discovery::probe(state.providers()?.transport()).await;
    Ok(ProbeLocalProvidersResponse {
        found: found.iter().map(DiscoveredProviderView::from).collect(),
    })
}
