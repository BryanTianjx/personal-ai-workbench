//! E2-01 integration tests: Goal/Area/Task CRUD with workspace scoping,
//! cross-workspace link denial at the constraint level, the 0003→0004
//! upgrade fixture (projects rebuild), and workspace-deletion cascade order.
//!
//! Exit evidence for the E2 vertical slice's data layer: offline CRUD,
//! scope-denial tests, and schema evolution with data preservation.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::ports::area_repository::{AreaRepository, AreaRepositoryError};
use paw_application::ports::goal_repository::{GoalRepository, GoalRepositoryError};
use paw_application::ports::task_repository::{TaskRepository, TaskRepositoryError};
use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::{
    Area, AreaId, Goal, GoalId, Project, ProjectId, ProjectStatus, Task, TaskId, TaskStatus,
    Workspace, WorkspaceId,
};
use paw_infrastructure::repositories::area_repository::SqliteAreaRepository;
use paw_infrastructure::repositories::goal_repository::SqliteGoalRepository;
use paw_infrastructure::repositories::task_repository::SqliteTaskRepository;
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e2-01-{tag}-{seq}-{}",
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
fn goal_area_task_crud_and_scope_denial() {
    let dir = temp_dir("crud");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let areas = SqliteAreaRepository::open(&db).unwrap();
    let goals = SqliteGoalRepository::open(&db).unwrap();
    let tasks = SqliteTaskRepository::open(&db).unwrap();

    let w1 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "主空间".into(),
    };
    workspaces.insert_workspace(&w1).unwrap();
    let w2 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "另一个空间".into(),
    };
    workspaces.insert_workspace(&w2).unwrap();

    // Area in w1.
    let area = Area::new(AreaId::new_v7(), w1.id.clone(), "工程", Some("建设类")).unwrap();
    areas.insert_area(&area).unwrap();

    // Goal in w1 bound to the area.
    let goal = Goal::new(
        GoalId::new_v7(),
        w1.id.clone(),
        Some(area.id.clone()),
        "发布 Alpha 0.1",
        None,
    )
    .unwrap();
    goals.insert_goal(&goal).unwrap();

    // Project in w1 linked to the goal and the area.
    let mut project = Project::new(ProjectId::new_v7(), w1.id.clone(), "E2 切片").unwrap();
    project.goal_id = Some(goal.id.clone());
    project.area_id = Some(area.id.clone());
    project.description = Some("工作管理纵向切片".into());
    workspaces.insert_project(&w1.id, &project).unwrap();

    // Task in w1 linked to the project, plus a standalone task.
    let task = Task::new(
        TaskId::new_v7(),
        w1.id.clone(),
        Some(project.id.clone()),
        "实现领域模型",
        Some("Goal/Area/Task 实体与状态机"),
    )
    .unwrap();
    tasks.insert_task(&task).unwrap();
    let solo = Task::new(TaskId::new_v7(), w1.id.clone(), None, "独立任务", None).unwrap();
    tasks.insert_task(&solo).unwrap();

    // CRUD reads: filters and workspace scoping.
    assert_eq!(areas.list_areas(&w1.id).unwrap().len(), 1, "areas of w1");
    assert!(
        areas.list_areas(&w2.id).unwrap().is_empty(),
        "no areas in w2"
    );
    assert_eq!(
        goals.list_goals(&w1.id, Some(&area.id)).unwrap().len(),
        1,
        "goals filtered by area"
    );
    assert_eq!(goals.list_goals(&w1.id, None).unwrap().len(), 1);
    assert!(goals.list_goals(&w2.id, None).unwrap().is_empty());
    assert_eq!(
        tasks
            .list_tasks(&w1.id, Some(&project.id), None)
            .unwrap()
            .len(),
        1,
        "tasks filtered by project"
    );
    assert_eq!(
        tasks
            .list_tasks(&w1.id, None, Some(TaskStatus::Todo))
            .unwrap()
            .len(),
        2,
        "both tasks are todo"
    );
    assert!(tasks.list_tasks(&w2.id, None, None).unwrap().is_empty());
    assert!(areas.find_area(&w2.id, &area.id).unwrap().is_none());
    assert!(goals.find_goal(&w2.id, &goal.id).unwrap().is_none());
    assert!(tasks.find_task(&w2.id, &task.id).unwrap().is_none());

    // Cross-workspace links are refused by the composite FKs.
    let ghost_goal = Goal::new(
        GoalId::new_v7(),
        w2.id.clone(),
        Some(area.id.clone()), // area of w1 referenced from w2
        "越界目标",
        None,
    )
    .unwrap();
    assert!(matches!(
        goals.insert_goal(&ghost_goal),
        Err(GoalRepositoryError::Invariant(_))
    ));
    let ghost_task = Task::new(
        TaskId::new_v7(),
        w2.id.clone(),
        Some(project.id.clone()), // project of w1 referenced from w2
        "越界任务",
        None,
    )
    .unwrap();
    assert!(matches!(
        tasks.insert_task(&ghost_task),
        Err(TaskRepositoryError::Invariant(_))
    ));

    // Task status transitions persist; terminal states hold.
    let mut started = task.clone();
    started.status = TaskStatus::InProgress;
    tasks.update_task(&started).unwrap();
    let mut finished = started.clone();
    finished.status = TaskStatus::Done;
    tasks.update_task(&finished).unwrap();
    let reloaded = tasks.find_task(&w1.id, &task.id).unwrap().unwrap();
    assert_eq!(reloaded.status, TaskStatus::Done);

    // Project updates: link changes and status persist.
    let mut edited = project.clone();
    edited.goal_id = None; // unlink
    edited.status = ProjectStatus::Paused;
    workspaces.update_project(&w1.id, &edited).unwrap();
    let reloaded = workspaces
        .list_projects(&w1.id)
        .unwrap()
        .into_iter()
        .find(|p| p.id == project.id)
        .unwrap();
    assert_eq!(reloaded.goal_id, None);
    assert_eq!(reloaded.status, ProjectStatus::Paused);
    assert_eq!(reloaded.area_id, Some(area.id.clone()));

    // Deleting a project that still has tasks is refused (RESTRICT, no
    // silent cascade).
    let mut reopened = edited.clone();
    reopened.status = ProjectStatus::Active;
    workspaces.update_project(&w1.id, &reopened).unwrap();
    assert!(matches!(
        workspaces.delete_project(&w1.id, &project.id),
        Err(e) if matches!(e, paw_application::ports::workspace_repository::WorkspaceRepositoryError::Invariant(ref m) if m.contains("constraint"))
    ));

    // After the task is deleted the project deletes cleanly.
    tasks.delete_task(&w1.id, &task.id).unwrap();
    workspaces.delete_project(&w1.id, &project.id).unwrap();
    assert!(workspaces.list_projects(&w1.id).unwrap().is_empty());

    // Deleting an area still referenced by a goal is refused.
    assert!(matches!(
        areas.delete_area(&w1.id, &area.id),
        Err(AreaRepositoryError::Invariant(_))
    ));
    goals.delete_goal(&w1.id, &goal.id).unwrap();
    areas.delete_area(&w1.id, &area.id).unwrap();

    // Typed not-founds.
    assert_eq!(
        tasks.delete_task(&w1.id, &task.id),
        Err(TaskRepositoryError::TaskNotFound)
    );
}

#[test]
fn upgrade_fixture_from_0003_preserves_projects() {
    let dir = temp_dir("upgrade");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");

    // Stage 1: apply only migrations 0001-0003 from a partial directory.
    let partial = dir.join("migrations-partial");
    fs::create_dir_all(&partial).unwrap();
    for entry in fs::read_dir(migrations_dir())
        .unwrap()
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("0001") || name.starts_with("0002") || name.starts_with("0003") {
            fs::copy(entry.path(), partial.join(&name)).unwrap();
        }
    }
    {
        let mut conn = sqlite::open(&db).unwrap();
        sqlite::migrate(&mut conn, &partial).unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('11111111-1111-7111-8111-111111111111', '升级前', 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, name, description, status, created_at, updated_at)
             VALUES ('22222222-2222-7222-8222-222222222222', '11111111-1111-7111-8111-111111111111', '升级前的项目', '数据保留验证', 'active', 1, 1)",
            [],
        )
        .unwrap();
    }

    // Stage 2: migrate with the full directory — 0004 rebuilds projects.
    {
        let mut conn = sqlite::open(&db).unwrap();
        sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
        let ledger: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ledger, 11);
    }

    // Stage 3: the pre-upgrade project survived with its data, and the new
    // link columns are NULL; new-model inserts work.
    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let projects = workspaces
        .list_projects(&WorkspaceId::new("11111111-1111-7111-8111-111111111111").unwrap())
        .unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "升级前的项目");
    assert_eq!(projects[0].description.as_deref(), Some("数据保留验证"));
    assert_eq!(projects[0].status, ProjectStatus::Active);
    assert_eq!(projects[0].goal_id, None);
    assert_eq!(projects[0].area_id, None);

    let areas = SqliteAreaRepository::open(&db).unwrap();
    let goals = SqliteGoalRepository::open(&db).unwrap();
    let area = Area::new(
        AreaId::new_v7(),
        WorkspaceId::new("11111111-1111-7111-8111-111111111111").unwrap(),
        "新区域",
        None,
    )
    .unwrap();
    areas.insert_area(&area).unwrap();
    let goal = Goal::new(
        GoalId::new_v7(),
        WorkspaceId::new("11111111-1111-7111-8111-111111111111").unwrap(),
        Some(area.id.clone()),
        "升级后目标",
        None,
    )
    .unwrap();
    goals.insert_goal(&goal).unwrap();
    let mut upgraded = projects[0].clone();
    upgraded.goal_id = Some(goal.id.clone());
    upgraded.area_id = Some(area.id.clone());
    workspaces
        .update_project(
            &WorkspaceId::new("11111111-1111-7111-8111-111111111111").unwrap(),
            &upgraded,
        )
        .unwrap();
}

#[test]
fn delete_workspace_cascades_in_dependency_order() {
    let dir = temp_dir("cascade");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let areas = SqliteAreaRepository::open(&db).unwrap();
    let goals = SqliteGoalRepository::open(&db).unwrap();
    let tasks = SqliteTaskRepository::open(&db).unwrap();

    let ws = Workspace {
        id: WorkspaceId::new_v7(),
        name: "将删除".into(),
    };
    workspaces.insert_workspace(&ws).unwrap();
    let area = Area::new(AreaId::new_v7(), ws.id.clone(), "区域", None).unwrap();
    areas.insert_area(&area).unwrap();
    let goal = Goal::new(
        GoalId::new_v7(),
        ws.id.clone(),
        Some(area.id.clone()),
        "目标",
        None,
    )
    .unwrap();
    goals.insert_goal(&goal).unwrap();
    let mut project = Project::new(ProjectId::new_v7(), ws.id.clone(), "项目").unwrap();
    project.goal_id = Some(goal.id.clone());
    project.area_id = Some(area.id.clone());
    workspaces.insert_project(&ws.id, &project).unwrap();
    let task = Task::new(
        TaskId::new_v7(),
        ws.id.clone(),
        Some(project.id.clone()),
        "任务",
        None,
    )
    .unwrap();
    tasks.insert_task(&task).unwrap();

    // Everything deletes atomically in child order.
    workspaces.delete_workspace(&ws.id).unwrap();
    assert!(areas.list_areas(&ws.id).unwrap().is_empty());
    assert!(goals.list_goals(&ws.id, None).unwrap().is_empty());
    assert!(workspaces.list_projects(&ws.id).unwrap().is_empty());
    assert!(tasks.list_tasks(&ws.id, None, None).unwrap().is_empty());
    assert!(workspaces.find_workspace(&ws.id).unwrap().is_none());
}
