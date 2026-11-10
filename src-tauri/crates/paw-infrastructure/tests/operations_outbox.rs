//! E1-07/E1-08 Tier 1 integration tests: durable Operation lifecycle and
//! transactional Outbox with idempotent consumption.
//!
//! Gate D items exercised (E1 delivery plan §3 Tier 1): outbox persisted
//! atomically with source state; duplicate delivery is harmless; crash
//! (lease expiry) causes safe redelivery; OutcomeUnknown blocks unsafe
//! retry until reconciliation; quarantine after bounded retries. Cancellation
//! and real-provider reconciliation are Tier 2 and not covered here.
//!
//! Clock convention: the dispatcher's clock is supplied explicitly, so the
//! tests use a fixed far-future epoch `T` — later than every real
//! `available_at` the enqueue path stamps — making claim/backoff sequences
//! deterministic without sleeping.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_infrastructure::operations::{
    self, AttemptOutcome, AttemptPhase, NewOperation, OperationError, OperationStatus,
};
use paw_infrastructure::outbox::{
    self, ConsumeOutcome, DispatchOptions, NewOutboxEvent, OutboxConsumer, OutboxError,
};
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::sqlite;
use rusqlite::Transaction;

/// Fixed dispatcher time (year 2096): after any real enqueue timestamp.
const T: i64 = 4_000_000_000_000;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e1-0708-{tag}-{seq}-{}",
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

fn new_operation(kind: &str) -> NewOperation {
    NewOperation {
        kind: kind.into(),
        owner_domain: "work_management".into(),
        owner_type: "project".into(),
        owner_id: "owner-1".into(),
        workspace_id: None,
        actor_id: "actor-1".into(),
        correlation_id: "corr-1".into(),
        causation_id: None,
        cancellation_support: paw_infrastructure::operations::CancellationSupport::Supported,
    }
}

#[test]
fn operation_lifecycle_happy_path_and_terminal_freeze() {
    let dir = temp_dir("lifecycle");
    let conn = open_migrated(&dir.join("app.sqlite"));

    let op = operations::create_operation(&conn, &new_operation("project.import")).unwrap();
    assert_eq!(op.status, OperationStatus::Pending);

    // A side-effecting attempt is recorded before the transition.
    let attempt = operations::record_attempt(
        &conn,
        &op.id,
        AttemptPhase::Submit,
        AttemptOutcome::Completed,
        None,
        None,
        Some("external-1"),
        false,
        None,
        None,
    )
    .unwrap();

    let running = operations::transition(&conn, &op.id, OperationStatus::Running, None).unwrap();
    assert_eq!(running.status, OperationStatus::Running);

    let done = operations::transition(&conn, &op.id, OperationStatus::Succeeded, None).unwrap();
    assert_eq!(done.status, OperationStatus::Succeeded);

    // Terminal states do not transition (E0.3.2 §2).
    assert!(matches!(
        operations::transition(&conn, &op.id, OperationStatus::Running, None),
        Err(OperationError::InvalidTransition { .. })
    ));

    // Attempt numbers are monotonic per operation.
    let no: i64 = conn
        .query_row(
            "SELECT attempt_no FROM operation_attempts WHERE id = ?1",
            [&attempt],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(no, 1);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn outcome_unknown_blocks_retry_until_reconciliation() {
    let dir = temp_dir("outcome-unknown");
    let conn = open_migrated(&dir.join("app.sqlite"));

    let op = operations::create_operation(&conn, &new_operation("agent.invoke")).unwrap();
    operations::transition(&conn, &op.id, OperationStatus::Running, None).unwrap();
    operations::transition(
        &conn,
        &op.id,
        OperationStatus::OutcomeUnknown,
        Some("timeout_after_submit"),
    )
    .unwrap();

    // The unsafe retry is blocked at state level.
    assert!(matches!(
        operations::transition(&conn, &op.id, OperationStatus::Running, None),
        Err(OperationError::InvalidTransition { .. })
    ));

    // Only explicit reconciliation reopens the envelope.
    let resumed =
        operations::resume_after_reconciliation(&conn, &op.id, "provider confirmed no effect")
            .unwrap();
    assert_eq!(resumed.status, OperationStatus::Running);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn restart_recovery_marks_running_operations_failed() {
    let dir = temp_dir("restart");
    let conn = open_migrated(&dir.join("app.sqlite"));

    let running = operations::create_operation(&conn, &new_operation("backup.run")).unwrap();
    operations::transition(&conn, &running.id, OperationStatus::Running, None).unwrap();
    let scheduled = operations::create_operation(&conn, &new_operation("index.rebuild")).unwrap();
    // RetryScheduled is only reachable from Running (E0.3.2 §2).
    operations::transition(&conn, &scheduled.id, OperationStatus::Running, None).unwrap();
    operations::transition(&conn, &scheduled.id, OperationStatus::RetryScheduled, None).unwrap();

    // "Restart": a fresh process runs recovery against the same database.
    let recovered = operations::recover_interrupted(&conn).unwrap();
    assert_eq!(recovered, 1);

    let after = operations::get_operation(&conn, &running.id)
        .unwrap()
        .unwrap();
    assert_eq!(after.status, OperationStatus::Failed);
    let category: String = conn
        .query_row(
            "SELECT last_error_category FROM operations WHERE id = ?1",
            [&running.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(category, "process_restarted");

    // RetryScheduled survives the restart (it is not Running).
    let still = operations::get_operation(&conn, &scheduled.id)
        .unwrap()
        .unwrap();
    assert_eq!(still.status, OperationStatus::RetryScheduled);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

fn sample_event(event_type: &str) -> NewOutboxEvent {
    NewOutboxEvent {
        workspace_id: None,
        producer_domain: "work_management".into(),
        aggregate_type: "project".into(),
        aggregate_id: "agg-1".into(),
        event_type: event_type.into(),
        schema_version: 1,
        payload_json: r#"{"name":"E1 切片"}"#.into(),
        correlation_id: "corr-1".into(),
        causation_id: None,
    }
}

/// Test consumer: records one row per applied event inside the dispatcher's
/// transaction (the consumer's "local effect").
struct CountingConsumer {
    name: &'static str,
    event_type: &'static str,
    fail_first_n: std::cell::Cell<i64>,
}

impl OutboxConsumer for CountingConsumer {
    fn consumer_name(&self) -> &'static str {
        self.name
    }

    fn event_type(&self) -> &'static str {
        self.event_type
    }

    fn handle(
        &self,
        tx: &Transaction<'_>,
        message: &paw_infrastructure::outbox::OutboxMessage,
    ) -> Result<ConsumeOutcome, String> {
        if self.fail_first_n.get() > 0 {
            self.fail_first_n.set(self.fail_first_n.get() - 1);
            return Err("consumer_transient_failure".into());
        }
        tx.execute(
            "INSERT INTO consumer_effects (consumer_name, message_id, payload) VALUES (?1, ?2, ?3)",
            rusqlite::params![self.consumer_name(), message.id, message.payload_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(ConsumeOutcome::Applied)
    }
}

fn effects_count(conn: &rusqlite::Connection, name: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM consumer_effects WHERE consumer_name = ?1",
        [name],
        |r| r.get(0),
    )
    .unwrap()
}

fn dispatch_opts(now: i64) -> DispatchOptions {
    DispatchOptions {
        now,
        lease_ms: 1_000,
        max_attempts: 3,
        backoff_base_ms: 1,
        batch_size: 8,
    }
}

#[test]
fn outbox_intent_commits_atomically_with_local_state() {
    let dir = temp_dir("atomic");
    let conn = open_migrated(&dir.join("app.sqlite"));

    // Commit case: local state change + outbox intent in one transaction.
    {
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('w1', 'alpha', 1, 1)",
            [],
        )
        .unwrap();
        outbox::enqueue_in_tx(&tx, &sample_event("project.created")).unwrap();
        tx.commit().unwrap();
    }
    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outbox_messages WHERE status = 'Pending'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pending, 1);
    let workspaces: i64 = conn
        .query_row("SELECT COUNT(*) FROM workspaces WHERE id = 'w1'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(workspaces, 1);

    // Rollback case: neither the state change nor the intent survives.
    {
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('w2', 'beta', 2, 2)",
            [],
        )
        .unwrap();
        outbox::enqueue_in_tx(&tx, &sample_event("project.created")).unwrap();
        drop(tx); // rollback
    }
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM outbox_messages", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 1);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn duplicate_delivery_is_harmless() {
    let dir = temp_dir("dedup");
    let conn = open_migrated(&dir.join("app.sqlite"));
    conn.execute(
        "CREATE TABLE consumer_effects (consumer_name TEXT, message_id TEXT, payload TEXT)",
        [],
    )
    .unwrap();

    let consumer = CountingConsumer {
        name: "test_consumer",
        event_type: "project.created",
        fail_first_n: std::cell::Cell::new(0),
    };
    {
        let tx = conn.unchecked_transaction().unwrap();
        outbox::enqueue_in_tx(&tx, &sample_event("project.created")).unwrap();
        tx.commit().unwrap();
    }

    let report = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(T)).unwrap();
    assert_eq!(report.delivered, 1);
    assert_eq!(effects_count(&conn, "test_consumer"), 1);

    // A second dispatch run finds nothing new; simulating a duplicate
    // redelivery of the same event id deduplicates without re-applying.
    {
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "UPDATE outbox_messages SET status = 'Leased', lease_owner = 'single', lease_until = 50 WHERE id = (SELECT id FROM outbox_messages)",
            [],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    let report2 = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(T)).unwrap();
    assert_eq!(report2.deduplicated, 1);
    assert_eq!(effects_count(&conn, "test_consumer"), 1);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn expired_lease_is_redelivered_after_simulated_crash() {
    let dir = temp_dir("lease");
    let conn = open_migrated(&dir.join("app.sqlite"));
    conn.execute(
        "CREATE TABLE consumer_effects (consumer_name TEXT, message_id TEXT, payload TEXT)",
        [],
    )
    .unwrap();

    let consumer = CountingConsumer {
        name: "test_consumer",
        event_type: "project.created",
        fail_first_n: std::cell::Cell::new(0),
    };
    {
        let tx = conn.unchecked_transaction().unwrap();
        outbox::enqueue_in_tx(&tx, &sample_event("project.created")).unwrap();
        tx.commit().unwrap();
    }

    // Crash simulation: a dispatcher claimed (leased) the message and died
    // before any effect. The lease expires; the next run redelivers safely.
    {
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "UPDATE outbox_messages SET status = 'Leased', lease_owner = 'dead-process', lease_until = 50",
            [],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    let report = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(T)).unwrap();
    assert_eq!(report.delivered, 1);
    assert_eq!(effects_count(&conn, "test_consumer"), 1);
    let status: String = conn
        .query_row("SELECT status FROM outbox_messages", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status, "Delivered");
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn poison_message_quarantines_after_bounded_retries() {
    let dir = temp_dir("poison");
    let conn = open_migrated(&dir.join("app.sqlite"));
    conn.execute(
        "CREATE TABLE consumer_effects (consumer_name TEXT, message_id TEXT, payload TEXT)",
        [],
    )
    .unwrap();

    let consumer = CountingConsumer {
        name: "test_consumer",
        event_type: "project.created",
        fail_first_n: std::cell::Cell::new(i64::MAX), // always fails
    };
    {
        let tx = conn.unchecked_transaction().unwrap();
        outbox::enqueue_in_tx(&tx, &sample_event("project.created")).unwrap();
        tx.commit().unwrap();
    }

    // Three dispatch runs: fail → rescheduled, fail → rescheduled, fail →
    // quarantined (max_attempts = 3). Backoff is 1 * 2^n ms, exhausted by
    // the explicit dispatcher clocks below.
    for now in [T, T + 1_000, T + 2_000] {
        outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(now)).unwrap();
    }
    let (status, category): (String, String) = conn
        .query_row(
            "SELECT status, last_error_category FROM outbox_messages",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "Quarantined");
    assert_eq!(category, "consumer_transient_failure");
    // No local effect ever committed despite repeated attempts.
    assert_eq!(effects_count(&conn, "test_consumer"), 0);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn event_without_consumer_quarantines_immediately() {
    let dir = temp_dir("no-consumer");
    let conn = open_migrated(&dir.join("app.sqlite"));
    conn.execute(
        "CREATE TABLE consumer_effects (consumer_name TEXT, message_id TEXT, payload TEXT)",
        [],
    )
    .unwrap();

    {
        let tx = conn.unchecked_transaction().unwrap();
        outbox::enqueue_in_tx(&tx, &sample_event("totally.unknown")).unwrap();
        tx.commit().unwrap();
    }
    let consumer = CountingConsumer {
        name: "test_consumer",
        event_type: "project.created",
        fail_first_n: std::cell::Cell::new(0),
    };
    let report = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(T)).unwrap();
    assert_eq!(report.quarantined, 1);
    let (status, category): (String, String) = conn
        .query_row(
            "SELECT status, last_error_category FROM outbox_messages",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "Quarantined");
    assert_eq!(category, "no_consumer");
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn invalid_payload_json_is_rejected_before_sqlite() {
    let dir = temp_dir("payload");
    let conn = open_migrated(&dir.join("app.sqlite"));
    let mut bad = sample_event("project.created");
    bad.payload_json = "{not json".into();
    let tx = conn.unchecked_transaction().unwrap();
    assert!(matches!(
        outbox::enqueue_in_tx(&tx, &bad),
        Err(OutboxError::InvalidPayload(_))
    ));
    drop(tx);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------
// E3-01: retry classification, due-claim scheduling, cancellation semantics
// ---------------------------------------------------------------------------

use paw_infrastructure::operations as ops;
use paw_infrastructure::operations::{CancellationSupport, FailureClassification, RetryPolicy};

#[test]
fn rejected_classification_schedules_backoff_and_due_claim_reruns() {
    let dir = temp_dir("e3-classify-rejected");
    let conn = open_migrated(&dir.join("app.sqlite"));
    let policy = RetryPolicy {
        base_ms: 500,
        max_backoff_ms: 4_000,
    };

    let op = ops::create_operation(&conn, &new_operation("index.rebuild")).unwrap();
    ops::transition(&conn, &op.id, OperationStatus::Running, None).unwrap();

    let scheduled = ops::classify_failure_and_schedule_retry(
        &conn,
        &op.id,
        FailureClassification::RejectedBeforeSubmit,
        "validated_out",
        &policy,
    )
    .unwrap();
    assert_eq!(scheduled.status, OperationStatus::RetryScheduled);
    let next = scheduled.next_attempt_at.expect("next_attempt_at set");

    // Not due before the backoff elapses; due after it.
    assert!(ops::claim_due_operation(&conn, next - 1).unwrap().is_none());
    let rerun = ops::claim_due_operation(&conn, next).unwrap().expect("due");
    assert_eq!(rerun.status, OperationStatus::Running);
    assert_eq!(rerun.next_attempt_at, None);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn accepted_classification_stays_running_and_indeterminate_blocks() {
    let dir = temp_dir("e3-classify-accept-unknown");
    let conn = open_migrated(&dir.join("app.sqlite"));

    let accepted = ops::create_operation(&conn, &new_operation("backup.run")).unwrap();
    ops::transition(&conn, &accepted.id, OperationStatus::Running, None).unwrap();
    let after = ops::classify_failure_and_schedule_retry(
        &conn,
        &accepted.id,
        FailureClassification::Accepted,
        "target_accepted",
        &RetryPolicy::default(),
    )
    .unwrap();
    // Poll/reconcile: still Running, no resubmission scheduling.
    assert_eq!(after.status, OperationStatus::Running);
    assert_eq!(after.next_attempt_at, None);

    let unknown = ops::create_operation(&conn, &new_operation("agent.invoke")).unwrap();
    ops::transition(&conn, &unknown.id, OperationStatus::Running, None).unwrap();
    let blocked = ops::classify_failure_and_schedule_retry(
        &conn,
        &unknown.id,
        FailureClassification::Indeterminate,
        "timeout_after_submit",
        &RetryPolicy::default(),
    )
    .unwrap();
    assert_eq!(blocked.status, OperationStatus::OutcomeUnknown);

    // The unsafe retry is blocked until reconciliation.
    assert!(matches!(
        ops::transition(&conn, &unknown.id, OperationStatus::Running, None),
        Err(OperationError::InvalidTransition { .. })
    ));
    ops::resume_after_reconciliation(&conn, &unknown.id, "provider confirmed no effect").unwrap();
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn backoff_grows_exponentially_and_caps() {
    let policy = RetryPolicy {
        base_ms: 1_000,
        max_backoff_ms: 60_000,
    };
    // Private helper re-derivation: 1st failure 1s, 2nd 2s, 3rd 4s, cap 60s.
    let derive = |failures: i64| {
        let exp = (failures - 1).clamp(0, 16);
        (policy.base_ms.saturating_mul(1i64 << exp)).min(policy.max_backoff_ms)
    };
    assert_eq!(derive(1), 1_000);
    assert_eq!(derive(2), 2_000);
    assert_eq!(derive(3), 4_000);
    assert_eq!(derive(7), 60_000, "capped at max_backoff_ms");
}

#[test]
fn cancellation_request_semantics() {
    let dir = temp_dir("e3-cancel");
    let conn = open_migrated(&dir.join("app.sqlite"));

    // Supported capability: the request is stamped; the cooperative
    // Cancelled transition then goes through the state machine.
    let mut supported = new_operation("export.data");
    supported.cancellation_support = CancellationSupport::Supported;
    let op = ops::create_operation(&conn, &supported).unwrap();
    ops::transition(&conn, &op.id, OperationStatus::Running, None).unwrap();
    let requested = ops::request_cancellation(&conn, &op.id).unwrap();
    assert!(requested.cancellation_requested_at.is_some());
    let cancelled = ops::transition(&conn, &op.id, OperationStatus::Cancelled, None).unwrap();
    assert_eq!(cancelled.status, OperationStatus::Cancelled);
    // A request on a terminal envelope is refused.
    assert!(matches!(
        ops::request_cancellation(&conn, &op.id),
        Err(ops::ClassificationError::InvalidTransition { .. })
    ));

    // Unsupported capability: explicit typed error, no stamp.
    let mut unsupported = new_operation("hard.job");
    unsupported.cancellation_support = CancellationSupport::Unsupported;
    let hard = ops::create_operation(&conn, &unsupported).unwrap();
    ops::transition(&conn, &hard.id, OperationStatus::Running, None).unwrap();
    assert!(matches!(
        ops::request_cancellation(&conn, &hard.id),
        Err(ops::ClassificationError::InvalidTransition { .. })
    ));
    let stamp: Option<i64> = conn
        .query_row(
            "SELECT cancellation_requested_at FROM operations WHERE id = ?1",
            [&hard.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stamp, None);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn list_operations_is_workspace_scoped_and_filtered() {
    let dir = temp_dir("e3-list");
    let conn = open_migrated(&dir.join("app.sqlite"));

    // The FK requires the workspace row to exist.
    conn.execute(
        "INSERT INTO workspaces (id, name, created_at, updated_at) \
         VALUES ('11111111-1111-7111-8111-111111111111', '队列空间', 1, 1)",
        [],
    )
    .unwrap();

    let mut scoped = new_operation("report.build");
    scoped.workspace_id = Some("11111111-1111-7111-8111-111111111111".into());
    let in_ws = ops::create_operation(&conn, &scoped).unwrap();
    let system = ops::create_operation(&conn, &new_operation("system.job")).unwrap();
    assert_ne!(in_ws.workspace_id, system.workspace_id);

    let listed = ops::list_operations(
        &conn,
        "11111111-1111-7111-8111-111111111111",
        Some(OperationStatus::Pending),
    )
    .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, in_ws.id);

    // System-scope rows (NULL workspace) never appear in a workspace query.
    let listed_all =
        ops::list_operations(&conn, "11111111-1111-7111-8111-111111111111", None).unwrap();
    assert_eq!(listed_all.len(), 1);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------
// E3-02/E3-03: re-drive, retention export/purge, audit detach-on-delete
// ---------------------------------------------------------------------------

use paw_infrastructure::outbox::QuarantinedMessage;
use paw_infrastructure::retention::{self, RetentionPolicy};

fn quarantined_event() -> NewOutboxEvent {
    NewOutboxEvent {
        workspace_id: None,
        producer_domain: "work_management".into(),
        aggregate_type: "project".into(),
        aggregate_id: "agg-9".into(),
        event_type: "poison.event".into(),
        schema_version: 1,
        payload_json: r#"{"bad":true}"#.into(),
        correlation_id: "corr-9".into(),
        causation_id: None,
    }
}

#[test]
fn redrive_preserves_event_id_and_records_operator() {
    let dir = temp_dir("e3-redrive");
    let conn = open_migrated(&dir.join("app.sqlite"));
    conn.execute(
        "CREATE TABLE consumer_effects (consumer_name TEXT, message_id TEXT, payload TEXT)",
        [],
    )
    .unwrap();

    let event_id;
    {
        let tx = conn.unchecked_transaction().unwrap();
        event_id = outbox::enqueue_in_tx(&tx, &quarantined_event()).unwrap();
        tx.commit().unwrap();
    }

    // No consumer → quarantined immediately.
    outbox::dispatch_once(&conn, &[], &dispatch_opts(T)).unwrap();
    let quarantined: QuarantinedMessage = outbox::list_quarantined(&conn, None).unwrap().remove(0);
    assert_eq!(quarantined.id, event_id);
    assert_eq!(
        quarantined.last_error_category.as_deref(),
        Some("no_consumer")
    );

    // Re-drive preserves the event id, records the operator, and makes the
    // message dispatchable again.
    let redriven = outbox::redrive_quarantined(&conn, &event_id, "owner-admin", T + 5).unwrap();
    assert_eq!(redriven, event_id);

    // Still no consumer → quarantined again, but with re-drive bookkeeping.
    outbox::dispatch_once(&conn, &[], &dispatch_opts(T + 10)).unwrap();
    let again = outbox::list_quarantined(&conn, None).unwrap().remove(0);
    assert_eq!(again.id, event_id, "event id preserved across re-drive");
    assert_eq!(again.redrive_count, 1);
    assert_eq!(again.last_redrive_by.as_deref(), Some("owner-admin"));
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn retention_export_and_purge_boundaries() {
    let dir = temp_dir("e3-retention");
    let conn = open_migrated(&dir.join("app.sqlite"));

    // The operations live in one workspace (export is workspace-scoped).
    let ws_id = "11111111-1111-7111-8111-111111111113";
    conn.execute(
        "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?1, '保留空间', 1, 1)",
        [ws_id],
    )
    .unwrap();
    let mut scoped = new_operation("old.job");
    scoped.workspace_id = Some(ws_id.to_string());

    // Two operations: one terminal long ago, one reconciled unknown recently.
    let old_done = ops::create_operation(&conn, &scoped).unwrap();
    ops::transition(&conn, &old_done.id, OperationStatus::Running, None).unwrap();
    ops::transition(&conn, &old_done.id, OperationStatus::Failed, None).unwrap();
    ops::record_attempt(
        &conn,
        &old_done.id,
        AttemptPhase::Submit,
        AttemptOutcome::Failed,
        None,
        None,
        None,
        false,
        Some("boom"),
        None,
    )
    .unwrap();
    let old_finished: i64 = conn
        .query_row(
            "SELECT finished_at FROM operations WHERE id = ?1",
            [&old_done.id],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute(
        "UPDATE operations SET finished_at = ?1 WHERE id = ?2",
        rusqlite::params![old_finished - 91 * 24 * 60 * 60 * 1000, old_done.id],
    )
    .unwrap();

    let mut scoped_fresh = scoped.clone();
    scoped_fresh.kind = "fresh.job".into();
    let fresh_unknown = ops::create_operation(&conn, &scoped_fresh).unwrap();
    ops::transition(&conn, &fresh_unknown.id, OperationStatus::Running, None).unwrap();
    ops::classify_failure_and_schedule_retry(
        &conn,
        &fresh_unknown.id,
        FailureClassification::Indeterminate,
        "timeout",
        &RetryPolicy::default(),
    )
    .unwrap();

    // Export includes both with their attempts (user-initiated, pre-purge).
    let export = retention::export_operations_json(&conn, ws_id).unwrap();
    assert!(export.contains("old.job"));
    assert!(export.contains("fresh.job"));
    assert!(export.contains("attempts"));
    let parsed: serde_json::Value = serde_json::from_str(&export).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);

    // Purge at the default policy: the 91-day-old terminal row goes; the
    // unreconciled unknown never expires automatically.
    let now: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let (operations, attempts) =
        retention::purge_expired_operations(&conn, now, &RetentionPolicy::default()).unwrap();
    assert_eq!((operations, attempts), (1, 1));
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM operations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining, 1, "unreconciled unknown survives");
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn workspace_delete_detaches_audit_rows_to_system_scope() {
    let dir = temp_dir("e3-audit-detach");
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    // A workspace with one audited action (direct audit_events insert: the
    // sink is System & Security's; this test exercises the migration-0007
    // FK semantics, not the sink).
    let ws_id = "11111111-1111-7111-8111-111111111112";
    {
        let conn = sqlite::open(&db).unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?1, '带审计', 1, 1)",
            [ws_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_events (id, workspace_id, actor_id, action, resource_type, \
             resource_id, outcome, correlation_id, created_at)
             VALUES ('a-1', ?1, 'local-user', 'workspace.create', 'workspace', ?1, 'allowed', \
             'corr', 1)",
            [ws_id],
        )
        .unwrap();
        drop(conn);
    }

    // Deletion succeeds despite audit history (migration 0007: SET NULL) —
    // the E1 refusal is closed by policy.
    {
        let repo = SqliteWorkspaceRepository::open(&db).unwrap();
        use paw_application::ports::workspace_repository::WorkspaceRepository;
        repo.delete_workspace(&paw_application::WorkspaceId::new(ws_id).unwrap())
            .unwrap();
        assert!(
            repo.find_workspace(&paw_application::WorkspaceId::new(ws_id).unwrap())
                .unwrap()
                .is_none()
        );
    }

    // The audit rows survive as system-scope evidence (workspace_id NULL).
    let audit_conn = sqlite::open(&db).unwrap();
    let (detached, scoped_left): (i64, i64) = audit_conn
        .query_row(
            "SELECT COUNT(*) FILTER (WHERE workspace_id IS NULL), \
             COUNT(*) FILTER (WHERE workspace_id IS NOT NULL) FROM audit_events",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(detached, 1, "audit row survives as system scope");
    assert_eq!(
        scoped_left, 0,
        "no audit row still binds the deleted workspace"
    );
    drop(audit_conn);
    std::fs::remove_dir_all(&dir).unwrap();
}
