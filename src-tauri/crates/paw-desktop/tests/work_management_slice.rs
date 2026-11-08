//! E1-05 UI-to-SQLite integration test.
//!
//! Drives the typed desktop command layer (`paw_desktop::commands`) — the
//! exact boundary the UI will call — against real SQLite, including a
//! process-restart simulation (reopen the database in a fresh API instance)
//! and cross-workspace denial through the use-case layer. This is the exit
//! evidence of work package E1-05; the windowed Tauri run itself is manual
//! Gate E evidence and lands with the Tauri shell unit.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::ports::workspace_repository::WorkspaceRepositoryError;
use paw_application::use_cases::work_management::WorkManagementError;
// Domain contract type re-exported by the application layer (paw-desktop
// must not depend on paw-domain directly).
use paw_application::WorkspaceId;
use paw_desktop::commands::WorkManagementApi;
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    // This integration test's manifest directory is `src-tauri/crates/paw-desktop`;
    // the migrations live under `src-tauri/migrations/sqlite`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_db_dir(tag: &str) -> PathBuf {
    // Avoid thread names here: libtest thread names contain `::`, which is
    // illegal in Windows file names.
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e1-05-{tag}-{seq}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn open_migrated(db: &Path) {
    let mut conn = sqlite::open(db).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
    drop(conn);
}

#[test]
fn create_workspace_and_project_persist_across_restart() {
    let dir = temp_db_dir("restart");
    let db = dir.join("app.sqlite");

    // First launch: initialize schema, create a workspace and a project.
    let workspace_id = {
        open_migrated(&db);
        let api = WorkManagementApi::open(&db).unwrap();
        let ws = api.create_workspace("自磨工坊主空间").unwrap();
        let project = api.create_project(&ws.id, "E1 纵向切片").unwrap();
        assert_eq!(project.workspace_id, ws.id);
        ws.id
    }; // API and its connection dropped: simulates process exit.

    // Second launch: reopen and restore (roadmap E1 slice outcome).
    let project_id = {
        let api = WorkManagementApi::open(&db).unwrap();
        let projects = api.list_projects(&workspace_id).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "E1 纵向切片");
        projects[0].id.clone()
    };

    // Third launch: the workspace is still there and still owns its project.
    {
        let api = WorkManagementApi::open(&db).unwrap();
        let projects = api.list_projects(&workspace_id).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, project_id);
    }

    drop(project_id);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cross_workspace_and_missing_workspace_requests_are_denied() {
    let dir = temp_db_dir("deny");
    let db = dir.join("app.sqlite");

    open_migrated(&db);
    let api = WorkManagementApi::open(&db).unwrap();

    let ws = api.create_workspace("alpha").unwrap();
    let ghost = WorkspaceId::new_v7(); // valid id, never inserted

    // Project creation into a workspace that does not exist is a typed
    // not-found, not a constraint error.
    assert!(matches!(
        api.create_project(&ghost, "ghost project"),
        Err(WorkManagementError::Repository(
            WorkspaceRepositoryError::WorkspaceNotFound
        ))
    ));
    // Listing for an absent workspace is likewise refused.
    assert!(matches!(
        api.list_projects(&ghost),
        Err(WorkManagementError::Repository(
            WorkspaceRepositoryError::WorkspaceNotFound
        ))
    ));

    // Domain validation fires before the repository is touched.
    assert!(matches!(
        api.create_workspace("   "),
        Err(WorkManagementError::Validation(_))
    ));

    // The existing workspace still lists as empty and accepts projects.
    assert!(api.list_projects(&ws.id).unwrap().is_empty());
    assert!(api.create_project(&ws.id, "p1").is_ok());

    drop(api);
    fs::remove_dir_all(&dir).unwrap();
}
