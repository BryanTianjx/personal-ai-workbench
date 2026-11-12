//! E6 end-to-end flow through the desktop command surface: skill matrix
//! with evidence-linked user-confirmed assessment, project learning-gap
//! proposal → confirm (binding a learning item) with versioned outbox
//! events, and the creator content pipeline. Versioned event emission and
//! workspace isolation are asserted through direct backend calls.

use std::path::PathBuf;

use paw_application::ItemKind;
use paw_infrastructure::sqlite;

fn temp_db(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "paw-e6-e2e-{tag}-{}",
        uuid::Uuid::now_v7().simple()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let migrations = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite");
    let db_path = dir.join("app.sqlite");
    let mut conn = sqlite::open(&db_path).expect("open db");
    sqlite::migrate(&mut conn, &migrations).expect("migrate");
    db_path
}

fn outbox_events(db_path: &std::path::Path, event_type: &str) -> usize {
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
fn learning_growth_and_creator_flow_end_to_end() {
    let db_path = temp_db("flow");
    let api = paw_desktop::commands::WorkManagementApi::open(&db_path).expect("api");
    let workspace = api.create_workspace("E6 e2e").expect("workspace");

    // Project for the gap loop + learning chain for the confirmed item.
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
            "实现帧聚合解析",
            None,
            None,
        )
        .expect("item");

    // 1. Skill matrix: create skill, add evidence, user-confirmed assessment.
    let skill = api
        .create_skill(&workspace.id, "协议工程", "systems", None)
        .expect("skill");
    let e1 = api
        .add_skill_evidence(
            &workspace.id,
            skill.id.as_str(),
            "implementation",
            "project",
            project.id.as_str(),
            Some("codec 实现"),
        )
        .expect("evidence 1");
    let e2 = api
        .add_skill_evidence(
            &workspace.id,
            skill.id.as_str(),
            "test_verification",
            "task",
            "task-1",
            Some("互操作测试"),
        )
        .expect("evidence 2");
    let assessment = api
        .create_assessment(
            &workspace.id,
            skill.id.as_str(),
            3,
            "medium",
            Some("实现 + 测试均落地"),
            &[e1.id.as_str().to_string(), e2.id.as_str().to_string()],
        )
        .expect("assessment");
    assert_eq!(assessment.level, 3);
    assert_eq!(assessment.scale_version, 1);
    // Without evidence the assessment is refused (ADR-015 invariant).
    let no_evidence = api.create_assessment(&workspace.id, skill.id.as_str(), 4, "high", None, &[]);
    assert!(no_evidence.is_err(), "assessment requires evidence");

    // 2. Learning gap: propose from the project → confirm with the item.
    let gap = api
        .propose_learning_gap(&workspace.id, project.id.as_str(), "需要补充帧聚合协议知识")
        .expect("gap proposed");
    assert_eq!(gap.status, paw_application::GapProposalStatus::Proposed);
    assert_eq!(outbox_events(&db_path, "LearningGapProposed.v1"), 1);
    let confirmed = api
        .confirm_learning_gap(&workspace.id, gap.id.as_str(), item.id.as_str())
        .expect("gap confirmed");
    assert_eq!(
        confirmed.status,
        paw_application::GapProposalStatus::Confirmed
    );
    assert_eq!(
        confirmed.confirmed_item_id.as_ref().map(|i| i.as_str()),
        Some(item.id.as_str())
    );
    assert_eq!(outbox_events(&db_path, "LearningGapConfirmed.v1"), 1);
    // A confirmed gap is terminal: re-confirmation is refused.
    let reconfirm = api.confirm_learning_gap(&workspace.id, gap.id.as_str(), item.id.as_str());
    assert!(reconfirm.is_err());

    // 3. Creator pipeline: topic → draft → version → review → approved.
    let topic = api
        .create_creator_topic(&workspace.id, "ADR-022 落地记", None)
        .expect("topic");
    let draft = api
        .create_creator_draft(&workspace.id, topic.id.as_str(), "ADR-022 落地记")
        .expect("draft");
    let after_v1 = api
        .append_creator_draft_version(&workspace.id, draft.id.as_str(), "第一版正文")
        .expect("version 1");
    assert_eq!(after_v1.current_version_no, 1);
    let in_review = api
        .transition_creator_draft(&workspace.id, draft.id.as_str(), "InReview")
        .expect("in review");
    assert_eq!(in_review.status, paw_application::DraftStatus::InReview);
    let review = api
        .submit_creator_review(&workspace.id, draft.id.as_str(), 1, "Approved", None)
        .expect("review approved");
    assert_eq!(review.decision, paw_application::ReviewDecision::Approved);
    let approved = api
        .transition_creator_draft(&workspace.id, draft.id.as_str(), "Approved")
        .expect("approved");
    assert_eq!(approved.status, paw_application::DraftStatus::Approved);
    // Cross-workspace: another workspace sees nothing.
    let other = api.create_workspace("other").expect("other");
    let cross_skills = api.list_skills(&other.id, None).expect("empty skills");
    assert!(cross_skills.is_empty());

    let _ = std::fs::remove_dir_all(db_path.parent().expect("parent"));
}
