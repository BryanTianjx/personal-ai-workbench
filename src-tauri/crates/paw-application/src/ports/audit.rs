//! Audit port published by the application layer (E1-09).
//!
//! System & Security owns audit semantics (system-architecture §4); the
//! application layer defines only the hook. Events carry stable action
//! keys, resource references and outcomes — never credentials, prompts,
//! full document bodies or raw driver messages (`E0.3.2` §1.8), so the
//! trail cannot leak secrets by construction.

use paw_domain::work_management::WorkspaceId;

/// Outcome recorded for one audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// The action was authorized and executed.
    Allowed,
    /// The action was refused by authorization.
    Denied,
    /// The action was authorized but failed downstream (constraint, io, ...).
    Error,
}

impl AuditOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
            Self::Error => "error",
        }
    }
}

/// One consequential-action evidence record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// Workspace scope when the action is workspace-scoped.
    pub workspace_id: Option<WorkspaceId>,
    pub actor_id: String,
    /// Stable action key (e.g. `workspace.create`).
    pub action: &'static str,
    /// Resource type key (e.g. `workspace`, `project`).
    pub resource_type: &'static str,
    /// Opaque resource id when the action produced or targeted one.
    pub resource_id: Option<String>,
    pub outcome: AuditOutcome,
    pub correlation_id: String,
}

/// Audit sink port. Recording is a hook, not a transaction participant: an
/// audit write failure must not roll back or fail the business action it
/// observes (the SQLite adapter persists best-effort with the connection
/// policy; audit integrity is revisited with the E3 audit scope).
pub trait AuditSink {
    fn record(&self, event: &AuditEvent);
}
