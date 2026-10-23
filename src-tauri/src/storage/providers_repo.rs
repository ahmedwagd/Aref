//! Providers and their key metadata, as hand-written SQL (design.md §8.1, §8.2).
//!
//! Two things this file deliberately does not do:
//!
//! * it never computes a `masked_preview` — the column is written once by
//!   `providers::keys` at save time and only ever read back here (`G-27`), which is what
//!   makes "listing keys performs zero `KeyStore::read` calls" true at the repo level;
//! * it never relies on `ON DELETE CASCADE` — `PRAGMA foreign_keys` is per-connection and
//!   the pool belongs to the plugin, so `remove_provider` deletes both sets of rows in
//!   one explicit transaction (design.md §8.1).

use std::future::Future;
use std::pin::Pin;

use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::security::keyring::KeyId;
use crate::security::mask::MaskedPreview;
use crate::storage::StorageError;

/// A repository call, boxed so the trait stays usable as `Arc<dyn ProvidersRepo>`
/// (design.md §3.3) without an `async-trait` dependency.
pub type RepoFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, StorageError>> + Send + 'a>>;

/// `providers.kind` (SRS §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAiCompatible,
    Local,
}

impl ProviderKind {
    fn as_db(self) -> &'static str {
        match self {
            ProviderKind::OpenAiCompatible => "openai_compatible",
            ProviderKind::Local => "local",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        match value {
            "openai_compatible" => Some(ProviderKind::OpenAiCompatible),
            "local" => Some(ProviderKind::Local),
            _ => None,
        }
    }
}

/// `providers.local_flavor` — the discriminator the `FR-312` order needs (SRS §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFlavor {
    OpenCode,
    Ollama,
    LmStudio,
}

impl LocalFlavor {
    fn as_db(self) -> &'static str {
        match self {
            LocalFlavor::OpenCode => "opencode",
            LocalFlavor::Ollama => "ollama",
            LocalFlavor::LmStudio => "lm_studio",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        match value {
            "opencode" => Some(LocalFlavor::OpenCode),
            "ollama" => Some(LocalFlavor::Ollama),
            "lm_studio" => Some(LocalFlavor::LmStudio),
            _ => None,
        }
    }
}

/// `provider_keys.status` (SRS §5.3). `invalid` means the provider rejected the
/// credential (`FR-305`); an unreadable keyring entry is *not* this (`G-20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStatus {
    Valid,
    Invalid,
}

impl KeyStatus {
    fn as_db(self) -> &'static str {
        match self {
            KeyStatus::Valid => "valid",
            KeyStatus::Invalid => "invalid",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        match value {
            "valid" => Some(KeyStatus::Valid),
            "invalid" => Some(KeyStatus::Invalid),
            _ => None,
        }
    }
}

/// One `providers` row (SRS §5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub kind: ProviderKind,
    pub local_flavor: Option<LocalFlavor>,
    pub base_url: String,
    pub model_id: String,
    pub label: String,
    pub enabled: bool,
    pub updated_at: String,
}

/// One `provider_keys` row (SRS §5.3).
///
/// `masked_preview` is a [`MaskedPreview`], not a `String`: the only way to build one
/// from storage is [`MaskedPreview::from_stored`], so the type records that this value
/// was read back rather than recomputed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMetadata {
    pub id: KeyId,
    pub provider_id: String,
    pub label: String,
    pub masked_preview: MaskedPreview,
    pub status: KeyStatus,
    pub last_error_code: Option<String>,
    pub invalid_since: Option<String>,
    pub updated_at: String,
}

/// Providers and key metadata, at the level of rows.
pub trait ProvidersRepo: Send + Sync {
    fn list_providers(&self) -> RepoFuture<'_, Vec<ProviderConfig>>;
    fn find_provider(&self, id: &str) -> RepoFuture<'_, Option<ProviderConfig>>;
    fn insert_provider(&self, provider: &ProviderConfig) -> RepoFuture<'_, ()>;
    fn update_provider(&self, provider: &ProviderConfig) -> RepoFuture<'_, bool>;
    fn remove_provider(&self, id: &str) -> RepoFuture<'_, bool>;

    /// Every key row. This is the `list_providers` key-metadata path: it reads
    /// `masked_preview` from the column and touches no credential store (`G-27`).
    fn list_keys(&self) -> RepoFuture<'_, Vec<KeyMetadata>>;
    fn find_key(&self, id: &KeyId) -> RepoFuture<'_, Option<KeyMetadata>>;
    fn insert_key(&self, key: &KeyMetadata) -> RepoFuture<'_, ()>;
    fn update_key(&self, key: &KeyMetadata) -> RepoFuture<'_, bool>;
    fn remove_key(&self, id: &KeyId) -> RepoFuture<'_, bool>;
}

/// The SQLite implementation. Every statement is written out in this file: the schema is
/// small and fixed, and a query builder would hide the column list that
/// `design.md` §8.2 constrains.
pub struct SqliteProvidersRepo {
    pool: SqlitePool,
}

impl SqliteProvidersRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

const PROVIDER_COLUMNS: &str =
    "id, kind, local_flavor, base_url, model_id, label, enabled, updated_at";

const KEY_COLUMNS: &str =
    "id, provider_id, label, masked_preview, status, last_error_code, invalid_since, updated_at";

fn text(row: &SqliteRow, column: &str) -> Result<String, StorageError> {
    Ok(row.try_get::<String, _>(column)?)
}

fn provider_from_row(row: &SqliteRow) -> Result<ProviderConfig, StorageError> {
    let kind = ProviderKind::from_db(&text(row, "kind")?).ok_or_else(|| {
        StorageError::Database(sqlx::Error::Protocol(
            "a providers row carries an unknown kind".to_owned(),
        ))
    })?;
    let local_flavor = match row.try_get::<Option<String>, _>("local_flavor")? {
        Some(value) => Some(LocalFlavor::from_db(&value).ok_or_else(|| {
            StorageError::Database(sqlx::Error::Protocol(
                "a providers row carries an unknown local_flavor".to_owned(),
            ))
        })?),
        None => None,
    };
    Ok(ProviderConfig {
        id: text(row, "id")?,
        kind,
        local_flavor,
        base_url: text(row, "base_url")?,
        model_id: text(row, "model_id")?,
        label: text(row, "label")?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        updated_at: text(row, "updated_at")?,
    })
}

fn key_from_row(row: &SqliteRow) -> Result<KeyMetadata, StorageError> {
    let status = KeyStatus::from_db(&text(row, "status")?).ok_or_else(|| {
        StorageError::Database(sqlx::Error::Protocol(
            "a provider_keys row carries an unknown status".to_owned(),
        ))
    })?;
    Ok(KeyMetadata {
        id: KeyId::new(text(row, "id")?),
        provider_id: text(row, "provider_id")?,
        label: text(row, "label")?,
        masked_preview: MaskedPreview::from_stored(text(row, "masked_preview")?),
        status,
        last_error_code: row.try_get::<Option<String>, _>("last_error_code")?,
        invalid_since: row.try_get::<Option<String>, _>("invalid_since")?,
        updated_at: text(row, "updated_at")?,
    })
}

impl ProvidersRepo for SqliteProvidersRepo {
    fn list_providers(&self) -> RepoFuture<'_, Vec<ProviderConfig>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let sql = format!("SELECT {PROVIDER_COLUMNS} FROM providers ORDER BY id");
            let rows = sqlx::query(&sql).fetch_all(&pool).await?;
            rows.iter().map(provider_from_row).collect()
        })
    }

    fn find_provider(&self, id: &str) -> RepoFuture<'_, Option<ProviderConfig>> {
        let id = id.to_owned();
        let pool = self.pool.clone();
        Box::pin(async move {
            let sql = format!("SELECT {PROVIDER_COLUMNS} FROM providers WHERE id = ?");
            let row = sqlx::query(&sql).bind(&id).fetch_optional(&pool).await?;
            row.as_ref().map(provider_from_row).transpose()
        })
    }

    fn insert_provider(&self, provider: &ProviderConfig) -> RepoFuture<'_, ()> {
        let provider = provider.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO providers (id, kind, local_flavor, base_url, model_id, label, \
                 enabled, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&provider.id)
            .bind(provider.kind.as_db())
            .bind(provider.local_flavor.map(LocalFlavor::as_db))
            .bind(&provider.base_url)
            .bind(&provider.model_id)
            .bind(&provider.label)
            .bind(provider.enabled)
            .bind(&provider.updated_at)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn update_provider(&self, provider: &ProviderConfig) -> RepoFuture<'_, bool> {
        let provider = provider.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            let updated = sqlx::query(
                "UPDATE providers SET kind = ?, local_flavor = ?, base_url = ?, model_id = ?, \
                 label = ?, enabled = ?, updated_at = ? WHERE id = ?",
            )
            .bind(provider.kind.as_db())
            .bind(provider.local_flavor.map(LocalFlavor::as_db))
            .bind(&provider.base_url)
            .bind(&provider.model_id)
            .bind(&provider.label)
            .bind(provider.enabled)
            .bind(&provider.updated_at)
            .bind(&provider.id)
            .execute(&pool)
            .await?
            .rows_affected()
                > 0;
            Ok(updated)
        })
    }

    fn remove_provider(&self, id: &str) -> RepoFuture<'_, bool> {
        let id = id.to_owned();
        let pool = self.pool.clone();
        Box::pin(async move {
            // Explicit child-first deletes in one transaction: no orphan
            // `provider_keys` row survives, with or without the foreign-key pragma.
            let mut transaction = pool.begin().await?;
            sqlx::query("DELETE FROM provider_keys WHERE provider_id = ?")
                .bind(&id)
                .execute(&mut *transaction)
                .await?;
            let removed = sqlx::query("DELETE FROM providers WHERE id = ?")
                .bind(&id)
                .execute(&mut *transaction)
                .await?
                .rows_affected()
                > 0;
            transaction.commit().await?;
            // An absent id is a value, not an error: the §7 catalog has no not-found
            // code (design.md §4.2).
            Ok(removed)
        })
    }

    fn list_keys(&self) -> RepoFuture<'_, Vec<KeyMetadata>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let sql = format!("SELECT {KEY_COLUMNS} FROM provider_keys ORDER BY provider_id, id");
            let rows = sqlx::query(&sql).fetch_all(&pool).await?;
            rows.iter().map(key_from_row).collect()
        })
    }

    fn find_key(&self, id: &KeyId) -> RepoFuture<'_, Option<KeyMetadata>> {
        let id = id.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            let sql = format!("SELECT {KEY_COLUMNS} FROM provider_keys WHERE id = ?");
            let row = sqlx::query(&sql)
                .bind(id.to_string())
                .fetch_optional(&pool)
                .await?;
            row.as_ref().map(key_from_row).transpose()
        })
    }

    fn insert_key(&self, key: &KeyMetadata) -> RepoFuture<'_, ()> {
        let key = key.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO provider_keys (id, provider_id, label, masked_preview, status, \
                 last_error_code, invalid_since, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(key.id.to_string())
            .bind(&key.provider_id)
            .bind(&key.label)
            .bind(key.masked_preview.to_string())
            .bind(key.status.as_db())
            .bind(&key.last_error_code)
            .bind(&key.invalid_since)
            .bind(&key.updated_at)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    fn update_key(&self, key: &KeyMetadata) -> RepoFuture<'_, bool> {
        let key = key.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            let updated = sqlx::query(
                "UPDATE provider_keys SET provider_id = ?, label = ?, masked_preview = ?, \
                 status = ?, last_error_code = ?, invalid_since = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&key.provider_id)
            .bind(&key.label)
            .bind(key.masked_preview.to_string())
            .bind(key.status.as_db())
            .bind(&key.last_error_code)
            .bind(&key.invalid_since)
            .bind(&key.updated_at)
            .bind(key.id.to_string())
            .execute(&pool)
            .await?
            .rows_affected()
                > 0;
            Ok(updated)
        })
    }

    fn remove_key(&self, id: &KeyId) -> RepoFuture<'_, bool> {
        let id = id.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            let removed = sqlx::query("DELETE FROM provider_keys WHERE id = ?")
                .bind(id.to_string())
                .execute(&pool)
                .await?
                .rows_affected()
                > 0;
            Ok(removed)
        })
    }
}

/// A provider row for the tests and for `add_provider`, with the column defaults the
/// schema has no defaults for.
impl ProviderConfig {
    pub fn new(
        id: impl Into<String>,
        kind: ProviderKind,
        local_flavor: Option<LocalFlavor>,
        base_url: impl Into<String>,
        model_id: impl Into<String>,
        label: impl Into<String>,
        enabled: bool,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            local_flavor,
            base_url: base_url.into(),
            model_id: model_id.into(),
            label: label.into(),
            enabled,
            updated_at: crate::storage::timestamp_now(),
        }
    }
}
