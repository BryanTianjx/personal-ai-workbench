//! E1-07/E1-08 Tier 1 integration tests: durable Operation lifecycle and
//! transactional Outbox with idempotent consumption.
//!
//! Gate D items exercised (E1 delivery plan §3 Tier 1): outbox persisted
//! atomically with source state; duplicate delivery is harmless; crash
//! (lease expiry) causes safe redelivery; OutcomeUnknown blocks unsafe
//! retry until reconciliation; quarantine after bounded retries. Cancellation
//! and real-provider reconciliation are Tier 2 and not covered here.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_infrastructure::operations::{
    self, AttemptOutcome, AttemptPhase, NewOperation, OperationError, OperationStatus,
};
use paw_infrastructure::outbox::{
    self, ConsumeOutcome, DispatchOptions, NewOutboxEvent, OutboxConsumer, OutboxError,
};
use paw_infrastructure::sqlite;
use rusqlite::Transaction;

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
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn restart_recovery_marks_running_operations_failed() {
    let dir = temp_dir("restart");
    let conn = open_migrated(&dir.join("app.sqlite"));

    let running = operations::create_operation(&conn, &new_operation("backup.run")).unwrap();
    operations::transition(&conn, &running.id, OperationStatus::Running, None).unwrap();
    let scheduled = operations::create_operation(&conn, &new_operation("index.rebuild")).unwrap();
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

    let report = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(100)).unwrap();
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
    let report2 = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(100)).unwrap();
    assert_eq!(report2.deduplicated, 1);
    assert_eq!(effects_count(&conn, "test_consumer"), 1);
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
    let report = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(100)).unwrap();
    assert_eq!(report.delivered, 1);
    assert_eq!(effects_count(&conn, "test_consumer"), 1);
    let status: String = conn
        .query_row("SELECT status FROM outbox_messages", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status, "Delivered");
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
    // quarantined (max_attempts = 3). Backoff (1 * 2^n) is exhausted by the
    // explicit `now` values below.
    for now in [100_i64, 200, 400] {
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
    let report = outbox::dispatch_once(&conn, &[&consumer], &dispatch_opts(100)).unwrap();
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
    std::fs::remove_dir_all(&dir).unwrap();
}
