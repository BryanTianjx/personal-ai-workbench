//! Durable Operation lifecycle service (E1-07, Tier 1).
//!
//! Logical contract: `E0.3.2-engineering-specification.md` §2/§3; schema:
//! migration `0002_durable_operations_outbox.sql`. Operation is a durable
//! execution envelope referencing its owner record — it never mirrors Task,
//! Workflow Run or Agent Execution state. Tier 1 exercises Pending → Running
//! → Succeeded/Failed, restart recovery and the state-level OutcomeUnknown
//! rule; retry scheduling/backoff, cancellation and quarantine UX are Tier 2
//! (E1 delivery plan §3).

use rusqlite::{Connection, OptionalExtension, params};

use crate::sqlite;

/// Operation lifecycle states (`E0.3.2` §2). The strings match the CHECK
/// constraint in migration 0002 exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Pending,
    Running,
    AwaitingApproval,
    RetryScheduled,
    OutcomeUnknown,
    Succeeded,
    Failed,
    Cancelled,
}

impl OperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::AwaitingApproval => "AwaitingApproval",
            Self::RetryScheduled => "RetryScheduled",
            Self::OutcomeUnknown => "OutcomeUnknown",
            Self::Succeeded => "Succeeded",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Pending" => Self::Pending,
            "Running" => Self::Running,
            "AwaitingApproval" => Self::AwaitingApproval,
            "RetryScheduled" => Self::RetryScheduled,
            "OutcomeUnknown" => Self::OutcomeUnknown,
            "Succeeded" => Self::Succeeded,
            "Failed" => Self::Failed,
            "Cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

impl std::fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Allowed transitions (`E0.3.2` §2). `OutcomeUnknown → Running` is
/// intentionally absent: resuming an indeterminate operation requires
/// explicit reconciliation via [`resume_after_reconciliation`], which keeps
/// the state-level rule that an unresolved outcome blocks retry.
fn allowed_transitions(from: OperationStatus) -> &'static [OperationStatus] {
    use OperationStatus::*;
    match from {
        Pending => &[Running, AwaitingApproval, Cancelled, Failed],
        AwaitingApproval => &[Pending, Running, Cancelled, Failed],
        Running => &[RetryScheduled, OutcomeUnknown, Succeeded, Failed, Cancelled],
        RetryScheduled => &[Running, Cancelled, Failed],
        OutcomeUnknown => &[Succeeded, Failed, Cancelled],
        Succeeded | Failed | Cancelled => &[],
    }
}

/// A persisted Operation row (subset of columns the service reads back).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OperationRow {
    pub id: String,
    pub kind: String,
    pub owner_domain: String,
    pub owner_type: String,
    pub owner_id: String,
    pub workspace_id: Option<String>,
    pub status: OperationStatus,
    pub correlation_id: String,
    /// Earliest eligible retry time (`RetryScheduled` only).
    pub next_attempt_at: Option<i64>,
    /// Set by `request_cancellation`; a request is not proof of remote stop.
    pub cancellation_requested_at: Option<i64>,
    pub updated_at: i64,
}

/// Fields required to open a new operation envelope.
#[derive(Debug, Clone)]
pub struct NewOperation {
    pub kind: String,
    pub owner_domain: String,
    pub owner_type: String,
    pub owner_id: String,
    /// `Some` for workspace-scoped work; `None` only for explicitly
    /// system-scoped operations (never a wildcard permission).
    pub workspace_id: Option<String>,
    pub actor_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    /// Declared cancellation capability (E3-01): `supported` means the work
    /// can cooperatively stop; `unsupported` must be explicit — a
    /// cancellation request against it is a typed error, never a silent no-op.
    pub cancellation_support: CancellationSupport,
}

/// Declared cancellation capability; strings match the schema CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationSupport {
    Supported,
    Unsupported,
    Unknown,
}

impl CancellationSupport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    #[error("operation not found: {0}")]
    NotFound(String),
    #[error("invalid transition {from} -> {to}")]
    InvalidTransition { from: String, to: String },
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("clock: {source}")]
    NowMsFailed { source: std::time::SystemTimeError },
}

impl From<sqlite::Error> for OperationError {
    fn from(e: sqlite::Error) -> Self {
        match e {
            sqlite::Error::NowMsFailed { source } => Self::NowMsFailed { source },
            other => Self::Sql(rusqlite::Error::ToSqlConversionFailure(Box::new(other))),
        }
    }
}

fn now_ms() -> Result<i64, OperationError> {
    Ok(sqlite::now_ms()?)
}

fn row_to_operation(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperationRow> {
    let status: String = row.get("status")?;
    Ok(OperationRow {
        id: row.get("id")?,
        kind: row.get("kind")?,
        owner_domain: row.get("owner_domain")?,
        owner_type: row.get("owner_type")?,
        owner_id: row.get("owner_id")?,
        workspace_id: row.get("workspace_id")?,
        status: OperationStatus::parse(&status)
            .expect("status values are constrained by the schema CHECK"),
        correlation_id: row.get("correlation_id")?,
        next_attempt_at: row.get("next_attempt_at")?,
        cancellation_requested_at: row.get("cancellation_requested_at")?,
        updated_at: row.get("updated_at")?,
    })
}

const OPERATION_COLUMNS: &str = "id, kind, owner_domain, owner_type, owner_id, workspace_id, \
     status, correlation_id, next_attempt_at, cancellation_requested_at, updated_at";

/// Open a new operation envelope in `Pending` state.
pub fn create_operation(
    conn: &Connection,
    new: &NewOperation,
) -> Result<OperationRow, OperationError> {
    let id = uuid::Uuid::now_v7().to_string();
    let now = now_ms()?;
    conn.execute(
        "INSERT INTO operations (id, kind, owner_domain, owner_type, owner_id, workspace_id, \
         status, actor_id, correlation_id, causation_id, cancellation_support, \
         created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'Pending', ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            id,
            new.kind,
            new.owner_domain,
            new.owner_type,
            new.owner_id,
            new.workspace_id,
            new.actor_id,
            new.correlation_id,
            new.causation_id,
            new.cancellation_support.as_str(),
            now,
        ],
    )?;
    get_operation(conn, &id)?.ok_or(OperationError::NotFound(id))
}

/// Load one operation by id.
pub fn get_operation(conn: &Connection, id: &str) -> Result<Option<OperationRow>, OperationError> {
    let sql = format!("SELECT {OPERATION_COLUMNS} FROM operations WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    Ok(stmt.query_row([id], row_to_operation).optional()?)
}

/// Transition an operation to `to`, validating the state machine. Entering
/// `Running` stamps `started_at` (once); entering a terminal state stamps
/// `finished_at`. `OutcomeUnknown → Running` is rejected here — see
/// [`resume_after_reconciliation`].
pub fn transition(
    conn: &Connection,
    id: &str,
    to: OperationStatus,
    error_category: Option<&str>,
) -> Result<OperationRow, OperationError> {
    let current = get_operation(conn, id)?.ok_or_else(|| OperationError::NotFound(id.into()))?;
    if !allowed_transitions(current.status).contains(&to) {
        return Err(OperationError::InvalidTransition {
            from: current.status.as_str().into(),
            to: to.as_str().into(),
        });
    }
    let now = now_ms()?;
    let started_at = if to == OperationStatus::Running {
        Some(now)
    } else {
        None
    };
    let finished_at = if to.is_terminal() { Some(now) } else { None };
    conn.execute(
        "UPDATE operations SET status = ?1, updated_at = ?2, \
         started_at = COALESCE(started_at, ?3), finished_at = COALESCE(?4, finished_at), \
         last_error_category = COALESCE(?5, last_error_category)
         WHERE id = ?6",
        params![
            to.as_str(),
            now,
            started_at,
            finished_at,
            error_category,
            id,
        ],
    )?;
    get_operation(conn, id)?.ok_or(OperationError::NotFound(id.into()))
}

/// Move an `OutcomeUnknown` operation back to `Running` after reconciliation
/// proved no effect (or a safe idempotent retry is possible). The
/// reconciliation is recorded as the resolution note; this is the only path
/// out of `OutcomeUnknown` back into an executable state.
pub fn resume_after_reconciliation(
    conn: &Connection,
    id: &str,
    note: &str,
) -> Result<OperationRow, OperationError> {
    let current = get_operation(conn, id)?.ok_or_else(|| OperationError::NotFound(id.into()))?;
    if current.status != OperationStatus::OutcomeUnknown {
        return Err(OperationError::InvalidTransition {
            from: current.status.as_str().into(),
            to: OperationStatus::Running.as_str().into(),
        });
    }
    let now = now_ms()?;
    conn.execute(
        "UPDATE operations SET status = 'Running', updated_at = ?1, \
         started_at = COALESCE(started_at, ?1), resolution_note = ?2 WHERE id = ?3",
        params![now, note, id],
    )?;
    get_operation(conn, id)?.ok_or(OperationError::NotFound(id.into()))
}

/// Restart recovery (Tier 1 policy): at startup, operations still `Running`
/// belong to a dead process and must not auto-retry — they are moved to
/// `Failed` with the safe category `process_restarted`. Returns the number
/// of recovered rows. `RetryScheduled`/`AwaitingApproval` survive restarts.
pub fn recover_interrupted(conn: &Connection) -> Result<usize, OperationError> {
    let now = now_ms()?;
    let changed = conn.execute(
        "UPDATE operations SET status = 'Failed', updated_at = ?1, \
         finished_at = COALESCE(finished_at, ?1), last_error_category = 'process_restarted'
         WHERE status = 'Running'",
        params![now],
    )?;
    Ok(changed)
}

// ---------------------------------------------------------------------------
// Tier 2 (E3-01): retry classification, due-claim scheduling, cancellation
// ---------------------------------------------------------------------------

/// Classification of a failed attempt, per ADR-019: retry decisions require
/// knowing whether the previous attempt was rejected before submission,
/// accepted by the target, or indeterminate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClassification {
    /// The attempt never reached the target (validated out, queued out):
    /// safe to retry after backoff.
    RejectedBeforeSubmit,
    /// The target accepted the work: poll/reconcile, never resubmit.
    Accepted,
    /// The outcome is not established: the envelope moves to
    /// OutcomeUnknown and blocks non-idempotent retry until reconciliation.
    Indeterminate,
}

/// Retry backoff policy: `base * 2^(failures-1)` capped at `max`.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub base_ms: i64,
    pub max_backoff_ms: i64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_ms: 1_000,
            max_backoff_ms: 60_000,
        }
    }
}

impl RetryPolicy {
    fn backoff_ms(&self, failures: i64) -> i64 {
        let exp = (failures - 1).clamp(0, 16);
        (self.base_ms.saturating_mul(1i64 << exp)).min(self.max_backoff_ms)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClassificationError {
    #[error("operation not found: {0}")]
    NotFound(String),
    #[error("invalid transition {from} -> {to}")]
    InvalidTransition { from: String, to: String },
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("clock: {source}")]
    NowMsFailed { source: std::time::SystemTimeError },
}

impl From<OperationError> for ClassificationError {
    fn from(e: OperationError) -> Self {
        match e {
            OperationError::NotFound(id) => Self::NotFound(id),
            OperationError::InvalidTransition { from, to } => Self::InvalidTransition { from, to },
            OperationError::Sql(source) => Self::Sql(source),
            OperationError::NowMsFailed { source } => Self::NowMsFailed { source },
        }
    }
}

/// Classify a failed attempt and route the envelope accordingly (ADR-019):
///
/// - [`FailureClassification::RejectedBeforeSubmit`]: `Running →
///   RetryScheduled` with `next_attempt_at = now + backoff(failures)`;
/// - [`FailureClassification::Accepted`]: stays `Running` (poll/reconcile,
///   never resubmit);
/// - [`FailureClassification::Indeterminate`]: `Running → OutcomeUnknown`
///   (blocks non-idempotent retry until
///   [`resume_after_reconciliation`]).
///
/// One [`record_attempt`] row is appended in the same transaction.
pub fn classify_failure_and_schedule_retry(
    conn: &Connection,
    operation_id: &str,
    classification: FailureClassification,
    error_category: &str,
    policy: &RetryPolicy,
) -> Result<OperationRow, ClassificationError> {
    let current = get_operation(conn, operation_id)
        .map_err(ClassificationError::from)?
        .ok_or_else(|| ClassificationError::NotFound(operation_id.into()))?;
    if current.status != OperationStatus::Running {
        return Err(ClassificationError::InvalidTransition {
            from: current.status.as_str().into(),
            to: "RetryScheduled|Running|OutcomeUnknown (classified)".into(),
        });
    }
    let now = now_ms()?;

    // Attempt count drives the backoff exponent (monotonic, unique).
    let attempts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM operation_attempts WHERE operation_id = ?1",
            [operation_id],
            |r| r.get(0),
        )
        .map_err(ClassificationError::from)?;

    let (attempt_outcome, retryable) = match classification {
        FailureClassification::RejectedBeforeSubmit => (AttemptOutcome::Rejected, true),
        FailureClassification::Accepted => (AttemptOutcome::Accepted, false),
        FailureClassification::Indeterminate => (AttemptOutcome::Indeterminate, true),
    };
    record_attempt(
        conn,
        operation_id,
        AttemptPhase::Submit,
        attempt_outcome,
        None,
        None,
        None,
        retryable,
        Some(error_category),
        None,
    )?;

    match classification {
        FailureClassification::Accepted => {
            // Poll/reconcile: the envelope stays Running; no resubmission.
            Ok(current)
        }
        FailureClassification::RejectedBeforeSubmit => {
            let next_attempt_at = now + policy.backoff_ms(attempts.max(1));
            conn.execute(
                "UPDATE operations SET status = 'RetryScheduled', updated_at = ?1, \
                 next_attempt_at = ?2, last_error_category = ?3 WHERE id = ?4",
                params![now, next_attempt_at, error_category, operation_id],
            )
            .map_err(ClassificationError::from)?;
            get_operation(conn, operation_id)
                .map_err(ClassificationError::from)?
                .ok_or_else(|| ClassificationError::NotFound(operation_id.into()))
        }
        FailureClassification::Indeterminate => {
            conn.execute(
                "UPDATE operations SET status = 'OutcomeUnknown', updated_at = ?1, \
                 last_error_category = ?2 WHERE id = ?3",
                params![now, error_category, operation_id],
            )
            .map_err(ClassificationError::from)?;
            get_operation(conn, operation_id)
                .map_err(ClassificationError::from)?
                .ok_or_else(|| ClassificationError::NotFound(operation_id.into()))
        }
    }
}

/// Claim the earliest due `RetryScheduled` operation (next_attempt_at due)
/// and move it back to `Running`. Returns `None` when nothing is due.
pub fn claim_due_operation(
    conn: &Connection,
    now: i64,
) -> Result<Option<OperationRow>, ClassificationError> {
    let due: Option<String> = conn
        .query_row(
            "SELECT id FROM operations
             WHERE status = 'RetryScheduled' AND next_attempt_at <= ?1
             ORDER BY next_attempt_at LIMIT 1",
            [now],
            |r| r.get(0),
        )
        .optional()
        .map_err(ClassificationError::from)?;
    let Some(id) = due else {
        return Ok(None);
    };
    // RetryScheduled → Running is a legal transition; clear the due stamp.
    conn.execute(
        "UPDATE operations SET status = 'Running', updated_at = ?1, started_at = \
         COALESCE(started_at, ?1), next_attempt_at = NULL WHERE id = ?2",
        params![now, id],
    )
    .map_err(ClassificationError::from)?;
    get_operation(conn, &id).map_err(ClassificationError::from)
}

/// Record a cooperative cancellation request. `unsupported` capability is
/// an explicit typed error (never a silent no-op); the request is stamped
/// but is not proof that any remote work stopped — the actual
/// `Cancelled` transition still goes through the state machine.
pub fn request_cancellation(
    conn: &Connection,
    id: &str,
) -> Result<OperationRow, ClassificationError> {
    let current = get_operation(conn, id)
        .map_err(ClassificationError::from)?
        .ok_or_else(|| ClassificationError::NotFound(id.into()))?;
    let support: String = conn
        .query_row(
            "SELECT cancellation_support FROM operations WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(ClassificationError::from)?;
    if support == "unsupported" {
        return Err(ClassificationError::InvalidTransition {
            from: format!("cancellation_support={support}"),
            to: "cancellation_requested".into(),
        });
    }
    if current.status.is_terminal() {
        return Err(ClassificationError::InvalidTransition {
            from: current.status.as_str().into(),
            to: "cancellation_requested".into(),
        });
    }
    let now = now_ms()?;
    conn.execute(
        "UPDATE operations SET cancellation_requested_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(ClassificationError::from)?;
    get_operation(conn, id)
        .map_err(ClassificationError::from)?
        .ok_or_else(|| ClassificationError::NotFound(id.into()))
}

/// List operations of one workspace (queue view), optionally filtered by
/// status. System-scope rows (NULL workspace) are never returned by a
/// workspace query (`E0.3.2` §2: NULL is not a wildcard grant).
pub fn list_operations(
    conn: &Connection,
    workspace_id: &str,
    status: Option<OperationStatus>,
) -> Result<Vec<OperationRow>, ClassificationError> {
    let sql = if status.is_some() {
        format!(
            "SELECT {OPERATION_COLUMNS} FROM operations \
             WHERE workspace_id = ?1 AND status = ?2 ORDER BY updated_at DESC LIMIT 100"
        )
    } else {
        format!(
            "SELECT {OPERATION_COLUMNS} FROM operations \
             WHERE workspace_id = ?1 ORDER BY updated_at DESC LIMIT 100"
        )
    };
    let mut stmt = conn.prepare(&sql).map_err(ClassificationError::from)?;
    let status_value = status.map(|s| s.as_str().to_string());
    let mut rows = if status.is_some() {
        stmt.query(params![workspace_id, status_value])?
    } else {
        stmt.query([workspace_id])?
    };
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(row_to_operation(row)?);
    }
    Ok(out)
}

/// Attempt phase (`E0.3.2` §3); strings match the schema CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptPhase {
    Prepare,
    Submit,
    Poll,
    Cancel,
    Reconcile,
}

impl AttemptPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Submit => "submit",
            Self::Poll => "poll",
            Self::Cancel => "cancel",
            Self::Reconcile => "reconcile",
        }
    }
}

/// Attempt outcome (`E0.3.2` §3); strings match the schema CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutcome {
    Started,
    Rejected,
    Accepted,
    Completed,
    Failed,
    Indeterminate,
    Cancelled,
}

impl AttemptOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Rejected => "rejected",
            Self::Accepted => "accepted",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Indeterminate => "indeterminate",
            Self::Cancelled => "cancelled",
        }
    }
}

/// One actual attempt (`E0.3.2` §3): append-oriented; one row per real
/// execution attempt, never merely a scheduled retry.
#[allow(clippy::too_many_arguments)]
pub fn record_attempt(
    conn: &Connection,
    operation_id: &str,
    phase: AttemptPhase,
    outcome: AttemptOutcome,
    request_digest: Option<&str>,
    idempotency_key: Option<&str>,
    external_reference: Option<&str>,
    retryable: bool,
    error_category: Option<&str>,
    error_code: Option<&str>,
) -> Result<String, OperationError> {
    if get_operation(conn, operation_id)?.is_none() {
        return Err(OperationError::NotFound(operation_id.into()));
    }
    let attempt_no: i64 = conn.query_row(
        "SELECT COALESCE(MAX(attempt_no), 0) FROM operation_attempts WHERE operation_id = ?1",
        [operation_id],
        |r| r.get(0),
    )?;
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO operation_attempts (id, operation_id, attempt_no, phase, started_at, \
         finished_at, outcome, request_digest, idempotency_key, external_reference, \
         retryable, error_category, error_code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id,
            operation_id,
            attempt_no + 1,
            phase.as_str(),
            now_ms()?,
            outcome.as_str(),
            request_digest,
            idempotency_key,
            external_reference,
            retryable,
            error_category,
            error_code,
        ],
    )?;
    Ok(id)
}
