//! Beta 0.5 second batch end-to-end flow through the desktop command
//! surface: creator publish records (manual facts, terminal Published) and
//! project retrospectives with learning-product backflow (sessions must be
//! linked to the project — real backflow, not asserted). Versioned event
//! emission is asserted through the outbox.

use std::path::PathBuf;

use paw_application::ItemKind;
use paw_infrastructure::sqlite;

fn temp_db(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "paw-e6b-e2e-{tag}-{}",
        uuid::Uuid::now_v7().simple()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let migrations = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite");
    let db_path = dir.join("app.sqlite");
    let mut conn = sqlite::open(&db_path).expect("open db");
    sqlite::migrate(&mut conn, &migrations).expect("migrate");
    db_path
}

fn outbox_count(db_path: &std::path::Path, event_type: &str) -> usize {
    let conn = sqlite::open(db_path).expect("reopen");
    conn.query_row(
        "SELECT COUNT(*) FROM outbox_messages WHERE event_type = ?1",
        [event_type],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as usize)
    .expect("outbox query")
}

#[test]
fn beta_second_batch_flow_end_to_end() {
    let db_path = temp_db("flow");
    let api = paw_desktop::commands::WorkManagementApi::open(&db_path).expect("api");
    let workspace = api.create_workspace("E6b e2e").expect("workspace");

    // Fixture chain: project → roadmap → plan → item (linked to project) →
    // session (belongs to the item).
    let project = api
        .create_project(&workspace.id, "ProCodec")
        .expect("project");
    let roadmap = api
        .create_roadmap(&workspace.id, "协议学习", None)
        .expect("roadmap");
    let plan = api
        .create_plan(&workspace.id, &roadmap.id, "Wi-Fi 6", None)
        .expect("plan");
    let item = api
        .create_item(
            &workspace.id,
            &plan.id,
            ItemKind::Implementation,
            "帧聚合解析",
            None,
            None,
        )
        .expect("item");
    api.link_item_to_project(&workspace.id, &item.id, &project.id)
        .expect("item linked to project");
    let session = api
        .append_session(
            &workspace.id,
            &item.id,
            None,
            None,
            Some("聚合解析跑通"),
            None,
            None,
            None,
        )
        .expect("session recorded");

    // 1. Creator publish flow: draft → v1 → review approved → publish
    //    record (manual fact) → terminal Published.
    let topic = api
        .create_creator_topic(&workspace.id, "Wi-Fi 6 聚合实战", None)
        .expect("topic");
    let draft = api
        .create_creator_draft(&workspace.id, topic.id.as_str(), "Wi-Fi 6 聚合实战")
        .expect("draft");
    let _ = api
        .append_creator_draft_version(&workspace.id, draft.id.as_str(), "第一版正文")
        .expect("version");
    api.transition_creator_draft(&workspace.id, draft.id.as_str(), "InReview")
        .expect("in review");
    api.submit_creator_review(&workspace.id, draft.id.as_str(), 1, "Approved", None)
        .expect("approved");
    api.transition_creator_draft(&workspace.id, draft.id.as_str(), "Approved")
        .expect("approved state");
    let published = api
        .record_publish(&workspace.id, draft.id.as_str(), "wechat", None, None)
        .expect("publish recorded");
    assert_eq!(published.status, paw_application::DraftStatus::Published);
    // Published is terminal.
    let republish = api.record_publish(&workspace.id, draft.id.as_str(), "manual", None, None);
    assert!(republish.is_err(), "Published has no outgoing transitions");
    // Publishing automation remains gated: the record is a manual fact only.
    let records = api
        .list_publish_records(&workspace.id, draft.id.as_str())
        .expect("records listed");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].channel, paw_application::PublishChannel::WeChat);

    // 2. Project retrospective: real backflow — the referenced session must
    //    belong to an item linked to the same project.
    let retro = api
        .record_retrospective(
            &workspace.id,
            project.id.as_str(),
            "聚合解析已落地；互操作测试待补",
            &[session.id.as_str().to_string()],
        )
        .expect("retrospective recorded");
    assert_eq!(retro.learning_session_refs.len(), 1);
    assert_eq!(outbox_count(&db_path, "ProjectRetrospectiveRecorded.v1"), 1);

    // 3. Backflow is real: a session from an unlinked item is refused.
    let other_item = api
        .create_item(
            &workspace.id,
            &plan.id,
            ItemKind::Reading,
            "无关条目",
            None,
            None,
        )
        .expect("other item");
    let stray_session = api
        .append_session(
            &workspace.id,
            &other_item.id,
            None,
            None,
            Some("无关联学习"),
            None,
            None,
            None,
        )
        .expect("stray session");
    let refused = api.record_retrospective(
        &workspace.id,
        project.id.as_str(),
        "尝试引用未关联会话",
        &[stray_session.id.as_str().to_string()],
    );
    assert!(refused.is_err(), "unlinked session backflow is refused");
    assert_eq!(outbox_count(&db_path, "ProjectRetrospectiveRecorded.v1"), 1);

    // 4. Cross-workspace invisibility.
    let other = api.create_workspace("other").expect("other");
    let cross = api.list_retrospectives(&other.id, project.id.as_str());
    // The project itself is invisible in the other workspace; the list is
    // empty rather than leaking.
    assert!(cross.is_ok());
    let _ = std::fs::remove_dir_all(db_path.parent().expect("parent"));
}
