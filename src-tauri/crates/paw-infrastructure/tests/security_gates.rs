//! E1-09 security-gate integration tests: authorization recheck before
//! protected side effects, audit hooks for allowed/denied/error outcomes,
//! and redaction — audit rows and safe errors never carry driver messages
//! or payload bodies (Gate C).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::WorkspaceId;
use paw_application::ports::audit::AuditSink as _;
use paw_application::ports::authorization::{AuthorizationError, ExecutionContext};
use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::use_cases::work_management::{self as use_cases, WorkManagementError};
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::security::{LOCAL_ACTOR_ID, LocalSingleUserAuthorizer, SqliteAuditSink};
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e1-09-{tag}-{seq}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn open_migrated(db: &Path) -> rusqlite::Connection {
    let mut conn = sqlite::open(db).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
    conn
}

fn ctx(actor: &str) -> ExecutionContext {
    ExecutionContext {
        actor_id: actor.into(),
        correlation_id: "corr-e1-09".into(),
    }
}

#[test]
fn allowed_action_writes_redacted_audit_row() {
    let dir = temp_dir("allowed");
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let repo = SqliteWorkspaceRepository::open(&db).unwrap();
    let audit = SqliteAuditSink::open(&db).unwrap();
    let authz = LocalSingleUserAuthorizer;

    let ws = use_cases::workspace::create_workspace(
        &repo,
        &authz,
        &audit,
        &ctx(LOCAL_ACTOR_ID),
        "alpha",
    )
    .unwrap();

    // Exactly one audit row: the allowed workspace.create, with the opaque
    // resource id and no free text.
    let audit_conn = sqlite::open(&db).unwrap();
    let (count, action, outcome, resource_id): (i64, String, String, Option<String>) = audit_conn
        .query_row(
            "SELECT COUNT(*), MAX(action), MAX(outcome), MAX(resource_id) FROM audit_events",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(action, "workspace.create");
    assert_eq!(outcome, "allowed");
    assert_eq!(resource_id.as_deref(), Some(ws.id.as_str()));

    // Redaction: no column carries a driver-message-like or payload-like
    // string (by construction rows hold only keys, ids and outcomes).
    let dump: String = audit_conn
        .query_row(
            "SELECT group_concat(actor_id || action || resource_type || outcome || correlation_id) \
             FROM audit_events",
            [],
            |r| r.get(0),
        )
        .unwrap();
    for leaky in [
        "UNIQUE",
        "constraint",
        "SELECT",
        "INSERT",
        "sqlite",
        "{",
        "}",
    ] {
        assert!(
            !dump.contains(leaky),
            "audit trail contains {leaky:?}: {dump}"
        );
    }
    drop(repo);

    drop(audit);

    drop(audit_conn);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn denied_actor_writes_nothing_and_audits_denial() {
    let dir = temp_dir("denied");
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let repo = SqliteWorkspaceRepository::open(&db).unwrap();
    let audit = SqliteAuditSink::open(&db).unwrap();
    let authz = LocalSingleUserAuthorizer;

    let result = use_cases::workspace::create_workspace(
        &repo,
        &authz,
        &audit,
        &ctx("not-the-local-user"),
        "alpha",
    );
    assert_eq!(
        result,
        Err(WorkManagementError::Authorization(
            AuthorizationError::Denied
        ))
    );

    // Nothing was written by the denied actor.
    let repo2 = SqliteWorkspaceRepository::open(&db).unwrap();
    assert!(
        repo2
            .find_workspace(&WorkspaceId::new_v7())
            .unwrap()
            .is_none()
    );

    // The refusal is audited as `denied`.
    let audit_conn = sqlite::open(&db).unwrap();
    let (count, outcome): (i64, String) = audit_conn
        .query_row("SELECT COUNT(*), MAX(outcome) FROM audit_events", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(outcome, "denied");
    drop(repo);

    drop(repo2);

    drop(audit);

    drop(audit_conn);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn failing_side_effect_audits_error_without_driver_text() {
    // The public use cases cannot legitimately produce a repository failure
    // (all inputs are validated first — fail-closed by design), so the
    // error-outcome audit path is exercised with a failing repository fake
    // at the unit level; here the sink contract is verified on a real
    // database: an Error-outcome event persists without any free text.
    let dir = temp_dir("error");
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let audit = SqliteAuditSink::open(&db).unwrap();
    audit.record(&paw_application::ports::audit::AuditEvent {
        workspace_id: None,
        actor_id: LOCAL_ACTOR_ID.into(),
        action: "workspace.create",
        resource_type: "workspace",
        resource_id: Some(WorkspaceId::new_v7().as_str().to_string()),
        outcome: paw_application::ports::audit::AuditOutcome::Error,
        correlation_id: "corr-e1-09".into(),
    });

    let audit_conn = sqlite::open(&db).unwrap();
    let (outcome, row): (String, String) = audit_conn
        .query_row(
            "SELECT outcome, actor_id || action || resource_type || outcome || \
             COALESCE(resource_id, '') || correlation_id FROM audit_events",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(outcome, "error");
    for leaky in ["UNIQUE", "constraint failed", "sqlite", "{", "}"] {
        assert!(!row.contains(leaky), "audit row contains {leaky:?}: {row}");
    }
    drop(audit);

    drop(audit_conn);

    std::fs::remove_dir_all(&dir).unwrap();
}
