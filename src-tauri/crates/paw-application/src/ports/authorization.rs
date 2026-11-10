//! Authorization port published by the application layer (E1-09).
//!
//! Every protected side effect is preceded by a fresh authorization check
//! (ADR-017; Gate C). The port is intentionally capability-based and
//! deny-by-default: an adapter that does not explicitly grant an
//! (actor, action) pair denies it. Possession of an id is not authorization
//! (ADR-020): workspace scoping is re-checked here and in the repository.

use paw_domain::work_management::WorkspaceId;

/// A protected side effect requiring fresh authorization before execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedAction {
    /// Create a new workspace (a consequential, user-visible record).
    CreateWorkspace,
    /// Create a project inside the given workspace.
    CreateProject { workspace_id: WorkspaceId },
    /// Delete a workspace and all its projects.
    DeleteWorkspace { workspace_id: WorkspaceId },
}

impl ProtectedAction {
    /// Stable audit action key (also used by the audit trail).
    pub fn action_key(&self) -> &'static str {
        match self {
            Self::CreateWorkspace => "workspace.create",
            Self::CreateProject { .. } => "project.create",
            Self::DeleteWorkspace { .. } => "workspace.delete",
        }
    }

    /// The workspace this action is scoped to, when applicable.
    pub fn workspace(&self) -> Option<&WorkspaceId> {
        match self {
            Self::CreateWorkspace => None,
            Self::CreateProject { workspace_id } | Self::DeleteWorkspace { workspace_id } => {
                Some(workspace_id)
            }
        }
    }
}

/// Execution context carried from the boundary into use cases: who acts and
/// how to correlate the audit trail. `actor_id` is an initiating principal,
/// never an authorization grant (E0.3.2 §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    pub actor_id: String,
    pub correlation_id: String,
}

/// Typed authorization rejection. Variants stay non-leaky: no policy
/// internals, no actor enumeration, no resource details beyond the action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationError {
    /// The actor does not hold the capability for this action.
    Denied,
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied => write!(f, "action denied by authorization policy"),
        }
    }
}

impl std::error::Error for AuthorizationError {}

/// Authorization port: evaluated immediately before each protected side
/// effect, not merely once when an operation starts (ADR-017 clarification).
pub trait Authorizer {
    fn authorize(
        &self,
        ctx: &ExecutionContext,
        action: &ProtectedAction,
    ) -> Result<(), AuthorizationError>;
}
