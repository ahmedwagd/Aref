//! The key pool: failover, invalidation, and backoff (`FR-302`, `FR-305`, `FR-307`,
//! `FR-308`, `FR-313`; tasks 3.8, 3.9 and 3.18).
//!
//! The attempt sequence is `design.md` §6.4, and every branch has exactly one outcome:
//!
//! | Response | What happens |
//! |---|---|
//! | 2xx with usable content | the rewrite is returned |
//! | 2xx that `output::validate` rejects | the error escapes the pool: a bad rewrite is not a key's fault |
//! | `401` | the key is marked `invalid`, its metadata written, `providers://key_invalid` emitted, and the next key is tried |
//! | `429` with a usable `Retry-After` | that key is exhausted at once and the next key is tried (no wait) |
//! | `429` without one | `resolve(n)` and wait on the injected clock, up to the attempt budget, then the next key |
//! | `5xx`, drop, timeout, cut body | `resolve(n)` and retry this key, then the next key |
//! | any other non-2xx | the next key, with no retry |
//!
//! Two guarantees are properties of this file rather than conventions:
//!
//! * **`G-20` on read.** A credential the keyring cannot hand back is *skipped*: its
//!   status is left untouched, nothing is cached, and the next attempt reads the keyring
//!   again. `invalid` means "the provider rejected this credential", never "the store
//!   could not return it".
//! * **No key material in an outcome.** The only key-derived value that leaves the pool is
//!   the stored `masked_preview`, which is what `E_KEY_INVALID` and the event carry.

use std::sync::Arc;
use std::time::Duration;

use crate::error::{AppError, KeyringFault};
use crate::providers::client::{ChatRequest, Transport, TransportError};
use crate::providers::clock::{Clock, Jitter};
use crate::providers::events::{resets_at, EventSink, KeyInvalid, RateLimited};
use crate::providers::output;
use crate::providers::retry::{minutes_for, parse_retry_after, BackoffPolicy};
use crate::providers::selection::Candidate;
use crate::security::keyring::{KeyId, KeyStore};
use crate::security::redact::Secret;
use crate::storage::providers_repo::{
    KeyMetadata, KeyStatus, ProviderConfig, ProviderKind, ProvidersRepo,
};
use crate::storage::timestamp_now;

/// The `last_error_code` a 401 writes. Kept as a name so the column and the catalog cannot
/// drift; `KEY_INVALID_CODE` is asserted against `AppError::code` in this module's tests.
const KEY_INVALID_CODE: &str = "E_KEY_INVALID";

/// Why one credential stopped being usable.
enum KeyFailure {
    /// The provider rejected the credential (401).
    Invalid,
    /// The provider is rate-limiting it (429), and this is the wait it asked for.
    RateLimited { wait: Duration },
    /// Another failure; the next key may still work.
    Other(AppError),
    /// A failure of the response, not of the credential: leave the pool.
    Terminal(AppError),
}

/// The request path's pool of providers and keys.
///
/// Every dependency is a trait object (`design.md` §3.3), so a test assembles the same
/// struct with doubles and exercises this file's real logic.
pub struct KeyPool {
    transport: Arc<dyn Transport>,
    keystore: Arc<dyn KeyStore>,
    repo: Arc<dyn ProvidersRepo>,
    events: Arc<dyn EventSink>,
    clock: Arc<dyn Clock>,
    jitter: Arc<dyn Jitter>,
}

impl KeyPool {
    pub fn new(
        transport: Arc<dyn Transport>,
        keystore: Arc<dyn KeyStore>,
        repo: Arc<dyn ProvidersRepo>,
        events: Arc<dyn EventSink>,
        clock: Arc<dyn Clock>,
        jitter: Arc<dyn Jitter>,
    ) -> Self {
        Self {
            transport,
            keystore,
            repo,
            events,
            clock,
            jitter,
        }
    }

    /// Runs the request with the configured policy.
    pub async fn execute(
        &self,
        candidates: &[Candidate],
        request: &ChatRequest,
    ) -> Result<String, AppError> {
        self.execute_with(candidates, request, &BackoffPolicy::from_config())
            .await
    }

    /// The same run with an injected budget, so the humanize change can pass its own.
    pub async fn execute_with(
        &self,
        candidates: &[Candidate],
        request: &ChatRequest,
        policy: &BackoffPolicy,
    ) -> Result<String, AppError> {
        if candidates.is_empty() {
            return Err(AppError::NoProvider {
                reason: "no_candidates",
            });
        }

        let mut unreadable_keys: u32 = 0;
        let mut keyless_attempted = false;
        let mut failure_count: u32 = 0;
        let mut invalid_count: u32 = 0;
        let mut rate_limited_count: u32 = 0;
        let mut last_error: Option<AppError> = None;
        let mut last_invalid: Option<(String, String)> = None;
        let mut last_rate_limit: Option<(String, Duration)> = None;

        for candidate in candidates {
            let provider = &candidate.provider;
            let valid: Vec<&KeyMetadata> = candidate
                .keys
                .iter()
                .filter(|key| key.status == KeyStatus::Valid)
                .collect();

            // A local provider needs no key (`FR-303`): one slot with no credential. A
            // cloud provider with no valid key cannot serve and is skipped here even if a
            // caller passed it directly instead of through `selection::candidates`.
            let slots: Vec<Option<&KeyMetadata>> = if valid.is_empty() {
                if provider.kind == ProviderKind::Local {
                    vec![None]
                } else {
                    Vec::new()
                }
            } else {
                valid.into_iter().map(Some).collect()
            };

            for slot in slots {
                let credential: Option<Secret> = match slot {
                    Some(key) => match self.read_key(&key.id).await {
                        Some(secret) => Some(secret),
                        None => {
                            // `G-20`: skip, keep the status, cache nothing.
                            unreadable_keys += 1;
                            continue;
                        }
                    },
                    None => {
                        keyless_attempted = true;
                        None
                    }
                };

                match self
                    .attempt(provider, credential.as_ref(), request, policy)
                    .await
                {
                    Ok(text) => return Ok(text),
                    Err(KeyFailure::Terminal(error)) => return Err(error),
                    Err(KeyFailure::Invalid) => {
                        failure_count += 1;
                        invalid_count += 1;
                        match slot {
                            Some(key) => {
                                self.mark_invalid(provider, key).await;
                                last_invalid =
                                    Some((provider.id.clone(), key.masked_preview.to_string()));
                            }
                            // A keyless provider that demands a credential has no key to
                            // name; the code is still the §7 row for a rejected credential.
                            None => last_invalid = Some((provider.id.clone(), String::new())),
                        }
                    }
                    Err(KeyFailure::RateLimited { wait }) => {
                        failure_count += 1;
                        rate_limited_count += 1;
                        let replace = match &last_rate_limit {
                            Some((_, current)) => wait > *current,
                            None => true,
                        };
                        if replace {
                            last_rate_limit = Some((provider.id.clone(), wait));
                        }
                    }
                    Err(KeyFailure::Other(error)) => {
                        failure_count += 1;
                        last_error = Some(error);
                    }
                }
            }
        }

        if failure_count == 0 {
            // Nothing was attempted at all. If keys existed but none could be read, `G-20`
            // names the reason; otherwise the configuration simply has no usable provider.
            return if unreadable_keys > 0 && !keyless_attempted {
                Err(AppError::KeyringUnavailable {
                    reason: KeyringFault::Unreadable,
                })
            } else {
                Err(AppError::NoProvider {
                    reason: "no_usable_provider",
                })
            };
        }

        if invalid_count == failure_count {
            let (provider, masked) = last_invalid.unwrap_or_else(|| (String::new(), String::new()));
            return Err(AppError::KeyInvalid { provider, masked });
        }

        if rate_limited_count == failure_count {
            if let Some((provider, wait)) = last_rate_limit {
                self.events.rate_limited(&RateLimited {
                    provider_id: provider.clone(),
                    retry_after_seconds: wait.as_secs(),
                    resets_at: resets_at(self.clock.now(), wait),
                });
                return Err(AppError::RateLimited {
                    minutes: minutes_for(wait),
                });
            }
        }

        Err(last_error.unwrap_or_else(AppError::internal))
    }

    /// One credential's attempts, up to the policy's budget.
    async fn attempt(
        &self,
        provider: &ProviderConfig,
        credential: Option<&Secret>,
        request: &ChatRequest,
        policy: &BackoffPolicy,
    ) -> Result<String, KeyFailure> {
        let max_attempts = policy.max_attempts.max(1);
        for n in 1..=max_attempts {
            let last_attempt = n == max_attempts;
            match self.transport.send(provider, credential, request).await {
                Ok(response) if (200..=299).contains(&response.status) => {
                    // A rejected body is a failed attempt for the humanize loop to retry
                    // with tighter instructions, not a reason to try another key.
                    return output::validate(&response.body).map_err(KeyFailure::Terminal);
                }
                Ok(response) if response.status == 401 => return Err(KeyFailure::Invalid),
                Ok(response) if response.status == 429 => {
                    if let Some(parsed) = parse_retry_after(response.retry_after.as_deref()) {
                        // A usable header replaces the curve, so there is nothing to wait
                        // for on this key: fail over immediately.
                        return Err(KeyFailure::RateLimited { wait: parsed.wait });
                    }
                    let wait = policy.resolve(n, None, self.jitter.as_ref());
                    if last_attempt {
                        return Err(KeyFailure::RateLimited { wait });
                    }
                    self.clock.sleep(wait).await;
                }
                Ok(response) => {
                    let error = AppError::Internal {
                        status: Some(response.status),
                    };
                    if response.status >= 500 {
                        if last_attempt {
                            return Err(KeyFailure::Other(error));
                        }
                        let wait = policy.resolve(n, None, self.jitter.as_ref());
                        self.clock.sleep(wait).await;
                    } else {
                        // Another 4xx: this key will not start working, so fail over.
                        return Err(KeyFailure::Other(error));
                    }
                }
                Err(error) => {
                    let mapped = match error {
                        TransportError::Dropped => AppError::NetworkOffline,
                        TransportError::Timeout => AppError::ProviderTimeout,
                        TransportError::Interrupted => AppError::StreamInterrupted,
                    };
                    if last_attempt {
                        return Err(KeyFailure::Other(mapped));
                    }
                    let wait = policy.resolve(n, None, self.jitter.as_ref());
                    self.clock.sleep(wait).await;
                }
            }
        }
        // The loop returns on its last iteration; this keeps the signature total without a
        // panic (AGENTS.md §9).
        Err(KeyFailure::Other(AppError::internal()))
    }

    /// Reads one credential, off the runtime thread.
    ///
    /// `None` covers every unreadable state — absent, `Unreadable`, or an unreachable store
    /// — because `G-20` treats them the same way: skip this key, change nothing, and read
    /// again next time. Nothing is cached.
    async fn read_key(&self, id: &KeyId) -> Option<Secret> {
        let store = Arc::clone(&self.keystore);
        let id = id.clone();
        match tokio::task::spawn_blocking(move || store.read(&id)).await {
            Ok(Ok(Some(secret))) => Some(secret),
            Ok(_) | Err(_) => None,
        }
    }

    /// Writes the `invalid` status, then emits the notice.
    ///
    /// The write comes first (`design.md` §4.3), and the event carries the **stored**
    /// preview (`G-27`). A failed write does not change the run: the request must still
    /// fail over, and the next `list_providers` read is the source of truth for the UI.
    async fn mark_invalid(&self, provider: &ProviderConfig, key: &KeyMetadata) {
        let mut updated = key.clone();
        updated.status = KeyStatus::Invalid;
        updated.last_error_code = Some(KEY_INVALID_CODE.to_owned());
        updated.invalid_since = Some(timestamp_now());
        updated.updated_at = timestamp_now();
        // A failed write is not reported as a request failure: the request must still fail
        // over, and the next `list_providers` read is the source of truth for the UI. A
        // `StorageError` carries no content, so ignoring it cannot leak a key.
        let _ = self.repo.update_key(&updated).await;

        self.events.key_invalid(&KeyInvalid {
            provider_id: provider.id.clone(),
            key_id: key.id.to_string(),
            masked_preview: key.masked_preview.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::providers::client::doubles::{Scripted, ScriptedTransport};
    use crate::providers::client::ChatRequest;
    use crate::providers::clock::{FakeClock, FixedJitter, SequenceJitter};
    use crate::providers::events::RecordingSink;
    use crate::providers::selection;
    use crate::security::keyring::{Fault, InMemoryKeyStore};
    use crate::security::mask::MaskedPreview;
    use crate::storage::db::{self, test_support::TempDatabase};
    use crate::storage::providers_repo::{LocalFlavor, SqliteProvidersRepo};

    const KEY: &str = "fixture-secret-a";
    const PREVIEW: &str = "••••••••et-a";

    fn ready<T, E: std::fmt::Debug>(outcome: Result<T, E>) -> T {
        match outcome {
            Ok(value) => value,
            Err(error) => panic!("fixture failed: {error:?}"),
        }
    }

    fn millis(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn at(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    fn check<T>(outcome: Result<T, AppError>) -> AppError {
        match outcome {
            Ok(_) => panic!("expected an error, got a value"),
            Err(error) => error,
        }
    }

    fn envelope(error: &AppError) -> serde_json::Value {
        match serde_json::to_value(error) {
            Ok(value) => value,
            Err(error) => panic!("the envelope did not serialize: {error}"),
        }
    }

    fn params(error: &AppError) -> serde_json::Value {
        envelope(error)
            .get("params")
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    }

    /// One provider, its keys, and the four doubles around the real pool.
    struct Fixture {
        /// Held only for its `Drop`: the temp-file database is removed with it.
        _database: TempDatabase,
        repo: Arc<SqliteProvidersRepo>,
        store: Arc<InMemoryKeyStore>,
        transport: Arc<ScriptedTransport>,
        sink: Arc<RecordingSink>,
        clock: Arc<FakeClock>,
        provider: ProviderConfig,
    }

    impl Fixture {
        async fn new(
            label: &str,
            kind: ProviderKind,
            flavor: Option<LocalFlavor>,
            script: impl IntoIterator<Item = Scripted>,
        ) -> Self {
            let database = TempDatabase::new(label);
            let (sql, path) = ready(database.connect().await);
            ready(db::migrate(&sql, &path).await);
            let repo = Arc::new(SqliteProvidersRepo::new(sql));
            let provider = ProviderConfig::new(
                "provider-1",
                kind,
                flavor,
                "http://127.0.0.1:1/v1",
                "model-a",
                "Primary",
                true,
            );
            ready(repo.insert_provider(&provider).await);
            Self {
                _database: database,
                repo,
                store: Arc::new(InMemoryKeyStore::new()),
                transport: Arc::new(ScriptedTransport::new(script)),
                sink: Arc::new(RecordingSink::new()),
                clock: Arc::new(FakeClock::new()),
                provider,
            }
        }

        async fn cloud(label: &str, script: impl IntoIterator<Item = Scripted>) -> Self {
            Self::new(label, ProviderKind::OpenAiCompatible, None, script).await
        }

        async fn local(label: &str, script: impl IntoIterator<Item = Scripted>) -> Self {
            Self::new(
                label,
                ProviderKind::Local,
                Some(LocalFlavor::Ollama),
                script,
            )
            .await
        }

        /// Replaces the credential store before any key is added.
        ///
        /// The fault has to exist at construction (`InMemoryKeyStore::with_fault`), and
        /// the pool takes its `Arc` at [`Fixture::pool`] time, so swapping here is enough.
        fn set_store(&mut self, store: Arc<InMemoryKeyStore>) {
            self.store = store;
        }

        /// Adds a key row and seeds its credential, as `add_key` would have.
        async fn with_key(&self, id: &str) -> String {
            let key = KeyMetadata {
                id: KeyId::new(id),
                provider_id: self.provider.id.clone(),
                label: id.to_owned(),
                masked_preview: MaskedPreview::from_stored(PREVIEW),
                status: KeyStatus::Valid,
                last_error_code: None,
                invalid_since: None,
                updated_at: "2026-10-24T00:00:00Z".to_owned(),
            };
            ready(self.repo.insert_key(&key).await);
            self.store.seed(key.id.clone(), &Secret::new(KEY));
            id.to_owned()
        }

        async fn provider_row(&self) -> ProviderConfig {
            ready(
                self.repo
                    .find_provider(&self.provider.id)
                    .await
                    .map(|found| found.unwrap_or_else(|| panic!("the provider row vanished"))),
            )
        }

        async fn keys(&self) -> Vec<KeyMetadata> {
            ready(self.repo.list_keys().await)
        }

        async fn key_status(&self, id: &str) -> String {
            let keys = self.keys().await;
            let key = keys
                .iter()
                .find(|key| key.id.to_string() == id)
                .unwrap_or_else(|| panic!("key {id} vanished"));
            match key.status {
                KeyStatus::Valid => "valid".to_owned(),
                KeyStatus::Invalid => "invalid".to_owned(),
            }
        }

        async fn stored_preview(&self, id: &str) -> String {
            let keys = self.keys().await;
            keys.iter()
                .find(|key| key.id.to_string() == id)
                .map(|key| key.masked_preview.to_string())
                .unwrap_or_default()
        }

        async fn candidates_for(&self, providers: &[ProviderConfig]) -> Vec<Candidate> {
            let keys = self.keys().await;
            selection::candidates(providers, &keys)
        }

        fn pool_with(&self, jitter: Arc<dyn Jitter>) -> KeyPool {
            KeyPool::new(
                Arc::clone(&self.transport) as Arc<dyn Transport>,
                Arc::clone(&self.store) as Arc<dyn KeyStore>,
                Arc::clone(&self.repo) as Arc<dyn ProvidersRepo>,
                Arc::clone(&self.sink) as Arc<dyn EventSink>,
                Arc::clone(&self.clock) as Arc<dyn Clock>,
                jitter,
            )
        }

        fn pool(&self) -> KeyPool {
            self.pool_with(Arc::new(FixedJitter(Duration::ZERO)))
        }

        async fn run(&self, pool: &KeyPool) -> Result<String, AppError> {
            let candidates = self
                .candidates_for(std::slice::from_ref(&self.provider))
                .await;
            pool.execute_with(&candidates, &ChatRequest::user("fixture text"), &policy())
                .await
        }
    }

    fn policy() -> BackoffPolicy {
        BackoffPolicy::from_config()
    }

    // ------------------------------------------------------------- 3.8 failover

    #[tokio::test]
    async fn a_401_invalidates_the_key_and_the_outcome_comes_from_the_second() {
        let fixture = Fixture::cloud(
            "pool-401-failover",
            [Scripted::status(401), Scripted::ok("second")],
        )
        .await;
        let first = fixture.with_key("key-1").await;
        let second = fixture.with_key("key-2").await;

        let outcome = fixture.run(&fixture.pool()).await;

        assert_eq!(outcome, Ok("second".to_owned()), "the second key answered");
        assert_eq!(fixture.transport.call_count(), 2);

        // The rejected key is marked, and the accepted one is untouched.
        assert_eq!(fixture.key_status(&first).await, "invalid");
        assert_eq!(fixture.key_status(&second).await, "valid");
        let keys = fixture.keys().await;
        let invalidated = keys
            .iter()
            .find(|key| key.id.to_string() == first)
            .unwrap_or_else(|| panic!("key-1 vanished"));
        assert_eq!(
            invalidated.last_error_code.as_deref(),
            Some(KEY_INVALID_CODE)
        );
        assert!(
            invalidated.invalid_since.is_some(),
            "the instant is recorded"
        );
        // `FR-306`: the invalid key is retained, not deleted.
        assert_eq!(keys.len(), 2);

        // `providers://key_invalid`, field for field, with the stored preview.
        let events = fixture.sink.key_invalid_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].provider_id, "provider-1");
        assert_eq!(events[0].key_id, first);
        assert_eq!(
            events[0].masked_preview,
            fixture.stored_preview(&first).await
        );
        assert_eq!(events[0].masked_preview, PREVIEW);
    }

    #[tokio::test]
    async fn all_401_is_e_key_invalid_carrying_the_stored_preview() {
        let fixture = Fixture::cloud("pool-all-401", [Scripted::status(401)]).await;
        let key = fixture.with_key("key-1").await;

        let error = check(fixture.run(&fixture.pool()).await);

        assert_eq!(error.code(), "E_KEY_INVALID");
        assert_eq!(
            params(&error),
            serde_json::json!({ "provider": "provider-1", "masked": PREVIEW })
        );
        assert_eq!(fixture.key_status(&key).await, "invalid");
        assert_eq!(
            params(&error)["masked"],
            serde_json::json!(fixture.stored_preview(&key).await),
            "the preview is the stored one, never a fresh derivation"
        );
    }

    #[tokio::test]
    async fn an_invalid_key_is_never_sent_and_the_next_one_is() {
        let fixture = Fixture::cloud("pool-invalid-skipped", [Scripted::ok("second")]).await;
        let first = fixture.with_key("key-1").await;
        let _second = fixture.with_key("key-2").await;

        // Seeds the state a previous 401 left behind.
        let mut keys = fixture.keys().await;
        for key in keys.iter_mut() {
            if key.id.to_string() == first {
                key.status = KeyStatus::Invalid;
            }
        }
        for key in &keys {
            ready(fixture.repo.update_key(key).await);
        }
        let reads_before = fixture.store.read_calls();

        let outcome = fixture.run(&fixture.pool()).await;

        assert_eq!(outcome, Ok("second".to_owned()));
        assert_eq!(fixture.transport.call_count(), 1, "one request, not two");
        assert_eq!(
            fixture.store.read_calls() - reads_before,
            1,
            "only the valid key's credential was read"
        );
    }

    #[tokio::test]
    async fn a_429_with_a_usable_header_fails_over_immediately() {
        let fixture = Fixture::cloud(
            "pool-429-header",
            [Scripted::rate_limited(Some("120")), Scripted::ok("second")],
        )
        .await;
        fixture.with_key("key-1").await;
        fixture.with_key("key-2").await;

        let outcome = fixture.run(&fixture.pool()).await;

        assert_eq!(outcome, Ok("second".to_owned()));
        assert_eq!(fixture.transport.call_count(), 2);
        assert_eq!(
            fixture.clock.sleep_count(),
            0,
            "a present header means no wait on this key"
        );
    }

    #[tokio::test]
    async fn a_429_without_a_header_follows_the_curve_then_fails_over() {
        let fixture = Fixture::cloud(
            "pool-429-curve",
            [
                Scripted::rate_limited(None),
                Scripted::rate_limited(None),
                Scripted::rate_limited(None),
                Scripted::ok("second"),
            ],
        )
        .await;
        fixture.with_key("key-1").await;
        fixture.with_key("key-2").await;
        let jitter = Arc::new(SequenceJitter::new([millis(400), millis(1_600)]));
        let pool = fixture.pool_with(jitter);

        let outcome = fixture.run(&pool).await;

        assert_eq!(outcome, Ok("second".to_owned()));
        assert_eq!(fixture.transport.call_count(), 4);
        // Two waits between the three attempts, each the injected draw within the
        // computed delay. The third attempt is terminal, so it does not wait again.
        assert_eq!(
            fixture.clock.sleeps(),
            vec![millis(400), millis(1_600)],
            "the schedule is the drawn sequence"
        );
    }

    #[tokio::test]
    async fn all_429_reports_the_wait_in_minutes_and_emits_the_event() {
        let fixture = Fixture::cloud("pool-all-429", [Scripted::rate_limited(Some("120"))]).await;
        fixture.with_key("key-1").await;

        let error = check(fixture.run(&fixture.pool()).await);

        assert_eq!(error.code(), "E_RATE_LIMITED");
        assert_eq!(params(&error), serde_json::json!({ "minutes": 2 }));

        let events = fixture.sink.rate_limited_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].provider_id, "provider-1");
        assert_eq!(events[0].retry_after_seconds, 120);
        // The fake clock starts at the epoch, so the instant is a literal.
        assert_eq!(events[0].resets_at, "1970-01-01T00:02:00Z");
    }

    #[tokio::test]
    async fn all_429_without_a_header_reports_the_curve_wait() {
        let fixture = Fixture::cloud(
            "pool-all-429-curve",
            [
                Scripted::rate_limited(None),
                Scripted::rate_limited(None),
                Scripted::rate_limited(None),
            ],
        )
        .await;
        fixture.with_key("key-1").await;
        // A jitter that asks for everything gets exactly the computed delay, which is
        // what makes the emitted wait a literal.
        let pool = fixture.pool_with(Arc::new(FixedJitter(Duration::MAX)));

        let error = check(fixture.run(&pool).await);

        assert_eq!(error.code(), "E_RATE_LIMITED");
        // The curve at n = 3 is 4 s, and 4 s rounds up to one minute.
        assert_eq!(params(&error), serde_json::json!({ "minutes": 1 }));
        let events = fixture.sink.rate_limited_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].retry_after_seconds, 4);
        // The elapsed waits (1 s at n = 1, 2 s at n = 2) are part of the schedule, so the
        // instant is "now" — three seconds of fake time — plus the final four.
        assert_eq!(fixture.clock.sleeps(), vec![at(1), at(2)]);
        assert_eq!(events[0].resets_at, "1970-01-01T00:00:07Z");
    }

    #[tokio::test]
    async fn a_5xx_is_retried_on_the_schedule_then_the_next_key_answers() {
        let fixture = Fixture::cloud(
            "pool-5xx",
            [
                Scripted::status(503),
                Scripted::status(503),
                Scripted::status(503),
                Scripted::ok("second"),
            ],
        )
        .await;
        fixture.with_key("key-1").await;
        fixture.with_key("key-2").await;
        let jitter = Arc::new(SequenceJitter::new([millis(100), millis(200)]));
        let pool = fixture.pool_with(jitter);

        let outcome = fixture.run(&pool).await;

        assert_eq!(outcome, Ok("second".to_owned()));
        assert_eq!(fixture.transport.call_count(), 4);
        assert_eq!(fixture.clock.sleeps(), vec![millis(100), millis(200)]);
    }

    #[tokio::test]
    async fn all_5xx_reports_the_last_status_and_never_leaks_a_body() {
        let fixture = Fixture::cloud(
            "pool-all-5xx",
            [
                Scripted::status(500),
                Scripted::status(500),
                Scripted::status(500),
            ],
        )
        .await;
        fixture.with_key("key-1").await;

        let error = check(fixture.run(&fixture.pool()).await);

        assert_eq!(error.code(), "E_INTERNAL");
        assert_eq!(params(&error), serde_json::json!({ "status": 500 }));
    }

    #[tokio::test]
    async fn a_transport_drop_is_retried_and_then_reported_as_offline() {
        let fixture = Fixture::cloud(
            "pool-drop",
            [
                Scripted::Fail(TransportError::Dropped),
                Scripted::Fail(TransportError::Dropped),
                Scripted::Fail(TransportError::Dropped),
            ],
        )
        .await;
        fixture.with_key("key-1").await;

        let error = check(fixture.run(&fixture.pool()).await);

        assert_eq!(error.code(), "E_NETWORK_OFFLINE");
        assert_eq!(fixture.transport.call_count(), 3, "the budget is three");
        assert_eq!(
            fixture.clock.sleep_count(),
            2,
            "waits between attempts only"
        );
    }

    #[tokio::test]
    async fn a_timeout_and_a_cut_body_map_to_their_own_codes() {
        for (label, failure, expected) in [
            (
                "pool-timeout",
                TransportError::Timeout,
                "E_PROVIDER_TIMEOUT",
            ),
            (
                "pool-cut",
                TransportError::Interrupted,
                "E_STREAM_INTERRUPTED",
            ),
        ] {
            let fixture = Fixture::cloud(
                label,
                [
                    Scripted::Fail(failure),
                    Scripted::Fail(failure),
                    Scripted::Fail(failure),
                ],
            )
            .await;
            fixture.with_key("key-1").await;
            let error = check(fixture.run(&fixture.pool()).await);
            assert_eq!(error.code(), expected);
        }
    }

    #[tokio::test]
    async fn another_4xx_fails_over_without_a_retry() {
        let fixture =
            Fixture::cloud("pool-4xx", [Scripted::status(404), Scripted::ok("second")]).await;
        fixture.with_key("key-1").await;
        fixture.with_key("key-2").await;

        let outcome = fixture.run(&fixture.pool()).await;

        assert_eq!(outcome, Ok("second".to_owned()));
        assert_eq!(fixture.transport.call_count(), 2);
        assert_eq!(fixture.clock.sleep_count(), 0);
    }

    // --------------------------------------------------------- 3.9 `G-20` on read

    #[tokio::test]
    async fn an_unreadable_key_is_skipped_across_two_attempts_with_its_status_unchanged() {
        let mut fixture = Fixture::cloud("pool-unreadable", [Scripted::ok("first")]).await;
        // The first read succeeds, every later one fails: readable, then unreadable.
        fixture.set_store(Arc::new(InMemoryKeyStore::with_fault(Fault::FailAfter(1))));
        let key = fixture.with_key("key-1").await;
        let pool = fixture.pool();

        let first = fixture.run(&pool).await;
        assert_eq!(first, Ok("first".to_owned()));
        assert_eq!(fixture.key_status(&key).await, "valid");

        let error = check(fixture.run(&pool).await);

        assert_eq!(error.code(), "E_KEYRING_UNAVAILABLE");
        assert_eq!(
            params(&error),
            serde_json::json!({ "reason": "unreadable" })
        );
        assert_eq!(
            fixture.store.read_calls(),
            2,
            "the second attempt read the keyring again instead of a cached copy"
        );
        assert_eq!(
            fixture.key_status(&key).await,
            "valid",
            "an unreadable key is not invalid (`G-20`)"
        );
        assert!(fixture.sink.key_invalid_events().is_empty());
    }

    #[tokio::test]
    async fn a_keyless_local_provider_proceeds_and_raises_nothing() {
        let fixture = Fixture::local("pool-keyless", [Scripted::ok("local answer")]).await;

        let outcome = fixture.run(&fixture.pool()).await;

        assert_eq!(outcome, Ok("local answer".to_owned()));
        let calls = fixture.transport.calls();
        assert_eq!(calls.len(), 1);
        assert!(!calls[0].credentialed, "no credential is sent");
        assert_eq!(fixture.store.read_calls(), 0);
    }

    #[tokio::test]
    async fn an_unreadable_cloud_key_does_not_block_a_keyless_local_provider() {
        let mut fixture =
            Fixture::local("pool-unreadable-plus-local", [Scripted::ok("local")]).await;
        // A cloud provider whose only key cannot be read, plus the keyless local entry.
        let cloud = ProviderConfig::new(
            "cloud",
            ProviderKind::OpenAiCompatible,
            None,
            "http://127.0.0.1:1/v1",
            "model-a",
            "Cloud",
            true,
        );
        ready(fixture.repo.insert_provider(&cloud).await);
        fixture.set_store(Arc::new(InMemoryKeyStore::with_fault(Fault::FailAfter(0))));
        let cloud_key = KeyMetadata {
            id: KeyId::new("cloud-key"),
            provider_id: "cloud".to_owned(),
            label: "personal".to_owned(),
            masked_preview: MaskedPreview::from_stored(PREVIEW),
            status: KeyStatus::Valid,
            last_error_code: None,
            invalid_since: None,
            updated_at: "2026-10-24T00:00:00Z".to_owned(),
        };
        ready(fixture.repo.insert_key(&cloud_key).await);
        fixture.store.seed(cloud_key.id.clone(), &Secret::new(KEY));

        let mut candidates = fixture
            .candidates_for(&[cloud, fixture.provider_row().await])
            .await;
        // Selection puts the local entry first. The pool is driven with the cloud entry
        // first on purpose: the claim is that an unreadable key is *skipped*, not that it
        // never had to be reached.
        candidates.reverse();
        assert_eq!(candidates[0].provider.id, "cloud");

        let outcome = fixture
            .pool()
            .execute_with(&candidates, &ChatRequest::user("fixture text"), &policy())
            .await;

        assert_eq!(outcome, Ok("local".to_owned()));
        assert_eq!(fixture.transport.call_count(), 1);
        assert!(!fixture.transport.calls()[0].credentialed);
        assert_eq!(
            fixture.store.read_calls(),
            1,
            "one attempt to read the cloud key"
        );
    }

    #[tokio::test]
    async fn with_no_candidate_at_all_the_result_is_e_no_provider() {
        let fixture = Fixture::cloud("pool-none", []).await;
        let error = check(
            fixture
                .pool()
                .execute_with(&[], &ChatRequest::user("x"), &policy())
                .await,
        );
        assert_eq!(error.code(), "E_NO_PROVIDER");
        assert_eq!(fixture.transport.call_count(), 0);
    }

    // ------------------------------------------------------ 3.13 at the pool edge

    #[tokio::test]
    async fn a_marked_refusal_escapes_the_pool_without_failing_over() {
        let fixture = Fixture::cloud(
            "pool-refused",
            [Scripted::Respond(crate::providers::client::ChatResponse {
                status: 200,
                retry_after: None,
                body: serde_json::json!({
                    "choices": [{
                        "message": { "role": "assistant", "content": "not returned" },
                        "finish_reason": "content_filter"
                    }]
                })
                .to_string(),
            })],
        )
        .await;
        let key = fixture.with_key("key-1").await;
        fixture.with_key("key-2").await;

        let error = check(fixture.run(&fixture.pool()).await);

        assert_eq!(error.code(), "E_OUTPUT_REJECTED");
        assert_eq!(params(&error), serde_json::json!({ "reason": "refused" }));
        assert_eq!(fixture.transport.call_count(), 1, "no failover to key-2");
        assert_eq!(
            fixture.key_status(&key).await,
            "valid",
            "no key is invalidated"
        );
        assert!(fixture.sink.key_invalid_events().is_empty());
    }

    // ---------------------------------------------- 3.18 redaction at the failure path

    #[tokio::test]
    async fn a_failing_attempt_carries_no_key_material() {
        let fixture = Fixture::cloud("pool-redaction", [Scripted::status(401)]).await;
        let key = fixture.with_key("key-1").await;

        let error = check(fixture.run(&fixture.pool()).await);
        let serialized = envelope(&error).to_string();

        assert!(!serialized.contains(KEY), "the secret reached the envelope");
        let prefix: String = KEY.chars().take(4).collect();
        assert!(
            !serialized.contains(&prefix),
            "a four-character prefix of the secret reached the envelope"
        );

        // The only key-derived value is the stored preview, and nothing else travels.
        assert_eq!(
            serialized,
            serde_json::json!({
                "code": "E_KEY_INVALID",
                "message_key": "error.provider.key_invalid",
                "params": { "provider": "provider-1", "masked": PREVIEW }
            })
            .to_string()
        );
        assert_eq!(
            params(&error)["masked"],
            serde_json::json!(fixture.stored_preview(&key).await)
        );
    }

    #[test]
    fn the_invalid_code_column_holds_the_catalog_string() {
        let code = AppError::KeyInvalid {
            provider: String::new(),
            masked: String::new(),
        }
        .code();
        assert_eq!(KEY_INVALID_CODE, code);
    }

    // ---------------------------------------------------------------- the fixtures

    #[test]
    fn the_fixture_secret_is_not_key_shaped() {
        assert!(KEY.starts_with("fixture-secret-"));
        assert!(!KEY.starts_with("sk-"));
    }
}
