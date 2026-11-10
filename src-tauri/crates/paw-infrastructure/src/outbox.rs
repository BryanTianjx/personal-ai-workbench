//! Transactional Outbox and idempotent consumption (E1-08, Tier 1).
//!
//! Logical contract: `E0.3.2-engineering-specification.md` §4/§5; schema:
//! migration `0002_durable_operations_outbox.sql`. Tier 1 per the E1
//! delivery plan §3: intent is committed atomically with the source state
//! change (callers enqueue inside their own transaction), the dispatcher is
//! an in-process single-threaded poll over a bounded batch with a lease, and
//! consumers deduplicate through `processed_messages` committed in the same
//! transaction as their local effect. Bounded retries quarantine poison
//! messages; no network, no external provider, no background thread here.

use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::sqlite;

/// Outbox message status; strings match the schema CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    Leased,
    Delivered,
    Quarantined,
}

impl OutboxStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Leased => "Leased",
            Self::Delivered => "Delivered",
            Self::Quarantined => "Quarantined",
        }
    }
}

/// Fields a producer supplies to publish one versioned event.
#[derive(Debug, Clone)]
pub struct NewOutboxEvent {
    /// `Some` for workspace-scoped events; `None` only for explicitly
    /// system-scoped events.
    pub workspace_id: Option<String>,
    pub producer_domain: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub schema_version: i64,
    /// Minimal, versioned payload — never secrets, prompts or full bodies.
    pub payload_json: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
}

/// A claimed outbox message handed to a consumer.
#[derive(Debug, Clone)]
pub struct OutboxMessage {
    pub id: String,
    pub event_type: String,
    pub schema_version: i64,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub attempts: i64,
    pub payload_json: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OutboxError {
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("clock: {source}")]
    NowMsFailed { source: std::time::SystemTimeError },
}

impl From<sqlite::Error> for OutboxError {
    fn from(e: sqlite::Error) -> Self {
        match e {
            sqlite::Error::NowMsFailed { source } => Self::NowMsFailed { source },
            other => Self::InvalidPayload(other.to_string()),
        }
    }
}

fn now_ms() -> Result<i64, OutboxError> {
    Ok(sqlite::now_ms()?)
}

fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<OutboxMessage> {
    Ok(OutboxMessage {
        id: row.get("id")?,
        event_type: row.get("event_type")?,
        schema_version: row.get("schema_version")?,
        aggregate_type: row.get("aggregate_type")?,
        aggregate_id: row.get("aggregate_id")?,
        attempts: row.get("attempts")?,
        payload_json: row.get("payload_json")?,
    })
}

/// Enqueue one event **inside the caller's transaction** so the durable
/// intent commits atomically with the local state change that requires it
/// (`E0.3.2` §1.4). Returns the generated event id.
pub fn enqueue_in_tx(tx: &Transaction<'_>, event: &NewOutboxEvent) -> Result<String, OutboxError> {
    // Guard the json_valid() CHECK with a typed error before SQLite sees it.
    if serde_json::from_str::<serde_json::Value>(&event.payload_json).is_err() {
        return Err(OutboxError::InvalidPayload(event.payload_json.clone()));
    }
    let id = uuid::Uuid::now_v7().to_string();
    let now = now_ms()?;
    tx.execute(
        "INSERT INTO outbox_messages (id, workspace_id, producer_domain, aggregate_type, \
         aggregate_id, event_type, schema_version, occurred_at, correlation_id, causation_id, \
         payload_json, status, attempts, available_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'Pending', 0, ?8, ?8)",
        params![
            id,
            event.workspace_id,
            event.producer_domain,
            event.aggregate_type,
            event.aggregate_id,
            event.event_type,
            event.schema_version,
            now,
            event.correlation_id,
            event.causation_id,
            event.payload_json,
        ],
    )?;
    Ok(id)
}

/// Outcome a consumer reports for one message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeOutcome {
    /// The consumer applied its local effect.
    Applied,
    /// The consumer recognized the event but had nothing to apply (allowed
    /// only where no required invariant depends on it, `E0.3.2` §5).
    Ignored,
}

/// An idempotent in-process consumer of outbox events. `handle` runs inside
/// the dispatcher's transaction: the consumer's local effect and the dedup
/// marker commit atomically.
pub trait OutboxConsumer {
    fn consumer_name(&self) -> &'static str;
    /// Event type this consumer subscribes to.
    fn event_type(&self) -> &'static str;
    fn handle(
        &self,
        tx: &Transaction<'_>,
        message: &OutboxMessage,
    ) -> Result<ConsumeOutcome, String>;
}

/// Dispatcher knobs; `now` is explicit for deterministic tests.
#[derive(Debug, Clone)]
pub struct DispatchOptions {
    /// Dispatcher time (UTC epoch ms).
    pub now: i64,
    /// Lease duration handed to the claiming dispatcher.
    pub lease_ms: i64,
    /// Dispatch attempts before a message is quarantined.
    pub max_attempts: i64,
    /// Base for exponential retry backoff: `base * 2^(attempts-1)`.
    pub backoff_base_ms: i64,
    /// Maximum messages claimed per run.
    pub batch_size: usize,
}

impl Default for DispatchOptions {
    fn default() -> Self {
        Self {
            now: 0,
            lease_ms: 30_000,
            max_attempts: 3,
            backoff_base_ms: 1_000,
            batch_size: 16,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DispatchReport {
    pub claimed: usize,
    pub delivered: usize,
    pub deduplicated: usize,
    pub quarantined: usize,
}

/// Claim one bounded batch (Pending or expired-Leased), then dispatch each
/// message to its consumer in a short transaction: consumer effect + dedup
/// marker + `Delivered` commit together. A consumer failure rolls back and
/// reschedules with bounded exponential backoff until `max_attempts`, then
/// quarantines. Messages with no registered consumer quarantine immediately
/// with the safe category `no_consumer`.
pub fn dispatch_once(
    conn: &Connection,
    consumers: &[&dyn OutboxConsumer],
    options: &DispatchOptions,
) -> Result<DispatchReport, OutboxError> {
    let mut report = DispatchReport::default();

    // 1. Claim a bounded batch in one short transaction.
    {
        let tx = conn.unchecked_transaction()?;
        let ids: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT id FROM outbox_messages
                 WHERE (status = 'Pending' AND available_at <= ?1)
                    OR (status = 'Leased' AND lease_until < ?1)
                 ORDER BY created_at
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![options.now, options.batch_size as i64], |row| {
                row.get::<_, String>(0)
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for id in &ids {
            tx.execute(
                "UPDATE outbox_messages SET status = 'Leased', lease_owner = 'single', \
                 lease_until = ?1 WHERE id = ?2",
                params![options.now + options.lease_ms, id],
            )?;
        }
        tx.commit()?;
        report.claimed = ids.len();
    }

    // 2. Dispatch each claimed message.
    for id in read_claimed_ids(conn, options.now)? {
        let Some(message) = load_message(conn, &id)? else {
            continue;
        };
        let Some(consumer) = consumers
            .iter()
            .copied()
            .find(|c| c.event_type() == message.event_type)
        else {
            // Unknown event type: quarantined with a safe diagnostic, never
            // retried forever (`E0.3.2` §5: unknown types are ignored only
            // when no invariant depends on them — with no consumer at all
            // there is nothing to depend on, but the event is preserved).
            quarantine(conn, &message.id, "no_consumer")?;
            report.quarantined += 1;
            continue;
        };
        dispatch_one(conn, consumer, &message, options, &mut report)?;
    }

    Ok(report)
}

fn read_claimed_ids(conn: &Connection, now: i64) -> Result<Vec<String>, OutboxError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM outbox_messages WHERE status = 'Leased' AND lease_owner = 'single' \
         AND lease_until > ?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map([now], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn load_message(conn: &Connection, id: &str) -> Result<Option<OutboxMessage>, OutboxError> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, schema_version, aggregate_type, aggregate_id, attempts, \
         payload_json FROM outbox_messages WHERE id = ?1",
    )?;
    Ok(stmt.query_row([id], row_to_message).optional()?)
}

fn dispatch_one(
    conn: &Connection,
    consumer: &dyn OutboxConsumer,
    message: &OutboxMessage,
    options: &DispatchOptions,
    report: &mut DispatchReport,
) -> Result<(), OutboxError> {
    let tx = conn.unchecked_transaction()?;

    // Duplicate delivery guard: the dedup marker commits with the effect, so
    // a prior Applied/Ignored outcome replays without re-running the handler.
    let prior: Option<String> = tx
        .query_row(
            "SELECT outcome FROM processed_messages WHERE consumer_name = ?1 AND message_id = ?2",
            params![consumer.consumer_name(), message.id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(_prior_outcome) = prior {
        tx.execute(
            "UPDATE outbox_messages SET status = 'Delivered', delivered_at = COALESCE(?1, delivered_at), \
             attempts = attempts + 1 WHERE id = ?2",
            params![options.now, message.id],
        )?;
        tx.commit()?;
        report.deduplicated += 1;
        return Ok(());
    }

    match consumer.handle(&tx, message) {
        Ok(outcome) => {
            tx.execute(
                "INSERT INTO processed_messages (consumer_name, message_id, processed_at, outcome) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    consumer.consumer_name(),
                    message.id,
                    options.now,
                    outcome.as_str()
                ],
            )?;
            tx.execute(
                "UPDATE outbox_messages SET status = 'Delivered', delivered_at = ?1, \
                 attempts = attempts + 1, lease_owner = NULL, lease_until = NULL WHERE id = ?2",
                params![options.now, message.id],
            )?;
            tx.commit()?;
            report.delivered += 1;
            Ok(())
        }
        Err(category) => {
            // Roll the consumer's partial effect back with the transaction.
            drop(tx);
            fail_message(conn, &message.id, &category, options)?;
            report.quarantined += 1;
            Ok(())
        }
    }
}

fn fail_message(
    conn: &Connection,
    id: &str,
    category: &str,
    options: &DispatchOptions,
) -> Result<(), OutboxError> {
    let attempts: i64 = conn.query_row(
        "SELECT attempts FROM outbox_messages WHERE id = ?1",
        [id],
        |r| r.get(0),
    )?;
    let next_attempts = attempts + 1;
    if next_attempts >= options.max_attempts {
        quarantine(conn, id, category)?;
    } else {
        let backoff = options.backoff_base_ms * (1 << (next_attempts - 1).min(16));
        conn.execute(
            "UPDATE outbox_messages SET status = 'Pending', attempts = ?1, \
             available_at = ?2, last_error_category = ?3, lease_owner = NULL, lease_until = NULL \
             WHERE id = ?4",
            params![next_attempts, options.now + backoff, category, id],
        )?;
    }
    Ok(())
}

fn quarantine(conn: &Connection, id: &str, category: &str) -> Result<(), OutboxError> {
    conn.execute(
        "UPDATE outbox_messages SET status = 'Quarantined', last_error_category = ?1, \
         lease_owner = NULL, lease_until = NULL WHERE id = ?2",
        params![category, id],
    )?;
    Ok(())
}

impl ConsumeOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "Applied",
            Self::Ignored => "Ignored",
        }
    }
}

/// Retry pacing constant for reference (Tier 1 uses the caller-provided
/// base); kept pub for the E3 service that wraps this dispatcher.
pub const DISPATCH_PACE: Duration = Duration::from_millis(5);

// ---------------------------------------------------------------------------
// E3-02: quarantine visibility and explicit re-drive
// ---------------------------------------------------------------------------

/// A quarantined message as surfaced by the queue view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantinedMessage {
    pub id: String,
    pub event_type: String,
    pub producer_domain: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub attempts: i64,
    pub redrive_count: i64,
    pub last_error_category: Option<String>,
    pub last_redrive_by: Option<String>,
    pub payload_json: String,
}

fn row_to_quarantined(row: &rusqlite::Row<'_>) -> rusqlite::Result<QuarantinedMessage> {
    Ok(QuarantinedMessage {
        id: row.get("id")?,
        event_type: row.get("event_type")?,
        producer_domain: row.get("producer_domain")?,
        aggregate_type: row.get("aggregate_type")?,
        aggregate_id: row.get("aggregate_id")?,
        attempts: row.get("attempts")?,
        redrive_count: row.get("redrive_count")?,
        last_error_category: row.get("last_error_category")?,
        last_redrive_by: row.get("last_redrive_by")?,
        payload_json: row.get("payload_json")?,
    })
}

/// List every quarantined message (poison queue view). Workspace-scoped when
/// a workspace filter is supplied; system-scope rows (NULL workspace) are
/// only visible to the unfiltered system view.
pub fn list_quarantined(
    conn: &Connection,
    workspace_id: Option<&str>,
) -> Result<Vec<QuarantinedMessage>, OutboxError> {
    let (sql, bind_ws): (&str, Option<Option<&str>>) = match workspace_id {
        Some(ws) => (
            "SELECT id, event_type, producer_domain, aggregate_type, aggregate_id, attempts, \
             redrive_count, last_error_category, last_redrive_by, payload_json
             FROM outbox_messages WHERE status = 'Quarantined' AND workspace_id = ?1
             ORDER BY created_at ASC",
            Some(Some(ws)),
        ),
        None => (
            "SELECT id, event_type, producer_domain, aggregate_type, aggregate_id, attempts, \
             redrive_count, last_error_category, last_redrive_by, payload_json
             FROM outbox_messages WHERE status = 'Quarantined'
             ORDER BY created_at ASC",
            None,
        ),
    };
    let mut stmt = conn.prepare(sql)?;
    let mut rows = match bind_ws {
        Some(Some(ws)) => stmt.query(params![ws])?,
        _ => stmt.query([])?,
    };
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(row_to_quarantined(row)?);
    }
    Ok(out)
}

/// Re-drive one quarantined message back into `Pending` (E0.3.2 §4: an
/// explicit action that preserves the event id and records the operator;
/// never silently a different event). Returns the message id on success.
pub fn redrive_quarantined(
    conn: &Connection,
    message_id: &str,
    operator: &str,
    now: i64,
) -> Result<String, OutboxError> {
    let changed = conn.execute(
        "UPDATE outbox_messages SET status = 'Pending', available_at = ?1, \
         redrive_count = redrive_count + 1, last_redrive_by = ?2, \
         lease_owner = NULL, lease_until = NULL
         WHERE id = ?3 AND status = 'Quarantined'",
        params![now, operator, message_id],
    )?;
    if changed == 0 {
        return Err(OutboxError::InvalidPayload(format!(
            "message {message_id} is not quarantined (or does not exist)"
        )));
    }
    Ok(message_id.to_string())
}
