//! E6 integration tests: skills/evidence/assessments (PAW-LG-008/009),
//! learning-gap proposals with versioned outbox events (PAW-LG-007), and
//! the creator content pipeline. Workspace isolation and the append-only
//! assessment discipline are asserted through direct backend calls.

use std::path::PathBuf;

use paw_application::ports::creator_repository::{
    CreatorDraftRepository as _, CreatorTopicRepository as _,
};
use paw_application::ports::learning_repository::GapProposalRepository as _;
use paw_application::ports::skill_repository::{
    AssessmentRepository as _, SkillEvidenceRepository as _, SkillRepository as _,
};
use paw_domain::creator::{
    CreatorDraft, CreatorDraftId, CreatorDraftReview, CreatorDraftVersion, CreatorTopic,
    CreatorTopicId, DraftStatus, ReviewDecision, TopicStatus,
};
use paw_domain::learning::{GapProposalStatus, LearningGapProposal, LearningGapProposalId};
use paw_domain::skills::{
    Assessment, AssessmentId, Confidence, EvidenceKind, EvidenceSourceKind, Skill, SkillEvidence,
    SkillEvidenceId, SkillId,
};
use paw_domain::work_management::{ProjectId, WorkspaceId};
use paw_infrastructure::repositories::creator::{
    SqliteCreatorDraftRepository, SqliteCreatorTopicRepository,
};
use paw_infrastructure::repositories::gap_proposals::SqliteGapProposalRepository;
use paw_infrastructure::repositories::skills::{
    SqliteAssessmentRepository, SqliteSkillEvidenceRepository, SqliteSkillRepository,
};
use paw_infrastructure::sqlite;

fn temp_db(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("paw-e6-{tag}-{}", uuid::Uuid::now_v7().simple()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let migrations = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite");
    let db_path = dir.join("app.sqlite");
    let mut conn = sqlite::open(&db_path).expect("open db");
    sqlite::migrate(&mut conn, &migrations).expect("migrate");
    db_path
}

fn outbox_events(db_path: &std::path::Path, event_type: &str) -> Vec<(String, String)> {
    let conn = sqlite::open(db_path).expect("reopen");
    let mut stmt = conn
        .prepare("SELECT aggregate_id, payload_json FROM outbox_messages WHERE event_type = ?1")
        .expect("prepare");
    let rows = stmt
        .query_map([event_type], |row| {
            Ok((
                row.get::<_, String>(0).expect("agg"),
                row.get::<_, String>(1).expect("payload"),
            ))
        })
        .expect("query");
    rows.map(|r| r.expect("row")).collect()
}

fn insert_workspace(db_path: &std::path::Path, workspace_id: &WorkspaceId) {
    let conn = sqlite::open(db_path).expect("conn");
    conn
        .execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?1, 'e6 test ws', strftime('%s','now') * 1000, strftime('%s','now') * 1000)",
            rusqlite::params![workspace_id.as_str()],
        )
        .expect("workspace fixture");
}

#[test]
fn skills_evidence_assessment_flow() {
    let db_path = temp_db("skills");
    let skills = SqliteSkillRepository::open(&db_path).expect("skills repo");
    let evidence = SqliteSkillEvidenceRepository::open(&db_path).expect("evidence repo");
    let assessments = SqliteAssessmentRepository::open(&db_path).expect("assessment repo");
    let workspace = WorkspaceId::new_v7();
    insert_workspace(&db_path, &workspace);

    // Skill lifecycle + scale bump.
    let mut skill = Skill::new(
        SkillId::new_v7(),
        workspace.clone(),
        "Rust",
        "language",
        None,
    )
    .expect("valid skill");
    skills.insert_skill(&skill).expect("inserted");
    let mut evidence_ids = Vec::new();
    for (kind, source, id) in [
        (
            EvidenceKind::Implementation,
            EvidenceSourceKind::Project,
            "project-1",
        ),
        (
            EvidenceKind::TestVerification,
            EvidenceSourceKind::Task,
            "task-7",
        ),
    ] {
        let e = SkillEvidence::new(
            SkillEvidenceId::new_v7(),
            workspace.clone(),
            skill.id.clone(),
            kind,
            source,
            id,
            Some("landed and verified"),
        )
        .expect("valid evidence");
        evidence.insert_evidence(&e).expect("evidence stored");
        evidence_ids.push(e.id);
    }
    let listed = evidence
        .list_evidence(&workspace, &skill.id)
        .expect("evidence listed");
    assert_eq!(listed.len(), 2);

    // User-confirmed assessment: evidence-linked, level bounded.
    let assessment = Assessment::new(
        AssessmentId::new_v7(),
        workspace.clone(),
        skill.id.clone(),
        3,
        skill.scale_version,
        Confidence::Medium,
        Some("codec landed; tests green"),
        evidence_ids,
    )
    .expect("valid assessment");
    assessments
        .insert_assessment(&assessment, None)
        .expect("assessment stored");
    let latest = assessments
        .latest_assessment(&workspace, &skill.id)
        .expect("latest")
        .expect("has one");
    assert_eq!(latest.level, 3);
    assert_eq!(latest.evidence_ids.len(), 2);
    assert_eq!(latest.scale_version, 1);

    // Scale bump: new assessments snapshot the new version; history keeps
    // the old one (never reinterpreted).
    skill.bump_scale_version();
    skills.update_skill(&skill).expect("scale bumped");
    let e3 = SkillEvidence::new(
        SkillEvidenceId::new_v7(),
        workspace.clone(),
        skill.id.clone(),
        EvidenceKind::TeachingDocumentation,
        EvidenceSourceKind::Note,
        "note-9",
        None,
    )
    .expect("valid");
    evidence.insert_evidence(&e3).expect("stored");
    let assessment_v2 = Assessment::new(
        AssessmentId::new_v7(),
        workspace.clone(),
        skill.id.clone(),
        4,
        skill.scale_version,
        Confidence::High,
        None,
        vec![e3.id.clone()],
    )
    .expect("valid v2 assessment");
    assessments
        .insert_assessment(&assessment_v2, Some(&latest))
        .expect("stored");
    let history = assessments
        .list_assessments(&workspace, &skill.id)
        .expect("history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].scale_version, 2, "newest first");
    assert_eq!(history[1].scale_version, 1, "history keeps its snapshot");

    // Cross-workspace invisibility.
    let other = WorkspaceId::new_v7();
    let cross = skills.get_skill(&skill.id, &other);
    assert!(matches!(
        cross,
        Err(paw_application::ports::skill_repository::SkillStoreError::WorkspaceMismatch)
    ));
    let _ = std::fs::remove_dir_all(db_path.parent().expect("parent"));
}

#[test]
fn gap_proposal_flow_emits_versioned_events() {
    let db_path = temp_db("gaps");
    let gaps = SqliteGapProposalRepository::open(&db_path).expect("gap repo");
    let workspace = WorkspaceId::new_v7();
    let project = ProjectId::new_v7();

    // Insert a workspace + project fixture (gap proposals FK both).
    {
        let conn = sqlite::open(&db_path).expect("conn");
        conn
            .execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?1, 'gap ws', strftime('%s','now') * 1000, strftime('%s','now') * 1000)",
            rusqlite::params![workspace.as_str()],
        )
        .expect("workspace fixture");
        conn
            .execute(
            "INSERT INTO projects (id, workspace_id, name, status, created_at, updated_at) VALUES (?1, ?2, 'gap project', 'active', strftime('%s','now') * 1000, strftime('%s','now') * 1000)",
            rusqlite::params![project.as_str(), workspace.as_str()],
        )
        .expect("project fixture");
    }

    // Propose → LearningGapProposed.v1 in the outbox (same transaction).
    let mut gap = LearningGapProposal::new(
        LearningGapProposalId::new_v7(),
        workspace.clone(),
        project.clone(),
        "需要补充 Wi-Fi 6 帧聚合协议知识",
    )
    .expect("valid gap");
    gaps.insert_gap_proposal(&gap).expect("proposed");
    let proposed = outbox_events(&db_path, "LearningGapProposed.v1");
    assert_eq!(proposed.len(), 1);
    assert!(proposed[0].0 == gap.id.as_str());

    // Confirm binds the item and emits LearningGapConfirmed.v1.
    let item_id = paw_domain::learning::ItemId::new_v7();
    {
        // Fixture: the learning item the confirmation binds to.
        let conn = sqlite::open(&db_path).expect("conn");
        conn
            .execute(
                "INSERT INTO learning_roadmaps (id, workspace_id, name, created_at, updated_at)
                 SELECT ?1, w.id, 'gap roadmap', strftime('%s','now') * 1000, strftime('%s','now') * 1000
                 FROM workspaces w WHERE w.id = ?2",
                rusqlite::params![uuid::Uuid::now_v7().to_string(), workspace.as_str()],
            )
            .expect("roadmap fixture");
        conn
            .execute(
                "INSERT INTO learning_plans (id, workspace_id, roadmap_id, name, status, created_at, updated_at)
                 SELECT ?1, w.id, r.id, 'gap plan', 'active', strftime('%s','now') * 1000, strftime('%s','now') * 1000
                 FROM workspaces w JOIN learning_roadmaps r ON r.workspace_id = w.id WHERE w.id = ?2 LIMIT 1",
                rusqlite::params![uuid::Uuid::now_v7().to_string(), workspace.as_str()],
            )
            .expect("plan fixture");
        conn
            .execute(
                "INSERT INTO learning_items (id, workspace_id, plan_id, kind, title, status, created_at, updated_at)
                 SELECT ?1, w.id, p.id, 'implementation', 'Wi-Fi 6 聚合', 'todo', strftime('%s','now') * 1000, strftime('%s','now') * 1000
                 FROM workspaces w JOIN learning_plans p ON p.workspace_id = w.id WHERE w.id = ?2 LIMIT 1",
                rusqlite::params![item_id.as_str(), workspace.as_str()],
            )
            .expect("item fixture");
    }
    let item_id_captured = item_id.as_str().to_string();
    gap.confirm(item_id).expect("confirmed");
    gaps.update_gap_proposal(&gap).expect("persisted");
    assert_eq!(gap.status, GapProposalStatus::Confirmed);
    let confirmed = outbox_events(&db_path, "LearningGapConfirmed.v1");
    assert_eq!(confirmed.len(), 1);
    let payload = &confirmed[0].1;
    assert!(
        payload.contains(&item_id_captured),
        "payload binds the item"
    );

    // Dismissed is terminal history with its own event.
    let mut gap2 = LearningGapProposal::new(
        LearningGapProposalId::new_v7(),
        workspace.clone(),
        project,
        "另一个缺口",
    )
    .expect("valid");
    gaps.insert_gap_proposal(&gap2).expect("proposed");
    gap2.dismiss().expect("dismissed");
    gaps.update_gap_proposal(&gap2).expect("persisted");
    assert_eq!(gap2.status, GapProposalStatus::Dismissed);
    assert_eq!(outbox_events(&db_path, "LearningGapDismissed.v1").len(), 1);

    let listed = gaps
        .list_gap_proposals(&workspace, Some(GapProposalStatus::Confirmed))
        .expect("filtered list");
    assert_eq!(listed.len(), 1);
    let _ = std::fs::remove_dir_all(db_path.parent().expect("parent"));
}

#[test]
fn creator_pipeline_flow() {
    let db_path = temp_db("creator");
    let topics = SqliteCreatorTopicRepository::open(&db_path).expect("topics");
    let drafts = SqliteCreatorDraftRepository::open(&db_path).expect("drafts");
    let workspace = WorkspaceId::new_v7();
    insert_workspace(&db_path, &workspace);

    // Topic lifecycle.
    let mut topic = CreatorTopic::new(
        CreatorTopicId::new_v7(),
        workspace.clone(),
        "Agent 治理实践",
        None,
    )
    .expect("valid topic");
    topics.insert_topic(&topic).expect("inserted");
    topic.status = TopicStatus::Selected;
    topics.update_topic(&topic).expect("selected");

    // Draft + immutable versions (head bumps atomically).
    let mut draft = CreatorDraft::new(
        CreatorDraftId::new_v7(),
        workspace.clone(),
        topic.id.clone(),
        "ADR-022 落地记",
    )
    .expect("valid draft");
    drafts.insert_draft(&draft).expect("inserted");
    let v1 = CreatorDraftVersion::new(
        paw_domain::creator::CreatorDraftVersionId::new_v7(),
        workspace.clone(),
        draft.id.clone(),
        1,
        "第一版正文",
    )
    .expect("valid version");
    drafts.append_version(&v1).expect("appended");
    let stored = drafts.get_draft(&draft.id, &workspace).expect("draft");
    assert_eq!(stored.current_version_no, 1);

    // Review bound to v1 → rejected → back to writing → v2 → approved.
    draft.transition(DraftStatus::InReview).expect("to review");
    drafts.update_draft(&draft).expect("status persisted");
    drafts
        .insert_review(
            &CreatorDraftReview::new(
                paw_domain::creator::CreatorDraftReviewId::new_v7(),
                workspace.clone(),
                draft.id.clone(),
                1,
                ReviewDecision::Rejected,
                Some("结构需要重排"),
            )
            .expect("review"),
        )
        .expect("review stored");
    draft.transition(DraftStatus::Writing).expect("revise");
    drafts.update_draft(&draft).expect("status persisted");
    let v2 = CreatorDraftVersion::new(
        paw_domain::creator::CreatorDraftVersionId::new_v7(),
        workspace.clone(),
        draft.id.clone(),
        2,
        "第二版正文",
    )
    .expect("valid");
    drafts.append_version(&v2).expect("appended");
    draft.transition(DraftStatus::InReview).expect("to review");
    drafts
        .insert_review(
            &CreatorDraftReview::new(
                paw_domain::creator::CreatorDraftReviewId::new_v7(),
                workspace.clone(),
                draft.id.clone(),
                2,
                ReviewDecision::Approved,
                None,
            )
            .expect("review"),
        )
        .expect("review stored");
    draft.transition(DraftStatus::Approved).expect("approved");
    drafts.update_draft(&draft).expect("persisted");

    // History: two versions, two reviews in order.
    let versions = drafts
        .list_versions(&workspace, &draft.id)
        .expect("versions");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version_no, 1);
    let reviews = drafts.list_reviews(&workspace, &draft.id).expect("reviews");
    assert_eq!(reviews.len(), 2);
    assert_eq!(reviews[0].decision, ReviewDecision::Rejected);
    assert_eq!(reviews[1].decision, ReviewDecision::Approved);

    // Cross-workspace invisibility.
    let other = WorkspaceId::new_v7();
    assert!(matches!(
        drafts.get_draft(&draft.id, &other),
        Err(paw_application::ports::creator_repository::CreatorStoreError::WorkspaceMismatch)
    ));
    let _ = std::fs::remove_dir_all(db_path.parent().expect("parent"));
}
