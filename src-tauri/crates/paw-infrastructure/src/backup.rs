//! Backup and restore for the authoritative local database (E1-06).
//!
//! Contract (`E1-sqlite-schema-migrations-workspace-isolation.md` §5):
//! - the SQLite online backup API is used; a live WAL database file is never
//!   copied as-is;
//! - the snapshot carries its schema version, creation timestamp,
//!   application version and its attachment guarantee inside a
//!   `paw_backup_metadata` table, so a backup is one self-describing,
//!   portable file;
//! - **attachment bytes are explicitly excluded from a snapshot** (REL-006,
//!   owner decision). Managed attachment files live outside the database in
//!   `<app_data_dir>/attachments/<workspace-id>/<managed-uuid>` (E2-04
//!   `filesystem`), so copying `app.sqlite` cannot copy them. Every snapshot
//!   declares `attachments_excluded = true`; a snapshot without that
//!   declaration is refused rather than silently restored into a state where
//!   attachment metadata points at missing bytes. Because the bytes are never
//!   part of a backup, [`restore_backup`] clears the managed attachment
//!   directory of the restored profile instead of leaving orphaned files that
//!   no metadata row refers to;
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

/// Metadata table written into every snapshot.
const BACKUP_METADATA_TABLE: &str = "paw_backup_metadata";

/// Metadata key declaring whether attachment bytes are part of the snapshot.
const ATTACHMENTS_EXCLUDED_KEY: &str = "attachments_excluded";

/// The only value this format writes and accepts for
/// [`ATTACHMENTS_EXCLUDED_KEY`]: attachment bytes are never copied.
const ATTACHMENTS_EXCLUDED_VALUE: &str = "true";

/// Directory (next to the active database) holding managed attachment bytes.
const ATTACHMENTS_DIR: &str = "attachments";

/// Metadata recorded inside every backup snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    /// Highest applied `schema_migrations` version at backup time.
    pub schema_version: i64,
    /// Snapshot creation time (UTC epoch milliseconds).
    pub created_at: i64,
    /// Version of the application that produced the snapshot.
    pub app_version: String,
    /// `true` whenever attachment bytes are excluded from the snapshot.
    ///
    /// This module never copies managed attachment files into a snapshot, so
    /// the value is always `true` for a backup it produced; it is recorded and
    /// validated so the exclusion is an explicit, checkable guarantee rather
    /// than an assumption (REL-006).
    pub attachments_excluded: bool,
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
    #[error(
        "backup file does not declare `{ATTACHMENTS_EXCLUDED_KEY}`: this build refuses snapshots whose \
         attachment-backup guarantee is unknown"
    )]
    MissingAttachmentsExcluded,
    #[error(
        "backup file declares `{ATTACHMENTS_EXCLUDED_KEY} = {value}`, but this backup format never \
         carries attachment bytes"
    )]
    AttachmentsNotExcluded { value: String },
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
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![BACKUP_METADATA_TABLE],
        |r| r.get(0),
    )?;
    if table_present == 0 {
        return Err(BackupError::MissingMetadata);
    }
    let load = |key: &str| -> Result<String, BackupError> {
        conn.query_row(
            &format!("SELECT value FROM {BACKUP_METADATA_TABLE} WHERE key = ?1"),
            params![key],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => BackupError::MissingMetadata,
            other => other.into(),
        })
    };

    // The attachment guarantee must be declared, not assumed (REL-006): a
    // snapshot without the key, or one claiming the bytes are included, is
    // refused before anything is restored.
    let attachments_excluded = load(ATTACHMENTS_EXCLUDED_KEY).map_err(|e| match e {
        BackupError::MissingMetadata => BackupError::MissingAttachmentsExcluded,
        other => other,
    })?;
    if attachments_excluded != ATTACHMENTS_EXCLUDED_VALUE {
        return Err(BackupError::AttachmentsNotExcluded {
            value: attachments_excluded,
        });
    }

    Ok(BackupInfo {
        schema_version: load("schema_version")?.parse().unwrap_or(0),
        created_at: load("created_at")?.parse().unwrap_or(0),
        app_version: load("app_version")?,
        attachments_excluded: true,
    })
}

/// Write one `key`/`value` row into a snapshot's metadata table.
fn stamp_metadata(conn: &Connection, key: &str, value: &str) -> Result<(), BackupError> {
    conn.execute(
        &format!("INSERT INTO {BACKUP_METADATA_TABLE} (key, value) VALUES (?1, ?2)"),
        params![key, value],
    )?;
    Ok(())
}

/// Create a consistent snapshot of `source` at `dest_path` (online backup
/// API), stamp it with metadata, switch it to a rollback journal for
/// portability and verify its integrity. Any pre-existing file at
/// `dest_path` (and its SQLite sidecars) is removed first.
///
/// Attachment bytes are **not** part of the snapshot (REL-006): managed files
/// live outside the database under `attachments/`, so copying the database
/// cannot include them. The snapshot records `attachments_excluded = true` so
/// this guarantee travels with the file.
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

    dest.execute_batch(&format!(
        "CREATE TABLE {BACKUP_METADATA_TABLE} (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )"
    ))?;
    for (key, value) in [
        ("schema_version", schema_version.to_string()),
        ("created_at", created_at.to_string()),
        ("app_version", env!("CARGO_PKG_VERSION").to_string()),
        // Self-describing attachment guarantee: the snapshot holds the
        // database only, never the managed attachment bytes under
        // `attachments/`.
        (
            ATTACHMENTS_EXCLUDED_KEY,
            ATTACHMENTS_EXCLUDED_VALUE.to_string(),
        ),
    ] {
        stamp_metadata(&dest, key, &value)?;
    }

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
        attachments_excluded: true,
    })
}

/// Restore `backup_path` over `active_path`:
///
/// 1. validate the backup (read-only): integrity, foreign keys, metadata
///    (including the `attachments_excluded = true` declaration);
/// 2. copy it to a same-volume temporary path and re-validate the copy;
/// 3. remove stale `-wal`/`-shm` sidecars of the active database;
/// 4. atomically rename the validated copy over the active database;
/// 5. clear the managed attachment directory next to `active_path`.
///
/// Step 5 enforces the owner decision behind the `attachments_excluded`
/// guarantee (REL-006): a snapshot never carries attachment bytes, so the
/// attachment files of the profile being restored are stale by definition —
/// their metadata rows come from the snapshot, their bytes cannot. Removing
/// them keeps metadata and bytes from silently disagreeing. The `attachments/`
/// root itself is kept (empty); restored profiles must re-add attachment
/// files.
///
/// Validation happens before anything is touched: a rejected snapshot leaves
/// both the active database and the attachment directory untouched. Step 5
/// runs after the database swap (the commit point), so a cleanup failure
/// never destroys the pre-restore attachments of a restore that did not
/// happen.
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

    clear_managed_attachments(active_path)?;

    Ok(info)
}

/// Remove every entry under `<active_path's directory>/attachments`, the
/// managed attachment root of the profile (E2-04). The root directory itself
/// is kept so the managed file store keeps working; only its contents (stale
/// workspace directories, managed files and temp files) are deleted. A
/// missing root is already the desired end state.
fn clear_managed_attachments(active_path: &Path) -> Result<(), BackupError> {
    let Some(data_dir) = active_path.parent() else {
        // A bare file name has no sibling directory to clean.
        return Ok(());
    };
    let root = data_dir.join(ATTACHMENTS_DIR);
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(BackupError::Io { source: e }),
    };
    for entry in entries {
        let entry = entry.map_err(|e| BackupError::Io { source: e })?;
        let path = entry.path();
        // `file_type` does not follow symlinks: a link is unlinked, never
        // followed into whatever it points at.
        let file_type = entry
            .file_type()
            .map_err(|e| BackupError::Io { source: e })?;
        let removed = if file_type.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        removed.map_err(|e| BackupError::Io { source: e })?;
    }
    Ok(())
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
