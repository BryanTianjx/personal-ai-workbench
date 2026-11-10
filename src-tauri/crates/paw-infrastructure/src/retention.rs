//! Retention policy, export and purge for durable-operation history and
//! audit events (E3-02 / E3-03).
//!
//! Policy baseline (E0.3.2 engineering specification §7):
//! - successful/failed/cancelled operation metadata is retained for
//!   [`RetentionPolicy::terminal_days`] (default 90) after its
//!   `finished_at`;
//! - `OutcomeUnknown` operations are retained until resolved, then for the
//!   same grace period — implemented as: unknown operations whose
//!   `resolution_note` is set (i.e. reconciled) expire from
//!   `updated_at + resolved_grace_days`; unreconciled unknown operations
//!   never expire automatically;
//! - export is user-initiated and available before any purge;
//! - there is no automatic purge: purging is an explicit command, and
//!   quarantined outbox messages are never purged by this module at all
//!   (they must be re-driven or explicitly resolved first).
//!
//! All numbers are configurable through [`RetentionPolicy`]; the defaults
//! encode the specification values.

use rusqlite::{Connection, params};

use crate::sqlite;

/// Retention configuration (days). Defaults encode the E0.3.2 §7 baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// Days a terminal (Succeeded/Failed/Cancelled) operation is retained
    /// after `finished_at`.
    pub terminal_days: i64,
    /// Days a reconciled `OutcomeUnknown` operation is retained after
    /// `updated_at`. Unreconciled unknown operations never expire.
    pub resolved_grace_days: i64,
    /// Days audit events are retained (E3-03; configurable, default 365).
    pub audit_days: i64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            terminal_days: 90,
            resolved_grace_days: 90,
            audit_days: 365,
        }
    }
}

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, thiserror::Error)]
pub enum RetentionError {
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("serialization: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Purge terminal operations whose `finished_at` elapsed `terminal_days`
/// and reconciled `OutcomeUnknown` operations whose `updated_at` elapsed
/// `resolved_grace_days`. Attempt rows are deleted first (RESTRICT FK).
/// Unreconciled unknown operations and quarantined messages are never
/// purged here. Returns the number of operation rows removed.
pub fn purge_expired_operations(
    conn: &Connection,
    now: i64,
    policy: &RetentionPolicy,
) -> Result<(usize, usize), RetentionError> {
    let terminal_cutoff = now - policy.terminal_days * DAY_MS;
    let resolved_cutoff = now - policy.resolved_grace_days * DAY_MS;

    // Attempt rows of the doomed operations first.
    let attempts = conn.execute(
        "DELETE FROM operation_attempts WHERE operation_id IN (
             SELECT id FROM operations
             WHERE (status IN ('Succeeded','Failed','Cancelled')
                    AND finished_at IS NOT NULL AND finished_at < ?1)
                 OR (status = 'OutcomeUnknown' AND resolution_note IS NOT NULL
                     AND updated_at < ?2)
         )",
        params![terminal_cutoff, resolved_cutoff],
    )?;
    let operations = conn.execute(
        "DELETE FROM operations
         WHERE (status IN ('Succeeded','Failed','Cancelled')
                AND finished_at IS NOT NULL AND finished_at < ?1)
             OR (status = 'OutcomeUnknown' AND resolution_note IS NOT NULL
                 AND updated_at < ?2)",
        params![terminal_cutoff, resolved_cutoff],
    )?;
    Ok((operations, attempts))
}

/// Purge audit events older than `policy.audit_days` (explicit command;
/// E3-03 policy). Returns the number of rows removed.
pub fn purge_expired_audit_events(
    conn: &Connection,
    now: i64,
    policy: &RetentionPolicy,
) -> Result<usize, RetentionError> {
    let cutoff = now - policy.audit_days * DAY_MS;
    let removed = conn.execute(
        "DELETE FROM audit_events WHERE created_at < ?1",
        params![cutoff],
    )?;
    Ok(removed)
}

/// Export the full durable-operation history (operations + attempts) of one
/// workspace as a JSON document — the user-initiated step before any purge.
/// Rows carry no secrets by schema construction; the export therefore never
/// introduces any.
pub fn export_operations_json(
    conn: &Connection,
    workspace_id: &str,
) -> Result<String, RetentionError> {
    let mut operations: Vec<serde_json::Value> = Vec::new();

    let mut stmt = conn.prepare(
        "SELECT id, kind, owner_domain, owner_type, owner_id, workspace_id, status, \
         actor_id, correlation_id, causation_id, idempotency_key, request_digest, \
         cancellation_support, cancellation_requested_at, created_at, updated_at, \
         started_at, finished_at, next_attempt_at, last_error_category, last_error_code, \
         resolution_note
         FROM operations WHERE workspace_id = ?1 ORDER BY created_at ASC",
    )?;
    let op_columns: Vec<String> = (0..stmt.column_count())
        .map(|i| stmt.column_name(i).unwrap().to_string())
        .collect();
    let mut rows = stmt.query(params![workspace_id])?;
    while let Some(row) = rows.next()? {
        let mut obj = serde_json::Map::new();
        for (i, name) in op_columns.iter().enumerate() {
            let value = rusqlite_value_to_json(row, i)?;
            obj.insert(name.clone(), value);
        }
        let operation_id = row.get::<_, String>(0)?;
        let mut attempts: Vec<serde_json::Value> = Vec::new();
        let mut attempt_stmt = conn.prepare(
            "SELECT id, operation_id, attempt_no, phase, started_at, finished_at, outcome, \
             request_digest, idempotency_key, external_reference, retryable, error_category, \
             error_code, evidence_ref
             FROM operation_attempts WHERE operation_id = ?1 ORDER BY attempt_no ASC",
        )?;
        let attempt_columns: Vec<String> = (0..attempt_stmt.column_count())
            .map(|i| attempt_stmt.column_name(i).unwrap().to_string())
            .collect();
        let mut attempt_rows = attempt_stmt.query(params![operation_id])?;
        while let Some(attempt_row) = attempt_rows.next()? {
            let mut obj = serde_json::Map::new();
            for (i, name) in attempt_columns.iter().enumerate() {
                let value = rusqlite_value_to_json(attempt_row, i)?;
                obj.insert(name.clone(), value);
            }
            attempts.push(serde_json::Value::Object(obj));
        }
        obj.insert("attempts".into(), serde_json::Value::Array(attempts));
        operations.push(serde_json::Value::Object(obj));
    }

    Ok(serde_json::to_string_pretty(&operations)?)
}

// now_ms re-export guard: the module reads the shared clock through
// sqlite::now_ms at call sites (purge commands stamp nothing; `now` is
// passed in for deterministic tests).
#[allow(unused_imports)]
use sqlite as _sqlite;

/// Read one column as a JSON value: SQLite storage classes map directly
/// (INTEGER → number, REAL → number, TEXT/NULL → string/null). Timestamps
/// stay numeric so exports are machine-comparable.
fn rusqlite_value_to_json(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> Result<serde_json::Value, rusqlite::Error> {
    let value: rusqlite::types::Value = row.get(index)?;
    Ok(match value {
        rusqlite::types::Value::Null => serde_json::Value::Null,
        rusqlite::types::Value::Integer(n) => serde_json::Value::Number(n.into()),
        rusqlite::types::Value::Real(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        rusqlite::types::Value::Text(t) => serde_json::Value::String(t),
        rusqlite::types::Value::Blob(b) => {
            // Blobs are never stored in these tables; hex-encode defensively.
            serde_json::Value::String(b.iter().map(|x| format!("{x:02x}")).collect::<String>())
        }
    })
}
