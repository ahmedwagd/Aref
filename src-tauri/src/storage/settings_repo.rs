//! The `settings` table (SRS §5.1).
//!
//! No command in this change reads or writes a setting — `G-12`, the settings inventory,
//! is still open — so this repository exists for two reasons only: `FR-404`'s "preserve
//! existing settings records" needs a row to seed and read back after a migration, and
//! the table is part of the schema `design.md` §8.1 fixes.
//!
//! `value` is JSON text and is never key material (`NFR-201`, design.md §8.2). Nothing
//! here parses it: a repository that understands setting values would be a settings
//! service, and this change has none.

use sqlx::{Row, SqlitePool};

use crate::storage::{timestamp_now, StorageError};

pub struct SettingsRepo {
    pool: SqlitePool,
}

impl SettingsRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// The stored JSON text, or `None` when the key was never set.
    pub async fn get(&self, key: &str) -> Result<Option<String>, StorageError> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(row) => Ok(Some(row.try_get::<String, _>("value")?)),
            None => Ok(None),
        }
    }

    /// Inserts or replaces one setting.
    pub async fn put(
        &self,
        key: &str,
        value: &str,
        schema_version: i64,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO settings (key, value, schema_version, updated_at) VALUES (?, ?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, \
             schema_version = excluded.schema_version, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(schema_version)
        .bind(timestamp_now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
