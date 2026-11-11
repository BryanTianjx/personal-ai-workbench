//! AI Collaboration persistence ports (E5-03/E5-04; PAW-INF-005/006).
//!
//! Repositories for agent registrations, agent executions, approvals and
//! agent grants. Workspace predicates are mandatory on every query (Gate C):
//! an execution or approval outside the requested workspace is invisible, and
//! id-based lookups verify workspace consistency before returning. Execution
//! history is append-oriented: registrations never cascade-delete their
//! executions (contract §7 discipline from migration 0008 carries over).

use paw_domain::ai_collaboration::{
    AgentExecution, AgentExecutionId, AgentRegistration, AgentRegistrationId, Approval, ApprovalId,
    ExecutionStatus,
};
use paw_domain::work_management::WorkspaceId;

/// Shared persistence errors for the AI Collaboration ports. Variants stay
/// non-leaky: no driver messages, no payload contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStoreError {
    /// A workspace-consistent record was not found for the given id.
    NotFound,
    /// The record exists but belongs to a different workspace; callers
    /// must not be able to distinguish it from `NotFound` by timing alone,
    /// so both surface the same audit-relevant outcome.
    WorkspaceMismatch,
    /// A schema-level invariant was violated (mirrors a CHECK constraint).
    Invariant(String),
    /// The persistence adapter is temporarily unavailable (fail closed).
    Storage(String),
}

impl std::fmt::Display for AgentStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "record not found"),
            Self::WorkspaceMismatch => write!(f, "record outside the requested workspace"),
            Self::Invariant(msg) => write!(f, "invariant violated: {msg}"),
            Self::Storage(msg) => write!(f, "storage failure: {msg}"),
        }
    }
}

impl std::error::Error for AgentStoreError {}

/// Registration persistence (E5-03).
pub trait AgentRegistrationRepository {
    fn insert_registration(&self, registration: &AgentRegistration) -> Result<(), AgentStoreError>;
    /// Workspace-consistent fetch: a registration from another workspace is
    /// reported as `WorkspaceMismatch`, never returned.
    fn get_registration(
        &self,
        id: &AgentRegistrationId,
        workspace_id: &WorkspaceId,
    ) -> Result<AgentRegistration, AgentStoreError>;
    fn list_registrations(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<AgentRegistration>, AgentStoreError>;
    /// Persist a lifecycle/status change (the entity carries the new status).
    fn update_registration_status(
        &self,
        registration: &AgentRegistration,
    ) -> Result<(), AgentStoreError>;
}

/// Execution persistence (E5-03). Append-oriented: status updates only.
pub trait AgentExecutionRepository {
    fn insert_execution(&self, execution: &AgentExecution) -> Result<(), AgentStoreError>;
    fn get_execution(
        &self,
        id: &AgentExecutionId,
        workspace_id: &WorkspaceId,
    ) -> Result<AgentExecution, AgentStoreError>;
    fn list_executions(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<AgentExecution>, AgentStoreError>;
    /// Persist a lifecycle transition or the operation link.
    fn update_execution(&self, execution: &AgentExecution) -> Result<(), AgentStoreError>;
    /// Executions still in an unresolved status (TimedOut/OutcomeUnknown)
    /// — the reconciliation queue (ADR-019).
    fn list_unresolved_executions(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<AgentExecution>, AgentStoreError>;
    /// Status filter helper for the desktop queue page.
    fn count_executions_by_status(
        &self,
        workspace_id: &WorkspaceId,
        status: ExecutionStatus,
    ) -> Result<usize, AgentStoreError>;
}

/// Approval persistence (E5-04). Consumption updates carry the incremented
/// attempt count and the new status.
pub trait ApprovalRepository {
    fn insert_approval(&self, approval: &Approval) -> Result<(), AgentStoreError>;
    fn get_approval(
        &self,
        id: &ApprovalId,
        workspace_id: &WorkspaceId,
    ) -> Result<Approval, AgentStoreError>;
    /// Open approvals for a workspace, oldest first (the decision queue).
    fn list_pending_approvals(
        &self,
        workspace_id: &WorkspaceId,
        now_ms: i64,
    ) -> Result<Vec<Approval>, AgentStoreError>;
    fn update_approval(&self, approval: &Approval) -> Result<(), AgentStoreError>;
    /// Open approval for an exact canonical plan digest (the propose-time
    /// record the owner decides on). `None` when no active approval exists.
    fn find_active_by_digest(
        &self,
        workspace_id: &WorkspaceId,
        canonical_digest: &str,
    ) -> Result<Option<Approval>, AgentStoreError>;
}
