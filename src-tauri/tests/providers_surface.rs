//! Tasks 3.4 (architecture half), 3.12, 3.14 (one policy), 3.16, 3.17 and 3.20 — the wired
//! command surface, the status response, and the cold start.
//!
//! The state is assembled the way `lib.rs` assembles it (real repository on a temp-file
//! database, one transport, one keystore) and then exercised through the same service calls
//! the handlers make. Handler bodies are three lines each and are registered in
//! `generate_handler!`; what these tests pin is the behaviour behind them.

mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use aref_lib::commands::dto::{
    AddKeyRequest, AddProviderRequest, AppStatus, EditKeyRequest, KeyMetadataView,
    ListProvidersResponse, ProviderView, RemoveKeyRequest, RemoveProviderRequest,
    UpdateProviderRequest,
};
use aref_lib::error::AppError;
use aref_lib::providers::client::{ChatRequest, ReqwestTransport};
use aref_lib::providers::clock::{Clock, FakeClock, FixedJitter};
use aref_lib::providers::keys::KeyService;
use aref_lib::providers::pool::KeyPool;
use aref_lib::providers::retry::BackoffPolicy;
use aref_lib::providers::selection::Candidate;
use aref_lib::providers::{ProviderChanges, ProviderDraft, ProviderService};
use aref_lib::security::keyring::{KeyId, KeyStore};
use aref_lib::security::mask::MaskedPreview;
use aref_lib::status::{AppStatusService, ConstantDetectorMode};
use aref_lib::storage::providers_repo::{
    KeyMetadata, KeyStatus, LocalFlavor, ProviderConfig, ProviderKind, ProvidersRepo,
    SqliteProvidersRepo,
};
use aref_lib::AppState;
use support::{closed_port, CountingTransport, RecordingKeyStore, SilentSink, TempDir};

const PREVIEW: &str = "••••••••et-a";

struct Fixture {
    _directory: TempDir,
    repo: Arc<dyn ProvidersRepo>,
    store: Arc<RecordingKeyStore>,
    transport: Arc<CountingTransport>,
    providers: Arc<ProviderService>,
    keys: Arc<KeyService>,
}

impl Fixture {
    async fn new(label: &str) -> Self {
        let directory = TempDir::new(label);
        let pool = directory.open().await;
        let repo: Arc<dyn ProvidersRepo> = Arc::new(SqliteProvidersRepo::new(pool));
        let store = Arc::new(RecordingKeyStore::new());
        let transport = Arc::new(CountingTransport::new());
        let keystore = Arc::clone(&store) as Arc<dyn KeyStore>;
        let providers = Arc::new(ProviderService::new(
            Arc::clone(&repo),
            Arc::clone(&transport) as Arc<dyn aref_lib::providers::client::Transport>,
            Arc::clone(&keystore),
        ));
        let keys = Arc::new(KeyService::new(Arc::clone(&repo), keystore));
        Self {
            _directory: directory,
            repo,
            store,
            transport,
            providers,
            keys,
        }
    }

    fn status(&self) -> AppStatusService {
        AppStatusService::with_repository(
            Arc::new(ConstantDetectorMode::statistical_only()),
            "0.1.0",
            Arc::clone(&self.repo),
        )
    }

    fn state(&self) -> AppState {
        AppState::wired(
            self.status(),
            Arc::clone(&self.providers),
            Arc::clone(&self.keys),
        )
    }

    async fn add_provider(&self, draft: ProviderDraft) -> ProviderConfig {
        self.providers
            .add_provider(draft)
            .await
            .unwrap_or_else(|error| panic!("add_provider failed: {}", error.code()))
            .provider
    }

    async fn insert_key(&self, provider_id: &str, id: &str) {
        let key = KeyMetadata {
            id: KeyId::new(id),
            provider_id: provider_id.to_owned(),
            label: "personal".to_owned(),
            masked_preview: MaskedPreview::from_stored(PREVIEW),
            status: KeyStatus::Valid,
            last_error_code: None,
            invalid_since: None,
            updated_at: "2026-10-24T00:00:00Z".to_owned(),
        };
        self.repo
            .insert_key(&key)
            .await
            .unwrap_or_else(|error| panic!("insert_key failed: {error:?}"));
    }
}

fn cloud(id: &str, base_url: &str, enabled: bool) -> ProviderConfig {
    ProviderConfig::new(
        id,
        ProviderKind::OpenAiCompatible,
        None,
        base_url,
        "model-a",
        "Cloud",
        enabled,
    )
}

fn keyless_local(id: &str, enabled: bool) -> ProviderConfig {
    ProviderConfig::new(
        id,
        ProviderKind::Local,
        Some(LocalFlavor::Ollama),
        "http://127.0.0.1:11434/v1",
        "model-b",
        "Ollama",
        enabled,
    )
}

fn envelope(error: &AppError) -> serde_json::Value {
    serde_json::to_value(error).unwrap_or_else(|error| panic!("no envelope: {error}"))
}

fn field_of(error: &AppError) -> String {
    match envelope(error)
        .get("params")
        .and_then(|params| params.get("field"))
        .and_then(|value| value.as_str())
    {
        Some(field) => field.to_owned(),
        None => panic!("no field in {}", envelope(error)),
    }
}

// ------------------------------------------------------- 3.16 the status response

#[tokio::test]
async fn status_carries_the_four_documented_fields() {
    let fixture = Fixture::new("status-empty").await;
    let snapshot = fixture
        .status()
        .resolve()
        .await
        .unwrap_or_else(|error| panic!("status failed: {}", error.code()));

    assert_eq!(snapshot.detector_mode.as_str(), "statistical_only");
    assert!(!snapshot.humanize_available);
    assert_eq!(snapshot.configured_providers, 0);
    assert_eq!(snapshot.enabled_providers, 0);
    assert_eq!(snapshot.usable_keys, 0);
    assert_eq!(snapshot.selected_provider_id, None);

    let value = serde_json::to_value(AppStatus::from(snapshot))
        .unwrap_or_else(|error| panic!("no status JSON: {error}"));
    let object = value.as_object().unwrap_or_else(|| panic!("not an object"));
    let mut names: Vec<&str> = object.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "app_version",
            "detector_mode",
            "humanize_available",
            "provider_summary"
        ]
    );
    let summary = object
        .get("provider_summary")
        .and_then(|value| value.as_object())
        .unwrap_or_else(|| panic!("no provider_summary"));
    let mut summary_names: Vec<&str> = summary.keys().map(String::as_str).collect();
    summary_names.sort_unstable();
    assert_eq!(
        summary_names,
        vec![
            "configured",
            "enabled",
            "selected_provider_id",
            "usable_keys"
        ]
    );
}

#[tokio::test]
async fn a_keyless_local_makes_humanize_available_and_an_invalid_only_cloud_does_not() {
    let fixture = Fixture::new("status-selection").await;
    let local = keyless_local("ollama", true);
    fixture
        .repo
        .insert_provider(&local)
        .await
        .unwrap_or_else(|error| panic!("insert failed: {error:?}"));

    let available = fixture.status().resolve().await.expect("status");
    assert!(available.humanize_available, "FR-303: no key is needed");
    assert_eq!(available.selected_provider_id.as_deref(), Some("ollama"));

    // A cloud provider whose only key is invalid is not a candidate (`FR-306`), and with the
    // local entry disabled nothing is selected.
    let mut disabled_local = local;
    disabled_local.enabled = false;
    fixture
        .repo
        .update_provider(&disabled_local)
        .await
        .expect("update");
    let cloud = cloud("cloud", "http://127.0.0.1:1/v1", true);
    fixture.repo.insert_provider(&cloud).await.expect("insert");
    let invalid = KeyMetadata {
        id: KeyId::new("cloud-key"),
        provider_id: "cloud".to_owned(),
        label: "personal".to_owned(),
        masked_preview: MaskedPreview::from_stored(PREVIEW),
        status: KeyStatus::Invalid,
        last_error_code: Some("E_KEY_INVALID".to_owned()),
        invalid_since: Some("2026-10-24T00:00:00Z".to_owned()),
        updated_at: "2026-10-24T00:00:00Z".to_owned(),
    };
    fixture.repo.insert_key(&invalid).await.expect("insert key");

    let unavailable = fixture.status().resolve().await.expect("status");
    assert!(!unavailable.humanize_available);
    assert_eq!(unavailable.selected_provider_id, None);
    assert_eq!(unavailable.configured_providers, 2);
    assert_eq!(unavailable.enabled_providers, 1, "only the cloud entry");
    assert_eq!(unavailable.usable_keys, 0, "an invalid key is not usable");
}

// ------------------------------------------------ 3.17 the provider command surface

#[tokio::test]
async fn list_providers_carries_the_six_fields_and_key_metadata_without_a_secret() {
    let fixture = Fixture::new("list").await;
    let provider = fixture
        .add_provider(ProviderDraft {
            kind: ProviderKind::OpenAiCompatible,
            local_flavor: None,
            base_url: "http://127.0.0.1:1/v1".to_owned(),
            model_id: "model-a".to_owned(),
            label: "Primary".to_owned(),
            enabled: true,
        })
        .await;
    fixture.insert_key(&provider.id, "key-1").await;

    let snapshots = fixture.providers.list_providers().await.expect("list");
    let views: Vec<ProviderView> = snapshots.iter().map(ProviderView::from).collect();
    let response = ListProvidersResponse {
        providers: views.clone(),
    };
    let value = serde_json::to_value(&response).expect("json");
    let first = &value["providers"][0];

    for field in [
        "id",
        "kind",
        "local_flavor",
        "base_url",
        "model_id",
        "label",
        "enabled",
        "keys",
    ] {
        assert!(first.get(field).is_some(), "missing {field}");
    }
    assert_eq!(first["kind"], serde_json::json!("openai_compatible"));
    assert_eq!(first["local_flavor"], serde_json::json!(null));
    assert_eq!(
        first["base_url"],
        serde_json::json!("http://127.0.0.1:1/v1")
    );
    assert_eq!(first["model_id"], serde_json::json!("model-a"));
    assert_eq!(first["label"], serde_json::json!("Primary"));
    assert_eq!(first["enabled"], serde_json::json!(true));
    assert_eq!(
        first["keys"][0]["masked_preview"],
        serde_json::json!(PREVIEW)
    );
    assert_eq!(first["keys"][0]["status"], serde_json::json!("valid"));
    // The whole response: no secret, no key-shaped string.
    assert!(!value.to_string().contains("fixture-secret"));
}

#[tokio::test]
async fn a_removed_provider_disappears_and_its_credentials_are_deleted_first() {
    let fixture = Fixture::new("remove").await;
    let provider = fixture
        .add_provider(ProviderDraft {
            kind: ProviderKind::OpenAiCompatible,
            local_flavor: None,
            base_url: "http://127.0.0.1:1/v1".to_owned(),
            model_id: "model-a".to_owned(),
            label: "Primary".to_owned(),
            enabled: true,
        })
        .await;
    fixture.insert_key(&provider.id, "key-1").await;

    let removed = fixture
        .providers
        .remove_provider(&provider.id)
        .await
        .expect("remove");
    assert!(removed.removed);
    assert!(fixture
        .providers
        .list_providers()
        .await
        .expect("list")
        .is_empty());
    // The credential went before the row: no secret is left with no row to name it.
    assert_eq!(fixture.store.deleted(), vec!["key-1".to_owned()]);

    let again = fixture
        .providers
        .remove_provider(&provider.id)
        .await
        .expect("remove again");
    assert!(
        !again.removed,
        "removing an absent id is a value, not an error"
    );
}

#[tokio::test]
async fn an_omitted_update_field_is_unchanged_and_a_kind_change_is_rejected() {
    let fixture = Fixture::new("update").await;
    let provider = fixture
        .add_provider(ProviderDraft {
            kind: ProviderKind::OpenAiCompatible,
            local_flavor: None,
            base_url: "http://127.0.0.1:1/v1".to_owned(),
            model_id: "model-a".to_owned(),
            label: "Primary".to_owned(),
            enabled: true,
        })
        .await;

    let updated = fixture
        .providers
        .update_provider(
            &provider.id,
            ProviderChanges {
                enabled: Some(false),
                ..ProviderChanges::default()
            },
        )
        .await
        .expect("update");
    assert!(!updated.provider.enabled);
    assert_eq!(updated.provider.label, "Primary", "label was not sent");
    assert_eq!(updated.provider.model_id, "model-a", "model was not sent");
    assert_eq!(updated.provider.base_url, "http://127.0.0.1:1/v1");

    let rejected = fixture
        .providers
        .update_provider(
            &provider.id,
            ProviderChanges {
                kind: Some(ProviderKind::Local),
                ..ProviderChanges::default()
            },
        )
        .await;
    let error = match rejected {
        Ok(_) => panic!("a kind change must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "E_INTERNAL");
    // And the stored row is untouched by the rejected call.
    let listed = fixture.providers.list_providers().await.expect("list");
    assert_eq!(listed[0].provider.kind, ProviderKind::OpenAiCompatible);
}

#[test]
fn the_request_dtos_validate_presence_and_name_the_field() {
    let empty_add = AddProviderRequest {
        kind: None,
        local_flavor: None,
        base_url: None,
        model_id: None,
        label: None,
        enabled: None,
    };
    assert_eq!(
        field_of(&empty_add.draft().expect_err("kind is required")),
        "kind"
    );

    let local_without_flavor = AddProviderRequest {
        kind: Some("local".to_owned()),
        local_flavor: None,
        base_url: Some("http://127.0.0.1:11434/v1".to_owned()),
        model_id: Some("model-b".to_owned()),
        label: Some("Ollama".to_owned()),
        enabled: Some(true),
    };
    assert_eq!(
        field_of(&local_without_flavor.draft().expect_err("flavour required")),
        "local_flavor"
    );

    let cloud_with_flavor = AddProviderRequest {
        kind: Some("openai_compatible".to_owned()),
        local_flavor: Some("ollama".to_owned()),
        ..local_without_flavor.clone()
    };
    assert_eq!(
        cloud_with_flavor
            .draft()
            .expect_err("cloud has no flavour")
            .code(),
        "E_INTERNAL"
    );

    let missing_enabled = AddProviderRequest {
        kind: Some("openai_compatible".to_owned()),
        local_flavor: None,
        base_url: Some("http://127.0.0.1:1/v1".to_owned()),
        model_id: Some("model-a".to_owned()),
        label: Some("Primary".to_owned()),
        enabled: None,
    };
    assert_eq!(
        field_of(&missing_enabled.draft().expect_err("enabled required")),
        "enabled"
    );

    let update = UpdateProviderRequest {
        id: None,
        kind: None,
        local_flavor: None,
        base_url: None,
        model_id: None,
        label: None,
        enabled: None,
    };
    assert_eq!(field_of(&update.id().expect_err("id required")), "id");

    let remove_key = RemoveKeyRequest { key_id: None };
    assert_eq!(
        field_of(&remove_key.key_id().expect_err("key_id required")),
        "key_id"
    );

    let edit = EditKeyRequest {
        key_id: Some("key-1".to_owned()),
        secret: None,
    };
    assert_eq!(
        field_of(&edit.parts().expect_err("secret required")),
        "secret"
    );

    let add_key = AddKeyRequest {
        provider_id: Some("provider-1".to_owned()),
        label: Some("personal".to_owned()),
        secret: None,
    };
    assert_eq!(
        field_of(&add_key.parts().expect_err("secret required")),
        "secret"
    );

    let remove_provider = RemoveProviderRequest { id: None };
    assert_eq!(
        field_of(&remove_provider.id().expect_err("id required")),
        "id"
    );
}

#[test]
fn a_key_metadata_view_carries_the_eight_fields_and_no_secret() {
    let row = KeyMetadata {
        id: KeyId::new("key-1"),
        provider_id: "provider-1".to_owned(),
        label: "personal".to_owned(),
        masked_preview: MaskedPreview::from_stored(PREVIEW),
        status: KeyStatus::Invalid,
        last_error_code: Some("E_KEY_INVALID".to_owned()),
        invalid_since: Some("2026-10-24T00:00:00Z".to_owned()),
        updated_at: "2026-10-24T00:00:01Z".to_owned(),
    };
    let value = serde_json::to_value(KeyMetadataView::from(&row)).expect("json");
    let mut names: Vec<&str> = value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "id",
            "invalid_since",
            "label",
            "last_error_code",
            "masked_preview",
            "provider_id",
            "status",
            "updated_at"
        ]
    );
    assert_eq!(value["status"], serde_json::json!("invalid"));
    assert_eq!(value["masked_preview"], serde_json::json!(PREVIEW));
}

// --------------------------------------------------- 3.12 nothing probes while idle

#[tokio::test]
async fn an_idle_app_records_no_probe() {
    let fixture = Fixture::new("idle").await;
    // Configured: one local entry and one cloud entry with a key row.
    let local = keyless_local("ollama", true);
    fixture.repo.insert_provider(&local).await.expect("insert");
    let provider = fixture
        .add_provider(ProviderDraft {
            kind: ProviderKind::OpenAiCompatible,
            local_flavor: None,
            base_url: "http://127.0.0.1:1/v1".to_owned(),
            model_id: "model-a".to_owned(),
            label: "Primary".to_owned(),
            enabled: true,
        })
        .await;
    fixture.insert_key(&provider.id, "key-1").await;

    let state = fixture.state();
    let _ = state.status().resolve().await.expect("status answers");
    let _ = state.providers().expect("wired").list_providers().await;

    assert_eq!(
        fixture.transport.probes(),
        0,
        "nothing probes without a command"
    );
    assert_eq!(
        fixture.transport.sends(),
        0,
        "nothing sends without a command"
    );
}

#[tokio::test]
async fn a_refused_probe_is_attempted_once_per_target() {
    let transport = CountingTransport::new();
    let found = aref_lib::providers::discovery::probe(&transport).await;
    assert!(found.is_empty());
    assert_eq!(
        transport.probes(),
        2,
        "one attempt per target, never a retry"
    );
}

#[test]
fn discovery_probe_has_one_call_site_and_it_is_the_command() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&source, &mut files);
    let mut call_sites = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read");
        let hits = text.matches("discovery::probe").count();
        if hits > 0 {
            call_sites.push((
                path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/"),
                hits,
            ));
        }
    }
    assert_eq!(
        call_sites,
        vec![("src/commands/providers.rs".to_owned(), 1)],
        "probe_local_providers must be the only caller of the probe"
    );
}

fn rust_files(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("read_dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

// ------------------------------------- 3.14 one policy serves a drop, a timeout, a 429

async fn schedule_for(
    fixture: &Fixture,
    base_url: &str,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> (String, Vec<Duration>) {
    let clock = Arc::new(FakeClock::new());
    let pool = KeyPool::new(
        Arc::new(ReqwestTransport::with_timeouts(
            connect_timeout,
            request_timeout,
            request_timeout,
        )) as Arc<dyn aref_lib::providers::client::Transport>,
        Arc::clone(&fixture.store) as Arc<dyn KeyStore>,
        Arc::clone(&fixture.repo),
        Arc::new(SilentSink),
        Arc::clone(&clock) as Arc<dyn Clock>,
        Arc::new(FixedJitter(Duration::MAX)),
    );
    let mut candidate = Candidate {
        provider: keyless_local("ollama", true),
        keys: Vec::new(),
    };
    // Point the local entry at the case's address: the pool only reads `base_url` here.
    candidate.provider.base_url = base_url.to_owned();

    let outcome = pool
        .execute_with(
            &[candidate],
            &ChatRequest::user("fixture text"),
            &BackoffPolicy::from_config(),
        )
        .await;
    let code = match outcome {
        Ok(text) => panic!("expected a failure, got {text:?}"),
        Err(error) => error.code().to_owned(),
    };
    (code, clock.sleeps())
}

#[tokio::test]
async fn one_policy_serves_a_drop_a_timeout_and_a_headerless_429() {
    let fixture = Fixture::new("policy").await;

    let refused = closed_port();
    let (drop_code, drop_schedule) = schedule_for(
        &fixture,
        &format!("http://{refused}/v1"),
        Duration::from_secs(2),
        Duration::from_secs(5),
    )
    .await;

    let hanging = support::MockServer::start([
        support::Response::hang(),
        support::Response::hang(),
        support::Response::hang(),
    ]);
    let (timeout_code, timeout_schedule) = schedule_for(
        &fixture,
        &hanging.base_url(),
        Duration::from_secs(2),
        Duration::from_millis(50),
    )
    .await;

    let limited = support::MockServer::start([
        support::Response::empty(429),
        support::Response::empty(429),
        support::Response::empty(429),
    ]);
    let (limited_code, limited_schedule) = schedule_for(
        &fixture,
        &limited.base_url(),
        Duration::from_secs(2),
        Duration::from_secs(5),
    )
    .await;

    assert_eq!(drop_code, "E_NETWORK_OFFLINE");
    assert_eq!(timeout_code, "E_PROVIDER_TIMEOUT");
    assert_eq!(limited_code, "E_RATE_LIMITED");
    // G-13: one curve, so the same `n` produces the same wait in all three cases.
    assert_eq!(
        drop_schedule,
        vec![Duration::from_secs(1), Duration::from_secs(2)]
    );
    assert_eq!(timeout_schedule, drop_schedule);
    assert_eq!(limited_schedule, drop_schedule);
}

// ------------------------------------------------ 3.4 and 3.20: nothing at cold start

#[tokio::test]
async fn no_client_exists_while_the_state_is_constructed_and_list_providers_answers() {
    let fixture = Fixture::new("client-lazy").await;
    let transport = Arc::new(ReqwestTransport::new());
    let providers = Arc::new(ProviderService::new(
        Arc::clone(&fixture.repo),
        Arc::clone(&transport) as Arc<dyn aref_lib::providers::client::Transport>,
        Arc::clone(&fixture.store) as Arc<dyn KeyStore>,
    ));
    let state = AppState::wired(
        fixture.status(),
        Arc::clone(&providers),
        Arc::clone(&fixture.keys),
    );
    assert_eq!(
        transport.client_builds(),
        0,
        "composing the state must not build a Client"
    );

    let listed = state
        .providers()
        .expect("wired")
        .list_providers()
        .await
        .expect("list answers");
    assert!(listed.is_empty());
    assert_eq!(
        transport.client_builds(),
        0,
        "answering list_providers must not build a Client"
    );
}

#[tokio::test]
async fn a_cold_start_reaches_idle_with_zero_outbound_attempts() {
    let fixture = Fixture::new("cold-start").await;
    // "Start": the state exists and nothing has been asked yet.
    let state = fixture.state();
    assert_eq!(fixture.transport.sends(), 0);
    assert_eq!(fixture.transport.probes(), 0);

    // "Idle": the status command answers, and no connection attempt was made on the way.
    let snapshot = state.status().resolve().await.expect("status answers");
    assert_eq!(snapshot.detector_mode.as_str(), "statistical_only");
    assert_eq!(fixture.transport.sends(), 0);
    assert_eq!(fixture.transport.probes(), 0);
}
