//! Cross-domain data contract tests (E4): the subset of
//! `docs/02-architecture/cross-domain-data-contracts.md` §11 that this file
//! owns.
//!
//! Four tests exist here — exactly these four, no more:
//!
//! 1. `reject_cross_workspace_learning_references` — §11.1 "Reject
//!    cross-workspace references."
//! 2. `duplicate_delivery_does_not_duplicate_learning_items` — §11.4
//!    "Deliver duplicate events without duplicate Learning Items."
//! 3. `unavailable_or_deleted_source_preserves_learning_records` — §11.5
//!    "Preserve sessions and assessments when referenced projects or sources
//!    are archived/unavailable."
//! 4. `backup_restore_preserves_learning_associations` — §11.8 "Restore
//!    database backups with all Learning associations and event metadata
//!    intact."
//!
//! The remaining §11 items have **no** dedicated contract test anywhere in this
//! repository. This header does not claim coverage for them:
//!
//! * §11.2 (reject unknown source domain/type combinations) — `KnowledgeSource`
//!   and `Availability` are closed enums, so an unknown value fails serde
//!   decoding at the IPC boundary, and `KnowledgeSource::parse` returns `None`.
//!   No test asserts either path.
//! * §11.3 (consumer modules cannot mutate source-owned records) — structural
//!   only: `ports::learning_repository` and `ports::knowledge_repository`
//!   expose no method that writes projects, goals or tasks (item↔project and
//!   item↔task association rows only). No test asserts it.
//! * §11.6 (authorization at reference resolution) — structural only: every
//!   `KnowledgeRef` mutation in `use_cases::knowledge` passes through
//!   `learning::gate`. No test asserts it.
//! * §11.7 (event schema-version compatibility) — not implemented: the outbox
//!   stores and returns `schema_version` without rejecting unsupported values,
//!   and no test covers it.
//!
//! Reverse-verification: each of the four tests above was confirmed to fail
//! when its guard was weakened (dedup marker removed, learning rows dropped
//! from the backup, composite key reduced to a single column, source deletion
//! cascading into sessions). See the card's delivery notes.

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::ports::knowledge_repository::KnowledgeRefRepository;
use paw_application::ports::learning_repository::{
    ItemRepository, PlanRepository, RoadmapRepository, SessionRepository,
};
use paw_application::ports::task_repository::TaskRepository;
use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::{
    Item, ItemId, ItemKind, KnowledgeRef, KnowledgeRefId, KnowledgeSource, Plan, PlanId, Project,
    ProjectId, Roadmap, RoadmapId, Session, SessionId, Task, TaskId, Workspace, WorkspaceId,
};
use paw_domain::knowledge::Availability;
use paw_infrastructure::backup;
use paw_infrastructure::outbox::{
    self, ConsumeOutcome, DispatchOptions, NewOutboxEvent, OutboxConsumer, OutboxMessage,
};
use paw_infrastructure::repositories::knowledge_repository::SqliteKnowledgeRefRepository;
use paw_infrastructure::repositories::learning_repository::{
    SqliteItemRepository, SqlitePlanRepository, SqliteRoadmapRepository, SqliteSessionRepository,
};
use paw_infrastructure::repositories::task_repository::SqliteTaskRepository;
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::sqlite;
use rusqlite::{Transaction, params};

/// Fixed dispatcher time (year 2096): after any real enqueue timestamp, so
/// claim decisions are deterministic without sleeping.
const T: i64 = 4_000_000_000_000;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e4-xdc-{tag}-{seq}-{}",
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

fn open_migrated(db: &Path) -> rusqlite::Connection {
    let mut conn = sqlite::open(db).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
    conn
}

struct Graph {
    roadmap: Roadmap,
    plan: Plan,
    item: Item,
}

/// Seed one workspace with a roadmap → plan → item chain through the real
/// adapters (so the schema, not the test, enforces the invariants).
fn seed_graph(
    roadmaps: &SqliteRoadmapRepository,
    plans: &SqlitePlanRepository,
    items: &SqliteItemRepository,
    ws: &Workspace,
    suffix: &str,
) -> Graph {
    let roadmap_name = format!("路线-{suffix}");
    let roadmap = Roadmap::new(RoadmapId::new_v7(), ws.id.clone(), &roadmap_name, None).unwrap();
    roadmaps.insert_roadmap(&roadmap).unwrap();

    let plan_name = format!("计划-{suffix}");
    let plan = Plan::new(
        PlanId::new_v7(),
        ws.id.clone(),
        roadmap.id.clone(),
        &plan_name,
        None,
    )
    .unwrap();
    plans.insert_plan(&plan).unwrap();

    let item_title = format!("学习项-{suffix}");
    let item = Item::new(
        ItemId::new_v7(),
        ws.id.clone(),
        plan.id.clone(),
        ItemKind::Implementation,
        &item_title,
        Some("可运行的示例"),
        Some("至少覆盖一次恢复路径"),
        None,
    )
    .unwrap();
    items.insert_item(&item).unwrap();

    Graph {
        roadmap,
        plan,
        item,
    }
}

fn new_workspace(workspaces: &SqliteWorkspaceRepository, name: &str) -> Workspace {
    let ws = Workspace {
        id: WorkspaceId::new_v7(),
        name: name.into(),
    };
    workspaces.insert_workspace(&ws).unwrap();
    ws
}

// ---------------------------------------------------------------------------
// §11.1 — cross-workspace references are rejected
// ---------------------------------------------------------------------------

#[test]
fn reject_cross_workspace_learning_references() {
    let dir = temp_dir("scope");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let roadmaps = SqliteRoadmapRepository::open(&db).unwrap();
    let plans = SqlitePlanRepository::open(&db).unwrap();
    let items = SqliteItemRepository::open(&db).unwrap();
    let sessions = SqliteSessionRepository::open(&db).unwrap();
    let tasks = SqliteTaskRepository::open(&db).unwrap();

    let w1 = new_workspace(&workspaces, "学习空间");
    let w2 = new_workspace(&workspaces, "别处");
    let g1 = seed_graph(&roadmaps, &plans, &items, &w1, "w1");
    let g2 = seed_graph(&roadmaps, &plans, &items, &w2, "w2");

    let p1 = Project::new(ProjectId::new_v7(), w1.id.clone(), "本空间项目").unwrap();
    workspaces.insert_project(&w1.id, &p1).unwrap();
    let p2 = Project::new(ProjectId::new_v7(), w2.id.clone(), "别处项目").unwrap();
    workspaces.insert_project(&w2.id, &p2).unwrap();

    // (a) Item → Project: the composite (project_id, workspace_id) key
    // refuses a project that lives in another workspace; the refused link
    // leaves no row behind.
    items
        .attach_item_to_project(&w1.id, &g1.item.id, &p1.id)
        .unwrap();
    assert!(
        items
            .attach_item_to_project(&w1.id, &g1.item.id, &p2.id)
            .is_err(),
        "a project from another workspace must not be linked"
    );
    assert_eq!(
        items
            .list_projects_for_item(&w1.id, &g1.item.id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        items
            .list_projects_for_item(&w2.id, &g2.item.id)
            .unwrap()
            .len(),
        0
    );

    // (b) Plan → Roadmap: a plan cannot be filed under a roadmap of another
    // workspace, even when the plan carries its own correct workspace id.
    let stray_plan = Plan::new(
        PlanId::new_v7(),
        w1.id.clone(),
        g2.roadmap.id.clone(),
        "越界计划",
        None,
    )
    .unwrap();
    assert!(
        plans.insert_plan(&stray_plan).is_err(),
        "a plan must not reference a roadmap from another workspace"
    );
    assert!(plans.find_plan(&w1.id, &stray_plan.id).unwrap().is_none());

    // (c) Session → Item/Project: a session cannot point at an item or a
    // project of another workspace.
    let stray_session = Session::new(
        SessionId::new_v7(),
        w1.id.clone(),
        g2.item.id.clone(),
        None,
        None,
        None,
        None,
        Some("越界记录"),
        None,
        None,
        None,
        1_000,
    )
    .unwrap();
    assert!(
        sessions.append_session(&stray_session).is_err(),
        "a session must not reference an item from another workspace"
    );
    let cross_project_session = Session::new(
        SessionId::new_v7(),
        w1.id.clone(),
        g1.item.id.clone(),
        None,
        Some(p2.id.clone()),
        None,
        None,
        Some("跨空间项目"),
        None,
        None,
        None,
        2_000,
    )
    .unwrap();
    assert!(
        sessions.append_session(&cross_project_session).is_err(),
        "a session must not reference a project from another workspace"
    );
    assert!(
        sessions
            .list_sessions(&w1.id, Some(&g1.item.id))
            .unwrap()
            .is_empty()
    );

    // (d) Item → Task: a task from another workspace cannot be recorded as
    // an action step of this workspace's item.
    let foreign_task = Task::new(TaskId::new_v7(), w2.id.clone(), None, "别处任务", None).unwrap();
    tasks.insert_task(&foreign_task).unwrap();
    assert!(
        items
            .attach_task_to_item(&w1.id, &g1.item.id, &foreign_task.id)
            .is_err(),
        "a task from another workspace must not be linked"
    );
    assert!(
        items
            .list_tasks_for_item(&w1.id, &g1.item.id)
            .unwrap()
            .is_empty()
    );

    // (e) Every read stays inside the caller's workspace.
    assert!(items.find_item(&w2.id, &g1.item.id).unwrap().is_none());
    assert!(plans.find_plan(&w2.id, &g1.plan.id).unwrap().is_none());
    assert!(
        roadmaps
            .find_roadmap(&w2.id, &g1.roadmap.id)
            .unwrap()
            .is_none()
    );
    assert_eq!(items.list_items(&w2.id, None, None).unwrap().len(), 1);
    assert_eq!(
        items
            .list_items(&w2.id, None, None)
            .unwrap()
            .first()
            .map(|i| i.id.clone()),
        Some(g2.item.id.clone())
    );
}

// ---------------------------------------------------------------------------
// §11.4 — duplicate event delivery does not duplicate learning items
// ---------------------------------------------------------------------------

/// Cross-domain consumer that materializes a Learning Item from a
/// Knowledge-owned proposal event.
///
/// The effect is deliberately **non-idempotent**: every `handle` call mints a
/// fresh row, so the only thing that can prevent a duplicate is the
/// dispatcher's dedup marker.
struct LearningItemMaterializer {
    calls: Cell<usize>,
}

impl OutboxConsumer for LearningItemMaterializer {
    fn consumer_name(&self) -> &'static str {
        "learning_item_materializer"
    }

    fn event_type(&self) -> &'static str {
        "LearningItemProposed"
    }

    fn handle(
        &self,
        tx: &Transaction<'_>,
        message: &OutboxMessage,
    ) -> Result<ConsumeOutcome, String> {
        self.calls.set(self.calls.get() + 1);
        let payload: serde_json::Value =
            serde_json::from_str(&message.payload_json).map_err(|e| e.to_string())?;
        let workspace_id = payload["workspace_id"]
            .as_str()
            .ok_or_else(|| "payload.workspace_id missing".to_string())?;
        let plan_id = payload["plan_id"]
            .as_str()
            .ok_or_else(|| "payload.plan_id missing".to_string())?;
        let title = payload["title"].as_str().unwrap_or("建议知识点");

        // New id on every call: a missing dedup guard shows up as a second
        // row, which is exactly what §11.4 forbids.
        let id = ItemId::new_v7();
        tx.execute(
            "INSERT INTO learning_items
             (id, workspace_id, plan_id, kind, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'concept', ?4, 'todo', 1, 1)",
            params![id.as_str(), workspace_id, plan_id, format!("建议：{title}")],
        )
        .map_err(|e| e.to_string())?;
        Ok(ConsumeOutcome::Applied)
    }
}

#[test]
fn duplicate_delivery_does_not_duplicate_learning_items() {
    let dir = temp_dir("dedup");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    let conn = open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let roadmaps = SqliteRoadmapRepository::open(&db).unwrap();
    let plans = SqlitePlanRepository::open(&db).unwrap();
    let items = SqliteItemRepository::open(&db).unwrap();
    let w1 = new_workspace(&workspaces, "学习空间");
    let graph = seed_graph(&roadmaps, &plans, &items, &w1, "dedup");

    let consumer = LearningItemMaterializer {
        calls: Cell::new(0),
    };

    // One Knowledge-owned proposal event.
    {
        let tx = conn.unchecked_transaction().unwrap();
        outbox::enqueue_in_tx(
            &tx,
            &NewOutboxEvent {
                workspace_id: Some(w1.id.as_str().to_string()),
                producer_domain: "knowledge".into(),
                aggregate_type: "knowledge_ref".into(),
                aggregate_id: KnowledgeRefId::new_v7().as_str().to_string(),
                event_type: "LearningItemProposed".into(),
                schema_version: 1,
                payload_json: format!(
                    "{{\"workspace_id\":\"{}\",\"plan_id\":\"{}\",\"title\":\"durable execution\"}}",
                    w1.id.as_str(),
                    graph.plan.id.as_str()
                ),
                correlation_id: "corr-dedup".into(),
                causation_id: None,
            },
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let options = DispatchOptions {
        now: T,
        ..Default::default()
    };
    let first = outbox::dispatch_once(&conn, &[&consumer], &options).unwrap();
    assert_eq!(first.delivered, 1);
    assert_eq!(first.deduplicated, 0);
    assert_eq!(consumer.calls.get(), 1);

    let count_items = |conn: &rusqlite::Connection| -> i64 {
        conn.query_row("SELECT COUNT(*) FROM learning_items", [], |r| r.get(0))
            .unwrap()
    };
    // The seeded item plus exactly one materialized item.
    assert_eq!(count_items(&conn), 2);

    // Simulate the at-least-once contract: the same message is redelivered
    // (status reset to Pending, no lease), which is what a crash between the
    // effect commit and the broker ack would look like.
    {
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "UPDATE outbox_messages
             SET status = 'Pending', available_at = 0, lease_owner = NULL, lease_until = NULL,
                 delivered_at = NULL
             WHERE event_type = 'LearningItemProposed'",
            [],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    let second = outbox::dispatch_once(&conn, &[&consumer], &options).unwrap();
    assert_eq!(second.claimed, 1);
    assert_eq!(second.delivered, 0);
    assert_eq!(
        second.deduplicated, 1,
        "the duplicate delivery must be recognized as already processed"
    );
    assert_eq!(
        consumer.calls.get(),
        1,
        "the consumer handler must not run twice for the same message"
    );
    assert_eq!(
        count_items(&conn),
        2,
        "duplicate delivery must not create a second learning item"
    );

    // The dedup marker is durable state, not a in-memory counter.
    let markers: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM processed_messages
             WHERE consumer_name = 'learning_item_materializer'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(markers, 1);
}

// ---------------------------------------------------------------------------
// §11.5 — an unavailable/deleted source never removes learning records
// ---------------------------------------------------------------------------

#[test]
fn unavailable_or_deleted_source_preserves_learning_records() {
    let dir = temp_dir("source");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let roadmaps = SqliteRoadmapRepository::open(&db).unwrap();
    let plans = SqlitePlanRepository::open(&db).unwrap();
    let items = SqliteItemRepository::open(&db).unwrap();
    let sessions = SqliteSessionRepository::open(&db).unwrap();
    let knowledge = SqliteKnowledgeRefRepository::open(&db).unwrap();

    let w1 = new_workspace(&workspaces, "学习空间");
    let graph = seed_graph(&roadmaps, &plans, &items, &w1, "source");

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
    knowledge.insert_knowledge_ref(&reference).unwrap();

    let session = Session::new(
        SessionId::new_v7(),
        w1.id.clone(),
        graph.item.id.clone(),
        None,
        None,
        Some("阅读"),
        Some(30),
        Some("研究了 checkpoint 语义"),
        None,
        Some("来源是 section 2"),
        None,
        1_000,
    )
    .unwrap();
    sessions.append_session(&session).unwrap();

    // The Session ↔ KnowledgeRef "source_material" association (no port API
    // for this E4 slice: the link is an association row).
    {
        let conn = sqlite::open(&db).unwrap();
        conn.execute(
            "INSERT INTO learning_session_refs (session_id, workspace_id, knowledge_ref_id, created_at)
             VALUES (?1, ?2, ?3, 1)",
            params![
                session.id.as_str(),
                w1.id.as_str(),
                reference.id.as_str()
            ],
        )
        .unwrap();
    }

    let count = |sql: &str, id: &str| -> i64 {
        let conn = sqlite::open(&db).unwrap();
        conn.query_row(sql, [id], |r| r.get(0)).unwrap()
    };
    let sessions_of_item = "SELECT COUNT(*) FROM learning_sessions WHERE item_id = ?1";
    let refs_of_session = "SELECT COUNT(*) FROM learning_session_refs WHERE session_id = ?1";

    // (a) Source degrades to Unavailable: the record stands, the learning
    // records are untouched.
    let stale = knowledge
        .set_availability(&w1.id, &reference.id, Availability::Unavailable)
        .unwrap();
    assert_eq!(stale.availability, Availability::Unavailable);
    assert_eq!(count(sessions_of_item, graph.item.id.as_str()), 1);
    assert_eq!(count(refs_of_session, session.id.as_str()), 1);
    let reloaded = sessions
        .list_sessions(&w1.id, Some(&graph.item.id))
        .unwrap();
    assert_eq!(reloaded.len(), 1);
    assert_eq!(
        reloaded[0].summary.as_deref(),
        Some("研究了 checkpoint 语义")
    );
    assert_eq!(
        reloaded[0].takeaway.as_deref(),
        Some("来源是 section 2"),
        "the note that cites the source is preserved verbatim"
    );

    // (b) The source record itself is deleted: the session (a historical
    // record) survives; only the association disappears.
    knowledge
        .delete_knowledge_ref(&w1.id, &reference.id)
        .unwrap();
    assert!(
        knowledge
            .find_knowledge_ref(&w1.id, &reference.id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        count(sessions_of_item, graph.item.id.as_str()),
        1,
        "deleting a source must not delete the learning session"
    );
    assert_eq!(count(refs_of_session, session.id.as_str()), 0);
    assert!(
        items.find_item(&w1.id, &graph.item.id).unwrap().is_some(),
        "the learning item survives as well"
    );

    // (c) The surviving session still protects the item: RESTRICT blocks a
    // hard delete of an item that a historical session references.
    assert!(items.delete_item(&w1.id, &graph.item.id).is_err());
}

// ---------------------------------------------------------------------------
// §11.8 — backup/restore keeps learning associations and event metadata
// ---------------------------------------------------------------------------

#[test]
fn backup_restore_preserves_learning_associations() {
    let dir = temp_dir("backup");
    let _guard = TempDirGuard::new(dir.clone());
    let active = dir.join("app.sqlite");
    let backup_file = dir.join("backups").join("app-backup.sqlite");
    fs::create_dir_all(backup_file.parent().unwrap()).unwrap();
    open_migrated(&active);

    let (ws, item_id, session_id, reference_id, task_id, project_id) = {
        let workspaces = SqliteWorkspaceRepository::open(&active).unwrap();
        let roadmaps = SqliteRoadmapRepository::open(&active).unwrap();
        let plans = SqlitePlanRepository::open(&active).unwrap();
        let items = SqliteItemRepository::open(&active).unwrap();
        let sessions = SqliteSessionRepository::open(&active).unwrap();
        let tasks = SqliteTaskRepository::open(&active).unwrap();
        let knowledge = SqliteKnowledgeRefRepository::open(&active).unwrap();

        let ws = new_workspace(&workspaces, "自磨工坊主空间");
        let graph = seed_graph(&roadmaps, &plans, &items, &ws, "backup");

        // Item ↔ Project (applies_to).
        let project = Project::new(ProjectId::new_v7(), ws.id.clone(), "实践项目").unwrap();
        workspaces.insert_project(&ws.id, &project).unwrap();
        items
            .attach_item_to_project(&ws.id, &graph.item.id, &project.id)
            .unwrap();

        // Item → Task (action_step); the task stays Work-Management-owned.
        let task = Task::new(
            TaskId::new_v7(),
            ws.id.clone(),
            Some(project.id.clone()),
            "学习项动作",
            None,
        )
        .unwrap();
        tasks.insert_task(&task).unwrap();
        items
            .attach_task_to_item(&ws.id, &graph.item.id, &task.id)
            .unwrap();

        // Session + Session ↔ KnowledgeRef (source_material).
        let reference = KnowledgeRef::new(
            KnowledgeRefId::new_v7(),
            ws.id.clone(),
            "Durable Execution 综述",
            KnowledgeSource::Hindsight,
            Some("hindsight://mem/durable-execution"),
            Some("chunk-3"),
            Some("v1"),
            None,
        )
        .unwrap();
        knowledge.insert_knowledge_ref(&reference).unwrap();
        let session = Session::new(
            SessionId::new_v7(),
            ws.id.clone(),
            graph.item.id.clone(),
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
        sessions.append_session(&session).unwrap();
        {
            let conn = sqlite::open(&active).unwrap();
            conn.execute(
                "INSERT INTO learning_session_refs (session_id, workspace_id, knowledge_ref_id, created_at)
                 VALUES (?1, ?2, ?3, 1)",
                params![
                    session.id.as_str(),
                    ws.id.as_str(),
                    reference.id.as_str()
                ],
            )
            .unwrap();
        }

        // A versioned outbox event, so the backup carries event metadata too
        // (the availability change enqueues it in the same transaction).
        knowledge
            .set_availability(&ws.id, &reference.id, Availability::Stale)
            .unwrap();

        // Snapshot while the adapters are open (online backup API).
        let conn = sqlite::open(&active).unwrap();
        let info = backup::create_backup(&conn, &backup_file).unwrap();
        assert_eq!(info.schema_version, 12);

        (
            ws.id.clone(),
            graph.item.id.clone(),
            session.id.clone(),
            reference.id.clone(),
            task.id.clone(),
            project.id.clone(),
        )
    };

    // Disaster: the active database and its sidecars are destroyed.
    fs::remove_file(&active).unwrap();
    let _ = fs::remove_file(active.with_extension("sqlite-wal"));
    let _ = fs::remove_file(active.with_extension("sqlite-shm"));

    let restored = backup::restore_backup(&backup_file, &active).unwrap();
    assert_eq!(restored.schema_version, 12);

    // Every learning association and the event metadata survived.
    {
        let conn = sqlite::open(&active).unwrap();
        let count =
            |sql: &str, id: &str| -> i64 { conn.query_row(sql, [id], |r| r.get(0)).unwrap() };
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM learning_roadmaps WHERE workspace_id = ?1",
                ws.as_str()
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM learning_plans WHERE workspace_id = ?1",
                ws.as_str()
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM learning_items WHERE workspace_id = ?1",
                ws.as_str()
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM learning_sessions WHERE item_id = ?1",
                item_id.as_str()
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM learning_item_projects WHERE item_id = ?1",
                item_id.as_str()
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM learning_item_tasks WHERE item_id = ?1",
                item_id.as_str()
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM learning_session_refs WHERE session_id = ?1",
                session_id.as_str()
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM knowledge_refs WHERE id = ?1",
                reference_id.as_str()
            ),
            1
        );

        // The session's own fields (including both action-step references)
        // round-trip, not just its id.
        let (summary, task, project, occurrence): (String, String, String, i64) = conn
            .query_row(
                "SELECT summary, task_id, project_id, occurred_at FROM learning_sessions WHERE id = ?1",
                [session_id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(summary, "第一次实验");
        assert_eq!(task, task_id.as_str());
        assert_eq!(project, project_id.as_str());
        assert_eq!(occurrence, 1_000);

        // Event metadata: type, schema version and the knowledge aggregate
        // survive the round trip.
        let (event_type, schema_version, producer, payload): (String, i64, String, String) = conn
            .query_row(
                "SELECT event_type, schema_version, producer_domain, payload_json
                 FROM outbox_messages WHERE aggregate_id = ?1",
                [reference_id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(event_type, "KnowledgeReferenceAvailabilityChanged");
        assert_eq!(schema_version, 1);
        assert_eq!(producer, "knowledge");
        assert!(payload.contains("\"to\":\"stale\""));
    }

    // The adapters read the restored records, not just raw SQL.
    let items = SqliteItemRepository::open(&active).unwrap();
    let sessions = SqliteSessionRepository::open(&active).unwrap();
    let found = items.find_item(&ws, &item_id).unwrap().unwrap();
    assert_eq!(found.id, item_id);
    assert_eq!(
        sessions
            .list_sessions(&ws, Some(&item_id))
            .unwrap()
            .first()
            .map(|s| s.id.clone()),
        Some(session_id)
    );
    assert_eq!(
        items.list_tasks_for_item(&ws, &item_id).unwrap(),
        vec![task_id]
    );
}
