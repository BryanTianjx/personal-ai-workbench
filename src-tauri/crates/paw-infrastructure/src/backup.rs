//! Backup and restore for the authoritative local database (E1-06).
//!
//! Contract (`E1-sqlite-schema-migrations-workspace-isolation.md` §5):
//! - the SQLite online backup API is used; a live WAL database file is never
//!   copied as-is;
//! - the snapshot carries its schema version, creation timestamp and
//!   application version inside a `paw_backup_metadata` table, so a backup is
//!   one self-describing, portable file;
//! - restore validates `integrity_check` and `foreign_key_check` on a
//!   same-volume temporary copy before atomically replacing the active
//!   database; stale `-wal`/`-shm` sidecars of the replaced database are
//!   removed (a restored database must never inherit the old WAL);
//! - the caller must hold no open connections to the active database while
//!   restoring (Windows keeps the file locked).

use std::path::Path;
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags, params};

use crate::sqlite::{self, Error};

/// Metadata recorded inside every backup snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    /// Highest applied `schema_migrations` version at backup time.
    pub schema_version: i64,
    /// Snapshot creation time (UTC epoch milliseconds).
    pub created_at: i64,
    /// Version of the application that produced the snapshot.
    pub app_version: String,
}

/// Alias of [`BackupInfo`]: a successful restore yields the same metadata.
pub type RestoreInfo = BackupInfo;

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("io: {source}")]
    Io { source: std::io::Error },
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("clock: {source}")]
    NowMsFailed { source: std::time::SystemTimeError },
    #[error("integrity check failed: {0}")]
    Integrity(String),
    #[error("backup file is missing the paw_backup_metadata table")]
    MissingMetadata,
}

impl From<Error> for BackupError {
    fn from(e: Error) -> Self {
        match e {
            Error::Io { source } => BackupError::Io { source },
            Error::NowMsFailed { source } => BackupError::NowMsFailed { source },
            Error::Sql(source) => BackupError::Sql(source),
            Error::ChecksumMismatch {
                version,
                expected,
                found,
            } => BackupError::Integrity(format!(
                "checksum mismatch for migration {version}: expected {expected}, found {found}"
            )),
        }
    }
}

/// Highest applied migration version in `conn` (0 when the ledger is absent).
fn schema_version_of(conn: &Connection) -> Result<i64, BackupError> {
    let present: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)?;
    if !present {
        return Ok(0);
    }
    let max: Option<i64> =
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })?;
    Ok(max.unwrap_or(0))
}

/// Run `PRAGMA integrity_check` and require the literal `ok` verdict.
fn require_integrity(conn: &Connection) -> Result<(), BackupError> {
    let verdict: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if verdict != "ok" {
        return Err(BackupError::Integrity(verdict));
    }
    Ok(())
}

/// Require `PRAGMA foreign_key_check` to report no violations.
fn require_no_fk_violations(conn: &Connection) -> Result<(), BackupError> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = stmt.query([])?;
    if rows.next()?.is_some() {
        return Err(BackupError::Integrity(
            "foreign_key_check reported violations".into(),
        ));
    }
    Ok(())
}

/// Read the `paw_backup_metadata` table from a snapshot.
fn read_metadata(conn: &Connection) -> Result<BackupInfo, BackupError> {
    // A valid SQLite file without our metadata table (e.g. a foreign copy)
    // fails the SELECT with an SQL error, not an empty result — check for
    // the table explicitly so the rejection stays typed.
    let table_present: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'paw_backup_metadata'",
        [],
        |r| r.get(0),
    )?;
    if table_present == 0 {
        return Err(BackupError::MissingMetadata);
    }
    let load = |key: &str| -> Result<String, BackupError> {
        conn.query_row(
            "SELECT value FROM paw_backup_metadata WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => BackupError::MissingMetadata,
            other => other.into(),
        })
    };
    Ok(BackupInfo {
        schema_version: load("schema_version")?.parse().unwrap_or(0),
        created_at: load("created_at")?.parse().unwrap_or(0),
        app_version: load("app_version")?,
    })
}

/// Create a consistent snapshot of `source` at `dest_path` (online backup
/// API), stamp it with metadata, switch it to a rollback journal for
/// portability and verify its integrity. Any pre-existing file at
/// `dest_path` (and its SQLite sidecars) is removed first.
pub fn create_backup(source: &Connection, dest_path: &Path) -> Result<BackupInfo, BackupError> {
    let schema_version = schema_version_of(source)?;
    let created_at = sqlite::now_ms()?;

    for stale in [
        dest_path.to_path_buf(),
        sidecar_path(dest_path, "-wal"),
        sidecar_path(dest_path, "-shm"),
    ] {
        match std::fs::remove_file(&stale) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(BackupError::Io { source: e }),
        }
    }

    let mut dest = Connection::open(dest_path)?;
    Backup::new(source, &mut dest)?.run_to_completion(64, Duration::from_millis(5), None)?;

    dest.execute_batch(
        "CREATE TABLE paw_backup_metadata (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )?;
    dest.execute(
        "INSERT INTO paw_backup_metadata (key, value) VALUES (?1, ?2)",
        params!["schema_version", schema_version.to_string()],
    )?;
    dest.execute(
        "INSERT INTO paw_backup_metadata (key, value) VALUES (?1, ?2)",
        params!["created_at", created_at.to_string()],
    )?;
    dest.execute(
        "INSERT INTO paw_backup_metadata (key, value) VALUES (?1, ?2)",
        params!["app_version", env!("CARGO_PKG_VERSION")],
    )?;

    // Force a rollback journal and a clean checkpoint so the snapshot is one
    // portable file (open() re-enables WAL for the active database).
    dest.pragma_update(None, "journal_mode", "DELETE")?;
    require_integrity(&dest)?;
    require_no_fk_violations(&dest)?;
    drop(dest);

    Ok(BackupInfo {
        schema_version,
        created_at,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Restore `backup_path` over `active_path`:
///
/// 1. validate the backup (read-only): integrity, foreign keys, metadata;
/// 2. copy it to a same-volume temporary path and re-validate the copy;
/// 3. remove stale `-wal`/`-shm` sidecars of the active database;
/// 4. atomically rename the validated copy over the active database.
///
/// The caller must hold no open connections to `active_path`.
pub fn restore_backup(backup_path: &Path, active_path: &Path) -> Result<RestoreInfo, BackupError> {
    let info = validate_snapshot(backup_path)?;

    let temp = active_path.with_extension("restore-tmp");
    std::fs::copy(backup_path, &temp).map_err(|e| BackupError::Io { source: e })?;
    let result = validate_snapshot(&temp);
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }

    for sidecar in [
        sidecar_path(active_path, "-wal"),
        sidecar_path(active_path, "-shm"),
    ] {
        match std::fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                let _ = std::fs::remove_file(&temp);
                return Err(BackupError::Io { source: e });
            }
        }
    }

    std::fs::rename(&temp, active_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        BackupError::Io { source: e }
    })?;

    Ok(info)
}

/// Validate a snapshot file read-only: integrity, foreign keys, metadata.
fn validate_snapshot(path: &Path) -> Result<RestoreInfo, BackupError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    require_integrity(&conn)?;
    require_no_fk_violations(&conn)?;
    let info = read_metadata(&conn)?;
    drop(conn);
    Ok(info)
}

/// Path of a SQLite sidecar file (`-wal` / `-shm`) for `db_path`.
fn sidecar_path(db_path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut os = db_path.as_os_str().to_os_string();
    os.push(suffix);
    std::path::PathBuf::from(os)
}
