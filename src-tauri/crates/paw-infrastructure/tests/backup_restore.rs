//! E1-06 backup/restore integration tests (exit evidence).
//!
//! Covers the required evidence from
//! `E1-sqlite-schema-migrations-workspace-isolation.md` §5/§7 on real
//! Windows files: snapshot round-trip with data intact, embedded metadata,
//! rejection of corrupt backups, and restore-then-migrate idempotence. The
//! full offline matrix cases (OFF-009/OFF-010) remain manual Gate E
//! evidence; the cross-version restore drill gains a second migration with
//! 0002 (E1-07/E1-08 unit).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::{Project, ProjectId, Workspace, WorkspaceId};
use paw_infrastructure::backup::{self, BackupError};
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    // No thread names here: libtest thread names contain `::`, illegal in
    // Windows file names.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e1-06-{tag}-{seq}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Guard removing the whole temp directory on drop, once all connections
/// have been released by the test body.
struct TempDirGuard {
    dir: PathBuf,
}

impl TempDirGuard {
    fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn open_migrated(db: &Path) -> rusqlite::Connection {
    let mut conn = sqlite::open(db).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
    conn
}

#[test]
fn backup_round_trip_restores_workspaces_and_projects() {
    let dir = temp_dir("roundtrip");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("backups").join("app-backup.sqlite");
    fs::create_dir_all(backup_file.parent().unwrap()).unwrap();

    // Arrange: migrated database; seed one workspace with one project.
    let (ws, project) = {
        let conn = open_migrated(&active);
        let repo = SqliteWorkspaceRepository::open(&active).unwrap();
        let ws = Workspace {
            id: WorkspaceId::new_v7(),
            name: "自磨工坊主空间".into(),
        };
        repo.insert_workspace(&ws).unwrap();
        let project = Project {
            id: ProjectId::new_v7(),
            workspace_id: ws.id.clone(),
            name: "E1 纵向切片".into(),
        };
        repo.insert_project(&ws.id, &project).unwrap();
        drop(repo);

        // Act: online backup while a connection to the WAL source is open.
        let info = backup::create_backup(&conn, &backup_file).unwrap();
        assert_eq!(info.schema_version, 1);
        assert_eq!(info.app_version, env!("CARGO_PKG_VERSION"));
        drop(conn);
        (ws, project)
    };

    // Disaster: the active database and its sidecars are destroyed.
    fs::remove_file(&active).unwrap();
    let _ = fs::remove_file(active.with_extension("sqlite-wal"));
    let _ = fs::remove_file(active.with_extension("sqlite-shm"));

    // Act: restore into the emptied location.
    let restored = backup::restore_backup(&backup_file, &active).unwrap();
    assert_eq!(restored.schema_version, 1);
    assert_eq!(restored.app_version, env!("CARGO_PKG_VERSION"));

    // Assert: data survived; integrity/fk checks ran inside restore, and
    // re-running migrations is a no-op (checksum of 0001 still matches).
    let repo = SqliteWorkspaceRepository::open(&active).unwrap();
    let conn = open_migrated(&active);
    drop(conn);

    let found = repo.find_workspace(&ws.id).unwrap();
    assert_eq!(found.as_ref().map(|w| &w.name), Some(&ws.name));
    let projects = repo.list_projects(&ws.id).unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project.id);
    assert_eq!(projects[0].name, project.name);
}

#[test]
fn restore_rejects_corrupt_backup_file() {
    let dir = temp_dir("corrupt");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("corrupt.sqlite");

    fs::write(&backup_file, b"this is not a sqlite database at all").unwrap();
    let result = backup::restore_backup(&backup_file, &active);
    assert!(
        matches!(
            result,
            Err(BackupError::Sql(_)) | Err(BackupError::Integrity(_))
        ),
        "expected a typed rejection, got {result:?}"
    );
    // The active location must not have been touched.
    assert!(!active.exists());
}

#[test]
fn restore_requires_backup_metadata_table() {
    let dir = temp_dir("nometa");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("foreign.sqlite");

    // A valid SQLite file without our metadata table (e.g. a foreign copy).
    let conn = open_migrated(&backup_file);
    drop(conn);
    let result = backup::restore_backup(&backup_file, &active);
    assert!(
        matches!(result, Err(BackupError::MissingMetadata)),
        "expected MissingMetadata, got {result:?}"
    );
}

#[test]
fn restored_database_migrates_idempotently() {
    let dir = temp_dir("migrate");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("app-backup.sqlite");

    // Backup a freshly migrated database.
    {
        let conn = open_migrated(&active);
        backup::create_backup(&conn, &backup_file).unwrap();
        drop(conn);
    }

    // Restore into a brand-new location (schema v1, ledger included).
    let target = dir.join("restored").join("app.sqlite");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    backup::restore_backup(&backup_file, &target).unwrap();

    // Re-running migrations must apply nothing new and preserve the ledger.
    let mut conn = sqlite::open(&target).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
    let ledger: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ledger, 1);
    drop(conn);
}
