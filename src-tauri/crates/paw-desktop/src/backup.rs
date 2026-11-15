//! Desktop backup/restore boundary (PAW-AUDIT-002 remediation).
//!
//! Wires the infrastructure backup engine (`paw_infrastructure::backup`) into
//! the desktop application:
//!
//! - backups live in a managed `<app_data_dir>/backups/` directory; the
//!   frontend only ever refers to a backup by **file name**, never by path
//!   (07-ipc-integration-contract.md §8: never trust frontend-supplied paths);
//! - restore is staged, not immediate: `request_restore` validates the
//!   snapshot and copies it next to the active database; the restore is
//!   applied by [`apply_pending_restore`] during the next startup bootstrap —
//!   the only point where the desktop app provably holds no open connections
//!   to the active database (Windows locks the file otherwise);
//! - a rejected snapshot removes the staged copy and keeps the existing
//!   database untouched (fail-open to current data, never to a broken state);
//! - the REL-006 owner decision stands: snapshots never carry attachment
//!   bytes, and a successful restore clears the managed attachment directory.
//!   The UI must state this in the restore confirmation.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use paw_infrastructure::backup::{self, BackupError, BackupInfo, RestoreInfo};
use serde::Serialize;

/// Serialized snapshot metadata (snake_case across the IPC boundary).
#[derive(Debug, Clone, Serialize)]
pub struct BackupInfoDto {
    pub schema_version: i64,
    pub created_at: i64,
    pub app_version: String,
    pub attachments_excluded: bool,
}

impl From<BackupInfo> for BackupInfoDto {
    fn from(info: BackupInfo) -> Self {
        Self {
            schema_version: info.schema_version,
            created_at: info.created_at,
            app_version: info.app_version,
            attachments_excluded: info.attachments_excluded,
        }
    }
}

/// One backup file in the managed directory.
#[derive(Debug, Clone, Serialize)]
pub struct BackupSnapshotDto {
    pub file_name: String,
    pub size_bytes: u64,
    pub info: BackupInfoDto,
}

/// A listed backup file; `valid = false` marks an unreadable/corrupt file.
#[derive(Debug, Clone, Serialize)]
pub struct BackupFileMetaDto {
    pub file_name: String,
    pub size_bytes: u64,
    pub modified_at: Option<i64>,
    pub info: Option<BackupInfoDto>,
    pub valid: bool,
}

/// Whether a staged restore is waiting to be applied at next startup.
#[derive(Debug, Clone, Serialize)]
pub struct PendingRestoreDto {
    pub pending: bool,
}

/// Failure surface for the desktop backup layer.
#[derive(Debug)]
pub enum BackupFailure {
    /// The active database or the requested backup file does not exist.
    NotFound,
    /// The snapshot failed validation (integrity, foreign keys or metadata).
    Invalid,
    /// Anything else (filesystem level).
    Io(std::io::Error),
}

impl std::fmt::Display for BackupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupFailure::NotFound => write!(f, "backup or database not found"),
            BackupFailure::Invalid => write!(f, "backup snapshot failed validation"),
            BackupFailure::Io(e) => write!(f, "backup io failure: {e}"),
        }
    }
}

fn classify(error: BackupError) -> BackupFailure {
    match error {
        BackupError::Io { source } => {
            if source.kind() == std::io::ErrorKind::NotFound {
                BackupFailure::NotFound
            } else {
                BackupFailure::Io(source)
            }
        }
        // Validation-grade failures mean the snapshot itself is unusable.
        BackupError::Sql(_)
        | BackupError::Integrity(_)
        | BackupError::MissingMetadata
        | BackupError::MissingAttachmentsExcluded
        | BackupError::AttachmentsNotExcluded { .. } => BackupFailure::Invalid,
        BackupError::NowMsFailed { .. } => BackupFailure::Invalid,
    }
}

fn now_label() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default()
}

fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn pending_restore_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("restore-pending")
}

/// Reject a frontend-supplied backup file name: generated names are plain
/// `backup-<epoch-ms>.sqlite`; anything carrying separators, `..` or control
/// characters is refused instead of resolved (path-traversal guard).
fn is_safe_backup_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && !name.chars().any(|c| c.is_control())
        && name.ends_with(".sqlite")
}

fn managed_backup_path(data_dir: &Path, name: &str) -> Result<PathBuf, BackupFailure> {
    if !is_safe_backup_name(name) {
        return Err(BackupFailure::NotFound);
    }
    let path = backups_dir(data_dir).join(name);
    if !path.is_file() {
        return Err(BackupFailure::NotFound);
    }
    Ok(path)
}

/// Create a snapshot of the active database in the managed backups directory.
///
/// Opens its own read-only connection: the online backup API takes a
/// consistent snapshot even while the coordination connection is serving
/// requests (WAL readers do not block).
pub fn create_backup_into(
    db_path: &Path,
    data_dir: &Path,
) -> Result<BackupSnapshotDto, BackupFailure> {
    if !db_path.is_file() {
        return Err(BackupFailure::NotFound);
    }
    let source =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| classify(BackupError::Sql(e)))?;

    let dir = backups_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(BackupFailure::Io)?;
    let file_name = format!("backup-{}.sqlite", now_label());
    let dest = dir.join(&file_name);

    let info = backup::create_backup(&source, &dest).map_err(classify)?;
    let size_bytes = std::fs::metadata(&dest)
        .map(|m| m.len())
        .map_err(BackupFailure::Io)?;
    Ok(BackupSnapshotDto {
        file_name,
        size_bytes,
        info: info.into(),
    })
}

/// List the managed backups, newest first. Corrupt or foreign files are
/// listed with `valid = false` instead of being hidden (the user should see
/// that something unusable exists in the backups directory).
pub fn list_managed_backups(data_dir: &Path) -> Vec<BackupFileMetaDto> {
    let dir = backups_dir(data_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut items: Vec<BackupFileMetaDto> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let name = path.file_name()?.to_str()?.to_string();
            if !name.ends_with(".sqlite") {
                return None;
            }
            let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
            let modified_at = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64);
            let info = match backup::validate_snapshot(&path) {
                Ok(info) => Some(BackupInfoDto::from(info)),
                Err(_) => None,
            };
            let valid = info.is_some();
            Some(BackupFileMetaDto {
                file_name: name,
                size_bytes,
                modified_at,
                info,
                valid,
            })
        })
        .collect();
    items.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    items
}

/// Validate the chosen snapshot and stage it next to the active database.
/// Nothing else is touched: the existing database is replaced only by
/// [`apply_pending_restore`] at the next startup.
pub fn stage_restore(db_path: &Path, backup_path: &Path) -> Result<RestoreInfo, BackupFailure> {
    if !db_path.is_file() {
        return Err(BackupFailure::NotFound);
    }
    let info = backup::validate_snapshot(backup_path).map_err(classify)?;
    let pending = pending_restore_path(db_path);
    std::fs::copy(backup_path, &pending).map_err(BackupFailure::Io)?;
    Ok(info)
}

/// Outcome of applying a staged restore at bootstrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingRestoreOutcome {
    /// The snapshot validated, replaced the active database and attachments
    /// were cleared.
    Restored,
    /// A staged file was present but failed validation: the staged copy was
    /// removed and the existing database was left untouched.
    Rejected,
    /// Nothing was staged.
    NoPending,
}

/// Apply the staged restore (if any) before any connection opens. Boot-time
/// callers surface `Err` as a blocking startup error — at that point the
/// snapshot passed validation but the filesystem refused the swap.
pub fn apply_pending_restore(db_path: &Path) -> Result<PendingRestoreOutcome, BackupFailure> {
    let pending = pending_restore_path(db_path);
    if !pending.is_file() {
        return Ok(PendingRestoreOutcome::NoPending);
    }
    match backup::restore_backup(&pending, db_path) {
        Ok(_info) => {
            let _ = std::fs::remove_file(&pending);
            Ok(PendingRestoreOutcome::Restored)
        }
        Err(e) => {
            // Validation-grade failure: the original database is untouched
            // (restore_backup validates before replacing). Remove the staged
            // copy and keep booting on the current data.
            let _ = std::fs::remove_file(&pending);
            match classify(e) {
                BackupFailure::Io(source) => Err(BackupFailure::Io(source)),
                _ => Ok(PendingRestoreOutcome::Rejected),
            }
        }
    }
}

/// Whether a staged restore is waiting (Settings banner).
pub fn pending_status(db_path: &Path) -> PendingRestoreDto {
    PendingRestoreDto {
        pending: pending_restore_path(db_path).is_file(),
    }
}

pub fn cancel_staged_restore(db_path: &Path) -> PendingRestoreDto {
    let _ = std::fs::remove_file(pending_restore_path(db_path));
    PendingRestoreDto { pending: false }
}

fn map_failure(failure: BackupFailure) -> crate::commands::CommandError {
    match failure {
        BackupFailure::NotFound => crate::commands::CommandError::BackupNotFound,
        BackupFailure::Invalid => crate::commands::CommandError::BackupInvalid,
        BackupFailure::Io(_) => crate::commands::CommandError::Unexpected,
    }
}

fn data_dir_of(app: &tauri::AppHandle) -> Result<PathBuf, crate::commands::CommandError> {
    use tauri::Manager;
    app.path()
        .app_data_dir()
        .map_err(|_| crate::commands::CommandError::Unexpected)
}

fn active_db_path(app: &tauri::AppHandle) -> Result<PathBuf, crate::commands::CommandError> {
    Ok(data_dir_of(app)?.join("app.sqlite"))
}

// -- Tauri commands -----------------------------------------------------------

#[tauri::command]
pub fn create_backup(
    app: tauri::AppHandle,
) -> Result<BackupSnapshotDto, crate::commands::CommandError> {
    let data_dir = data_dir_of(&app)?;
    create_backup_into(&active_db_path(&app)?, &data_dir).map_err(map_failure)
}

#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> Vec<BackupFileMetaDto> {
    match data_dir_of(&app) {
        Ok(data_dir) => list_managed_backups(&data_dir),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub fn read_backup_info(
    app: tauri::AppHandle,
    backup_name: String,
) -> Result<BackupInfoDto, crate::commands::CommandError> {
    let data_dir = data_dir_of(&app)?;
    let path = managed_backup_path(&data_dir, &backup_name).map_err(map_failure)?;
    backup::validate_snapshot(&path)
        .map(BackupInfoDto::from)
        .map_err(|e| map_failure(classify(e)))
}

/// Validate and stage the restore, then restart the shell: the staged
/// snapshot is applied during the next bootstrap, before any connection
/// opens. The frontend must not assume this invoke resolves.
#[tauri::command]
pub fn request_restore(
    app: tauri::AppHandle,
    backup_name: String,
) -> Result<PendingRestoreDto, crate::commands::CommandError> {
    let data_dir = data_dir_of(&app)?;
    let path = managed_backup_path(&data_dir, &backup_name).map_err(map_failure)?;
    stage_restore(&active_db_path(&app)?, &path).map_err(map_failure)?;
    app.restart();
}

#[tauri::command]
pub fn pending_restore_status(app: tauri::AppHandle) -> PendingRestoreDto {
    match active_db_path(&app) {
        Ok(db_path) => pending_status(&db_path),
        Err(_) => PendingRestoreDto { pending: false },
    }
}

#[tauri::command]
pub fn cancel_pending_restore(app: tauri::AppHandle) -> PendingRestoreDto {
    match active_db_path(&app) {
        Ok(db_path) => cancel_staged_restore(&db_path),
        Err(_) => PendingRestoreDto { pending: false },
    }
}
