//! Provider services: the request path and the key lifecycle.
//!
//! The split is by seam (design.md §3.1), so each piece is testable without the others:
//!
//! * `config` — the constants, and the probe targets (`FR-304`, `FR-307`, `FR-308`);
//! * `clock` — the injected time and jitter sources (`G-13`);
//! * `retry` — the backoff curve and `Retry-After` (`FR-307`, `FR-308`);
//! * `client` — the request shape, the `Transport` seam, and `ReqwestTransport`
//!   (`FR-301`, `FR-303`);
//! * `selection` — the pure `FR-312` order;
//! * `pool` — failover, invalidation, and the attempt sequence (`FR-302`, `FR-305`);
//! * `output` — response validation and the refusal marker (`FR-309`);
//! * `events` — the two events the request path emits (`FR-305`, `FR-307`);
//! * `discovery` — the loopback-only probe (`FR-304`, `NFR-206`);
//! * `keys` — the save/edit/remove path into the credential store (`FR-310`, `FR-313`).
//!
//! [`ProviderService`] is the only public entry for provider configuration, exactly as
//! `design.md` §3.1 draws it: the command layer calls one method and maps the result, and
//! no handler holds SQL or a branch on provider kind.

pub mod client;
pub mod clock;
pub mod config;
pub mod discovery;
pub mod events;
pub mod keys;
pub mod output;
pub mod pool;
pub mod retry;
pub mod selection;

use std::sync::Arc;

use crate::error::AppError;
use crate::providers::client::Transport;
use crate::providers::keys::Removed;
use crate::security::keyring::KeyStore;
use crate::storage::providers_repo::{
    KeyMetadata, LocalFlavor, ProviderConfig, ProviderKind, ProvidersRepo,
};
use crate::storage::{timestamp_now, StorageError};

/// One provider row with all of its key metadata rows.
///
/// A service-layer pair, not an IPC DTO: `commands::dto::ProviderView` maps it onto the
/// ratified wire shape, so `providers` never depends on the command layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSnapshot {
    pub provider: ProviderConfig,
    pub keys: Vec<KeyMetadata>,
}

/// What `add_provider` needs, already validated for presence.
///
/// The service re-checks the flavour/kind coherence the schema requires, so a caller that
/// skipped the DTO cannot turn a `CHECK` violation into `E_INTERNAL` halfway through an
/// insert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDraft {
    pub kind: ProviderKind,
    pub local_flavor: Option<LocalFlavor>,
    pub base_url: String,
    pub model_id: String,
    pub label: String,
    pub enabled: bool,
}

/// The fields `update_provider` may change.
///
/// `None` means **unchanged**, never "clear" (`design.md` §4.2): the ratified request says
/// absent or null leaves the field alone. `kind` is carried only so a client that sends it
/// can be rejected — an entry's kind decides whether it needs a key at all, so changing it
/// would silently invalidate the key policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderChanges {
    pub kind: Option<ProviderKind>,
    pub local_flavor: Option<LocalFlavor>,
    pub base_url: Option<String>,
    pub model_id: Option<String>,
    pub label: Option<String>,
    pub enabled: Option<bool>,
}

/// Provider configuration: the only public entry to the service (design.md §3.3).
pub struct ProviderService {
    repo: Arc<dyn ProvidersRepo>,
    transport: Arc<dyn Transport>,
    keystore: Arc<dyn KeyStore>,
}

impl ProviderService {
    pub fn new(
        repo: Arc<dyn ProvidersRepo>,
        transport: Arc<dyn Transport>,
        keystore: Arc<dyn KeyStore>,
    ) -> Self {
        Self {
            repo,
            transport,
            keystore,
        }
    }

    /// The transport discovery probes with.
    ///
    /// Handed out for `probe_local_providers` only: the pool owns every other use, and
    /// nothing in `get_app_status` or `selection` may touch this.
    pub fn transport(&self) -> &dyn Transport {
        self.transport.as_ref()
    }

    /// Every provider with its keys, ordered by id.
    ///
    /// Reads `masked_preview` from the metadata rows and never calls `KeyStore::read`
    /// (`G-27`, `FR-310`).
    pub async fn list_providers(&self) -> Result<Vec<ProviderSnapshot>, AppError> {
        let providers = self
            .repo
            .list_providers()
            .await
            .map_err(StorageError::into_app_error)?;
        let keys = self
            .repo
            .list_keys()
            .await
            .map_err(StorageError::into_app_error)?;
        Ok(snapshots(providers, &keys))
    }

    /// Inserts one provider and returns it with an empty key list.
    pub async fn add_provider(&self, draft: ProviderDraft) -> Result<ProviderSnapshot, AppError> {
        check_coherence(draft.kind, draft.local_flavor)?;
        let provider = ProviderConfig::new(
            keys::new_provider_id(),
            draft.kind,
            draft.local_flavor,
            draft.base_url,
            draft.model_id,
            draft.label,
            draft.enabled,
        );
        self.repo
            .insert_provider(&provider)
            .await
            .map_err(StorageError::into_app_error)?;
        Ok(ProviderSnapshot {
            provider,
            keys: Vec::new(),
        })
    }

    /// Applies the present fields and leaves the absent ones alone.
    ///
    /// An absent provider id is an internal inconsistency, not a user action: the §7
    /// catalog has no not-found code, and slice 2 already maps a vanished key that way.
    pub async fn update_provider(
        &self,
        id: &str,
        changes: ProviderChanges,
    ) -> Result<ProviderSnapshot, AppError> {
        let mut provider = self
            .repo
            .find_provider(id)
            .await
            .map_err(StorageError::into_app_error)?
            .ok_or_else(AppError::internal)?;

        // `kind` is immutable (`design.md` §4.2, ratified): a request that changes it is
        // not a shape the UI can produce, so it is an internal inconsistency rather than a
        // user-facing validation message the catalog has no code for.
        if let Some(kind) = changes.kind {
            if kind != provider.kind {
                return Err(AppError::internal());
            }
        }

        if let Some(flavor) = changes.local_flavor {
            check_coherence(provider.kind, Some(flavor))?;
            provider.local_flavor = Some(flavor);
        }
        if let Some(label) = changes.label {
            provider.label = label;
        }
        if let Some(enabled) = changes.enabled {
            provider.enabled = enabled;
        }
        if let Some(base_url) = changes.base_url {
            provider.base_url = base_url;
        }
        if let Some(model_id) = changes.model_id {
            provider.model_id = model_id;
        }
        provider.updated_at = timestamp_now();

        if !self
            .repo
            .update_provider(&provider)
            .await
            .map_err(StorageError::into_app_error)?
        {
            return Err(AppError::internal());
        }

        let keys: Vec<KeyMetadata> = self
            .repo
            .list_keys()
            .await
            .map_err(StorageError::into_app_error)?
            .into_iter()
            .filter(|key| key.provider_id == provider.id)
            .collect();
        Ok(ProviderSnapshot { provider, keys })
    }

    /// Removes a provider, its key rows, and their credentials.
    ///
    /// The credentials go first, one per key, in the order `KeyService::remove` established:
    /// entry, then row. Without this step the metadata rows would be the only handle to a
    /// stored secret, and deleting them first would leave that secret in the keyring with
    /// nothing able to name it again (`FR-310`, `NFR-201`). A store that cannot be reached
    /// leaves the configuration intact for a later repair, exactly as it does for one key.
    pub async fn remove_provider(&self, id: &str) -> Result<Removed, AppError> {
        let keys: Vec<KeyMetadata> = self
            .repo
            .list_keys()
            .await
            .map_err(StorageError::into_app_error)?
            .into_iter()
            .filter(|key| key.provider_id == id)
            .collect();
        for key in keys {
            if keys::delete_from_store(Arc::clone(&self.keystore), key.id)
                .await
                .is_err()
            {
                return Err(keys::keyring_unavailable());
            }
        }
        let removed = self
            .repo
            .remove_provider(id)
            .await
            .map_err(StorageError::into_app_error)?;
        Ok(Removed { removed })
    }
}

/// Enforces the schema's `(kind = 'local') = (local_flavor IS NOT NULL)` invariant.
///
/// `E_INPUT_EMPTY { field: "local_flavor" }` is the settled presence code (`D-1`) for a
/// local entry with no flavour; a flavour on a cloud entry has no catalog row and is the
/// same internal inconsistency as a changed kind.
fn check_coherence(kind: ProviderKind, flavor: Option<LocalFlavor>) -> Result<(), AppError> {
    match (kind, flavor) {
        (ProviderKind::Local, None) => Err(AppError::InputEmpty {
            field: "local_flavor",
        }),
        (ProviderKind::Local, Some(_)) | (ProviderKind::OpenAiCompatible, None) => Ok(()),
        (ProviderKind::OpenAiCompatible, Some(_)) => Err(AppError::internal()),
    }
}

/// Pairs every provider with its keys, keeping the repository's id order.
fn snapshots(providers: Vec<ProviderConfig>, keys: &[KeyMetadata]) -> Vec<ProviderSnapshot> {
    providers
        .into_iter()
        .map(|provider| {
            let keys = keys
                .iter()
                .filter(|key| key.provider_id == provider.id)
                .cloned()
                .collect();
            ProviderSnapshot { provider, keys }
        })
        .collect()
}
