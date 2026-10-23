//! The database file: the migration list, and the copy that guards a migration run.
//!
//! `FR-404` asks that an older schema is upgraded without data loss, and `design.md`
//! §8.3 fixes how: before applying pending migrations, copy the database file aside as
//! `aref.db.pre-<version>`, delete the copy when the run succeeds, and restore it when
//! the run fails. SQLite's DDL is transactional, so a failed migration also rolls back
//! inside the file; the copy is the belt for the case where it does not.
//!
//! The migration list is the same `Vec<tauri_plugin_sql::Migration>` the plugin
//! registers, so the plugin's startup run and the run here agree on the
//! `_sqlx_migrations` row — including the checksum sqlx validates on a second run.
//! Running it here is what makes the copy in §8.3 possible at all: the plugin's own
//! migrator is reached only from inside its `setup`, so nothing else could observe the
//! error that decides between "delete the copy" and "restore it".

use std::ffi::OsString;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use sqlx::error::BoxDynError;
use sqlx::migrate::{Migration as SqlxMigration, MigrationSource, Migrator};
use sqlx::SqlitePool;
use tauri_plugin_sql::{Migration, MigrationKind};

use crate::storage::StorageError;

/// The database file's name.
pub const DATABASE_FILE: &str = "aref.db";

/// The connection string the plugin is configured with. It must name [`DATABASE_FILE`],
/// which a test in `tests/storage.rs` asserts.
pub const DATABASE_URL: &str = "sqlite:aref.db";

/// The one migration this build carries.
pub const SCHEMA_VERSION: i64 = 1;

/// The migration list, in the shape `tauri-plugin-sql` registers.
pub fn migrations() -> Vec<Migration> {
    vec![Migration {
        version: SCHEMA_VERSION,
        description: "init",
        sql: include_str!("migrations/0001_init.sql"),
        kind: MigrationKind::Up,
    }]
}

/// `aref.db.pre-<version>`, beside the database.
pub fn pre_migration_path(db_path: &Path, version: i64) -> PathBuf {
    let mut name = db_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| OsString::from(DATABASE_FILE));
    name.push(format!(".pre-{version}"));
    db_path.with_file_name(name)
}

/// Brings the schema up to date, with the copy of §8.3 around the run.
pub async fn migrate(pool: &SqlitePool, db_path: &Path) -> Result<(), StorageError> {
    migrate_with(pool, db_path, SCHEMA_VERSION, migrations()).await
}

/// The same flow with an explicit list, so a test can drive a failing migration through
/// the same code path the real one takes.
pub async fn migrate_with(
    pool: &SqlitePool,
    db_path: &Path,
    version: i64,
    migrations: Vec<Migration>,
) -> Result<(), StorageError> {
    let backup = capture_backup(pool, db_path, version).await?;

    match apply(pool, migrations).await {
        Ok(()) => {
            if let Some(backup) = backup {
                // A leftover copy is harmless — the next run overwrites it — so a
                // failure to clean up must not turn a good migration into a bad start.
                let _ = fs::remove_file(backup);
            }
            Ok(())
        }
        Err(error) => {
            if let Some(backup) = backup {
                // Close first: replacing the file under a live connection would leave
                // that connection reading a database it no longer has.
                pool.close().await;
                restore_backup(db_path, &backup)?;
            }
            Err(error)
        }
    }
}

/// Copies the database aside, or reports that there was nothing to copy.
async fn capture_backup(
    pool: &SqlitePool,
    db_path: &Path,
    version: i64,
) -> Result<Option<PathBuf>, StorageError> {
    if !db_path.exists() {
        // A fresh install has no file to protect, and no data to lose.
        return Ok(None);
    }

    // Fold a write-ahead log into the main file before copying it, or the copy is not
    // the database: the WAL holds committed pages the file alone does not.
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await?;

    let backup = pre_migration_path(db_path, version);
    fs::copy(db_path, &backup)?;
    Ok(Some(backup))
}

/// Puts the copy back over the database file.
fn restore_backup(db_path: &Path, backup: &Path) -> Result<(), StorageError> {
    fs::copy(backup, db_path)?;
    Ok(())
}

/// Applies the list through sqlx's migrator, exactly as the plugin does.
async fn apply(pool: &SqlitePool, migrations: Vec<Migration>) -> Result<(), StorageError> {
    let migrator = Migrator::new(RegisteredMigrations(migrations)).await?;
    migrator.run(pool).await?;
    Ok(())
}

/// The plugin's `Migration` list in the shape sqlx's migrator consumes.
///
/// The mapping is copied from `tauri-plugin-sql`'s own `MigrationSource`: only
/// [`MigrationKind::Up`] entries run, and they become `ReversibleUp` with `no_tx`
/// false. Keeping it identical is what makes the two appliers agree on the row they
/// write to `_sqlx_migrations`.
#[derive(Debug)]
struct RegisteredMigrations(Vec<Migration>);

impl MigrationSource<'static> for RegisteredMigrations {
    fn resolve(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<SqlxMigration>, BoxDynError>> + Send + 'static>>
    {
        Box::pin(async move {
            let resolved = self
                .0
                .into_iter()
                .filter(|migration| matches!(migration.kind, MigrationKind::Up))
                .map(|migration| {
                    SqlxMigration::new(
                        migration.version,
                        migration.description.into(),
                        migration.kind.into(),
                        migration.sql.into(),
                        false,
                    )
                })
                .collect();
            Ok(resolved)
        })
    }
}

/// A temp-file database for the tests that need real bytes on disk.
///
/// In-memory databases are not enough for two of the checks in this slice: the
/// migration guard is about file contents, and "no key material on disk" is a statement
/// about a file. Keeping the file handling here is also what confines `std::fs` to this
/// module (`tests/composition_inert.rs`).
#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use sqlx::SqlitePool;

    use crate::storage::StorageError;

    static SEQUENCE: AtomicU32 = AtomicU32::new(0);

    /// A unique path in the system temp directory, removed when the value is dropped.
    pub(crate) struct TempDatabase {
        directory: PathBuf,
    }

    impl TempDatabase {
        pub(crate) fn new(label: &str) -> Self {
            let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default();
            // No thread name in the path: the test harness names threads
            // `module::test`, and `:` is not a filename character on Windows.
            let directory =
                std::env::temp_dir().join(format!("aref-storage-{label}-{nanos}-{sequence}"));
            Self { directory }
        }

        pub(crate) fn path(&self) -> PathBuf {
            self.directory.join(super::DATABASE_FILE)
        }

        pub(crate) async fn connect(&self) -> Result<(SqlitePool, PathBuf), StorageError> {
            use std::str::FromStr;

            use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

            fs::create_dir_all(&self.directory)?;
            let path = self.path();
            // `create_if_missing`, because `Pool::connect` alone refuses to create a
            // file and this fixture exists to make one.
            let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))?
                .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(2)
                .connect_with(options)
                .await?;
            Ok((pool, path))
        }
    }

    impl Drop for TempDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.directory);
        }
    }

    /// The file's bytes, for a scan that must find no key material (`NFR-201`).
    pub(crate) fn database_bytes(path: &Path) -> Result<Vec<u8>, std::io::Error> {
        fs::read(path)
    }
}
