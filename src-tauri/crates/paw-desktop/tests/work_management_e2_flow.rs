//! E2-02 UI-to-SQLite integration test: the full Work Management flow the
//! E2 UI drives — workspace picker, areas, goals, linked projects, tasks
//! with status transitions — including restart persistence and typed
//! denials, all through the Tauri-free command layer.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::use_cases::work_management::WorkManagementError;
use paw_application::{ProjectStatus, TaskStatus};
use paw_desktop::commands::WorkManagementApi;
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_db_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e2-02-{tag}-{seq}-{}",
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
fn full_work_management_flow_persists_across_restart() {
    let dir = temp_db_dir("flow");
    let db = dir.join("app.sqlite");

    // Session 1: build the full model.
    let (area_id, goal_id, project_id, task_id) = {
        open_migrated(&db);
        let api = WorkManagementApi::open(&db).unwrap();

        // Workspace picker: starts empty, gains one workspace.
        assert!(api.list_workspaces().unwrap().is_empty());
        let ws = api.create_workspace("自磨工坊").unwrap();
        assert_eq!(api.list_workspaces().unwrap().len(), 1);

        let area = api.create_area(&ws.id, "工程", None).unwrap();
        let goal = api
            .create_goal(&ws.id, Some(area.id.clone()), "发布 Alpha 0.1", None)
            .unwrap();
        let project = api.create_project(&ws.id, "E2 切片").unwrap();
        let linked = api
            .update_project(
                &ws.id,
                &project.id,
                "E2 切片",
                Some("工作管理纵向切片"),
                ProjectStatus::Active,
                Some(goal.id.clone()),
                Some(area.id.clone()),
            )
            .unwrap();
        assert_eq!(linked.goal_id, Some(goal.id.clone()));

        let task = api
            .create_task(&ws.id, Some(project.id.clone()), "实现领域模型", None)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Todo);
        api.transition_task(&ws.id, &task.id, TaskStatus::InProgress)
            .unwrap();

        // Rework is legal; completion is terminal. From `done`, any move is
        // a typed state-machine rejection.
        api.transition_task(&ws.id, &task.id, TaskStatus::Todo)
            .unwrap();
        api.transition_task(&ws.id, &task.id, TaskStatus::InProgress)
            .unwrap();
        api.transition_task(&ws.id, &task.id, TaskStatus::Done)
            .unwrap();
        assert_eq!(
            api.transition_task(&ws.id, &task.id, TaskStatus::Todo),
            Err(WorkManagementError::TaskTransition {
                from: TaskStatus::Done,
                to: TaskStatus::Todo,
            })
        );

        (area.id, goal.id, project.id, task.id)
    }; // API dropped: simulated process exit.

    // Session 2: the workspace picker finds the workspace again (Gate E
    // gap repaired), and every record restored with its links and status.
    {
        let api = WorkManagementApi::open(&db).unwrap();
        let workspaces = api.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 1);
        let ws = &workspaces[0];

        assert_eq!(api.list_areas(&ws.id).unwrap().len(), 1);
        assert_eq!(api.list_goals(&ws.id, None).unwrap().len(), 1);
        let projects = api.list_projects(&ws.id).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].goal_id, Some(goal_id));
        assert_eq!(projects[0].area_id, Some(area_id));
        let tasks = api.list_tasks(&ws.id, Some(&project_id), None).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Done);
        assert_eq!(tasks[0].id, task_id);

        // Dashboard (PAW-F-004): real counts, zero-filled status rows.
        let summary = api.dashboard(&ws.id).unwrap();
        assert_eq!(summary.area_count, 1);
        assert_eq!(summary.goal_count, 1);
        assert_eq!(summary.attachment_count, 0);
        assert_eq!(
            summary
                .projects_by_status
                .iter()
                .find(|(s, _)| s == "active"),
            Some(&("active".into(), 1))
        );
        assert_eq!(
            summary.tasks_by_status.iter().find(|(s, _)| s == "done"),
            Some(&("done".into(), 1))
        );
        assert_eq!(
            summary.tasks_by_status.iter().find(|(s, _)| s == "todo"),
            Some(&("todo".into(), 0))
        );
    }

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn operation_queue_commands_work_through_the_api() {
    let dir = temp_db_dir("e3-queue");
    let db = dir.join("app.sqlite");

    open_migrated(&db);
    let api = WorkManagementApi::open(&db).unwrap();
    let ws = api.create_workspace("队列空间").unwrap();

    // Seed one workspace-scoped operation directly (E1 helpers) and see it
    // in the queue view.
    let new_op = paw_infrastructure::operations::NewOperation {
        kind: "export.data".into(),
        owner_domain: "work_management".into(),
        owner_type: "workspace".into(),
        owner_id: ws.id.as_str().to_string(),
        workspace_id: Some(ws.id.as_str().to_string()),
        actor_id: "local-user".into(),
        correlation_id: "corr-e3".into(),
        causation_id: None,
        cancellation_support: paw_infrastructure::operations::CancellationSupport::Supported,
    };
    let op = {
        let conn = rusqlite::Connection::open(&db).unwrap();
        paw_infrastructure::operations::create_operation(&conn, &new_op).unwrap()
    };
    let listed = api.list_operations(&ws.id).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, op.id);

    // Cancellation request through the gated command layer.
    let requested = api.request_operation_cancellation(&ws.id, &op.id).unwrap();
    assert!(requested.cancellation_requested_at.is_some());

    // Export writes a temp JSON artifact and returns its path.
    let path = api.export_operations_to_temp(&ws.id).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("export.data"));
}
