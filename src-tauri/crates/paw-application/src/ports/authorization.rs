//! Authorization port published by the application layer (E1-09, extended
//! in E2 for the full Work Management surface).
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
    CreateProject {
        workspace_id: WorkspaceId,
    },
    /// Modify an existing project (name, description, status, links).
    UpdateProject {
        workspace_id: WorkspaceId,
    },
    /// Delete a project inside the given workspace.
    DeleteProject {
        workspace_id: WorkspaceId,
    },
    /// Delete a workspace and all its records.
    DeleteWorkspace {
        workspace_id: WorkspaceId,
    },
    /// Create an area inside the given workspace.
    CreateArea {
        workspace_id: WorkspaceId,
    },
    /// Modify an existing area.
    UpdateArea {
        workspace_id: WorkspaceId,
    },
    /// Delete an area inside the given workspace.
    DeleteArea {
        workspace_id: WorkspaceId,
    },
    /// Create a goal inside the given workspace.
    CreateGoal {
        workspace_id: WorkspaceId,
    },
    /// Modify an existing goal.
    UpdateGoal {
        workspace_id: WorkspaceId,
    },
    /// Delete a goal inside the given workspace.
    DeleteGoal {
        workspace_id: WorkspaceId,
    },
    /// Create a task inside the given workspace.
    CreateTask {
        workspace_id: WorkspaceId,
    },
    /// Modify an existing task (title, notes, project link).
    UpdateTask {
        workspace_id: WorkspaceId,
    },
    /// Move an existing task through its status machine.
    TransitionTask {
        workspace_id: WorkspaceId,
    },
    /// Delete a task inside the given workspace.
    DeleteTask {
        workspace_id: WorkspaceId,
    },
    /// Add an attachment to a project or task (bytes go to the managed
    /// file store; metadata to SQLite).
    CreateAttachment {
        workspace_id: WorkspaceId,
    },
    /// Remove an attachment (metadata row + stored bytes).
    DeleteAttachment {
        workspace_id: WorkspaceId,
    },
    /// Request cooperative cancellation of an operation (E3-01).
    CancelOperation {
        workspace_id: WorkspaceId,
    },
    /// Reconcile an OutcomeUnknown operation back into Running.
    ResumeOperation {
        workspace_id: WorkspaceId,
    },
    /// Re-drive a quarantined outbox message (explicit, audited).
    RedriveMessage {
        workspace_id: WorkspaceId,
    },
    /// Export the durable-operation history of a workspace (pre-purge step).
    ExportWorkspaceData {
        workspace_id: WorkspaceId,
    },
    /// Purge expired operations/audit rows (explicit, system-wide).
    PurgeExpiredData,
    /// Learning & Growth lifecycle actions (PAW-LG-001～006, E4).
    CreateLearningRoadmap {
        workspace_id: WorkspaceId,
    },
    UpdateLearningRoadmap {
        workspace_id: WorkspaceId,
    },
    DeleteLearningRoadmap {
        workspace_id: WorkspaceId,
    },
    CreateLearningPlan {
        workspace_id: WorkspaceId,
    },
    UpdateLearningPlan {
        workspace_id: WorkspaceId,
    },
    DeleteLearningPlan {
        workspace_id: WorkspaceId,
    },
    CreateLearningItem {
        workspace_id: WorkspaceId,
    },
    UpdateLearningItem {
        workspace_id: WorkspaceId,
    },
    DeleteLearningItem {
        workspace_id: WorkspaceId,
    },
    AppendLearningSession {
        workspace_id: WorkspaceId,
    },
    LinkLearningItemProject {
        workspace_id: WorkspaceId,
    },
    CreateTaskForLearningItem {
        workspace_id: WorkspaceId,
    },
    /// Knowledge & Context metadata actions (PAW-K-001, E4).
    CreateKnowledgeRef {
        workspace_id: WorkspaceId,
    },
    UpdateKnowledgeRef {
        workspace_id: WorkspaceId,
    },
    DeleteKnowledgeRef {
        workspace_id: WorkspaceId,
    },
    SetKnowledgeAvailability {
        workspace_id: WorkspaceId,
    },
    /// Register an agent for governed invocation (E5-03).
    RegisterAgent {
        workspace_id: WorkspaceId,
    },
    /// Suspend or retire an agent registration (E5-03).
    UpdateAgentRegistration {
        workspace_id: WorkspaceId,
    },
    /// Propose a governed agent invocation through the pipeline (E5-05).
    ProposeAgentInvocation {
        workspace_id: WorkspaceId,
    },
    /// Approve or deny a pending approval (E5-04).
    DecideApproval {
        workspace_id: WorkspaceId,
    },
    /// Revoke an active approval (E5-04).
    RevokeApproval {
        workspace_id: WorkspaceId,
    },
    /// Reconcile an execution whose outcome is unresolved (E5-05; ADR-019).
    ReconcileAgentExecution {
        workspace_id: WorkspaceId,
    },
}

impl ProtectedAction {
    /// Stable audit action key (also used by the audit trail).
    pub fn action_key(&self) -> &'static str {
        match self {
            Self::CreateWorkspace => "workspace.create",
            Self::CreateProject { .. } => "project.create",
            Self::UpdateProject { .. } => "project.update",
            Self::DeleteProject { .. } => "project.delete",
            Self::DeleteWorkspace { .. } => "workspace.delete",
            Self::CreateArea { .. } => "area.create",
            Self::UpdateArea { .. } => "area.update",
            Self::DeleteArea { .. } => "area.delete",
            Self::CreateGoal { .. } => "goal.create",
            Self::UpdateGoal { .. } => "goal.update",
            Self::DeleteGoal { .. } => "goal.delete",
            Self::CreateTask { .. } => "task.create",
            Self::UpdateTask { .. } => "task.update",
            Self::TransitionTask { .. } => "task.transition",
            Self::DeleteTask { .. } => "task.delete",
            Self::CreateAttachment { .. } => "attachment.create",
            Self::DeleteAttachment { .. } => "attachment.delete",
            Self::CancelOperation { .. } => "operation.cancel",
            Self::ResumeOperation { .. } => "operation.resume",
            Self::RedriveMessage { .. } => "message.redrive",
            Self::ExportWorkspaceData { .. } => "data.export",
            Self::PurgeExpiredData => "data.purge",
            Self::CreateLearningRoadmap { .. } => "learning.roadmap.create",
            Self::UpdateLearningRoadmap { .. } => "learning.roadmap.update",
            Self::DeleteLearningRoadmap { .. } => "learning.roadmap.delete",
            Self::CreateLearningPlan { .. } => "learning.plan.create",
            Self::UpdateLearningPlan { .. } => "learning.plan.update",
            Self::DeleteLearningPlan { .. } => "learning.plan.delete",
            Self::CreateLearningItem { .. } => "learning.item.create",
            Self::UpdateLearningItem { .. } => "learning.item.update",
            Self::DeleteLearningItem { .. } => "learning.item.delete",
            Self::AppendLearningSession { .. } => "learning.session.append",
            Self::LinkLearningItemProject { .. } => "learning.item.link_project",
            Self::CreateTaskForLearningItem { .. } => "learning.item.create_task",
            Self::CreateKnowledgeRef { .. } => "knowledge.ref.create",
            Self::UpdateKnowledgeRef { .. } => "knowledge.ref.update",
            Self::DeleteKnowledgeRef { .. } => "knowledge.ref.delete",
            Self::SetKnowledgeAvailability { .. } => "knowledge.ref.set_availability",
            Self::RegisterAgent { .. } => "agent.register",
            Self::UpdateAgentRegistration { .. } => "agent.registration.update",
            Self::ProposeAgentInvocation { .. } => "agent.invoke.propose",
            Self::DecideApproval { .. } => "approval.decide",
            Self::RevokeApproval { .. } => "approval.revoke",
            Self::ReconcileAgentExecution { .. } => "agent.execution.reconcile",
        }
    }

    /// The workspace this action is scoped to, when applicable.
    pub fn workspace(&self) -> Option<&WorkspaceId> {
        match self {
            Self::CreateWorkspace => None,
            Self::CreateProject { workspace_id }
            | Self::UpdateProject { workspace_id }
            | Self::DeleteProject { workspace_id }
            | Self::DeleteWorkspace { workspace_id }
            | Self::CreateArea { workspace_id }
            | Self::UpdateArea { workspace_id }
            | Self::DeleteArea { workspace_id }
            | Self::CreateGoal { workspace_id }
            | Self::UpdateGoal { workspace_id }
            | Self::DeleteGoal { workspace_id }
            | Self::CreateTask { workspace_id }
            | Self::UpdateTask { workspace_id }
            | Self::TransitionTask { workspace_id }
            | Self::DeleteTask { workspace_id } => Some(workspace_id),
            Self::CreateAttachment { workspace_id } | Self::DeleteAttachment { workspace_id } => {
                Some(workspace_id)
            }
            Self::CancelOperation { workspace_id }
            | Self::ResumeOperation { workspace_id }
            | Self::RedriveMessage { workspace_id }
            | Self::ExportWorkspaceData { workspace_id } => Some(workspace_id),
            Self::CreateLearningRoadmap { workspace_id }
            | Self::UpdateLearningRoadmap { workspace_id }
            | Self::DeleteLearningRoadmap { workspace_id }
            | Self::CreateLearningPlan { workspace_id }
            | Self::UpdateLearningPlan { workspace_id }
            | Self::DeleteLearningPlan { workspace_id }
            | Self::CreateLearningItem { workspace_id }
            | Self::UpdateLearningItem { workspace_id }
            | Self::DeleteLearningItem { workspace_id }
            | Self::AppendLearningSession { workspace_id }
            | Self::LinkLearningItemProject { workspace_id }
            | Self::CreateTaskForLearningItem { workspace_id }
            | Self::CreateKnowledgeRef { workspace_id }
            | Self::UpdateKnowledgeRef { workspace_id }
            | Self::DeleteKnowledgeRef { workspace_id }
            | Self::SetKnowledgeAvailability { workspace_id }
            | Self::RegisterAgent { workspace_id }
            | Self::UpdateAgentRegistration { workspace_id }
            | Self::ProposeAgentInvocation { workspace_id }
            | Self::DecideApproval { workspace_id }
            | Self::RevokeApproval { workspace_id }
            | Self::ReconcileAgentExecution { workspace_id } => Some(workspace_id),
            Self::PurgeExpiredData => None,
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
