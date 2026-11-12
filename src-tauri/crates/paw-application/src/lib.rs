//! paw-application: use cases, published ports and DTOs, authorization
//! orchestration, and the logical Coordination Layer.
//!
//! Dependency contract (ADR-018): depends on paw-domain only among PAW
//! crates. It must not depend on paw-infrastructure or paw-desktop, and it
//! must not leak SQL, Tauri, or provider SDK types into its contracts.

pub mod ports;
pub mod use_cases;

// Re-exported domain contract types: paw-desktop may not depend on
// paw-domain directly (E1-rust-workspace-layout §2), so the published
// application surface carries the entity types the UI boundary needs.
pub use paw_domain::ai_collaboration::{
    AdapterKind, AgentExecution, AgentExecutionId, AgentRegistration, AgentRegistrationId,
    Approval, ApprovalId, ApprovalStatus, AttemptPolicy, ExecutionStatus, RegistrationStatus,
};
pub use paw_domain::creator::{
    CreatorDraft, CreatorDraftId, CreatorDraftReview, CreatorDraftReviewId, CreatorDraftVersion,
    CreatorDraftVersionId, CreatorError, CreatorTopic, CreatorTopicId, DraftStatus, ReviewDecision,
    TopicStatus,
};
pub use paw_domain::knowledge::{Availability, KnowledgeRef, KnowledgeRefId, KnowledgeSource};
pub use paw_domain::learning::{
    GapProposalStatus, Item, ItemId, ItemKind, ItemStatus, LearningGapProposal,
    LearningGapProposalId, Plan, PlanId, PlanStatus, Roadmap, RoadmapId, Session, SessionId,
};
pub use paw_domain::skills::{
    Assessment, AssessmentId, Confidence, EvidenceKind, EvidenceSourceKind, Skill, SkillEvidence,
    SkillEvidenceId, SkillId, SkillsError,
};
pub use paw_domain::work_management::{
    Area, AreaId, Goal, GoalId, ParseError, Project, ProjectId, ProjectStatus, Task, TaskId,
    TaskStatus, ValidationError, Workspace, WorkspaceId,
};
