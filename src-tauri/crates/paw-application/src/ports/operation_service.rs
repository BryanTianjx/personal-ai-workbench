//! Durable Operation envelope port (E5-05; ADR-019).
//!
//! The agent invocation pipeline (Application Coordination, ADR-017) opens
//! and advances durable Operations for each governed invocation. The
//! SQLite-backed service lives in paw-infrastructure (`operations.rs`, E1/
//! E3); this port carries the subset of its surface the coordination layer
//! needs, with application-owned DTOs so no infrastructure type leaks
//! across the port (ADR-018).
//!
//! State semantics are the ADR-019 machine (`paw_domain::operation_machine`,
//! E4 refactor): transition validation, OutcomeUnknown reconciliation and
//! retry classification are delegated to the domain machine and the
//! infrastructure service — the port adds no policy of its own.

use paw_domain::operation_machine::OperationStatus;

/// Fields required to open an operation envelope (mirrors the
/// infrastructure `NewOperation` contract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenOperation {
    /// Stable kind tag (e.g. `agent.invocation`).
    pub kind: String,
    pub owner_domain: String,
    pub owner_type: String,
    pub owner_id: String,
    /// Workspace scope; `None` marks a system-scope envelope (queue
    /// queries never leak those into workspace reads).
    pub workspace_id: Option<String>,
    pub actor_id: String,
    pub correlation_id: String,
    /// Causation link (the parent event/operation that led to this one).
    pub causation_id: Option<String>,
}

/// The envelope view the coordination layer needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationEnvelope {
    pub id: String,
    pub status: OperationStatus,
    pub correlation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationPortError {
    /// The envelope id does not exist.
    NotFound(String),
    /// A transition the ADR-019 machine rejects (from/to surfaced for the
    /// audit trail; not user-facing detail).
    InvalidTransition { from: String, to: String },
    /// Uniqueness conflict (e.g. idempotency key already used).
    Conflict(String),
    /// Storage failure; fail closed.
    Storage(String),
}

impl std::fmt::Display for OperationPortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "operation {id} not found"),
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid operation transition {from} -> {to}")
            }
            Self::Conflict(msg) => write!(f, "operation conflict: {msg}"),
            Self::Storage(msg) => write!(f, "operation storage failure: {msg}"),
        }
    }
}

impl std::error::Error for OperationPortError {}

/// The durable envelope surface used by the invocation pipeline.
pub trait OperationService {
    fn open(&self, open: &OpenOperation) -> Result<OperationEnvelope, OperationPortError>;
    fn get(&self, id: &str) -> Result<OperationEnvelope, OperationPortError>;
    /// Advance the envelope through a published transition (validated by
    /// the ADR-019 machine in the implementation).
    fn transition(
        &self,
        id: &str,
        to: OperationStatus,
        error_category: Option<&str>,
    ) -> Result<OperationEnvelope, OperationPortError>;
}
