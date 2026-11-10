//! E2-03 integration tests: scoped FTS5 search over projects and tasks.
//!
//! Evidence: index follows CRUD in-step; cross-workspace records are never
//! visible; the projection is rebuildable; user queries cannot inject FTS5
//! syntax; empty queries are typed rejections.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::ports::search::{SearchError, SearchKind, SearchPort};
use paw_application::ports::task_repository::TaskRepository;
use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::{Project, ProjectId, Task, TaskId, Workspace, WorkspaceId};
use paw_infrastructure::repositories::task_repository::SqliteTaskRepository;
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::search::SqliteSearchService;
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e2-03-{tag}-{seq}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

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

fn open_migrated(db: &Path) {
    let mut conn = sqlite::open(db).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
    drop(conn);
}

#[test]
fn search_follows_crud_and_stays_scoped() {
    let dir = temp_dir("scoped");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let tasks = SqliteTaskRepository::open(&db).unwrap();
    let search = SqliteSearchService::open(&db).unwrap();

    let w1 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "主空间".into(),
    };
    workspaces.insert_workspace(&w1).unwrap();
    let w2 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "别处".into(),
    };
    workspaces.insert_workspace(&w2).unwrap();

    let project = Project::new(ProjectId::new_v7(), w1.id.clone(), "边缘计算项目").unwrap();
    workspaces.insert_project(&w1.id, &project).unwrap();
    let task = Task::new(
        TaskId::new_v7(),
        w1.id.clone(),
        Some(project.id.clone()),
        "编写边缘同步协议",
        Some("涉及离线队列与冲突解决"),
    )
    .unwrap();
    tasks.insert_task(&task).unwrap();

    // Hits are scoped: w1 sees both, w2 sees nothing.
    let hits = search.search(&w1.id, "边缘", 20).unwrap();
    assert_eq!(hits.len(), 2);
    assert!(
        hits.iter()
            .any(|h| h.kind == SearchKind::Project && h.title == "边缘计算项目")
    );
    assert!(
        hits.iter()
            .any(|h| h.kind == SearchKind::Task && h.title == "编写边缘同步协议")
    );
    assert!(search.search(&w2.id, "边缘", 20).unwrap().is_empty());

    // Update renames the index entry in step.
    let mut renamed = project.clone();
    renamed.name = "云同步项目".into();
    workspaces.update_project(&w1.id, &renamed).unwrap();
    assert!(search.search(&w1.id, "边缘计算", 20).unwrap().is_empty());
    assert_eq!(search.search(&w1.id, "云同步", 20).unwrap().len(), 1);

    // Delete removes the hit.
    tasks.delete_task(&w1.id, &task.id).unwrap();
    assert!(search.search(&w1.id, "离线队列", 20).unwrap().is_empty());
    assert_eq!(search.search(&w1.id, "云同步", 20).unwrap().len(), 1);
    workspaces.delete_project(&w1.id, &project.id).unwrap();
    assert!(search.search(&w1.id, "云同步", 20).unwrap().is_empty());
}

#[test]
fn rebuild_restores_the_projection_from_authoritative_tables() {
    let dir = temp_dir("rebuild");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let search = SqliteSearchService::open(&db).unwrap();

    let ws = Workspace {
        id: WorkspaceId::new_v7(),
        name: "空间".into(),
    };
    workspaces.insert_workspace(&ws).unwrap();
    let project = Project::new(ProjectId::new_v7(), ws.id.clone(), "可重建投影").unwrap();
    workspaces.insert_project(&ws.id, &project).unwrap();

    // Simulate a stale/corrupted projection: wipe it manually.
    {
        let conn = sqlite::open(&db).unwrap();
        conn.execute("DELETE FROM fts_projects", []).unwrap();
        drop(conn);
    }
    assert!(search.search(&ws.id, "可重建", 20).unwrap().is_empty());

    // Rebuild restores it from the authoritative rows.
    search.rebuild().unwrap();
    assert_eq!(search.search(&ws.id, "可重建", 20).unwrap().len(), 1);
}

#[test]
fn queries_cannot_inject_fts_syntax_and_empty_queries_are_typed() {
    let dir = temp_dir("injection");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let search = SqliteSearchService::open(&db).unwrap();

    let ws = Workspace {
        id: WorkspaceId::new_v7(),
        name: "空间".into(),
    };
    workspaces.insert_workspace(&ws).unwrap();
    let project = Project::new(ProjectId::new_v7(), ws.id.clone(), "注入测试项目").unwrap();
    workspaces.insert_project(&ws.id, &project).unwrap();

    // FTS5 syntax characters are quoted into harmless literals: these return
    // results (or nothing) but never a storage error. (Pure-quote input has
    // no searchable tokens at all and is covered by the typed rejections.)
    for hostile in ["AND OR NOT", "(*)", "col:val", "a OR b"] {
        let result = search.search(&ws.id, hostile, 20);
        assert!(
            result.is_ok(),
            "hostile query {hostile:?} errored: {result:?}"
        );
    }

    // Empty / quote-only queries are typed rejections.
    assert_eq!(
        search.search(&ws.id, "   ", 20),
        Err(SearchError::InvalidQuery)
    );
    assert_eq!(
        search.search(&ws.id, "\"\" \"", 20),
        Err(SearchError::InvalidQuery)
    );
}
