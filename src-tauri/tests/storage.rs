//! Tasks 2.1, 2.2 and 2.3 — the schema, the migration guard, and the repository.
//!
//! Every assertion here runs against a real temp-file database. An in-memory database
//! would not carry the two claims that matter most: that a failed migration can be
//! undone from a file copy, and that the schema is exactly the one
//! `design.md` §8.1/§8.2 fixes.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use aref_lib::storage::db::{self, DATABASE_FILE, DATABASE_URL, SCHEMA_VERSION};
use aref_lib::storage::providers_repo::{
    KeyMetadata, KeyStatus, LocalFlavor, ProviderConfig, ProviderKind, ProvidersRepo,
    SqliteProvidersRepo,
};
use aref_lib::storage::settings_repo::SettingsRepo;
use aref_lib::storage::StorageError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tauri_plugin_sql::{Migration, MigrationKind};

static SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// A temp directory that removes itself, and the file path inside it.
struct TempDir {
    directory: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let directory = std::env::temp_dir().join(format!("aref-tests-{label}-{nanos}-{sequence}"));
        if let Err(error) = fs::create_dir_all(&directory) {
            panic!("cannot create {}: {error}", directory.display());
        }
        Self { directory }
    }

    fn database(&self) -> PathBuf {
        self.directory.join(DATABASE_FILE)
    }

    /// Opens a pool on the database in this directory, creating the file.
    async fn connect(&self) -> SqlitePool {
        connect(&self.database(), true).await
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

async fn connect(path: &Path, foreign_keys: bool) -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(foreign_keys);
    match SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
    {
        Ok(pool) => pool,
        Err(error) => panic!("cannot open {}: {error}", path.display()),
    }
}

fn ready<T, E: std::fmt::Debug>(outcome: Result<T, E>) -> T {
    match outcome {
        Ok(value) => value,
        Err(error) => panic!("fixture failed: {error:?}"),
    }
}

async fn scalar(pool: &SqlitePool, sql: &str) -> i64 {
    ready(sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await)
}

/// Runs a statement that the schema is expected to reject, and returns the error text.
async fn rejection(pool: &SqlitePool, sql: &str) -> String {
    match sqlx::query(sql).execute(pool).await {
        Ok(_) => panic!("the schema accepted a row it must reject: {sql}"),
        Err(error) => error.to_string(),
    }
}

async fn provider(repo: &dyn ProvidersRepo, id: &str) {
    ready(
        repo.insert_provider(&ProviderConfig::new(
            id,
            ProviderKind::OpenAiCompatible,
            None,
            "http://127.0.0.1:1/v1",
            "model-a",
            "Primary",
            true,
        ))
        .await,
    );
}

fn key(provider_id: &str, id: &str) -> KeyMetadata {
    KeyMetadata {
        id: aref_lib::security::keyring::KeyId::new(id),
        provider_id: provider_id.to_owned(),
        label: "personal".to_owned(),
        masked_preview: aref_lib::security::mask::MaskedPreview::from_stored("••••••••et-a"),
        status: KeyStatus::Valid,
        last_error_code: None,
        invalid_since: None,
        updated_at: aref_lib::storage::timestamp_now(),
    }
}

// ------------------------------------------------------------------ task 2.1

#[test]
fn the_migration_is_registered_as_version_one_and_up() {
    let migrations = db::migrations();
    assert_eq!(migrations.len(), 1);
    let migration = &migrations[0];
    assert_eq!(migration.version, SCHEMA_VERSION);
    assert_eq!(migration.version, 1);
    assert!(
        matches!(migration.kind, MigrationKind::Up),
        "the migration is not an Up migration"
    );
    // The SQL is inlined into the binary, so the app never reads a file to migrate
    // (design.md §8.1).
    assert!(migration.sql.contains("CREATE TABLE provider_keys"));
}

#[test]
fn the_connection_string_names_the_database_file_the_guard_copies() {
    assert_eq!(DATABASE_URL, format!("sqlite:{DATABASE_FILE}"));
    assert_eq!(
        db::pre_migration_path(Path::new("aref.db"), 1),
        PathBuf::from("aref.db.pre-1")
    );
    assert_eq!(
        db::pre_migration_path(Path::new("C:\\data\\aref.db"), 7),
        PathBuf::from("C:\\data\\aref.db.pre-7")
    );
}

#[test]
fn the_schema_carries_no_column_that_could_hold_key_material() {
    // The exact scan task 2.1 names, over the shipped SQL file.
    let sql = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/storage/migrations/0001_init.sql"),
    )
    .unwrap_or_else(|error| panic!("cannot read the migration: {error}"));
    for forbidden in [
        "secret",
        "ciphertext",
        "encrypted",
        "key_hash",
        "last_four",
        "key_length",
        "keyring_ref",
    ] {
        assert!(
            !sql.contains(forbidden),
            "{forbidden} appears in the schema"
        );
    }
    // `design.md` §8.1: the `REFERENCES` clause is documentation, and nothing relies on
    // a cascade.
    assert!(!sql.contains("ON DELETE CASCADE"));
    // The one key-derived column the schema is allowed to have.
    assert!(sql.contains("masked_preview TEXT NOT NULL"));
}

#[tokio::test]
async fn the_migration_applies_to_a_temp_file_database_and_a_second_run_is_a_no_op() {
    let fixture = TempDir::new("migrate-applies");
    let pool = fixture.connect().await;
    let path = fixture.database();

    ready(db::migrate(&pool, &path).await);
    // The copy is cleaned up when the run succeeds.
    assert!(!db::pre_migration_path(&path, SCHEMA_VERSION).exists());

    let tables = scalar(
        &pool,
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN \
         ('settings','providers','provider_keys')",
    )
    .await;
    assert_eq!(tables, 3);
    assert_eq!(
        scalar(
            &pool,
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' \
             AND name = 'provider_keys_by_provider'",
        )
        .await,
        1
    );

    // A row that must survive the second run.
    ready(
        SettingsRepo::new(pool.clone())
            .put("ui.language", "\"ar\"", SCHEMA_VERSION)
            .await,
    );

    // Re-running is a no-op: nothing is applied twice, and nothing is lost.
    ready(db::migrate(&pool, &path).await);
    assert_eq!(
        scalar(&pool, "SELECT COUNT(*) FROM _sqlx_migrations").await,
        1
    );
    assert_eq!(
        ready(SettingsRepo::new(pool.clone()).get("ui.language").await),
        Some("\"ar\"".to_owned())
    );
}

#[tokio::test]
async fn the_tables_are_strict_and_enforce_their_checks() {
    let fixture = TempDir::new("schema-checks");
    let pool = fixture.connect().await;
    let path = fixture.database();
    ready(db::migrate(&pool, &path).await);

    // STRICT: a value the column's type cannot hold is rejected at the boundary. TEXT
    // scales up to TEXT losslessly, so the direction that proves strictness is text
    // that is not an integer.
    rejection(
        &pool,
        "INSERT INTO settings (key, value, schema_version, updated_at) \
         VALUES ('k', 'x', 'not-a-number', 't')",
    )
    .await;

    // The closed enumerations and the two structural checks.
    for sql in [
        "INSERT INTO providers (id, kind, local_flavor, base_url, model_id, label, enabled, \
         updated_at) VALUES ('p', 'cloud', NULL, 'u', 'm', 'l', 1, 't')",
        "INSERT INTO providers (id, kind, local_flavor, base_url, model_id, label, enabled, \
         updated_at) VALUES ('p', 'local', NULL, 'u', 'm', 'l', 1, 't')",
        "INSERT INTO providers (id, kind, local_flavor, base_url, model_id, label, enabled, \
         updated_at) VALUES ('p', 'openai_compatible', 'ollama', 'u', 'm', 'l', 1, 't')",
        "INSERT INTO providers (id, kind, local_flavor, base_url, model_id, label, enabled, \
         updated_at) VALUES ('p', 'openai_compatible', NULL, 'u', 'm', 'l', 2, 't')",
    ] {
        rejection(&pool, sql).await;
    }

    // A provider for the key row to reference, so the `status` check is what rejects it
    // and not the foreign key.
    provider(&SqliteProvidersRepo::new(pool.clone()), "provider-1").await;
    rejection(
        &pool,
        "INSERT INTO provider_keys (id, provider_id, label, masked_preview, status, \
         last_error_code, invalid_since, updated_at) \
         VALUES ('k', 'provider-1', 'l', 'p', 'bogus', NULL, NULL, 't')",
    )
    .await;
}

// ------------------------------------------------------------------ task 2.2

#[tokio::test]
async fn a_failed_migration_leaves_the_database_byte_identical_and_the_settings_row_readable() {
    let fixture = TempDir::new("migrate-fails");
    let path = fixture.database();
    let pool = fixture.connect().await;
    ready(db::migrate(&pool, &path).await);
    ready(
        SettingsRepo::new(pool.clone())
            .put("ui.language", "\"ar\"", SCHEMA_VERSION)
            .await,
    );

    // The pre-migration state, settled on disk before anything else touches it.
    ready(
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&pool)
            .await,
    );
    let before = ready(fs::read(&path));
    let backup = db::pre_migration_path(&path, 2);
    assert!(!backup.exists());

    // A migration that cannot apply. Version 2 is test-only: `db::migrations()` carries
    // version 1 and nothing else.
    let failing = vec![Migration {
        version: 2,
        description: "test-only failing migration",
        sql: "CREATE TABLE broken (id INTEGER NOT NULL) STRICT; NOT VALID SQL;",
        kind: MigrationKind::Up,
    }];
    let outcome = db::migrate_with(&pool, &path, 2, failing).await;
    assert!(
        matches!(outcome, Err(StorageError::Migration(_))),
        "the failing migration did not fail: {outcome:?}"
    );

    // The copy is the pre-migration file, and the database is back to those bytes.
    assert!(backup.exists(), "the guard copy was not kept for recovery");
    assert_eq!(ready(fs::read(&backup)), before);
    assert_eq!(ready(fs::read(&path)), before);

    // And it is a usable database, not just identical bytes.
    let reopened = connect(&path, true).await;
    assert_eq!(
        ready(SettingsRepo::new(reopened.clone()).get("ui.language").await),
        Some("\"ar\"".to_owned())
    );
    assert_eq!(
        scalar(&reopened, "SELECT COUNT(*) FROM provider_keys").await,
        0
    );
}

#[tokio::test]
async fn a_successful_migration_leaves_no_copy_behind() {
    let fixture = TempDir::new("migrate-succeeds");
    let path = fixture.database();
    let pool = fixture.connect().await;

    ready(db::migrate(&pool, &path).await);

    assert!(!db::pre_migration_path(&path, SCHEMA_VERSION).exists());
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM providers").await, 0);
}

// ------------------------------------------------------------------ task 2.3

#[tokio::test]
async fn removing_a_provider_deletes_its_keys_and_leaves_no_orphan() {
    let fixture = TempDir::new("remove-provider");
    let pool = fixture.connect().await;
    let path = fixture.database();
    ready(db::migrate(&pool, &path).await);
    let repo = SqliteProvidersRepo::new(pool.clone());
    provider(&repo, "provider-1").await;
    provider(&repo, "provider-2").await;
    ready(repo.insert_key(&key("provider-1", "key-a")).await);
    ready(repo.insert_key(&key("provider-1", "key-b")).await);
    ready(repo.insert_key(&key("provider-2", "key-c")).await);

    let removed = ready(repo.remove_provider("provider-1").await);

    assert!(removed);
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM providers").await, 1);
    assert_eq!(
        scalar(
            &pool,
            "SELECT COUNT(*) FROM provider_keys WHERE provider_id = 'provider-1'",
        )
        .await,
        0
    );
    // No orphan survives, and the other provider's key is untouched.
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM provider_keys").await, 1);
    assert_eq!(
        scalar(
            &pool,
            "SELECT COUNT(*) FROM provider_keys WHERE provider_id NOT IN \
             (SELECT id FROM providers)",
        )
        .await,
        0
    );
}

/// The same removal with `PRAGMA foreign_keys = OFF`, which is the state the plugin's
/// own pool cannot be assumed to avoid: the explicit child delete is what does the work,
/// not a cascade.
#[tokio::test]
async fn removing_a_provider_does_not_depend_on_foreign_key_enforcement() {
    let fixture = TempDir::new("remove-provider-no-fk");
    let path = fixture.database();
    let pool = fixture.connect().await;
    ready(db::migrate(&pool, &path).await);
    ready(
        SqliteProvidersRepo::new(pool.clone())
            .insert_provider(&ProviderConfig::new(
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
    ready(
        SqliteProvidersRepo::new(pool.clone())
            .insert_key(&key("provider-1", "key-a"))
            .await,
    );

    let without_foreign_keys = connect(&path, false).await;
    assert_eq!(
        scalar(&without_foreign_keys, "PRAGMA foreign_keys").await,
        0,
        "the no-cascade case was not set up"
    );

    let removed = ready(
        SqliteProvidersRepo::new(without_foreign_keys.clone())
            .remove_provider("provider-1")
            .await,
    );

    assert!(removed);
    assert_eq!(
        scalar(&without_foreign_keys, "SELECT COUNT(*) FROM provider_keys").await,
        0
    );
    assert_eq!(
        scalar(&without_foreign_keys, "SELECT COUNT(*) FROM providers").await,
        0
    );
}

#[tokio::test]
async fn removing_an_absent_provider_reports_false_rather_than_an_error() {
    let fixture = TempDir::new("remove-absent");
    let pool = fixture.connect().await;
    let path = fixture.database();
    ready(db::migrate(&pool, &path).await);
    let repo = SqliteProvidersRepo::new(pool.clone());

    assert!(!ready(repo.remove_provider("never-existed").await));
    // Idempotent: a second removal of the same id is still false, not an error.
    assert!(!ready(repo.remove_provider("never-existed").await));
}

#[tokio::test]
async fn the_key_columns_round_trip_through_the_repository() {
    let fixture = TempDir::new("key-round-trip");
    let pool = fixture.connect().await;
    let path = fixture.database();
    ready(db::migrate(&pool, &path).await);
    let repo = SqliteProvidersRepo::new(pool.clone());
    provider(&repo, "provider-1").await;

    let mut record = key("provider-1", "key-a");
    record.status = KeyStatus::Invalid;
    record.invalid_since = Some("2026-10-24T00:00:00Z".to_owned());
    record.last_error_code = Some("E_KEY_INVALID".to_owned());
    ready(repo.insert_key(&record).await);

    let found = ready(repo.find_key(&record.id).await);
    assert_eq!(found, Some(record.clone()));
    assert_eq!(
        found.map(|value| value.masked_preview.to_string()),
        Some("••••••••et-a".to_owned())
    );

    // The enum columns are the wire values the rest of the app expects.
    let status: String = ready(
        sqlx::query_scalar("SELECT status FROM provider_keys WHERE id = 'key-a'")
            .fetch_one(&pool)
            .await,
    );
    assert_eq!(status, "invalid");
    let kind: String = ready(
        sqlx::query_scalar("SELECT kind FROM providers WHERE id = 'provider-1'")
            .fetch_one(&pool)
            .await,
    );
    assert_eq!(kind, "openai_compatible");
}

#[tokio::test]
async fn a_local_provider_round_trips_with_its_flavour() {
    let fixture = TempDir::new("local-provider");
    let pool = fixture.connect().await;
    let path = fixture.database();
    ready(db::migrate(&pool, &path).await);
    let repo = SqliteProvidersRepo::new(pool.clone());

    for (id, flavor, stored) in [
        ("local-1", LocalFlavor::OpenCode, "opencode"),
        ("local-2", LocalFlavor::Ollama, "ollama"),
        ("local-3", LocalFlavor::LmStudio, "lm_studio"),
    ] {
        ready(
            repo.insert_provider(&ProviderConfig::new(
                id,
                ProviderKind::Local,
                Some(flavor),
                "http://127.0.0.1:1234/v1",
                "model-b",
                "Local",
                true,
            ))
            .await,
        );
        let found = match ready(repo.find_provider(id).await) {
            Some(found) => found,
            None => panic!("{id} disappeared"),
        };
        assert_eq!(found.local_flavor, Some(flavor));
        let column: String = ready(
            sqlx::query_scalar("SELECT local_flavor FROM providers WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await,
        );
        assert_eq!(column, stored);
    }
    assert_eq!(ready(repo.list_providers().await).len(), 3);
}

#[tokio::test]
async fn updating_a_provider_reports_whether_the_row_existed() {
    let fixture = TempDir::new("update-provider");
    let pool = fixture.connect().await;
    let path = fixture.database();
    ready(db::migrate(&pool, &path).await);
    let repo = SqliteProvidersRepo::new(pool.clone());
    provider(&repo, "provider-1").await;

    let mut existing = match ready(repo.find_provider("provider-1").await) {
        Some(found) => found,
        None => panic!("the provider disappeared"),
    };
    existing.enabled = false;
    existing.label = "Renamed".to_owned();
    assert!(ready(repo.update_provider(&existing).await));
    let reread = match ready(repo.find_provider("provider-1").await) {
        Some(found) => found,
        None => panic!("the provider disappeared"),
    };
    assert!(!reread.enabled);
    assert_eq!(reread.label, "Renamed");

    let mut absent = existing.clone();
    absent.id = "never-existed".to_owned();
    assert!(!ready(repo.update_provider(&absent).await));
}
