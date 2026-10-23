//! The `KeyStore` seam and the OS credential-store implementation (`FR-310`, `FR-313`).
//!
//! The trait is **sync**: an OS credential-store call blocks, so the *caller* adapts with
//! `tokio::task::spawn_blocking` (design.md §4.2) instead of every implementation and
//! every double becoming async. That is also what keeps the doubles trivial.
//!
//! The entry name is derived, never stored: [`KEYRING_SERVICE`] plus `key:{id}`
//! (design.md §5.1). Storing it would add a column that `design.md` §8.2 forbids and a
//! second source of truth for a one-way door: renaming an entry orphans every stored
//! secret, and nothing may read them to migrate (`R-1`).
//!
//! [`KeyStoreError::Backend`] carries a *kind*, never the platform's message, because a
//! platform message is the one place an OS could echo entry data back into a log.

use std::fmt;

use crate::security::redact::Secret;

/// The service name every entry is filed under (design.md §6.5).
pub const KEYRING_SERVICE: &str = "aref";

/// The `KeyMetadata.id` of a stored key.
///
/// Not secret: an id is a local identifier that appears in `params` and in records
/// (design.md §3.4, §4.1). It is the only input an entry name is built from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(String);

impl KeyId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Why a credential-store operation failed (design.md §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStoreError {
    /// No OS credential store, access denied, or the store is locked (`FR-313`).
    Unavailable,
    /// An entry exists but its value cannot be obtained — a *skip*, not an error the
    /// caller reports, and never a reason to mark a key invalid (`G-20`).
    Unreadable,
    /// A platform failure, classified. See [`BackendKind`].
    Backend(BackendKind),
}

/// The classification of a platform failure.
///
/// Every variant is fieldless on purpose: the shape makes "carries no platform message
/// string" a property of the type rather than a convention, and
/// `tests/`/unit tests assert the size so a payload cannot be added quietly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// The underlying platform storage reported a failure it did not classify.
    Platform,
    /// The store does not support the requested operation.
    Unsupported,
    /// A credential attribute or value the store cannot represent.
    Format,
}

/// Stores key material and nothing else.
///
/// `Write`/`read`/`delete` only: there is no enumeration, no bulk operation, and no
/// method that returns every entry, so a caller cannot walk the store.
pub trait KeyStore: Send + Sync {
    fn write(&self, id: &KeyId, secret: &Secret) -> Result<(), KeyStoreError>;
    fn read(&self, id: &KeyId) -> Result<Option<Secret>, KeyStoreError>;
    fn delete(&self, id: &KeyId) -> Result<(), KeyStoreError>;
}

/// The entry name for a key id.
///
/// One format string, one place: this is the derivation `R-1` pins. It is exposed for
/// tests and for nothing else in the crate.
pub fn entry_name(id: &KeyId) -> String {
    format!("key:{id}")
}

/// The OS credential store — Windows Credential Manager on this platform.
///
/// A unit struct, so no entry name can be cached in a field (design.md §5.1): every
/// operation derives the name from the id it is handed.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsKeyring;

impl OsKeyring {
    pub fn new() -> Self {
        Self
    }
}

impl KeyStore for OsKeyring {
    fn write(&self, id: &KeyId, secret: &Secret) -> Result<(), KeyStoreError> {
        let entry = open_entry(id)?;
        entry
            .set_password(secret.expose_secret())
            .map_err(classify_store_error)
    }

    fn read(&self, id: &KeyId) -> Result<Option<Secret>, KeyStoreError> {
        let entry = open_entry(id)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(Secret::new(value))),
            // Absent is a value, not a failure: the caller decides whether that is a
            // problem, and `G-20` treats it as a skip.
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(classify_read_error(error)),
        }
    }

    fn delete(&self, id: &KeyId) -> Result<(), KeyStoreError> {
        let entry = open_entry(id)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            // Removing an absent entry already reaches the desired end state, so this
            // stays idempotent (design.md §4.2).
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(classify_store_error(error)),
        }
    }
}

fn open_entry(id: &KeyId) -> Result<keyring::Entry, KeyStoreError> {
    keyring::Entry::new(KEYRING_SERVICE, &entry_name(id)).map_err(classify_read_error)
}

/// Maps a write/delete failure. `NoEntry` is handled by the caller.
///
/// `keyring::Error` is `#[non_exhaustive]`, so the final arm is the landing place for
/// both `PlatformFailure` and any variant a newer `keyring` adds: an unclassified
/// platform failure, never a panic.
fn classify_store_error(error: keyring::Error) -> KeyStoreError {
    use keyring::Error;
    match error {
        Error::NoStorageAccess(_) | Error::NoDefaultStore => KeyStoreError::Unavailable,
        Error::TooLong(_, _) | Error::Invalid(_, _) => KeyStoreError::Backend(BackendKind::Format),
        Error::NotSupportedByStore(_) => KeyStoreError::Backend(BackendKind::Unsupported),
        _ => KeyStoreError::Backend(BackendKind::Platform),
    }
}

/// Maps a read failure. `NoEntry` is turned into `Ok(None)` before this runs.
///
/// A value the store cannot decode is `Unreadable`, which is the state `G-20` says to
/// skip without touching the key's status.
fn classify_read_error(error: keyring::Error) -> KeyStoreError {
    use keyring::Error;
    match error {
        Error::NoStorageAccess(_) | Error::NoDefaultStore => KeyStoreError::Unavailable,
        Error::NoEntry
        | Error::BadEncoding(_)
        | Error::BadDataFormat(_, _)
        | Error::BadStoreFormat(_)
        | Error::Ambiguous(_) => KeyStoreError::Unreadable,
        Error::TooLong(_, _) | Error::Invalid(_, _) => KeyStoreError::Backend(BackendKind::Format),
        Error::NotSupportedByStore(_) => KeyStoreError::Backend(BackendKind::Unsupported),
        _ => KeyStoreError::Backend(BackendKind::Platform),
    }
}

#[cfg(any(test, feature = "test-support"))]
use std::collections::HashMap;
#[cfg(any(test, feature = "test-support"))]
use std::sync::{Mutex, MutexGuard};

/// How the double misbehaves, when it does (design.md §5.5).
///
/// Every variant is deterministic: there is no sleep, no clock, and no random draw, so
/// a failure sequence is reproduced exactly by running the test again.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Every operation succeeds.
    None,
    /// The credential store cannot be reached at all (`FR-313`).
    Unavailable,
    /// The store is present but refuses to return or accept values.
    Unreadable,
    /// `write` reports success and the value is stored, but every `read` fails — the
    /// "write that reads back unreadable" of `G-20`, which is not undone by a delete.
    WriteOkReadFails,
    /// The first `n` reads succeed, then every read fails. `FailAfter(2)` is readable,
    /// readable, unreadable.
    FailAfter(u32),
}

#[cfg(any(test, feature = "test-support"))]
struct InMemoryState {
    entries: HashMap<KeyId, String>,
    reads: u32,
    deletes: u32,
}

/// A credential store double (`FR-310`, `FR-313`; design.md §5.5).
///
/// Behind `cfg(test)`/`test-support` so no default-feature build contains it — release
/// included, because a release build never sets `cfg(test)`. It also counts `read`
/// calls, which is what makes "listing keys never re-reads the keyring" a mechanical
/// assertion rather than a convention.
#[cfg(any(test, feature = "test-support"))]
pub struct InMemoryKeyStore {
    state: Mutex<InMemoryState>,
    fault: Fault,
}

#[cfg(any(test, feature = "test-support"))]
impl InMemoryKeyStore {
    pub fn new() -> Self {
        Self::with_fault(Fault::None)
    }

    pub fn with_fault(fault: Fault) -> Self {
        Self {
            state: Mutex::new(InMemoryState {
                entries: HashMap::new(),
                reads: 0,
                deletes: 0,
            }),
            fault,
        }
    }

    /// Puts an entry in the store as though a previous save had written it.
    pub fn seed(&self, id: KeyId, secret: &Secret) {
        self.lock()
            .entries
            .insert(id, secret.expose_secret().to_owned());
    }

    /// Whether an entry is present — asserted after a read-back failure, which must not
    /// delete it (`G-20`).
    pub fn contains(&self, id: &KeyId) -> bool {
        self.lock().entries.contains_key(id)
    }

    /// How many entries the double holds.
    ///
    /// Used where the assertion is about absence: "nothing was persisted anywhere"
    /// is a count of zero, not a list of ids the test never learned.
    pub fn entry_count(&self) -> usize {
        self.lock().entries.len()
    }

    /// How many `read` calls the store has served, including failed ones.
    pub fn read_calls(&self) -> u32 {
        self.lock().reads
    }

    /// How many `delete` calls the store has received.
    pub fn delete_calls(&self) -> u32 {
        self.lock().deletes
    }

    fn lock(&self) -> MutexGuard<'_, InMemoryState> {
        // No `unwrap` (AGENTS.md §9): every critical section here is a single map or
        // counter operation, so a panic in one cannot leave the state inconsistent and
        // the inner guard is taken as-is.
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Default for InMemoryKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl KeyStore for InMemoryKeyStore {
    fn write(&self, id: &KeyId, secret: &Secret) -> Result<(), KeyStoreError> {
        let mut state = self.lock();
        match self.fault {
            Fault::Unavailable => Err(KeyStoreError::Unavailable),
            Fault::Unreadable => Err(KeyStoreError::Unreadable),
            Fault::None | Fault::WriteOkReadFails | Fault::FailAfter(_) => {
                state
                    .entries
                    .insert(id.clone(), secret.expose_secret().to_owned());
                Ok(())
            }
        }
    }

    fn read(&self, id: &KeyId) -> Result<Option<Secret>, KeyStoreError> {
        let mut state = self.lock();
        let attempt = state.reads;
        state.reads += 1;
        match self.fault {
            Fault::Unavailable => Err(KeyStoreError::Unavailable),
            Fault::Unreadable | Fault::WriteOkReadFails => Err(KeyStoreError::Unreadable),
            Fault::FailAfter(readable_reads) if attempt >= readable_reads => {
                Err(KeyStoreError::Unreadable)
            }
            Fault::None | Fault::FailAfter(_) => Ok(state
                .entries
                .get(id)
                .map(|value| Secret::new(value.clone()))),
        }
    }

    fn delete(&self, id: &KeyId) -> Result<(), KeyStoreError> {
        let mut state = self.lock();
        state.deletes += 1;
        match self.fault {
            Fault::Unavailable => Err(KeyStoreError::Unavailable),
            Fault::Unreadable => Err(KeyStoreError::Unreadable),
            Fault::None | Fault::WriteOkReadFails | Fault::FailAfter(_) => {
                state.entries.remove(id);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn the_service_name_is_the_one_the_one_way_door_pins() {
        assert_eq!(KEYRING_SERVICE, "aref");
    }

    #[test]
    fn the_entry_name_is_derived_from_the_id() {
        let id = KeyId::new("6f1c0d2e-0a3b-4c1d-9e8f-000000000001");
        assert_eq!(entry_name(&id), "key:6f1c0d2e-0a3b-4c1d-9e8f-000000000001");
        assert_eq!(entry_name(&KeyId::new("key-a")), "key:key-a");
        // Derived per call from the id alone: a different id cannot share a name.
        assert_ne!(
            entry_name(&KeyId::new("key-a")),
            entry_name(&KeyId::new("key-b"))
        );
    }

    #[test]
    fn os_keyring_holds_no_per_entry_state() {
        // A zero-sized struct cannot have cached an entry name in a field, which is the
        // struct half of "derived, not stored". The column half belongs to the schema
        // (`design.md` §8.2) and is asserted where the schema is.
        assert_eq!(std::mem::size_of::<OsKeyring>(), 0);
    }

    #[test]
    fn a_backend_failure_carries_a_kind_and_no_payload() {
        // Fieldless by shape, so the enum cannot hold a platform message string.
        assert_eq!(std::mem::size_of::<BackendKind>(), 1);
        assert_eq!(
            format!("{:?}", KeyStoreError::Backend(BackendKind::Unsupported)),
            "Backend(Unsupported)"
        );
        assert_eq!(
            format!("{:?}", KeyStoreError::Backend(BackendKind::Platform)),
            "Backend(Platform)"
        );
    }

    #[test]
    fn the_trait_is_usable_as_a_trait_object() {
        // Constructing either store performs no I/O, so this test never touches the OS
        // keyring (design.md §10).
        let stores: [Arc<dyn KeyStore>; 2] = [
            Arc::new(OsKeyring::new()),
            Arc::new(InMemoryKeyStore::new()),
        ];
        assert_eq!(stores.len(), 2);
    }

    #[test]
    fn the_double_round_trips_through_the_trait_object() -> Result<(), KeyStoreError> {
        let store: Arc<dyn KeyStore> = Arc::new(InMemoryKeyStore::new());
        let id = KeyId::new("key-a");
        store.write(&id, &Secret::new("fixture-secret-a"))?;
        assert_eq!(store.read(&id)?, Some(Secret::new("fixture-secret-a")));
        store.delete(&id)?;
        assert_eq!(store.read(&id)?, None);
        Ok(())
    }

    #[test]
    fn the_double_counts_reads() -> Result<(), KeyStoreError> {
        let store = InMemoryKeyStore::new();
        let id = KeyId::new("key-a");
        store.seed(id.clone(), &Secret::new("fixture-secret-a"));
        assert_eq!(store.read_calls(), 0);
        let _ = store.read(&id)?;
        let _ = store.read(&id)?;
        assert_eq!(store.read_calls(), 2);
        Ok(())
    }

    #[test]
    fn fail_after_two_is_readable_readable_unreadable() -> Result<(), KeyStoreError> {
        let store = InMemoryKeyStore::with_fault(Fault::FailAfter(2));
        let id = KeyId::new("key-a");
        store.seed(id.clone(), &Secret::new("fixture-secret-a"));

        assert_eq!(store.read(&id)?, Some(Secret::new("fixture-secret-a")));
        assert_eq!(store.read(&id)?, Some(Secret::new("fixture-secret-a")));
        assert_eq!(store.read(&id), Err(KeyStoreError::Unreadable));
        assert_eq!(store.read_calls(), 3);
        // The entry is still there: an unreadable read never deletes (`G-20`).
        assert!(store.contains(&id));
        Ok(())
    }

    #[test]
    fn write_ok_read_fails_stores_the_entry_and_never_deletes_it() -> Result<(), KeyStoreError> {
        let store = InMemoryKeyStore::with_fault(Fault::WriteOkReadFails);
        let id = KeyId::new("key-a");
        store.write(&id, &Secret::new("fixture-secret-a"))?;
        assert_eq!(store.read(&id), Err(KeyStoreError::Unreadable));
        assert!(store.contains(&id));
        assert_eq!(store.delete_calls(), 0);
        Ok(())
    }

    #[test]
    fn an_unavailable_store_stores_nothing() -> Result<(), KeyStoreError> {
        let store = InMemoryKeyStore::with_fault(Fault::Unavailable);
        let id = KeyId::new("key-a");
        assert_eq!(
            store.write(&id, &Secret::new("fixture-secret-a")),
            Err(KeyStoreError::Unavailable)
        );
        assert_eq!(store.read(&id), Err(KeyStoreError::Unavailable));
        assert!(!store.contains(&id));
        Ok(())
    }

    #[test]
    fn an_unreadable_store_accepts_nothing_and_returns_nothing() {
        let store = InMemoryKeyStore::with_fault(Fault::Unreadable);
        let id = KeyId::new("key-a");
        assert_eq!(
            store.write(&id, &Secret::new("fixture-secret-a")),
            Err(KeyStoreError::Unreadable)
        );
        assert_eq!(store.read(&id), Err(KeyStoreError::Unreadable));
        assert!(!store.contains(&id));
    }

    #[test]
    fn the_fault_sequence_is_reproducible() {
        // No randomness and no clock: the same script yields the same outcomes every
        // time it runs, so a failing test is always a real regression.
        let sequence = || {
            let store = InMemoryKeyStore::with_fault(Fault::FailAfter(1));
            let id = KeyId::new("key-a");
            store.seed(id.clone(), &Secret::new("fixture-secret-a"));
            let first = store.read(&id).is_ok();
            let second = store.read(&id).is_ok();
            let third = store.read(&id).is_ok();
            (first, second, third, store.read_calls())
        };
        assert_eq!(sequence(), (true, false, false, 3));
        assert_eq!(sequence(), sequence());
    }

    #[test]
    fn deleting_an_absent_entry_is_not_an_error() -> Result<(), KeyStoreError> {
        let store = InMemoryKeyStore::new();
        store.delete(&KeyId::new("key-a"))?;
        assert_eq!(store.delete_calls(), 1);
        Ok(())
    }
}
