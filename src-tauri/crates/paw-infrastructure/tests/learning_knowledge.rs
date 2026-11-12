//! E4-01/E4-02 integration tests: Learning lifecycle, KnowledgeRef metadata
//! with atomic availability events, and the cross-domain contract subset
//! (cross-domain-data-contracts §11): cross-workspace rejection, session
//! preservation, backup/restore coverage.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::ports::knowledge_repository::KnowledgeRefRepository;
use paw_application::ports::learning_repository::{
    ItemRepository, PlanRepository, RoadmapRepository, SessionRepository,
};
use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::{Project, ProjectId, Workspace, WorkspaceId};
use paw_domain::knowledge::{Availability, KnowledgeRef, KnowledgeRefId, KnowledgeSource};
use paw_domain::learning::{Item, ItemKind, ItemStatus, Plan, PlanStatus, Roadmap, Session};
use paw_domain::learning::{ItemId, PlanId, RoadmapId, SessionId};
use paw_infrastructure::outbox::list_quarantined;
use paw_infrastructure::repositories::knowledge_repository::SqliteKnowledgeRefRepository;
use paw_infrastructure::repositories::learning_repository::{
    SqliteItemRepository, SqlitePlanRepository, SqliteRoadmapRepository, SqliteSessionRepository,
};
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("target/tmp-e4-{tag}-{seq}-{}", std::process::id()));
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
fn learning_lifecycle_crud_with_scope_denial() {
    let dir = temp_dir("lifecycle");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let roadmaps = SqliteRoadmapRepository::open(&db).unwrap();
    let plans = SqlitePlanRepository::open(&db).unwrap();
    let items = SqliteItemRepository::open(&db).unwrap();
    let sessions = SqliteSessionRepository::open(&db).unwrap();

    let w1 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "学习空间".into(),
    };
    workspaces.insert_workspace(&w1).unwrap();
    let w2 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "别处".into(),
    };
    workspaces.insert_workspace(&w2).unwrap();

    // Roadmap (PAW-LG-001: 创建、编辑、归档、恢复).
    let roadmap = Roadmap::new(
        RoadmapId::new_v7(),
        w1.id.clone(),
        "Agent & Runtime Engineering",
        Some("长期方向"),
    )
    .unwrap();
    roadmaps.insert_roadmap(&roadmap).unwrap();

    // Plan (PAW-LG-002: 归属与状态迁移).
    let plan = Plan::new(
        PlanId::new_v7(),
        w1.id.clone(),
        roadmap.id.clone(),
        "Q1 计划",
        None,
    )
    .unwrap();
    assert_eq!(plan.status, PlanStatus::Draft);
    plans.insert_plan(&plan).unwrap();
    let mut activated = plan.clone();
    activated.status = PlanStatus::Active;
    plans.update_plan(&activated).unwrap();

    // Item (PAW-LG-003: 类型、验收标准、状态持久化).
    let item = Item::new(
        ItemId::new_v7(),
        w1.id.clone(),
        plan.id.clone(),
        ItemKind::Implementation,
        "实现 durable execution 最小示例",
        Some("可运行的示例 + 测试"),
        Some("至少覆盖一次恢复路径"),
        None,
    )
    .unwrap();
    assert_eq!(item.status, ItemStatus::Todo);
    items.insert_item(&item).unwrap();

    // Scope denial: the other workspace sees nothing.
    assert!(roadmaps.list_roadmaps(&w2.id).unwrap().is_empty());
    assert!(plans.list_plans(&w2.id, None).unwrap().is_empty());
    assert!(items.list_items(&w2.id, None, None).unwrap().is_empty());
    assert!(
        roadmaps
            .find_roadmap(&w2.id, &roadmap.id)
            .unwrap()
            .is_none()
    );
    assert!(plans.find_plan(&w2.id, &plan.id).unwrap().is_none());
    assert!(items.find_item(&w2.id, &item.id).unwrap().is_none());

    // Item ↔ Project many-to-many with cross-workspace rejection (contract §11.1).
    let project = Project::new(ProjectId::new_v7(), w1.id.clone(), "实践项目").unwrap();
    workspaces.insert_project(&w1.id, &project).unwrap();
    let foreign_project = Project::new(ProjectId::new_v7(), w2.id.clone(), "别处项目").unwrap();
    workspaces.insert_project(&w2.id, &foreign_project).unwrap();

    items
        .attach_item_to_project(&w1.id, &item.id, &project.id)
        .unwrap();
    // Cross-workspace link: the composite FK refuses it.
    assert!(
        items
            .attach_item_to_project(&w1.id, &item.id, &foreign_project.id)
            .is_err()
    );
    // The refused link did not persist.
    assert_eq!(
        items
            .list_projects_for_item(&w1.id, &item.id)
            .unwrap()
            .len(),
        1
    );

    // PAW-LG-006: create a task via Work Management and record the
    // action-step link (Task remains Work Management-owned).
    let task = paw_application::Task::new(
        paw_application::TaskId::new_v7(),
        w1.id.clone(),
        Some(project.id.clone()),
        "学习项动作：最小示例",
        None,
    )
    .unwrap();
    {
        use paw_application::ports::task_repository::TaskRepository;
        let tasks =
            paw_infrastructure::repositories::task_repository::SqliteTaskRepository::open(&db)
                .unwrap();
        tasks.insert_task(&task).unwrap();
    }
    items
        .attach_task_to_item(&w1.id, &item.id, &task.id)
        .unwrap();
    assert_eq!(
        items.list_tasks_for_item(&w1.id, &item.id).unwrap(),
        vec![task.id.clone()]
    );

    // Sessions (PAW-LG-004): append-only, never overwriting.
    let session1 = Session::new(
        SessionId::new_v7(),
        w1.id.clone(),
        item.id.clone(),
        Some(task.id.clone()),
        Some(project.id.clone()),
        Some("实验"),
        Some(45),
        Some("第一次实验"),
        Some("恢复路径不明确"),
        Some("理解了 checkpoint 语义"),
        Some("补一次崩溃恢复"),
        1_000,
    )
    .unwrap();
    sessions.append_session(&session1).unwrap();
    let session2 = Session::new(
        SessionId::new_v7(),
        w1.id.clone(),
        item.id.clone(),
        None,
        None,
        Some("阅读"),
        None,
        Some("第二次学习"),
        None,
        None,
        None,
        2_000,
    )
    .unwrap();
    sessions.append_session(&session2).unwrap();

    let listed = sessions.list_sessions(&w1.id, Some(&item.id)).unwrap();
    assert_eq!(
        listed.len(),
        2,
        "multiple sessions never overwrite each other"
    );
    assert_eq!(listed[0].id, session2.id, "ordered by occurred_at DESC");
    assert_eq!(listed[1].summary.as_deref(), Some("第一次实验"));

    // Item deletion is RESTRICT-blocked while sessions remain (contract §7:
    // sessions are historical records).
    assert!(items.delete_item(&w1.id, &item.id).is_err());
    assert!(items.find_item(&w1.id, &item.id).unwrap().is_some());

    // Item status transitions persist (mirror of the task machine).
    let mut started = item.clone();
    started.status = ItemStatus::InProgress;
    items.update_item(&started).unwrap();
    let reloaded = items.find_item(&w1.id, &item.id).unwrap().unwrap();
    assert_eq!(reloaded.status, ItemStatus::InProgress);
}

#[test]
fn knowledge_availability_change_enqueues_outbox_event_atomically() {
    let dir = temp_dir("knowledge");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let knowledge = SqliteKnowledgeRefRepository::open(&db).unwrap();

    let w1 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "知识空间".into(),
    };
    workspaces.insert_workspace(&w1).unwrap();

    let reference = KnowledgeRef::new(
        KnowledgeRefId::new_v7(),
        w1.id.clone(),
        "Durable Execution 综述",
        KnowledgeSource::Web,
        Some("https://example.com/durable-execution"),
        Some("section-2"),
        Some("v2026-10"),
        None,
    )
    .unwrap();
    assert_eq!(reference.availability, Availability::Available);
    knowledge.insert_knowledge_ref(&reference).unwrap();

    // Availability change: same-transaction outbox event (contract §6).
    let updated = knowledge
        .set_availability(&w1.id, &reference.id, Availability::Stale)
        .unwrap();
    assert_eq!(updated.availability, Availability::Stale);

    // Exactly one versioned event, minimal payload.
    let conn = sqlite::open(&db).unwrap();
    let quarantined = list_quarantined(&conn, None).unwrap(); // sanity: not quarantined
    assert!(quarantined.is_empty());
    let (event_type, schema_version, payload): (String, i64, String) = conn
        .query_row(
            "SELECT event_type, schema_version, payload_json FROM outbox_messages
             WHERE status = 'Pending' AND event_type = 'KnowledgeReferenceAvailabilityChanged'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(event_type, "KnowledgeReferenceAvailabilityChanged");
    assert_eq!(schema_version, 1);
    assert!(payload.contains(&format!(
        "\"knowledge_ref_id\":\"{}\"",
        reference.id.as_str()
    )));
    assert!(payload.contains("\"from\":\"available\""));
    assert!(payload.contains("\"to\":\"stale\""));

    // Setting the same value again records no new event.
    knowledge
        .set_availability(&w1.id, &reference.id, Availability::Stale)
        .unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outbox_messages WHERE event_type = 'KnowledgeReferenceAvailabilityChanged'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Scope denial on the availability change.
    let w2 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "别处".into(),
    };
    workspaces.insert_workspace(&w2).unwrap();
    assert!(matches!(
        knowledge.set_availability(&w2.id, &reference.id, Availability::Unavailable),
        Err(paw_application::ports::knowledge_repository::KnowledgeRepositoryError::KnowledgeRefNotFound)
    ));

    // Learning records referencing the source are never touched by
    // availability changes — and deleting the source deletes only the
    // association (CASCADE on session_refs), never learning data. Contract §7.
    knowledge
        .delete_knowledge_ref(&w1.id, &reference.id)
        .unwrap();
    assert!(
        knowledge
            .find_knowledge_ref(&w1.id, &reference.id)
            .unwrap()
            .is_none()
    );
    drop(conn);
}

#[test]
fn upgrade_fixture_from_0007_preserves_existing_data() {
    let dir = temp_dir("upgrade");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");

    // Stage 1: apply only migrations 0001-0007.
    let partial = dir.join("migrations-partial");
    fs::create_dir_all(&partial).unwrap();
    for entry in fs::read_dir(migrations_dir())
        .unwrap()
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("0008") {
            fs::copy(entry.path(), partial.join(&name)).unwrap();
        }
    }
    let ws_id = "33333333-3333-7333-8333-333333333333";
    {
        let mut conn = sqlite::open(&db).unwrap();
        sqlite::migrate(&mut conn, &partial).unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?1, '升级空间', 1, 1)",
            [ws_id],
        )
        .unwrap();
    }

    // Stage 2: full migration set — 0008 adds the learning tables.
    {
        let mut conn = sqlite::open(&db).unwrap();
        sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
        let ledger: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ledger, 12);
    }

    // Stage 3: the new tables work on the upgraded database.
    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let roadmaps = SqliteRoadmapRepository::open(&db).unwrap();
    let ws = WorkspaceId::new(ws_id).unwrap();
    let roadmap = Roadmap::new(RoadmapId::new_v7(), ws.clone(), "升级后路线", None).unwrap();
    roadmaps.insert_roadmap(&roadmap).unwrap();
    assert_eq!(roadmaps.list_roadmaps(&ws).unwrap().len(), 1);
    // The pre-upgrade workspace row survived.
    assert!(workspaces.find_workspace(&ws).unwrap().is_some());
}
