//! Saving, editing and removing a key: the only write path into the credential store.
//!
//! `FR-310` and `FR-313` fix two things here, and both are structural rather than
//! conventional:
//!
//! * **The order is `write → read-back → metadata row`** (design.md §5.2). Writing the
//!   row first would let a failed read-back leave a "saved" claim the core cannot
//!   verify. The accepted cost is that a read-back failure can leave one inert keyring
//!   entry with no metadata row: nothing points at it, it is never logged, and `G-20`
//!   forbids deleting it.
//! * **Every `KeyStore` call goes through `spawn_blocking`.** The trait is sync because
//!   an OS credential-store call blocks, and the runtime must not be blocked with it
//!   (design.md §4.2). One wrapper per trait method, so a new call site cannot forget.
//!
//! No secret is ever logged, serialized, or returned: the only key-derived value that
//! leaves this module is the [`MaskedPreview`](crate::security::mask::MaskedPreview)
//! computed once at save time (`G-27`).

use std::sync::Arc;

use serde::Serialize;

use crate::error::{AppError, KeyringFault};
use crate::security::keyring::{BackendKind, KeyId, KeyStore, KeyStoreError};
use crate::security::mask;
use crate::security::redact::Secret;
use crate::storage::providers_repo::{KeyMetadata, KeyStatus, ProvidersRepo};
use crate::storage::{timestamp_now, StorageError};

/// The `{ removed }` response (design.md §4.2, delta 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Removed {
    pub removed: bool,
}

pub struct KeyService {
    repo: Arc<dyn ProvidersRepo>,
    keystore: Arc<dyn KeyStore>,
}

impl KeyService {
    pub fn new(repo: Arc<dyn ProvidersRepo>, keystore: Arc<dyn KeyStore>) -> Self {
        Self { repo, keystore }
    }

    /// Stores `secret` and returns its metadata.
    ///
    /// No validation of `provider_id` or `label` happens here: presence is the command
    /// layer's job (`E_INPUT_EMPTY`, `D-1`) and a repository is the wrong place to
    /// reject a blank label with a UI error code.
    pub async fn add(
        &self,
        provider_id: &str,
        label: &str,
        secret: Secret,
    ) -> Result<KeyMetadata, AppError> {
        let id = KeyId::new(new_key_id());
        // Once, on the value in hand, before it moves into the blocking call.
        let masked_preview = mask::preview(&secret);

        let secret = self.write_secret(&id, secret).await?;
        self.verify_read_back(&id, &secret).await?;

        let record = KeyMetadata {
            id,
            provider_id: provider_id.to_owned(),
            label: label.to_owned(),
            masked_preview,
            status: KeyStatus::Valid,
            last_error_code: None,
            invalid_since: None,
            updated_at: timestamp_now(),
        };
        self.repo
            .insert_key(&record)
            .await
            .map_err(StorageError::into_app_error)?;
        Ok(record)
    }

    /// Replaces the value of an existing key and re-arms it.
    ///
    /// An edit *is* a save: `FR-306`'s "editing it makes it eligible again" means the
    /// status, the invalid timestamp and the last error all clear, and the preview is
    /// derived afresh from the submitted value (`G-27`).
    pub async fn edit(&self, key_id: &KeyId, secret: Secret) -> Result<KeyMetadata, AppError> {
        let mut record = self
            .repo
            .find_key(key_id)
            .await
            .map_err(StorageError::into_app_error)?
            // The §7 catalog has no not-found code, so a key that vanished between the
            // UI's read and this call is an internal inconsistency, not a user action.
            .ok_or_else(AppError::internal)?;

        let masked_preview = mask::preview(&secret);
        let secret = self.write_secret(key_id, secret).await?;
        self.verify_read_back(key_id, &secret).await?;

        record.masked_preview = masked_preview;
        record.status = KeyStatus::Valid;
        record.invalid_since = None;
        record.last_error_code = None;
        record.updated_at = timestamp_now();

        let updated = self
            .repo
            .update_key(&record)
            .await
            .map_err(StorageError::into_app_error)?;
        if !updated {
            return Err(AppError::internal());
        }
        Ok(record)
    }

    /// Deletes the keyring entry, then the metadata row.
    ///
    /// An absent id is `{ removed: false }`: the catalog has no not-found code, and the
    /// keyring half is idempotent, so a second removal reaches the same end state.
    pub async fn remove(&self, key_id: &KeyId) -> Result<Removed, AppError> {
        if delete_from_store(Arc::clone(&self.keystore), key_id.clone())
            .await
            .is_err()
        {
            // The row is the only handle a later repair has, so it stays. A store that
            // cannot be reached at all is the `unavailable` row of §4.6; there is no
            // delete-specific reason in the catalog.
            return Err(keyring_unavailable());
        }
        let removed = self
            .repo
            .remove_key(key_id)
            .await
            .map_err(StorageError::into_app_error)?;
        Ok(Removed { removed })
    }

    /// The key metadata `list_providers` reports.
    ///
    /// Reads the stored previews and never touches the credential store: `G-27` forbids
    /// recomputing a preview by re-reading a key, and this is the path that would.
    pub async fn list_keys(&self) -> Result<Vec<KeyMetadata>, AppError> {
        self.repo
            .list_keys()
            .await
            .map_err(StorageError::into_app_error)
    }

    /// Writes the secret, mapping the store's failure to the §4.6 row.
    async fn write_secret(&self, id: &KeyId, secret: Secret) -> Result<Secret, AppError> {
        match write_to_store(Arc::clone(&self.keystore), id.clone(), secret).await {
            Ok(secret) => Ok(secret),
            // The store did not accept the write, so nothing was persisted anywhere and
            // no fallback store is attempted (`FR-313`). A `Backend` kind is a platform
            // failure on the same step, and the catalog has one row for it.
            Err(KeyStoreError::Unavailable) | Err(KeyStoreError::Backend(_)) => {
                Err(keyring_unavailable())
            }
            Err(KeyStoreError::Unreadable) => Err(keyring_unreadable()),
        }
    }

    /// Compares the value the store hands back with the one just written.
    ///
    /// Any failure — `Unreadable`, `None`, a different value, or a store that stopped
    /// answering — means the write cannot be verified, which `design.md` §5.2 maps to
    /// the `unreadable` reason. The entry is not deleted (`G-20`).
    async fn verify_read_back(&self, id: &KeyId, written: &Secret) -> Result<(), AppError> {
        let read_back = read_from_store(Arc::clone(&self.keystore), id.clone()).await;
        match read_back {
            Ok(Some(value)) if &value == written => Ok(()),
            Ok(_) | Err(_) => Err(keyring_unreadable()),
        }
    }
}

fn keyring_unavailable() -> AppError {
    AppError::KeyringUnavailable {
        reason: KeyringFault::Unavailable,
    }
}

fn keyring_unreadable() -> AppError {
    AppError::KeyringUnavailable {
        reason: KeyringFault::Unreadable,
    }
}

/// One blocking `write`, off the runtime thread.
///
/// The closure hands the [`Secret`] back instead of keeping a second copy: the type is
/// not `Clone`, and the read-back comparison needs the original value.
async fn write_to_store(
    store: Arc<dyn KeyStore>,
    id: KeyId,
    secret: Secret,
) -> Result<Secret, KeyStoreError> {
    match tokio::task::spawn_blocking(move || {
        let outcome = store.write(&id, &secret);
        (secret, outcome)
    })
    .await
    {
        Ok((secret, Ok(()))) => Ok(secret),
        Ok((_secret, Err(error))) => Err(error),
        // The blocking task panicked. The secret went with it, and nothing can be
        // claimed about the store, so it is reported as an unclassified platform
        // failure rather than as a panic on the user's path.
        Err(_join) => Err(KeyStoreError::Backend(BackendKind::Platform)),
    }
}

/// One blocking `read`, off the runtime thread.
async fn read_from_store(
    store: Arc<dyn KeyStore>,
    id: KeyId,
) -> Result<Option<Secret>, KeyStoreError> {
    match tokio::task::spawn_blocking(move || store.read(&id)).await {
        Ok(outcome) => outcome,
        Err(_join) => Err(KeyStoreError::Backend(BackendKind::Platform)),
    }
}

/// One blocking `delete`, off the runtime thread.
async fn delete_from_store(store: Arc<dyn KeyStore>, id: KeyId) -> Result<(), KeyStoreError> {
    match tokio::task::spawn_blocking(move || store.delete(&id)).await {
        Ok(outcome) => outcome,
        Err(_join) => Err(KeyStoreError::Backend(BackendKind::Platform)),
    }
}

/// A fresh, locally generated key id.
///
/// `design.md` §5.2 fixes two things: the id is generated locally, and it is fresh per
/// save so it cannot collide with an earlier keyring entry (`R-1`). The `uuid` crate is
/// not a dependency of this crate, so the 128 bits are assembled here from two
/// independently seeded `RandomState` hashers plus a monotonic counter and the clock,
/// and rendered in the canonical 8-4-4-4-12 shape. The id is an identifier, not a
/// secret, and it never leaves as anything but an opaque string.
fn new_key_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn mix(state: &RandomState, first: u64, second: u64) -> u64 {
        let mut hasher = state.build_hasher();
        first.hash(&mut hasher);
        second.hash(&mut hasher);
        hasher.finish()
    }

    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or_default();

    let high = mix(&RandomState::new(), sequence, nanos);
    let low = mix(&RandomState::new(), nanos, sequence);

    let mut bytes = ((u128::from(high) << 64) | u128::from(low)).to_be_bytes();
    // RFC 4122 version and variant bits, so the shape is one every UUID parser accepts.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let mut rendered = String::with_capacity(36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            rendered.push('-');
        }
        // Writing to a `String` cannot fail; the `Result` is discarded rather than
        // unwrapped (AGENTS.md §9).
        let _ = std::fmt::Write::write_fmt(&mut rendered, format_args!("{byte:02x}"));
    }
    rendered
}

#[cfg(test)]
mod tests {
    //! Task 2.8–2.12 and the database half of 2.14.
    //!
    //! Every test runs against the real repository on a temp-file database, not a repo
    //! double: "the preview came from SQLite" and "the database file holds no key" are
    //! claims about the real thing, and a double would make them untestable. Only the
    //! credential store is a double, because no test may touch the OS keyring
    //! (design.md §10).

    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use sqlx::SqlitePool;

    use super::*;
    use crate::security::keyring::{Fault, InMemoryKeyStore};
    use crate::storage::db::{self, test_support::TempDatabase};
    use crate::storage::providers_repo::{
        LocalFlavor, ProviderConfig, ProviderKind, SqliteProvidersRepo,
    };

    /// The fixture every `masked_preview` assertion below is written against.
    const KEY: &str = "fixture-secret-a";
    const REPLACEMENT: &str = "fixture-secret-b";

    fn ready<T, E: std::fmt::Debug>(outcome: Result<T, E>) -> T {
        match outcome {
            Ok(value) => value,
            Err(error) => panic!("fixture failed: {error:?}"),
        }
    }

    fn failure<T>(outcome: Result<T, AppError>) -> AppError {
        match outcome {
            Ok(_) => panic!("expected an error, got a value"),
            Err(error) => error,
        }
    }

    /// The `reason` the UI receives, read from the serialized envelope.
    fn reason(error: &AppError) -> String {
        let envelope = match serde_json::to_value(error) {
            Ok(value) => value,
            Err(error) => panic!("the envelope did not serialize: {error}"),
        };
        match envelope
            .get("params")
            .and_then(|params| params.get("reason"))
        {
            Some(serde_json::Value::String(value)) => value.clone(),
            other => panic!("no string reason in {other:?}"),
        }
    }

    struct Fixture {
        database: TempDatabase,
        pool: SqlitePool,
        repo: Arc<dyn ProvidersRepo>,
        service: Arc<KeyService>,
    }

    impl Fixture {
        async fn with_store(label: &str, store: Arc<dyn KeyStore>) -> Self {
            let database = TempDatabase::new(label);
            let (pool, path) = ready(database.connect().await);
            ready(db::migrate(&pool, &path).await);
            let repo: Arc<dyn ProvidersRepo> = Arc::new(SqliteProvidersRepo::new(pool.clone()));
            ready(
                repo.insert_provider(&ProviderConfig::new(
                    "provider-1",
                    ProviderKind::OpenAiCompatible,
                    None,
                    "http://127.0.0.1:1/v1",
                    "model-a",
                    "Primary",
                    true,
                ))
                .await,
            );
            let service = Arc::new(KeyService::new(Arc::clone(&repo), store));
            Self {
                database,
                pool,
                repo,
                service,
            }
        }

        async fn new(label: &str) -> Self {
            Self::with_store(label, Arc::new(InMemoryKeyStore::new())).await
        }

        async fn key_rows(&self) -> i64 {
            ready(
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM provider_keys")
                    .fetch_one(&self.pool)
                    .await,
            )
        }

        async fn stored_preview(&self, id: &KeyId) -> String {
            ready(
                sqlx::query_scalar::<_, String>(
                    "SELECT masked_preview FROM provider_keys WHERE id = ?",
                )
                .bind(id.to_string())
                .fetch_one(&self.pool)
                .await,
            )
        }
    }

    // ---------------------------------------------------------------- 2.8 add

    #[tokio::test]
    async fn add_writes_the_row_once_the_read_back_matches() {
        let store = Arc::new(InMemoryKeyStore::new());
        let fixture = Fixture::with_store("add-writable", store.clone()).await;

        let record = ready(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );

        assert_eq!(fixture.key_rows().await, 1);
        assert_eq!(record.status, KeyStatus::Valid);
        assert_eq!(record.invalid_since, None);
        assert_eq!(record.last_error_code, None);
        assert_eq!(record.masked_preview.to_string(), "••••••••et-a");
        assert_eq!(store.entry_count(), 1);
        // The returned metadata is the only thing carrying anything key-derived, and it
        // carries the mask, not the key.
        assert!(!format!("{record:?}").contains(KEY));
    }

    #[tokio::test]
    async fn add_with_an_unavailable_keyring_persists_nothing_anywhere() {
        let store = Arc::new(InMemoryKeyStore::with_fault(Fault::Unavailable));
        let fixture = Fixture::with_store("add-unavailable", store.clone()).await;

        let error = failure(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );

        assert_eq!(error.code(), "E_KEYRING_UNAVAILABLE");
        assert_eq!(reason(&error), "unavailable");
        // Nothing was persisted: no metadata row, no keyring entry, and no second store
        // to fall back to (`FR-313`).
        assert_eq!(fixture.key_rows().await, 0);
        assert_eq!(store.entry_count(), 0);
    }

    #[tokio::test]
    async fn add_with_an_unreadable_read_back_writes_no_row_and_keeps_the_entry() {
        let store = Arc::new(InMemoryKeyStore::with_fault(Fault::WriteOkReadFails));
        let fixture = Fixture::with_store("add-read-back-unreadable", store.clone()).await;

        let error = failure(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );

        assert_eq!(error.code(), "E_KEYRING_UNAVAILABLE");
        assert_eq!(reason(&error), "unreadable");
        assert_eq!(fixture.key_rows().await, 0);
        // `G-20`: a write that reads back unreadable is not undone by deleting the key.
        assert_eq!(store.entry_count(), 1);
        assert_eq!(store.delete_calls(), 0);
    }

    /// A store whose write succeeds and whose read hands back something else.
    struct ReadBackStub {
        value: Option<String>,
    }

    impl KeyStore for ReadBackStub {
        fn write(&self, _id: &KeyId, _secret: &Secret) -> Result<(), KeyStoreError> {
            Ok(())
        }

        fn read(&self, _id: &KeyId) -> Result<Option<Secret>, KeyStoreError> {
            Ok(self.value.as_deref().map(Secret::new))
        }

        fn delete(&self, _id: &KeyId) -> Result<(), KeyStoreError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn add_with_an_absent_read_back_writes_no_row() {
        let fixture =
            Fixture::with_store("add-read-back-none", Arc::new(ReadBackStub { value: None })).await;

        let error = failure(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );

        assert_eq!(error.code(), "E_KEYRING_UNAVAILABLE");
        assert_eq!(reason(&error), "unreadable");
        assert_eq!(fixture.key_rows().await, 0);
    }

    #[tokio::test]
    async fn add_with_a_different_read_back_writes_no_row() {
        let fixture = Fixture::with_store(
            "add-read-back-different",
            Arc::new(ReadBackStub {
                value: Some(REPLACEMENT.to_owned()),
            }),
        )
        .await;

        let error = failure(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );

        assert_eq!(error.code(), "E_KEYRING_UNAVAILABLE");
        assert_eq!(reason(&error), "unreadable");
        assert_eq!(fixture.key_rows().await, 0);
    }

    // ---------------------------------------------------------------- 2.9 edit

    #[tokio::test]
    async fn edit_rearms_an_invalid_key_and_derives_a_fresh_preview() {
        let store = Arc::new(InMemoryKeyStore::new());
        let fixture = Fixture::with_store("edit", store.clone()).await;
        let added = ready(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );

        // Seed the state a 401 leaves behind (3.8 owns the write path).
        let mut invalid = added.clone();
        invalid.status = KeyStatus::Invalid;
        invalid.invalid_since = Some("2026-10-24T00:00:00Z".to_owned());
        invalid.last_error_code = Some("E_KEY_INVALID".to_owned());
        assert!(ready(fixture.repo.update_key(&invalid).await));

        let edited = ready(
            fixture
                .service
                .edit(&added.id, Secret::new(REPLACEMENT))
                .await,
        );

        assert_eq!(edited.id, added.id);
        assert_eq!(edited.status, KeyStatus::Valid);
        assert_eq!(edited.invalid_since, None);
        assert_eq!(edited.last_error_code, None);
        assert!(!edited.updated_at.is_empty());
        // Freshly derived from the submitted value, not carried over from the old one.
        assert_eq!(edited.masked_preview.to_string(), "••••••••et-b");
        assert_eq!(fixture.stored_preview(&added.id).await, "••••••••et-b");
        assert_eq!(fixture.key_rows().await, 1);
        assert_eq!(store.entry_count(), 1);
    }

    // -------------------------------------------------------------- 2.10 remove

    #[tokio::test]
    async fn remove_deletes_the_entry_then_the_row_and_is_idempotent() {
        let store = Arc::new(InMemoryKeyStore::new());
        let fixture = Fixture::with_store("remove", store.clone()).await;
        let added = ready(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );

        let removed = ready(fixture.service.remove(&added.id).await);
        assert!(removed.removed);
        assert_eq!(fixture.key_rows().await, 0);
        assert_eq!(store.entry_count(), 0);

        let again = ready(fixture.service.remove(&added.id).await);
        assert!(!again.removed);
        assert_eq!(fixture.key_rows().await, 0);
    }

    // -------------------------------------------------------------- 2.12 list

    #[tokio::test]
    async fn listing_reads_the_stored_previews_and_never_the_keyring() {
        let store = Arc::new(InMemoryKeyStore::new());
        let fixture = Fixture::with_store("list", store.clone()).await;
        let first = ready(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );
        let second = ready(
            fixture
                .service
                .add("provider-1", "work", Secret::new(REPLACEMENT))
                .await,
        );
        let reads_before = store.read_calls();

        let listed = ready(fixture.service.list_keys().await);

        assert_eq!(listed.len(), 2);
        // The mechanical form of "computed once, never recomputed" (`G-27`).
        assert_eq!(store.read_calls(), reads_before);
        let first_listed = listed
            .iter()
            .find(|key| key.id == first.id)
            .unwrap_or_else(|| panic!("the first key is missing from the list"));
        assert_eq!(first_listed.masked_preview, first.masked_preview);
        assert_eq!(
            first_listed.masked_preview.to_string(),
            fixture.stored_preview(&first.id).await
        );
        let second_listed = listed
            .iter()
            .find(|key| key.id == second.id)
            .unwrap_or_else(|| panic!("the second key is missing from the list"));
        assert_eq!(second_listed.masked_preview.to_string(), "••••••••et-b");
    }

    // ---------------------------------------------------- 2.11 off the runtime

    /// A store that keeps `write` inside the blocking call until the test releases it.
    struct GatedKeyStore {
        entered: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
        release: Mutex<Receiver<()>>,
        inner: InMemoryKeyStore,
    }

    impl KeyStore for GatedKeyStore {
        fn write(&self, id: &KeyId, secret: &Secret) -> Result<(), KeyStoreError> {
            let sender = self
                .entered
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take();
            if let Some(sender) = sender {
                let _ = sender.send(());
            }
            // Bounded on purpose: with `spawn_blocking` this returns as soon as the test
            // releases it, and without `spawn_blocking` the wait ends and the assertion
            // below fails. A test may fail; it must not hang.
            let _ = self
                .release
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .recv_timeout(Duration::from_secs(2));
            self.inner.write(id, secret)
        }

        fn read(&self, id: &KeyId) -> Result<Option<Secret>, KeyStoreError> {
            self.inner.read(id)
        }

        fn delete(&self, id: &KeyId) -> Result<(), KeyStoreError> {
            self.inner.delete(id)
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_timer_completes_while_a_slow_keystore_call_is_in_flight() {
        let (entered_sender, entered) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        let fixture = Fixture::with_store(
            "spawn-blocking",
            Arc::new(GatedKeyStore {
                entered: Mutex::new(Some(entered_sender)),
                release: Mutex::new(released),
                inner: InMemoryKeyStore::new(),
            }),
        )
        .await;

        let service = Arc::clone(&fixture.service);
        let call = tokio::spawn(async move {
            service
                .add("provider-1", "personal", Secret::new(KEY))
                .await
        });

        // The store is inside `write` and has not returned: on a single-threaded
        // runtime that is only observable if the call left this thread.
        ready(entered.await);

        // A concurrent timer still gets polled while the blocking call is in flight.
        ready(
            tokio::time::timeout(
                Duration::from_secs(1),
                tokio::time::sleep(Duration::from_millis(1)),
            )
            .await,
        );
        assert!(
            !call.is_finished(),
            "the keyring call blocked the runtime thread"
        );

        ready(release.send(()).map_err(|error| format!("{error:?}")));
        let record = ready(ready(call.await).map_err(|error| error.to_string()));
        assert_eq!(record.status, KeyStatus::Valid);
    }

    // ------------------------------------------------- 2.14 nothing on disk

    #[tokio::test]
    async fn no_key_material_reaches_the_database_file() {
        let fixture = Fixture::new("no-key-material").await;
        ready(
            fixture
                .service
                .add("provider-1", "personal", Secret::new(KEY))
                .await,
        );
        let listed = ready(fixture.service.list_keys().await);
        assert_eq!(listed.len(), 1);

        // Fold the write-ahead log into the file, then close: the file bytes are what
        // an attacker with the disk would have, and the WAL is not part of this scan.
        ready(
            sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
                .execute(&fixture.pool)
                .await,
        );
        fixture.pool.close().await;

        let bytes = ready(db::test_support::database_bytes(&fixture.database.path()));
        assert!(
            !holds(&bytes, KEY.as_bytes()),
            "the fixture key is in the database file"
        );
        // Positive control: the stored preview *is* in the file, so the scan above is
        // not passing on an empty database.
        assert!(holds(
            &bytes,
            listed[0].masked_preview.to_string().as_bytes()
        ));
    }

    fn holds(haystack: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty()
            && haystack
                .windows(needle.len())
                .any(|window| window == needle)
    }

    // ------------------------------------------------- make sure the fixture is not
    // key-shaped, which is what keeps `NFR-207` satisfiable.

    #[test]
    fn the_fixture_is_not_key_shaped() {
        for fixture in [KEY, REPLACEMENT] {
            assert!(!fixture.starts_with("sk-"));
            assert!(!fixture.starts_with("sk_"));
            assert!(!fixture.contains("api"));
            assert!(fixture.starts_with("fixture-secret-"));
        }
    }

    // ------------------------------------------------------------ the local id

    #[test]
    fn a_key_id_is_fresh_and_canonically_shaped() {
        let first = new_key_id();
        let second = new_key_id();
        assert_ne!(first, second);
        assert_eq!(first.len(), 36);
        assert_eq!(&first[8..9], "-");
        assert_eq!(&first[13..14], "-");
        assert_eq!(&first[18..19], "-");
        assert_eq!(&first[23..24], "-");
        assert_eq!(&first[14..15], "4", "the version nibble");
        assert!(matches!(&first[19..20], "8" | "9" | "a" | "b"));
    }

    // A provider row is only needed as a key's parent; this asserts the fixture that
    // every test above relies on is the shape the schema accepts.
    #[test]
    fn a_local_provider_needs_a_flavour_and_a_cloud_one_must_not_have_it() {
        let cloud = ProviderConfig::new(
            "cloud",
            ProviderKind::OpenAiCompatible,
            None,
            "http://127.0.0.1:1/v1",
            "model-a",
            "Cloud",
            true,
        );
        assert_eq!(cloud.kind, ProviderKind::OpenAiCompatible);
        assert_eq!(cloud.local_flavor, None);
        let local = ProviderConfig::new(
            "local",
            ProviderKind::Local,
            Some(LocalFlavor::Ollama),
            "http://127.0.0.1:11434/v1",
            "model-b",
            "Ollama",
            true,
        );
        assert_eq!(local.local_flavor, Some(LocalFlavor::Ollama));
    }
}
