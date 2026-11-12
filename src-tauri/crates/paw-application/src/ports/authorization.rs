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
    /// Create an agent grant (E5-02). Human-only: never grantable to agents
    /// (authorization model §2 — Admin is reserved for the human owner).
    CreateAgentGrant {
        workspace_id: WorkspaceId,
    },
    /// Revoke an agent grant (E5-02). Human-only, same rule.
    RevokeAgentGrant {
        workspace_id: WorkspaceId,
    },
    /// A dynamic capability invocation inside the agent pipeline (E5-05).
    /// The per-side-effect recheck authorizes the *agent subject* against
    /// this action; the capability id comes from the resolved descriptor.
    CapabilityInvoke {
        workspace_id: WorkspaceId,
        capability_id: String,
    },
    /// Skill matrix mutations (E6-01; levels are never inferred — only
    /// humans confirm assessments, ADR-015).
    CreateSkill {
        workspace_id: WorkspaceId,
    },
    UpdateSkill {
        workspace_id: WorkspaceId,
    },
    CreateSkillEvidence {
        workspace_id: WorkspaceId,
    },
    CreateAssessment {
        workspace_id: WorkspaceId,
    },
    /// Project learning-gap proposals (E6-02; PAW-LG-007).
    ProposeLearningGap {
        workspace_id: WorkspaceId,
    },
    ConfirmLearningGap {
        workspace_id: WorkspaceId,
    },
    /// Creator content pipeline (E6-03).
    CreateCreatorTopic {
        workspace_id: WorkspaceId,
    },
    UpdateCreatorTopic {
        workspace_id: WorkspaceId,
    },
    CreateCreatorDraft {
        workspace_id: WorkspaceId,
    },
    UpdateCreatorDraft {
        workspace_id: WorkspaceId,
    },
    ReviewCreatorDraft {
        workspace_id: WorkspaceId,
    },
    /// Creator publish records (E6 第二批: 发布辅助 — manual facts only;
    /// publishing automation stays gated).
    CreatePublishRecord {
        workspace_id: WorkspaceId,
    },
    /// Project retrospectives with learning-product backflow (E6 第二批).
    RecordProjectRetrospective {
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
            Self::CreateAgentGrant { .. } => "agent.grant.create",
            Self::RevokeAgentGrant { .. } => "agent.grant.revoke",
            Self::CapabilityInvoke { .. } => "agent.capability.invoke",
            Self::CreateSkill { .. } => "skill.create",
            Self::UpdateSkill { .. } => "skill.update",
            Self::CreateSkillEvidence { .. } => "skill.evidence.create",
            Self::CreateAssessment { .. } => "skill.assessment.create",
            Self::ProposeLearningGap { .. } => "learning.gap.propose",
            Self::ConfirmLearningGap { .. } => "learning.gap.decide",
            Self::CreateCreatorTopic { .. } => "creator.topic.create",
            Self::UpdateCreatorTopic { .. } => "creator.topic.update",
            Self::CreateCreatorDraft { .. } => "creator.draft.create",
            Self::UpdateCreatorDraft { .. } => "creator.draft.update",
            Self::ReviewCreatorDraft { .. } => "creator.draft.review",
            Self::CreatePublishRecord { .. } => "creator.publish.record",
            Self::RecordProjectRetrospective { .. } => "project.retrospective.record",
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
            | Self::ReconcileAgentExecution { workspace_id }
            | Self::CreateAgentGrant { workspace_id }
            | Self::RevokeAgentGrant { workspace_id }
            | Self::CreateSkill { workspace_id }
            | Self::UpdateSkill { workspace_id }
            | Self::CreateSkillEvidence { workspace_id }
            | Self::CreateAssessment { workspace_id }
            | Self::ProposeLearningGap { workspace_id }
            | Self::ConfirmLearningGap { workspace_id }
            | Self::CreateCreatorTopic { workspace_id }
            | Self::UpdateCreatorTopic { workspace_id }
            | Self::CreateCreatorDraft { workspace_id }
            | Self::UpdateCreatorDraft { workspace_id }
            | Self::ReviewCreatorDraft { workspace_id } => Some(workspace_id),
            Self::CreatePublishRecord { workspace_id }
            | Self::RecordProjectRetrospective { workspace_id } => Some(workspace_id),
            Self::CapabilityInvoke { workspace_id, .. } => Some(workspace_id),
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

/// An explicit agent grant (authorization model §2; E5-02). A grant binds a
/// subject to capabilities within a scope; possession of one capability
/// never implies another. Grants are read fresh on every evaluation so
/// revocation takes effect at the next side effect (ADR-029 §4.4).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AgentGrant {
    /// Stable grant id (opaque).
    pub id: String,
    /// Subject the grant binds: `agent:<registration_id>` for agent
    /// subjects; the local human subject is governed by policy, not grants.
    pub subject_id: String,
    /// Capability ids covered (e.g. `work_management.task.create`).
    pub capabilities: Vec<String>,
    /// The workspace the grant is scoped to; `None` would widen scope and
    /// is therefore not representable — grants are workspace-bound.
    pub workspace_id: WorkspaceId,
    /// Optional tool keys the grant may use (empty = no tool access).
    pub tools: Vec<String>,
    /// Validity window (epoch millis). `valid_until_ms: None` is a
    /// long-lived grant, still revocable at any time.
    pub valid_from_ms: i64,
    pub valid_until_ms: Option<i64>,
}

impl AgentGrant {
    /// Scope is the intersection of all constraints (authorization model
    /// §2): subject (fixed at load), capability membership, workspace and
    /// the validity window must all hold.
    pub fn covers(
        &self,
        subject_id: &str,
        capability_id: &str,
        workspace_id: &WorkspaceId,
        now_ms: i64,
    ) -> bool {
        if self.subject_id != subject_id || self.workspace_id != *workspace_id {
            return false;
        }
        if !self.capabilities.iter().any(|c| c == capability_id) {
            return false;
        }
        now_ms >= self.valid_from_ms && self.valid_until_ms.is_none_or(|until| now_ms < until)
    }
}

/// Grant storage port. Implementations must return the *current* rows so
/// that revocation/creation takes effect on the next evaluation.
pub trait GrantStore {
    fn grants_for_subject(&self, subject_id: &str) -> Result<Vec<AgentGrant>, AuthorizationError>;
}

/// Grant evaluation helper — the single source of the agent-subject grant
/// policy: a capability invocation is allowed when any fresh grant row
/// covers the subject, capability, workspace and time window (authorization
/// model §2 scope intersection; ADR-029 §4.4).
pub fn grant_allows(
    grants: &[AgentGrant],
    subject_id: &str,
    capability_id: &str,
    workspace_id: &WorkspaceId,
    now_ms: i64,
) -> bool {
    grants
        .iter()
        .any(|grant| grant.covers(subject_id, capability_id, workspace_id, now_ms))
}

/// Grant-driven authorizer for agent subjects (E5-02): every evaluation
/// reads the store fresh, so revocation takes effect at the next side
/// effect. The clock is injected so the pure application layer stays
/// time-source-free; the infrastructure wiring passes the platform clock.
pub struct GrantEvaluatedAuthorizer<G: GrantStore, F: Fn() -> i64> {
    grants: G,
    now_ms: F,
}

impl<G: GrantStore, F: Fn() -> i64> GrantEvaluatedAuthorizer<G, F> {
    pub fn new(grants: G, now_ms: F) -> Self {
        Self { grants, now_ms }
    }
}

impl<G: GrantStore, F: Fn() -> i64> Authorizer for GrantEvaluatedAuthorizer<G, F> {
    fn authorize(
        &self,
        ctx: &ExecutionContext,
        action: &ProtectedAction,
    ) -> Result<(), AuthorizationError> {
        match action {
            ProtectedAction::CapabilityInvoke {
                workspace_id,
                capability_id,
            } => {
                let grants = self.grants.grants_for_subject(&ctx.actor_id)?;
                if grant_allows(
                    &grants,
                    &ctx.actor_id,
                    capability_id,
                    workspace_id,
                    (self.now_ms)(),
                ) {
                    Ok(())
                } else {
                    Err(AuthorizationError::Denied)
                }
            }
            // Agent subjects hold no other authority: registration, grant
            // administration and approvals are human actions.
            _ => Err(AuthorizationError::Denied),
        }
    }
}
