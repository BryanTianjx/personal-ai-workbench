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

use paw_application::ports::attachment_repository::{
    Attachment, AttachmentOwner, AttachmentRepository,
};
use paw_application::ports::file_store::{FileStoreError, ManagedFileStore};
use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::{Project, ProjectId, ProjectStatus, Workspace, WorkspaceId};
use paw_infrastructure::backup::{self, BackupError};
use paw_infrastructure::filesystem::FsManagedFileStore;
use paw_infrastructure::repositories::attachment_repository::SqliteAttachmentRepository;
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

/// One attachment as it exists on a real profile: a metadata row plus its
/// managed bytes under `<dir>/attachments/<workspace-id>/<managed-uuid>`.
struct SeededAttachment {
    workspace_id: WorkspaceId,
    managed_id: String,
    path: PathBuf,
}

/// Seed a workspace, a project, one attachment metadata row and its bytes.
fn seed_attachment(dir: &Path, db: &Path) -> SeededAttachment {
    drop(open_migrated(db));
    let workspaces = SqliteWorkspaceRepository::open(db).unwrap();
    let attachments = SqliteAttachmentRepository::open(db).unwrap();
    let store = FsManagedFileStore::open(dir).unwrap();

    let ws = Workspace {
        id: WorkspaceId::new_v7(),
        name: "附件空间".into(),
    };
    workspaces.insert_workspace(&ws).unwrap();
    let project = Project {
        id: ProjectId::new_v7(),
        workspace_id: ws.id.clone(),
        goal_id: None,
        area_id: None,
        description: None,
        status: ProjectStatus::Active,
        name: "附件项目".into(),
    };
    workspaces.insert_project(&ws.id, &project).unwrap();

    let stored = store.store(&ws.id, "附件内容".as_bytes()).unwrap();
    attachments
        .insert_attachment(&Attachment {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id: ws.id.clone(),
            owner: AttachmentOwner::Project,
            owner_id: project.id.as_str().to_string(),
            managed_id: stored.managed_id.clone(),
            file_name: "说明.md".into(),
            size_bytes: stored.size_bytes,
            sha256: stored.sha256.clone(),
            created_at: 0,
        })
        .unwrap();

    let path = dir
        .join("attachments")
        .join(ws.id.as_str())
        .join(&stored.managed_id);
    SeededAttachment {
        workspace_id: ws.id,
        managed_id: stored.managed_id,
        path,
    }
}

/// A managed attachment file with no metadata row (a profile that has bytes
/// but no database yet), for "target untouched" assertions.
fn write_stray_attachment(dir: &Path) -> PathBuf {
    let path = dir
        .join("attachments")
        .join(uuid::Uuid::now_v7().to_string())
        .join(uuid::Uuid::now_v7().to_string());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"attachment bytes").unwrap();
    path
}

/// Read one `paw_backup_metadata` value straight from a snapshot file.
fn snapshot_metadata(backup_file: &Path, key: &str) -> Option<String> {
    let conn = rusqlite::Connection::open_with_flags(
        backup_file,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    conn.query_row(
        "SELECT value FROM paw_backup_metadata WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    )
    .ok()
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
            goal_id: None,
            area_id: None,
            description: None,
            status: ProjectStatus::Active,
            name: "E1 纵向切片".into(),
        };
        repo.insert_project(&ws.id, &project).unwrap();
        drop(repo);

        // Act: online backup while a connection to the WAL source is open.
        let info = backup::create_backup(&conn, &backup_file).unwrap();
        assert_eq!(info.schema_version, 11);
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
    assert_eq!(restored.schema_version, 11);
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
    let stray_attachment = write_stray_attachment(&dir);

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
    // A rejected restore must not purge existing attachment bytes either.
    assert!(stray_attachment.is_file(), "rejected restore purged bytes");
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
    assert_eq!(ledger, 11);
    drop(conn);
}

#[test]
fn backup_declares_attachment_bytes_excluded() {
    let dir = temp_dir("attachments-meta");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("app-backup.sqlite");

    let info = {
        let conn = open_migrated(&active);
        let info = backup::create_backup(&conn, &backup_file).unwrap();
        drop(conn);
        info
    };

    // The returned metadata states the guarantee, and so does the file itself.
    assert!(info.attachments_excluded);
    assert_eq!(
        snapshot_metadata(&backup_file, "attachments_excluded").as_deref(),
        Some("true")
    );
    // The remaining metadata keys are unchanged by the addition.
    assert_eq!(
        snapshot_metadata(&backup_file, "schema_version").as_deref(),
        Some("11")
    );
}

#[test]
fn restore_clears_attachment_bytes_and_keeps_metadata_rows() {
    let dir = temp_dir("attachments-purge");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("app-backup.sqlite");

    // Arrange: a real profile with one attachment (metadata row + bytes).
    let seeded = seed_attachment(&dir, &active);
    assert!(seeded.path.is_file(), "seed must put bytes on disk");
    {
        let conn = open_migrated(&active);
        backup::create_backup(&conn, &backup_file).unwrap();
        drop(conn);
    }
    // The snapshot carries the metadata row...
    {
        let snapshot = rusqlite::Connection::open_with_flags(
            &backup_file,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let rows: i64 = snapshot
            .query_row("SELECT COUNT(*) FROM attachments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1, "attachment metadata must be inside the backup");
    }
    // ...but not the bytes: they stay outside and are not readable from it.

    // Disaster: the database file is destroyed, attachment bytes survive.
    fs::remove_file(&active).unwrap();
    assert!(seeded.path.is_file());

    // Act.
    let restored = backup::restore_backup(&backup_file, &active).unwrap();
    assert!(restored.attachments_excluded);

    // Assert: metadata rows came back from the snapshot...
    let conn = sqlite::open(&active).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM attachments", [], |r| r.get(0))
        .unwrap();
    drop(conn);
    assert_eq!(rows, 1, "restored database keeps attachment metadata");

    // ...while the managed bytes were explicitly excluded and are gone:
    // restore clears the stale attachment directory instead of leaving files
    // that no restored metadata row can be matched against.
    assert!(
        !seeded.path.exists(),
        "restore must purge attachment bytes it never backed up"
    );
    let store = FsManagedFileStore::open(&dir).unwrap();
    assert!(matches!(
        store.open(&seeded.workspace_id, &seeded.managed_id),
        Err(FileStoreError::NotFound)
    ));
    let root = dir.join("attachments");
    assert!(root.is_dir(), "the managed store root is kept");
    assert_eq!(
        fs::read_dir(&root).unwrap().count(),
        0,
        "the restored profile starts with no attachment bytes"
    );
}

#[test]
fn restore_rejects_snapshot_without_attachment_exclusion_declaration() {
    let dir = temp_dir("attachments-undeclared");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("app-backup.sqlite");

    let seeded = seed_attachment(&dir, &active);
    {
        let conn = open_migrated(&active);
        backup::create_backup(&conn, &backup_file).unwrap();
        drop(conn);
    }
    // A snapshot that never declared its attachment guarantee (pre-REL-006
    // or tampered with) is not restorable.
    {
        let conn = rusqlite::Connection::open(&backup_file).unwrap();
        conn.execute(
            "DELETE FROM paw_backup_metadata WHERE key = 'attachments_excluded'",
            [],
        )
        .unwrap();
    }
    fs::remove_file(&active).unwrap();

    let result = backup::restore_backup(&backup_file, &active);
    assert!(
        matches!(result, Err(BackupError::MissingAttachmentsExcluded)),
        "expected MissingAttachmentsExcluded, got {result:?}"
    );
    if let Err(e) = &result {
        assert!(
            e.to_string().contains("attachments_excluded"),
            "the rejection must name the missing declaration: {e}"
        );
    }
    assert!(!active.exists());
    assert!(seeded.path.is_file());
}

#[test]
fn restore_rejects_snapshot_claiming_attachment_bytes_included() {
    let dir = temp_dir("attachments-declared-false");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("app-backup.sqlite");

    let seeded = seed_attachment(&dir, &active);
    {
        let conn = open_migrated(&active);
        backup::create_backup(&conn, &backup_file).unwrap();
        drop(conn);
    }
    {
        let conn = rusqlite::Connection::open(&backup_file).unwrap();
        conn.execute(
            "UPDATE paw_backup_metadata SET value = 'false' WHERE key = 'attachments_excluded'",
            [],
        )
        .unwrap();
    }
    fs::remove_file(&active).unwrap();

    let result = backup::restore_backup(&backup_file, &active);
    assert!(
        matches!(
            result,
            Err(BackupError::AttachmentsNotExcluded { ref value }) if value == "false"
        ),
        "expected AttachmentsNotExcluded, got {result:?}"
    );
    assert!(!active.exists());
    assert!(seeded.path.is_file());
}
