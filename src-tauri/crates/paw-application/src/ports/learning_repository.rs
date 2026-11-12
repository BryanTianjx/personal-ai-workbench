//! Learning & Growth repository ports (PAW-LG-001～006).
//!
//! Same contract shape as the Work Management ports: every method takes an
//! explicit `WorkspaceId`, cross-workspace records are never returned, and
//! rejections are typed and non-leaky. Sessions are append-only historical
//! records (cross-domain-data-contracts §7) — no update or delete here.

use paw_domain::learning::{Item, ItemStatus, Plan, PlanId, Roadmap, RoadmapId, Session};
use paw_domain::work_management::{ProjectId, TaskId, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LearningRepositoryError {
    RoadmapNotFound,
    PlanNotFound,
    ItemNotFound,
    SessionNotFound,
    /// A referenced record (plan/roadmap/project/task) does not exist in the
    /// workspace, or a constraint/scope invariant was violated (including
    /// cross-workspace links, refused by the composite schema keys).
    Invariant(String),
}

impl std::fmt::Display for LearningRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoadmapNotFound => write!(f, "roadmap not found"),
            Self::PlanNotFound => write!(f, "plan not found"),
            Self::ItemNotFound => write!(f, "learning item not found"),
            Self::SessionNotFound => write!(f, "learning session not found"),
            Self::Invariant(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for LearningRepositoryError {}

pub trait RoadmapRepository {
    fn insert_roadmap(&self, roadmap: &Roadmap) -> Result<(), LearningRepositoryError>;

    fn find_roadmap(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: &RoadmapId,
    ) -> Result<Option<Roadmap>, LearningRepositoryError>;

    fn list_roadmaps(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Roadmap>, LearningRepositoryError>;

    /// Persist name/description changes. The workspace binding is immutable.
    fn update_roadmap(&self, roadmap: &Roadmap) -> Result<(), LearningRepositoryError>;

    /// Archive/restore is a status flip via update; hard delete is refused
    /// while plans reference the roadmap (RESTRICT surfaces typed).
    fn delete_roadmap(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: &RoadmapId,
    ) -> Result<(), LearningRepositoryError>;
}

pub trait PlanRepository {
    fn insert_plan(&self, plan: &Plan) -> Result<(), LearningRepositoryError>;

    fn find_plan(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: &PlanId,
    ) -> Result<Option<Plan>, LearningRepositoryError>;

    /// List the workspace's plans, optionally filtered by roadmap.
    fn list_plans(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: Option<&RoadmapId>,
    ) -> Result<Vec<Plan>, LearningRepositoryError>;

    /// Persist name/description/status changes (PRD §3.2 statuses).
    fn update_plan(&self, plan: &Plan) -> Result<(), LearningRepositoryError>;

    fn delete_plan(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: &PlanId,
    ) -> Result<(), LearningRepositoryError>;
}

pub trait ItemRepository {
    fn insert_item(&self, item: &Item) -> Result<(), LearningRepositoryError>;

    fn find_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
    ) -> Result<Option<Item>, LearningRepositoryError>;

    /// List the workspace's items, optionally filtered by plan and/or status.
    fn list_items(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: Option<&paw_domain::learning::PlanId>,
        status: Option<ItemStatus>,
    ) -> Result<Vec<Item>, LearningRepositoryError>;

    /// Persist detail/status changes.
    fn update_item(&self, item: &Item) -> Result<(), LearningRepositoryError>;

    /// Delete an item. Sessions referencing it are historical records and
    /// RESTRICT-block deletion — the caller removes them explicitly first.
    fn delete_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
    ) -> Result<(), LearningRepositoryError>;

    // -- Item ↔ Project many-to-many (relationship: applies_to) -------------

    fn attach_item_to_project(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
        project_id: &ProjectId,
    ) -> Result<(), LearningRepositoryError>;

    fn detach_item_from_project(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
        project_id: &ProjectId,
    ) -> Result<(), LearningRepositoryError>;

    fn list_projects_for_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
    ) -> Result<Vec<ProjectId>, LearningRepositoryError>;

    // -- Item → Task zero-or-many action-step references ---------------------

    /// Record an action-step link. The task must exist inside the workspace;
    /// the Task remains Work Management-owned (PAW-LG-006).
    fn attach_task_to_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
        task_id: &TaskId,
    ) -> Result<(), LearningRepositoryError>;

    fn detach_task_from_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
        task_id: &TaskId,
    ) -> Result<(), LearningRepositoryError>;

    fn list_tasks_for_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &paw_domain::learning::ItemId,
    ) -> Result<Vec<TaskId>, LearningRepositoryError>;
}

pub trait SessionRepository {
    /// Append one session. Sessions are historical records: there is no
    /// update — multiple sessions of the same item never overwrite each
    /// other (PAW-LG-004).
    fn append_session(&self, session: &Session) -> Result<(), LearningRepositoryError>;

    fn find_session(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &paw_domain::learning::SessionId,
    ) -> Result<Option<Session>, LearningRepositoryError>;

    /// List sessions of one item (or the whole workspace when `None`).
    fn list_sessions(
        &self,
        workspace_id: &WorkspaceId,
        item_id: Option<&paw_domain::learning::ItemId>,
    ) -> Result<Vec<Session>, LearningRepositoryError>;
}

// ItemKind/ItemStatus/PlanStatus are re-used in signatures; keep them
// linked for callers.
#[allow(unused_imports)]
use paw_domain::learning as _learning_types;

// ---------------------------------------------------------------------------
// Project learning-gap proposals (E6-02, PAW-LG-007)
// ---------------------------------------------------------------------------

/// Learning-gap proposal persistence (PRD §3.8/§5.3). Proposals reference
/// projects by id without ownership transfer (ADR-014). `confirm` also
/// emits the versioned `LearningGapConfirmed.v1` outbox event in the same
/// transaction; `insert` emits `LearningGapProposed.v1` (ADR-020).
pub trait GapProposalRepository {
    fn insert_gap_proposal(
        &self,
        gap: &paw_domain::learning::LearningGapProposal,
    ) -> Result<(), LearningRepositoryError>;
    fn get_gap_proposal(
        &self,
        id: &paw_domain::learning::LearningGapProposalId,
        workspace_id: &paw_domain::work_management::WorkspaceId,
    ) -> Result<paw_domain::learning::LearningGapProposal, LearningRepositoryError>;
    fn list_gap_proposals(
        &self,
        workspace_id: &paw_domain::work_management::WorkspaceId,
        status: Option<paw_domain::learning::GapProposalStatus>,
    ) -> Result<Vec<paw_domain::learning::LearningGapProposal>, LearningRepositoryError>;
    /// Persist the confirmed/dismissed state (the entity carries it).
    fn update_gap_proposal(
        &self,
        gap: &paw_domain::learning::LearningGapProposal,
    ) -> Result<(), LearningRepositoryError>;
}
