//! Typed desktop command layer (E1-05, extended in E2): the boundary the UI
//! calls.
//!
//! The core methods are deliberately free of Tauri types so the
//! UI-to-SQLite integration tests in `paw-desktop/tests/` can drive the
//! exact code path the UI drives. `#[tauri::command]` wrappers delegate to
//! this module; no business logic may be added at the attribute layer.
//!
//! As the composition root, paw-desktop is the only crate that may hold
//! concrete repositories; use cases see ports only. Each repository opens
//! its own WAL connection (single-user desktop; SQLite serializes writers,
//! the busy timeout absorbs contention).

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;

use paw_application::ports::agent_repository::AgentStoreError;
use paw_application::ports::audit::AuditSink as _;
use paw_application::ports::authorization::ExecutionContext;
use paw_application::ports::workspace_repository::WorkspaceRepositoryError;
use paw_application::use_cases::knowledge as knowledge_use_cases;
use paw_application::use_cases::learning;
use paw_application::use_cases::work_management::{self as use_cases, WorkManagementError};
use paw_application::{
    AgentExecution, AgentRegistration, Approval, Area, AreaId, Assessment, Availability,
    CreatorDraft, CreatorDraftReview, CreatorDraftVersion, CreatorTopic, Goal, GoalId, Item,
    ItemId, ItemKind, ItemStatus, KnowledgeRefId, KnowledgeSource, LearningGapProposal, Plan,
    PlanId, PlanStatus, Project, ProjectId, ProjectStatus, Roadmap, RoadmapId, Session, Skill,
    SkillEvidence, Task, TaskId, TaskStatus, ValidationError, Workspace, WorkspaceId,
};
use paw_infrastructure::repositories::agent_governance::{
    SqliteAgentExecutionRepository, SqliteAgentGrantRepository, SqliteAgentRegistrationRepository,
    SqliteApprovalRepository,
};
use paw_infrastructure::repositories::area_repository::SqliteAreaRepository;
use paw_infrastructure::repositories::goal_repository::SqliteGoalRepository;
use paw_infrastructure::repositories::task_repository::SqliteTaskRepository;
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::security::{GovernedAuthorizer, LOCAL_ACTOR_ID, SqliteAuditSink};
use std::collections::BTreeMap;

/// The statically registered capability catalog (E5-01; ADR-026): one
/// descriptor per governed domain capability, built once at startup. The
/// set mirrors the already-gated Api surface — no new business capability
/// is introduced by the registry itself. `High` risk forces the approval
/// path; medium/low run under grants + audit only.
fn build_capability_registry() -> paw_application::ports::capability::CapabilityRegistry {
    use paw_application::ports::capability::{
        ApprovalHint, CapabilityDescriptor, CapabilityDomain, CapabilityRegistry, ContractVersion,
        Idempotency, OfflinePolicy, RiskClass, ScopeRequirement,
    };
    let mut registry = CapabilityRegistry::new();
    let mut register = |id: &str,
                        owner: CapabilityDomain,
                        risk: RiskClass,
                        idempotency: Idempotency,
                        offline: OfflinePolicy,
                        summary: &str| {
        registry
            .register(CapabilityDescriptor {
                id: id.to_string(),
                owner,
                contract_version: ContractVersion::new(1, 0, 0),
                required_scope: ScopeRequirement::none(),
                risk_class: risk,
                idempotency,
                offline_behavior: offline,
                approval_hint: if risk == RiskClass::High {
                    ApprovalHint::Required
                } else {
                    ApprovalHint::None
                },
                summary: summary.to_string(),
            })
            .expect("static capability catalog is valid");
    };
    register(
        "work_management.task.create",
        CapabilityDomain::WorkManagement,
        RiskClass::Low,
        Idempotency::Idempotent,
        OfflinePolicy::LocalOnly,
        "create a task in Work Management",
    );
    register(
        "work_management.task.update",
        CapabilityDomain::WorkManagement,
        RiskClass::Low,
        Idempotency::Idempotent,
        OfflinePolicy::LocalOnly,
        "update task details or status",
    );
    register(
        "work_management.task.delete",
        CapabilityDomain::WorkManagement,
        RiskClass::Medium,
        Idempotency::Idempotent,
        OfflinePolicy::LocalOnly,
        "delete a task",
    );
    register(
        "learning.item.create",
        CapabilityDomain::LearningGrowth,
        RiskClass::Low,
        Idempotency::Idempotent,
        OfflinePolicy::LocalOnly,
        "create a learning item",
    );
    register(
        "knowledge.ref.create",
        CapabilityDomain::KnowledgeContext,
        RiskClass::Low,
        Idempotency::Idempotent,
        OfflinePolicy::LocalOnly,
        "create a knowledge reference",
    );
    register(
        "knowledge.ref.update",
        CapabilityDomain::KnowledgeContext,
        RiskClass::Low,
        Idempotency::Idempotent,
        OfflinePolicy::LocalOnly,
        "update knowledge reference metadata",
    );
    register(
        "data.export",
        CapabilityDomain::SystemSecurity,
        RiskClass::Medium,
        Idempotency::Idempotent,
        OfflinePolicy::LocalOnly,
        "export workspace data",
    );
    register(
        "data.purge",
        CapabilityDomain::SystemSecurity,
        RiskClass::High,
        Idempotency::NonIdempotent,
        OfflinePolicy::LocalOnly,
        "purge expired data (destructive)",
    );
    registry
}

/// Work Management command surface constructed once at startup over the
/// authoritative local database (ADR-006: one `app.sqlite`). It owns the
/// concrete repositories plus the E1-09 authorization policy and audit
/// sink; use cases see ports only.
pub struct WorkManagementApi {
    workspaces: SqliteWorkspaceRepository,
    areas: SqliteAreaRepository,
    goals: SqliteGoalRepository,
    tasks: SqliteTaskRepository,
    roadmaps: paw_infrastructure::repositories::learning_repository::SqliteRoadmapRepository,
    plans: paw_infrastructure::repositories::learning_repository::SqlitePlanRepository,
    items: paw_infrastructure::repositories::learning_repository::SqliteItemRepository,
    sessions: paw_infrastructure::repositories::learning_repository::SqliteSessionRepository,
    knowledge: paw_infrastructure::repositories::knowledge_repository::SqliteKnowledgeRefRepository,
    search: paw_infrastructure::search::SqliteSearchService,
    dashboard: paw_infrastructure::dashboard::SqliteDashboard,
    attachments:
        paw_infrastructure::repositories::attachment_repository::SqliteAttachmentRepository,
    file_store: paw_infrastructure::filesystem::FsManagedFileStore,
    /// Coordination connection for the durable-operations services (E3):
    /// operations, outbox and retention operate on raw rusqlite connections.
    coordination: rusqlite::Connection,
    /// AI Collaboration governance stores (E5): registrations, executions,
    /// approvals and agent grants behind the governed authorizer.
    agent_registrations: SqliteAgentRegistrationRepository,
    agent_executions: SqliteAgentExecutionRepository,
    approvals: SqliteApprovalRepository,
    agent_grants: SqliteAgentGrantRepository,
    /// The mediated adapter for the fake transport (E5 conformance surface;
    /// real transports wait for ADR-022).
    fake_adapter: paw_infrastructure::agent_adapter::FakeAgentAdapter,
    /// Metadata-only capability registry, statically built at startup
    /// (ADR-026; no dynamic loading).
    registry: paw_application::ports::capability::CapabilityRegistry,
    /// The governed authorizer: local policy for the human subject, fresh
    /// grant evaluation for agent subjects (E5-02).
    authorizer: GovernedAuthorizer<SqliteAgentGrantRepository>,
    /// Durable envelope port for the invocation pipeline.
    operations_service: paw_infrastructure::operations::SqliteOperationService,
    /// Learning & Growth Beta stores (E6): skills/evidence/assessments
    /// (PAW-LG-008/009) and project learning-gap proposals (PAW-LG-007).
    skills: paw_infrastructure::repositories::skills::SqliteSkillRepository,
    skill_evidence: paw_infrastructure::repositories::skills::SqliteSkillEvidenceRepository,
    assessments: paw_infrastructure::repositories::skills::SqliteAssessmentRepository,
    gap_proposals: paw_infrastructure::repositories::gap_proposals::SqliteGapProposalRepository,
    /// Creator content pipeline stores (E6-03).
    creator_topics: paw_infrastructure::repositories::creator::SqliteCreatorTopicRepository,
    creator_drafts: paw_infrastructure::repositories::creator::SqliteCreatorDraftRepository,
    audit: SqliteAuditSink,
}

impl WorkManagementApi {
    /// Open the command surface over `db_path`. The database must already
    /// carry the current schema (migrations run before this call). Managed
    /// attachment bytes live in a sibling `attachments/` directory (E2-04).
    pub fn open(db_path: &Path) -> Result<Self, paw_infrastructure::sqlite::Error> {
        Ok(Self {
            workspaces: SqliteWorkspaceRepository::open(db_path)?,
            areas: SqliteAreaRepository::open(db_path)?,
            goals: SqliteGoalRepository::open(db_path)?,
            tasks: SqliteTaskRepository::open(db_path)?,
            roadmaps:
                paw_infrastructure::repositories::learning_repository::SqliteRoadmapRepository::open(
                    db_path,
                )?,
            plans:
                paw_infrastructure::repositories::learning_repository::SqlitePlanRepository::open(
                    db_path,
                )?,
            items:
                paw_infrastructure::repositories::learning_repository::SqliteItemRepository::open(
                    db_path,
                )?,
            sessions:
                paw_infrastructure::repositories::learning_repository::SqliteSessionRepository::open(
                    db_path,
                )?,
            knowledge:
                paw_infrastructure::repositories::knowledge_repository::SqliteKnowledgeRefRepository::open(
                    db_path,
                )?,
            search: paw_infrastructure::search::SqliteSearchService::open(db_path)?,
            dashboard: paw_infrastructure::dashboard::SqliteDashboard::open(db_path)?,
            attachments:
                paw_infrastructure::repositories::attachment_repository::SqliteAttachmentRepository::open(
                    db_path,
                )?,
            file_store: paw_infrastructure::filesystem::FsManagedFileStore::open(
                db_path
                    .parent()
                    .expect("db_path lives inside the app data directory"),
            )
            .map_err(|e| {
                paw_infrastructure::sqlite::Error::Io {
                    source: std::io::Error::other(e.to_string()),
                }
            })?,
            coordination: paw_infrastructure::sqlite::open(db_path)?,
            agent_registrations: SqliteAgentRegistrationRepository::open(db_path)?,
            agent_executions: SqliteAgentExecutionRepository::open(db_path)?,
            approvals: SqliteApprovalRepository::open(db_path)?,
            agent_grants: SqliteAgentGrantRepository::open(db_path)?,
            fake_adapter: paw_infrastructure::agent_adapter::FakeAgentAdapter::new(),
            registry: build_capability_registry(),
            authorizer: {
                let grants = SqliteAgentGrantRepository::open(db_path)?;
                GovernedAuthorizer::new(grants)
            },
            operations_service: paw_infrastructure::operations::SqliteOperationService::new(
                paw_infrastructure::sqlite::open(db_path)?,
            ),
            skills: paw_infrastructure::repositories::skills::SqliteSkillRepository::open(
                db_path,
            )?,
            skill_evidence:
                paw_infrastructure::repositories::skills::SqliteSkillEvidenceRepository::open(
                    db_path,
                )?,
            assessments:
                paw_infrastructure::repositories::skills::SqliteAssessmentRepository::open(
                    db_path,
                )?,
            gap_proposals:
                paw_infrastructure::repositories::gap_proposals::SqliteGapProposalRepository::open(
                    db_path,
                )?,
            creator_topics:
                paw_infrastructure::repositories::creator::SqliteCreatorTopicRepository::open(
                    db_path,
                )?,
            creator_drafts:
                paw_infrastructure::repositories::creator::SqliteCreatorDraftRepository::open(
                    db_path,
                )?,
            audit: SqliteAuditSink::open(db_path)?,
        })
    }

    /// Execution context for one command invocation: the local actor and a
    /// fresh correlation id (E1-09 audit trail).
    fn context(&self) -> ExecutionContext {
        ExecutionContext {
            actor_id: LOCAL_ACTOR_ID.into(),
            correlation_id: uuid::Uuid::now_v7().to_string(),
        }
    }

    // -- Workspaces ---------------------------------------------------------

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, WorkManagementError> {
        use_cases::list_workspaces(&self.workspaces)
    }

    pub fn create_workspace(&self, name: &str) -> Result<Workspace, WorkManagementError> {
        let ctx = self.context();
        use_cases::workspace::create_workspace(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            name,
        )
    }

    // -- Projects -----------------------------------------------------------

    pub fn create_project(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
    ) -> Result<Project, WorkManagementError> {
        let ctx = self.context();
        use_cases::project::create_project(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            name,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_project(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
        name: &str,
        description: Option<&str>,
        status: ProjectStatus,
        goal_id: Option<GoalId>,
        area_id: Option<AreaId>,
    ) -> Result<Project, WorkManagementError> {
        let ctx = self.context();
        use_cases::project::update_project(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            project_id,
            name,
            description,
            status,
            goal_id,
            area_id,
        )
    }

    pub fn delete_project(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::project::delete_project(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            project_id,
        )
    }

    pub fn list_projects(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Project>, WorkManagementError> {
        use_cases::workspace::list_projects(&self.workspaces, workspace_id)
    }

    // -- Areas --------------------------------------------------------------

    pub fn create_area(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Area, WorkManagementError> {
        let ctx = self.context();
        use_cases::area::create_area(
            &self.areas,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            name,
            description,
        )
    }

    pub fn update_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Area, WorkManagementError> {
        let ctx = self.context();
        use_cases::area::update_area(
            &self.areas,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            area_id,
            name,
            description,
        )
    }

    pub fn delete_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::area::delete_area(
            &self.areas,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            area_id,
        )
    }

    pub fn list_areas(&self, workspace_id: &WorkspaceId) -> Result<Vec<Area>, WorkManagementError> {
        use_cases::area::list_areas(&self.areas, workspace_id)
    }

    // -- Goals --------------------------------------------------------------

    pub fn create_goal(
        &self,
        workspace_id: &WorkspaceId,
        area_id: Option<AreaId>,
        name: &str,
        description: Option<&str>,
    ) -> Result<Goal, WorkManagementError> {
        let ctx = self.context();
        use_cases::goal::create_goal(
            &self.goals,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            area_id,
            name,
            description,
        )
    }

    pub fn update_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
        area_id: Option<AreaId>,
        name: &str,
        description: Option<&str>,
    ) -> Result<Goal, WorkManagementError> {
        let ctx = self.context();
        use_cases::goal::update_goal(
            &self.goals,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            goal_id,
            area_id,
            name,
            description,
        )
    }

    pub fn delete_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::goal::delete_goal(
            &self.goals,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            goal_id,
        )
    }

    pub fn list_goals(
        &self,
        workspace_id: &WorkspaceId,
        area_id: Option<&AreaId>,
    ) -> Result<Vec<Goal>, WorkManagementError> {
        use_cases::goal::list_goals(&self.goals, workspace_id, area_id)
    }

    // -- Search (E2-03) -----------------------------------------------------

    /// Scoped full-text search over projects and tasks.
    pub fn search(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<paw_application::ports::search::SearchHit>, CommandError> {
        use_cases::search(&self.search, workspace_id, query, limit).map_err(CommandError::from)
    }

    // -- Dashboard (E2-05) ----------------------------------------------------

    /// Real-data summary for one workspace (PAW-F-004).
    pub fn dashboard(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<paw_application::ports::dashboard::DashboardSummary, CommandError> {
        use paw_application::ports::dashboard::DashboardPort;
        self.dashboard.summary(workspace_id).map_err(|e| match e {
            paw_application::ports::dashboard::DashboardError::Invariant(_) => {
                CommandError::Unexpected
            }
        })
    }

    // -- Learning & Growth (E4, PAW-LG-001～006) ---------------------------------
    //
    // Boundary adapter only: each method mints the execution context and
    // delegates to the Learning use cases (ADR-017 — coordination lives in
    // paw-application). Authorization gates, audit rows and domain rules are
    // the use case layer's responsibility; typed failures are mapped to the
    // stable command error codes below.

    pub fn list_roadmaps(&self, workspace_id: &WorkspaceId) -> Result<Vec<Roadmap>, CommandError> {
        learning::roadmap::list_roadmaps(&self.roadmaps, workspace_id).map_err(CommandError::from)
    }

    pub fn create_roadmap(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Roadmap, CommandError> {
        let ctx = self.context();
        learning::roadmap::create_roadmap(
            &self.roadmaps,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            name,
            description,
        )
        .map_err(CommandError::from)
    }

    pub fn delete_roadmap(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: &RoadmapId,
    ) -> Result<(), CommandError> {
        let ctx = self.context();
        learning::roadmap::delete_roadmap(
            &self.roadmaps,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            roadmap_id,
        )
        .map_err(CommandError::from)
    }

    pub fn list_plans(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: Option<&RoadmapId>,
    ) -> Result<Vec<Plan>, CommandError> {
        learning::plan::list_plans(&self.plans, workspace_id, roadmap_id)
            .map_err(CommandError::from)
    }

    pub fn create_plan(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: &RoadmapId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Plan, CommandError> {
        let ctx = self.context();
        learning::plan::create_plan(
            &self.plans,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            roadmap_id,
            name,
            description,
        )
        .map_err(CommandError::from)
    }

    pub fn update_plan_status(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: &PlanId,
        status: PlanStatus,
    ) -> Result<Plan, CommandError> {
        let ctx = self.context();
        learning::plan::update_plan_status(
            &self.plans,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            plan_id,
            status,
        )
        .map_err(CommandError::from)
    }

    pub fn list_items(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: Option<&PlanId>,
    ) -> Result<Vec<Item>, CommandError> {
        learning::item::list_items(&self.items, workspace_id, plan_id).map_err(CommandError::from)
    }

    pub fn create_item(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: &PlanId,
        kind: ItemKind,
        title: &str,
        expected_outcome: Option<&str>,
        acceptance_criteria: Option<&str>,
    ) -> Result<Item, CommandError> {
        let ctx = self.context();
        learning::item::create_item(
            &self.items,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            plan_id,
            kind,
            title,
            expected_outcome,
            acceptance_criteria,
        )
        .map_err(CommandError::from)
    }

    pub fn transition_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        to: ItemStatus,
    ) -> Result<Item, CommandError> {
        let ctx = self.context();
        learning::item::transition_item(
            &self.items,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            item_id,
            to,
        )
        .map_err(CommandError::from)
    }

    pub fn link_item_to_project(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        project_id: &ProjectId,
    ) -> Result<(), CommandError> {
        let ctx = self.context();
        learning::item::link_item_to_project(
            &self.items,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            item_id,
            project_id,
        )
        .map_err(CommandError::from)
    }

    /// PAW-LG-006 (CreateTaskForLearningItem semantics): create a Work
    /// Management task for a learning item and record the action-step link.
    /// The orchestration lives in `learning::item` (ADR-017): the task is
    /// still created by the Work Management use case and stays
    /// Work-Management-owned.
    pub fn create_task_for_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        title: &str,
    ) -> Result<Task, CommandError> {
        let ctx = self.context();
        learning::item::request_task_for_item(
            &self.items,
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            item_id,
            title,
        )
        .map_err(CommandError::from)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append_session(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        method: Option<&str>,
        duration_minutes: Option<i64>,
        summary: Option<&str>,
        difficulty: Option<&str>,
        takeaway: Option<&str>,
        next_step: Option<&str>,
    ) -> Result<Session, CommandError> {
        // Clock reading stays at the boundary: the use case takes the
        // occurrence timestamp as data and never reads infrastructure time.
        let occurred_at =
            paw_infrastructure::sqlite::now_ms().map_err(|_| CommandError::Unexpected)?;
        let ctx = self.context();
        learning::session::append_session(
            &self.sessions,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            item_id,
            method,
            duration_minutes,
            summary,
            difficulty,
            takeaway,
            next_step,
            occurred_at,
        )
        .map_err(CommandError::from)
    }

    pub fn list_sessions(
        &self,
        workspace_id: &WorkspaceId,
        item_id: Option<&ItemId>,
    ) -> Result<Vec<Session>, CommandError> {
        learning::session::list_sessions(&self.sessions, workspace_id, item_id)
            .map_err(CommandError::from)
    }

    // -- Knowledge & Context (E4, PAW-K-001) -------------------------------------

    pub fn list_knowledge_refs(
        &self,
        workspace_id: &WorkspaceId,
        availability: Option<Availability>,
    ) -> Result<Vec<paw_application::KnowledgeRef>, CommandError> {
        knowledge_use_cases::list_knowledge_refs(&self.knowledge, workspace_id, availability)
            .map_err(CommandError::from)
    }

    pub fn create_knowledge_ref(
        &self,
        workspace_id: &WorkspaceId,
        title: &str,
        source: KnowledgeSource,
        external_uri: Option<&str>,
        locator: Option<&str>,
        version_label: Option<&str>,
    ) -> Result<paw_application::KnowledgeRef, CommandError> {
        let ctx = self.context();
        knowledge_use_cases::create_knowledge_ref(
            &self.knowledge,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            title,
            source,
            external_uri,
            locator,
            version_label,
        )
        .map_err(CommandError::from)
    }

    pub fn set_knowledge_availability(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
        availability: Availability,
    ) -> Result<paw_application::KnowledgeRef, CommandError> {
        let ctx = self.context();
        knowledge_use_cases::set_knowledge_availability(
            &self.knowledge,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            knowledge_ref_id,
            availability,
        )
        .map_err(CommandError::from)
    }

    pub fn delete_knowledge_ref(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
    ) -> Result<(), CommandError> {
        let ctx = self.context();
        knowledge_use_cases::delete_knowledge_ref(
            &self.knowledge,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            knowledge_ref_id,
        )
        .map_err(CommandError::from)
    }

    // -- Operation queue (E3-04) ------------------------------------------------

    /// Workspace-scoped operation queue view. Read-only (reads are not
    /// authorization-gated in this codebase's current policy).
    pub fn list_operations(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<paw_infrastructure::operations::OperationRow>, CommandError> {
        paw_infrastructure::operations::list_operations(
            &self.coordination,
            workspace_id.as_str(),
            None,
        )
        .map_err(|_| CommandError::Unexpected)
    }

    /// Request cooperative cancellation (audited, gated).
    pub fn request_operation_cancellation(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &str,
    ) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::CancelOperation {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "operation",
            Some(operation_id),
        )?;
        let row =
            paw_infrastructure::operations::request_cancellation(&self.coordination, operation_id)
                .map_err(map_classification_error)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "operation",
                resource_id: Some(operation_id.to_string()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(row)
    }

    /// Reconcile an OutcomeUnknown operation back into Running (audited).
    pub fn resume_operation(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &str,
        note: &str,
    ) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::ResumeOperation {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "operation",
            Some(operation_id),
        )?;
        let row = paw_infrastructure::operations::resume_after_reconciliation(
            &self.coordination,
            operation_id,
            note,
        )
        .map_err(map_operation_error)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "operation",
                resource_id: Some(operation_id.to_string()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(row)
    }

    /// List quarantined messages of a workspace (plus system-scope rows are
    /// excluded: only the unfiltered system view sees those).
    pub fn list_quarantined(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<paw_infrastructure::outbox::QuarantinedMessage>, CommandError> {
        paw_infrastructure::outbox::list_quarantined(
            &self.coordination,
            Some(workspace_id.as_str()),
        )
        .map_err(|_| CommandError::Unexpected)
    }

    /// Re-drive a quarantined message (gated, audited; operator recorded).
    pub fn redrive_message(
        &self,
        workspace_id: &WorkspaceId,
        message_id: &str,
    ) -> Result<String, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::RedriveMessage {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "message",
            Some(message_id),
        )?;
        let now = paw_infrastructure::sqlite::now_ms().map_err(|_| CommandError::Unexpected)?;
        let id = paw_infrastructure::outbox::redrive_quarantined(
            &self.coordination,
            message_id,
            &ctx.actor_id,
            now,
        )
        .map_err(|_| CommandError::Unexpected)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "message",
                resource_id: Some(id.clone()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(id)
    }

    /// Export the workspace's durable-operation history to a temp JSON file
    /// and return its path (pre-purge user-initiated step; audited). The
    /// command itself runs under a durable operation envelope so the queue
    /// view shows the consequential action (E3-05 windowed-run fix: the
    /// queue was unverifiable because no Alpha 0.1 flow opened envelopes).
    pub fn export_operations_to_temp(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<String, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::ExportWorkspaceData {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "operation",
            None,
        )?;
        // Durable envelope for the consequential command (E0.3.2 §2: a
        // bounded unit of work with persisted progress).
        let envelope = paw_infrastructure::operations::create_operation(
            &self.coordination,
            &paw_infrastructure::operations::NewOperation {
                kind: "data.export".into(),
                owner_domain: "system_security".into(),
                owner_type: "workspace".into(),
                owner_id: workspace_id.as_str().to_string(),
                workspace_id: Some(workspace_id.as_str().to_string()),
                actor_id: ctx.actor_id.clone(),
                correlation_id: ctx.correlation_id.clone(),
                causation_id: None,
                cancellation_support:
                    paw_infrastructure::operations::CancellationSupport::Unsupported,
            },
        )
        .map_err(map_operation_error)?;
        paw_infrastructure::operations::transition(
            &self.coordination,
            &envelope.id,
            paw_infrastructure::operations::OperationStatus::Running,
            None,
        )
        .map_err(map_operation_error)?;

        let result = (|| {
            let json = paw_infrastructure::retention::export_operations_json(
                &self.coordination,
                workspace_id.as_str(),
            )
            .map_err(|_| CommandError::Unexpected)?;
            let dir = std::env::temp_dir().join("paw-exports");
            std::fs::create_dir_all(&dir).map_err(CommandError::from_io)?;
            let path = dir.join(format!(
                "operations-{}-{}.json",
                workspace_id.as_str(),
                paw_infrastructure::sqlite::now_ms().map_err(|_| CommandError::Unexpected)?
            ));
            std::fs::write(&path, json).map_err(CommandError::from_io)?;
            Ok::<String, CommandError>(path.to_string_lossy().into_owned())
        })();

        match &result {
            Ok(_) => {
                paw_infrastructure::operations::transition(
                    &self.coordination,
                    &envelope.id,
                    paw_infrastructure::operations::OperationStatus::Succeeded,
                    None,
                )
                .map_err(map_operation_error)?;
            }
            Err(_) => {
                paw_infrastructure::operations::transition(
                    &self.coordination,
                    &envelope.id,
                    paw_infrastructure::operations::OperationStatus::Failed,
                    Some("export_failed"),
                )
                .map_err(map_operation_error)?;
            }
        }

        let path = result?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "export",
                resource_id: Some(path.clone()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(path)
    }

    /// Purge expired operations and audit rows under the default policy
    /// (gated, audited, explicit — no background purge exists). Runs under
    /// a system-scope durable envelope (E3-05 windowed-run fix, same as
    /// export).
    pub fn purge_expired(&self) -> Result<(usize, usize, usize), CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::PurgeExpiredData;
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            None,
            "data",
            None,
        )?;
        // Durable system-scope envelope for the consequential purge.
        let envelope = paw_infrastructure::operations::create_operation(
            &self.coordination,
            &paw_infrastructure::operations::NewOperation {
                kind: "data.purge".into(),
                owner_domain: "system_security".into(),
                owner_type: "system".into(),
                owner_id: "retention".into(),
                workspace_id: None,
                actor_id: ctx.actor_id.clone(),
                correlation_id: ctx.correlation_id.clone(),
                causation_id: None,
                cancellation_support:
                    paw_infrastructure::operations::CancellationSupport::Unsupported,
            },
        )
        .map_err(map_operation_error)?;
        paw_infrastructure::operations::transition(
            &self.coordination,
            &envelope.id,
            paw_infrastructure::operations::OperationStatus::Running,
            None,
        )
        .map_err(map_operation_error)?;

        let result = (|| {
            let now = paw_infrastructure::sqlite::now_ms().map_err(|_| CommandError::Unexpected)?;
            let operations = paw_infrastructure::retention::purge_expired_operations(
                &self.coordination,
                now,
                &Default::default(),
            )
            .map_err(|_| CommandError::Unexpected)?;
            let audit_rows = paw_infrastructure::retention::purge_expired_audit_events(
                &self.coordination,
                now,
                &Default::default(),
            )
            .map_err(|_| CommandError::Unexpected)?;
            Ok::<_, CommandError>((operations.0, operations.1, audit_rows))
        })();

        match &result {
            Ok(_) => {
                paw_infrastructure::operations::transition(
                    &self.coordination,
                    &envelope.id,
                    paw_infrastructure::operations::OperationStatus::Succeeded,
                    None,
                )
                .map_err(map_operation_error)?;
            }
            Err(_) => {
                paw_infrastructure::operations::transition(
                    &self.coordination,
                    &envelope.id,
                    paw_infrastructure::operations::OperationStatus::Failed,
                    Some("purge_failed"),
                )
                .map_err(map_operation_error)?;
            }
        }

        let (operations, attempts, audit_rows) = result?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: None,
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "data",
                resource_id: Some(format!(
                    "operations={operations},attempts={attempts},audit={audit_rows}"
                )),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok((operations, attempts, audit_rows))
    }

    // -- Attachments (E2-04) --------------------------------------------------

    pub fn add_attachment(
        &self,
        workspace_id: &WorkspaceId,
        owner: paw_application::ports::attachment_repository::AttachmentOwner,
        owner_id: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<paw_application::ports::attachment_repository::Attachment, WorkManagementError>
    {
        let ctx = self.context();
        use_cases::attachment::add_attachment(
            &self.attachments,
            &self.file_store,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            owner,
            owner_id,
            file_name,
            bytes,
        )
    }

    pub fn list_attachments(
        &self,
        workspace_id: &WorkspaceId,
        owner: paw_application::ports::attachment_repository::AttachmentOwner,
        owner_id: &str,
    ) -> Result<Vec<paw_application::ports::attachment_repository::Attachment>, WorkManagementError>
    {
        use_cases::attachment::list_attachments(&self.attachments, workspace_id, owner, owner_id)
    }

    pub fn read_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<
        (
            paw_application::ports::attachment_repository::Attachment,
            Vec<u8>,
        ),
        WorkManagementError,
    > {
        use_cases::attachment::read_attachment(
            &self.attachments,
            &self.file_store,
            workspace_id,
            attachment_id,
        )
    }

    /// Open an attachment with the OS default application (E2-04 UI fix:
    /// WebView2 swallows programmatic `<a download>` clicks, so bytes are
    /// exported to a temp file and handed to the shell instead). The copy
    /// lives under the user's temp directory; removing it is left to OS
    /// temp cleanup and nothing is executed.
    pub fn open_attachment_externally(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<String, WorkManagementError> {
        let (attachment, bytes) = self.read_attachment(workspace_id, attachment_id)?;
        let temp_dir = std::env::temp_dir().join("paw-attachments");
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            WorkManagementError::Repository(
                paw_application::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                    e.to_string(),
                ),
            )
        })?;
        // The attachment id prefixes the name: unique per attachment and
        // free of path components (the stored file name was validated at
        // add time).
        let path = temp_dir.join(format!("{}_{}", attachment.id, attachment.file_name));
        std::fs::write(&path, &bytes).map_err(|e| {
            WorkManagementError::Repository(
                paw_application::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                    e.to_string(),
                ),
            )
        })?;
        // `start "" "path"` opens with the default application; the empty
        // title argument guards paths with spaces.
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: no console flash
            .spawn()
            .map_err(|e| {
                WorkManagementError::Repository(
                    paw_application::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                        e.to_string(),
                    ),
                )
            })?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub fn remove_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::attachment::remove_attachment(
            &self.attachments,
            &self.file_store,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            attachment_id,
        )
    }

    // -- Tasks --------------------------------------------------------------

    pub fn create_task(
        &self,
        workspace_id: &WorkspaceId,
        project_id: Option<ProjectId>,
        title: &str,
        notes: Option<&str>,
    ) -> Result<Task, WorkManagementError> {
        let ctx = self.context();
        use_cases::task::create_task(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            project_id,
            title,
            notes,
        )
    }

    pub fn update_task_details(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
        title: &str,
        notes: Option<&str>,
        project_id: Option<ProjectId>,
    ) -> Result<Task, WorkManagementError> {
        let ctx = self.context();
        use_cases::task::update_task_details(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            task_id,
            title,
            notes,
            project_id,
        )
    }

    pub fn transition_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
        to: TaskStatus,
    ) -> Result<Task, WorkManagementError> {
        let ctx = self.context();
        use_cases::task::transition_task(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            task_id,
            to,
        )
    }

    pub fn delete_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::task::delete_task(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            task_id,
        )
    }

    pub fn list_tasks(
        &self,
        workspace_id: &WorkspaceId,
        project_id: Option<&ProjectId>,
        status: Option<TaskStatus>,
    ) -> Result<Vec<Task>, WorkManagementError> {
        use_cases::task::list_tasks(&self.tasks, workspace_id, project_id, status)
    }

    // -- Skill matrix & growth assessment (E6-01, PAW-LG-008/009) ----------------

    /// Register a skill in the matrix. Levels are never inferred here —
    /// assessments are separate user-confirmed records (ADR-015).
    pub fn create_skill(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
        category: &str,
        notes: Option<&str>,
    ) -> Result<Skill, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::CreateSkill {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "skill",
            None,
        )?;
        let skill = Skill::new(
            paw_application::SkillId::new_v7(),
            workspace_id.clone(),
            name,
            category,
            notes,
        )
        .map_err(CommandError::from)?;
        use paw_application::ports::skill_repository::SkillRepository as _;
        self.skills
            .insert_skill(&skill)
            .map_err(CommandError::from)?;
        Ok(skill)
    }

    /// Update skill metadata (rename/category/notes) or bump the scale
    /// version via the entity's own method before calling.
    pub fn update_skill(&self, skill: &Skill) -> Result<Skill, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::UpdateSkill {
                workspace_id: skill.workspace_id.clone(),
            },
            Some(&skill.workspace_id),
            "skill",
            Some(skill.id.as_str()),
        )?;
        use paw_application::ports::skill_repository::SkillRepository as _;
        self.skills
            .update_skill(skill)
            .map_err(CommandError::from)?;
        Ok(skill.clone())
    }

    pub fn list_skills(
        &self,
        workspace_id: &WorkspaceId,
        category: Option<&str>,
    ) -> Result<Vec<Skill>, CommandError> {
        use paw_application::ports::skill_repository::SkillRepository as _;
        self.skills
            .list_skills(workspace_id, category)
            .map_err(CommandError::from)
    }

    /// Add an evidence record pointing at its source (PRD §3.6).
    #[allow(clippy::too_many_arguments)]
    pub fn add_skill_evidence(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &str,
        kind: &str,
        source_kind: &str,
        source_id: &str,
        summary: Option<&str>,
    ) -> Result<SkillEvidence, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::CreateSkillEvidence {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "skill_evidence",
            None,
        )?;
        let skill = paw_application::SkillId::new(skill_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        let kind = paw_application::EvidenceKind::parse(kind).ok_or(CommandError::Unexpected)?;
        let source = paw_application::EvidenceSourceKind::parse(source_kind)
            .ok_or(CommandError::Unexpected)?;
        let record = SkillEvidence::new(
            paw_application::SkillEvidenceId::new_v7(),
            workspace_id.clone(),
            skill,
            kind,
            source,
            source_id,
            summary,
        )
        .map_err(CommandError::from)?;
        use paw_application::ports::skill_repository::SkillEvidenceRepository as _;
        self.skill_evidence
            .insert_evidence(&record)
            .map_err(CommandError::from)?;
        Ok(record)
    }

    pub fn list_skill_evidence(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &str,
    ) -> Result<Vec<SkillEvidence>, CommandError> {
        let skill = paw_application::SkillId::new(skill_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::skill_repository::SkillEvidenceRepository as _;
        self.skill_evidence
            .list_evidence(workspace_id, &skill)
            .map_err(CommandError::from)
    }

    /// Create a user-confirmed assessment (PAW-LG-009). The evidence ids
    /// are validated to exist in this workspace before the record is
    /// stored — assessment history is append-only and snapshots the skill's
    /// scale version (ADR-015).
    pub fn create_assessment(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &str,
        level: u8,
        confidence: &str,
        rationale: Option<&str>,
        evidence_ids: &[String],
    ) -> Result<Assessment, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::CreateAssessment {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "assessment",
            None,
        )?;
        let skill = paw_application::SkillId::new(skill_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::skill_repository::{
            SkillEvidenceRepository as _, SkillRepository as _,
        };
        // The skill exists and the assessment snapshots its current scale.
        let skill_record = self
            .skills
            .get_skill(&skill, workspace_id)
            .map_err(CommandError::from)?;
        let mut evidence = Vec::new();
        for id in evidence_ids {
            let evidence_id = paw_application::SkillEvidenceId::new(id)
                .map_err(|_| CommandError::InvalidWorkspaceId)?;
            // Validation at use time: the evidence must exist in this
            // workspace (and belong to the same skill).
            let record = self
                .skill_evidence
                .get_evidence(&evidence_id, workspace_id)
                .map_err(CommandError::from)?;
            if record.skill_id != skill {
                return Err(CommandError::SkillNotFound);
            }
            evidence.push(evidence_id);
        }
        let confidence =
            paw_application::Confidence::parse(confidence).ok_or(CommandError::Unexpected)?;
        let assessment = Assessment::new(
            paw_application::AssessmentId::new_v7(),
            workspace_id.clone(),
            skill,
            level,
            skill_record.scale_version,
            confidence,
            rationale,
            evidence,
        )
        .map_err(CommandError::from)?;
        use paw_application::ports::skill_repository::AssessmentRepository as _;
        self.assessments
            .insert_assessment(&assessment, None)
            .map_err(CommandError::from)?;
        Ok(assessment)
    }

    pub fn list_assessments(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &str,
    ) -> Result<Vec<Assessment>, CommandError> {
        let skill = paw_application::SkillId::new(skill_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::skill_repository::AssessmentRepository as _;
        self.assessments
            .list_assessments(workspace_id, &skill)
            .map_err(CommandError::from)
    }

    // -- Project learning gaps (E6-02, PAW-LG-007) -------------------------------

    /// Propose a learning gap from a project (AI/复盘 suggestions enter as
    /// `Proposed`; they become plan material only after user confirmation).
    pub fn propose_learning_gap(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &str,
        gap: &str,
    ) -> Result<LearningGapProposal, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::ProposeLearningGap {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "learning_gap",
            None,
        )?;
        let project = ProjectId::new(project_id).map_err(|_| CommandError::ProjectNotFound)?;
        let gap = LearningGapProposal::new(
            paw_application::LearningGapProposalId::new_v7(),
            workspace_id.clone(),
            project,
            gap,
        )
        .map_err(|e| match e {
            paw_application::ValidationError::EmptyName => CommandError::EmptyName,
            _ => CommandError::NameTooLong,
        })?;
        use paw_application::ports::learning_repository::GapProposalRepository as _;
        self.gap_proposals
            .insert_gap_proposal(&gap)
            .map_err(CommandError::from_learning)?;
        Ok(gap)
    }

    /// Confirm a gap: bind an existing learning item (creation stays a
    /// separate user decision through the learning commands).
    pub fn confirm_learning_gap(
        &self,
        workspace_id: &WorkspaceId,
        gap_id: &str,
        item_id: &str,
    ) -> Result<LearningGapProposal, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::ConfirmLearningGap {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "learning_gap",
            Some(gap_id),
        )?;
        let id = paw_application::LearningGapProposalId::new(gap_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        let item = paw_application::ItemId::new(item_id).map_err(|_| CommandError::ItemNotFound)?;
        use paw_application::ports::learning_repository::GapProposalRepository as _;
        let mut gap = self
            .gap_proposals
            .get_gap_proposal(&id, workspace_id)
            .map_err(CommandError::from_learning)?;
        gap.confirm(item)
            .map_err(|_| CommandError::TaskStatusInvalid)?;
        self.gap_proposals
            .update_gap_proposal(&gap)
            .map_err(CommandError::from_learning)?;
        Ok(gap)
    }

    pub fn dismiss_learning_gap(
        &self,
        workspace_id: &WorkspaceId,
        gap_id: &str,
    ) -> Result<LearningGapProposal, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::ConfirmLearningGap {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "learning_gap",
            Some(gap_id),
        )?;
        let id = paw_application::LearningGapProposalId::new(gap_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::learning_repository::GapProposalRepository as _;
        let mut gap = self
            .gap_proposals
            .get_gap_proposal(&id, workspace_id)
            .map_err(CommandError::from_learning)?;
        gap.dismiss().map_err(|_| CommandError::TaskStatusInvalid)?;
        self.gap_proposals
            .update_gap_proposal(&gap)
            .map_err(CommandError::from_learning)?;
        Ok(gap)
    }

    pub fn list_learning_gaps(
        &self,
        workspace_id: &WorkspaceId,
        status: Option<paw_application::GapProposalStatus>,
    ) -> Result<Vec<LearningGapProposal>, CommandError> {
        use paw_application::ports::learning_repository::GapProposalRepository as _;
        self.gap_proposals
            .list_gap_proposals(workspace_id, status)
            .map_err(CommandError::from)
    }

    // -- Creator content pipeline (E6-03) ----------------------------------------

    pub fn create_creator_topic(
        &self,
        workspace_id: &WorkspaceId,
        title: &str,
        notes: Option<&str>,
    ) -> Result<CreatorTopic, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::CreateCreatorTopic {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "creator_topic",
            None,
        )?;
        let topic = CreatorTopic::new(
            paw_application::CreatorTopicId::new_v7(),
            workspace_id.clone(),
            title,
            notes,
        )
        .map_err(CommandError::from)?;
        use paw_application::ports::creator_repository::CreatorTopicRepository as _;
        self.creator_topics
            .insert_topic(&topic)
            .map_err(CommandError::from)?;
        Ok(topic)
    }

    /// Update a topic (title/notes/status transitions validated by the
    /// domain state machine).
    pub fn update_creator_topic(&self, topic: &CreatorTopic) -> Result<CreatorTopic, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::UpdateCreatorTopic {
                workspace_id: topic.workspace_id.clone(),
            },
            Some(&topic.workspace_id),
            "creator_topic",
            Some(topic.id.as_str()),
        )?;
        use paw_application::ports::creator_repository::CreatorTopicRepository as _;
        self.creator_topics
            .update_topic(topic)
            .map_err(CommandError::from)?;
        Ok(topic.clone())
    }

    pub fn list_creator_topics(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<CreatorTopic>, CommandError> {
        use paw_application::ports::creator_repository::CreatorTopicRepository as _;
        self.creator_topics
            .list_topics(workspace_id, None)
            .map_err(CommandError::from)
    }

    pub fn create_creator_draft(
        &self,
        workspace_id: &WorkspaceId,
        topic_id: &str,
        title: &str,
    ) -> Result<CreatorDraft, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::CreateCreatorDraft {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "creator_draft",
            None,
        )?;
        let topic = paw_application::CreatorTopicId::new(topic_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        let draft = CreatorDraft::new(
            paw_application::CreatorDraftId::new_v7(),
            workspace_id.clone(),
            topic,
            title,
        )
        .map_err(CommandError::from)?;
        use paw_application::ports::creator_repository::CreatorDraftRepository as _;
        self.creator_drafts
            .insert_draft(&draft)
            .map_err(CommandError::from)?;
        Ok(draft)
    }

    /// Append an immutable version snapshot (head bumps atomically).
    pub fn append_creator_draft_version(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &str,
        body: &str,
    ) -> Result<CreatorDraft, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::UpdateCreatorDraft {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "creator_draft",
            Some(draft_id),
        )?;
        let id = paw_application::CreatorDraftId::new(draft_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::creator_repository::CreatorDraftRepository as _;
        let mut draft = self
            .creator_drafts
            .get_draft(&id, workspace_id)
            .map_err(CommandError::from)?;
        let next = draft.current_version_no + 1;
        let version = CreatorDraftVersion::new(
            paw_application::CreatorDraftVersionId::new_v7(),
            workspace_id.clone(),
            draft.id.clone(),
            next,
            body,
        )
        .map_err(CommandError::from)?;
        self.creator_drafts
            .append_version(&version)
            .map_err(CommandError::from)?;
        draft.current_version_no = next;
        Ok(draft)
    }

    /// Advance the draft status (domain machine validates the transition).
    pub fn transition_creator_draft(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &str,
        new_status: &str,
    ) -> Result<CreatorDraft, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::UpdateCreatorDraft {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "creator_draft",
            Some(draft_id),
        )?;
        let id = paw_application::CreatorDraftId::new(draft_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::creator_repository::CreatorDraftRepository as _;
        let mut draft = self
            .creator_drafts
            .get_draft(&id, workspace_id)
            .map_err(CommandError::from)?;
        let status =
            paw_application::DraftStatus::parse(new_status).ok_or(CommandError::Unexpected)?;
        draft.transition(status).map_err(CommandError::from)?;
        self.creator_drafts
            .update_draft(&draft)
            .map_err(CommandError::from)?;
        Ok(draft)
    }

    /// Record a review decision bound to one exact version; on approval the
    /// draft status machine is advanced by the caller's follow-up
    /// transition (reviews are history, not state writes).
    pub fn submit_creator_review(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &str,
        version_no: u32,
        decision: &str,
        comment: Option<&str>,
    ) -> Result<CreatorDraftReview, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::ReviewCreatorDraft {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "creator_review",
            Some(draft_id),
        )?;
        let id = paw_application::CreatorDraftId::new(draft_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        let decision =
            paw_application::ReviewDecision::parse(decision).ok_or(CommandError::Unexpected)?;
        let review = CreatorDraftReview::new(
            paw_application::CreatorDraftReviewId::new_v7(),
            workspace_id.clone(),
            id,
            version_no,
            decision,
            comment,
        )
        .map_err(CommandError::from)?;
        use paw_application::ports::creator_repository::CreatorDraftRepository as _;
        self.creator_drafts
            .insert_review(&review)
            .map_err(CommandError::from)?;
        Ok(review)
    }

    pub fn list_creator_drafts(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<CreatorDraft>, CommandError> {
        use paw_application::ports::creator_repository::CreatorDraftRepository as _;
        self.creator_drafts
            .list_drafts(workspace_id, None)
            .map_err(CommandError::from)
    }

    pub fn list_creator_reviews(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &str,
    ) -> Result<Vec<CreatorDraftReview>, CommandError> {
        let id = paw_application::CreatorDraftId::new(draft_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::creator_repository::CreatorDraftRepository as _;
        self.creator_drafts
            .list_reviews(workspace_id, &id)
            .map_err(CommandError::from)
    }

    // -- Agent governance (E5, PAW-INF-005/006) ----------------------------------

    fn coordinator(
        &self,
    ) -> paw_application::use_cases::agent_invocation::AgentInvocationCoordinator<
        '_,
        paw_infrastructure::operations::SqliteOperationService,
        SqliteAgentRegistrationRepository,
        SqliteAgentExecutionRepository,
        SqliteApprovalRepository,
        GovernedAuthorizer<SqliteAgentGrantRepository>,
        paw_infrastructure::agent_adapter::FakeAgentAdapter,
    > {
        paw_application::use_cases::agent_invocation::AgentInvocationCoordinator::new(
            &self.operations_service,
            &self.agent_registrations,
            &self.agent_executions,
            &self.approvals,
            &self.authorizer,
            &self.fake_adapter,
            &self.registry,
        )
    }

    fn now_ms(&self) -> i64 {
        self.coordination
            .query_row(
                "SELECT CAST(strftime('%s','now') AS INTEGER) * 1000",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or_default()
    }

    /// Register an agent for governed invocation (human-only action).
    pub fn register_agent(
        &self,
        workspace_id: &WorkspaceId,
        display_name: &str,
        adapter_kind: &str,
    ) -> Result<AgentRegistration, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::RegisterAgent {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "agent_registration",
            None,
        )?;
        let kind = paw_application::AdapterKind::parse(adapter_kind)
            .unwrap_or(paw_application::AdapterKind::Fake);
        let registration = AgentRegistration::new(
            paw_application::AgentRegistrationId::new_v7(),
            workspace_id.clone(),
            display_name,
            kind,
            // Declared capabilities start empty; grants — not declarations —
            // authorize (ADR-027).
            Vec::new(),
        )
        .map_err(|_| CommandError::EmptyName)?;
        use paw_application::ports::agent_repository::AgentRegistrationRepository as _;
        self.agent_registrations
            .insert_registration(&registration)
            .map_err(CommandError::from)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: "agent.register",
                resource_type: "agent_registration",
                resource_id: Some(registration.id.as_str().to_string()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(registration)
    }

    /// Suspend/retire/reactivate a registration (human-only action).
    pub fn update_agent_registration(
        &self,
        workspace_id: &WorkspaceId,
        registration_id: &str,
        new_status: &str,
    ) -> Result<AgentRegistration, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::UpdateAgentRegistration {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "agent_registration",
            Some(registration_id),
        )?;
        let id = paw_application::AgentRegistrationId::new(registration_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::agent_repository::AgentRegistrationRepository as _;
        let mut registration = self
            .agent_registrations
            .get_registration(&id, workspace_id)
            .map_err(CommandError::from)?;
        let status = paw_application::RegistrationStatus::parse(new_status)
            .ok_or(CommandError::Unexpected)?;
        if !registration.status.can_transition_to(status) {
            return Err(CommandError::TaskStatusInvalid);
        }
        registration.status = status;
        self.agent_registrations
            .update_registration_status(&registration)
            .map_err(CommandError::from)?;
        Ok(registration)
    }

    pub fn list_agents(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<AgentRegistration>, CommandError> {
        use paw_application::ports::agent_repository::AgentRegistrationRepository as _;
        self.agent_registrations
            .list_registrations(workspace_id)
            .map_err(CommandError::from)
    }

    /// Run the governed invocation pipeline (step 1–7). The proposing
    /// actor is the local human; the invocation itself runs as the agent
    /// subject against its grants (ADR-029 §4.1).
    pub fn propose_agent_invocation(
        &self,
        workspace_id: &WorkspaceId,
        registration_id: &str,
        capability_id: &str,
        contract_version: &str,
        parameters_json: &str,
    ) -> Result<AgentProposalOutcome, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::ProposeAgentInvocation {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "agent_execution",
            None,
        )?;
        let id = paw_application::AgentRegistrationId::new(registration_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        let version = parse_contract_version(contract_version)?;
        let parameters: BTreeMap<String, paw_application::ports::canonical_plan::PlanValue> =
            serde_json::from_str(parameters_json).unwrap_or_default();
        let proposal = paw_application::use_cases::agent_invocation::AgentProposal {
            workspace_id: workspace_id.clone(),
            registration_id: id,
            capability_id: capability_id.to_string(),
            capability_contract_version: version,
            parameters,
            correlation_id: ctx.correlation_id.clone(),
            approval_ttl_ms: 15 * 60 * 1000,
            attempt_policy: paw_application::AttemptPolicy::Single,
        };
        let outcome = self
            .coordinator()
            .propose(&proposal, ctx.actor_id.as_str(), self.now_ms())
            .map_err(CommandError::from)?;
        Ok(match outcome {
            paw_application::use_cases::agent_invocation::ProposalOutcome::Completed {
                execution,
                operation,
                summary,
            } => AgentProposalOutcome {
                status: AgentProposalStatus::Completed,
                execution,
                operation: Some(OperationEnvelopeDto {
                    id: operation.id,
                    status: operation.status.as_str().to_string(),
                    correlation_id: operation.correlation_id,
                }),
                display: None,
                summary: Some(summary),
            },
            paw_application::use_cases::agent_invocation::ProposalOutcome::AwaitingApproval {
                execution,
                display,
            } => AgentProposalOutcome {
                status: AgentProposalStatus::AwaitingApproval,
                execution,
                operation: None,
                display: Some(ApprovalDisplayDto {
                    execution_id: display.execution_id,
                    capability_id: display.capability_id,
                    capability_contract_version: display.capability_contract_version,
                    tool_adapter: display.tool_adapter,
                    target_identity: display.target_identity,
                    side_effect_set: display.side_effect_set,
                    risk_class: display.risk_class,
                    expires_at_ms: display.expires_at_ms,
                    attempt_policy: format!("{:?}", display.attempt_policy).to_lowercase(),
                }),
                summary: None,
            },
        })
    }

    /// Human decision on a pending approval (approve runs the parked
    /// execution; deny revokes the approval and cancels).
    pub fn decide_approval(
        &self,
        workspace_id: &WorkspaceId,
        execution_id: &str,
        approve: bool,
    ) -> Result<Option<AgentRunOutcomeDto>, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::DecideApproval {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "agent_execution",
            Some(execution_id),
        )?;
        let id = paw_application::AgentExecutionId::new(execution_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        let decision = if approve {
            paw_application::use_cases::agent_invocation::ApprovalDecision::Approve
        } else {
            paw_application::use_cases::agent_invocation::ApprovalDecision::Deny
        };
        let outcome = self
            .coordinator()
            .decide_approval(workspace_id, &id, decision, self.now_ms())
            .map_err(CommandError::from)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: "approval.decide",
                resource_type: "agent_execution",
                resource_id: Some(execution_id.to_string()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(outcome.map(AgentRunOutcomeDto::from))
    }

    /// Revoke an active approval (human-only).
    pub fn revoke_approval(
        &self,
        workspace_id: &WorkspaceId,
        approval_id: &str,
    ) -> Result<(), CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::RevokeApproval {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "approval",
            Some(approval_id),
        )?;
        let id = paw_application::ApprovalId::new(approval_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        use paw_application::ports::agent_repository::ApprovalRepository as _;
        let mut approval = self
            .approvals
            .get_approval(&id, workspace_id)
            .map_err(CommandError::from)?;
        approval
            .revoke()
            .map_err(|_| CommandError::ApprovalNotActive)?;
        self.approvals
            .update_approval(&approval)
            .map_err(CommandError::from)?;
        Ok(())
    }

    /// Owner reconciliation of an unresolved execution (ADR-019).
    pub fn reconcile_agent_execution(
        &self,
        workspace_id: &WorkspaceId,
        execution_id: &str,
        complete: bool,
    ) -> Result<AgentExecution, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::ReconcileAgentExecution {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "agent_execution",
            Some(execution_id),
        )?;
        let id = paw_application::AgentExecutionId::new(execution_id)
            .map_err(|_| CommandError::InvalidWorkspaceId)?;
        let resolution = if complete {
            paw_application::use_cases::agent_invocation::ReconcileResolution::Complete
        } else {
            paw_application::use_cases::agent_invocation::ReconcileResolution::Fail
        };
        self.coordinator()
            .reconcile(workspace_id, &id, resolution, &ctx)
            .map_err(CommandError::from)
    }

    pub fn list_agent_executions(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<AgentExecution>, CommandError> {
        use paw_application::ports::agent_repository::AgentExecutionRepository as _;
        self.agent_executions
            .list_executions(workspace_id, limit)
            .map_err(CommandError::from)
    }

    pub fn list_pending_approvals(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Approval>, CommandError> {
        use paw_application::ports::agent_repository::ApprovalRepository as _;
        self.approvals
            .list_pending_approvals(workspace_id, self.now_ms())
            .map_err(CommandError::from)
    }

    /// Create an agent grant (human-only; Admin is never grantable to
    /// agents — authorization model §2).
    pub fn create_agent_grant(
        &self,
        workspace_id: &WorkspaceId,
        subject_id: &str,
        capabilities: Vec<String>,
        ttl_ms: Option<i64>,
    ) -> Result<String, CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::CreateAgentGrant {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "agent_grant",
            None,
        )?;
        let now = self.now_ms();
        let grant = paw_application::ports::authorization::AgentGrant {
            id: uuid::Uuid::now_v7().to_string(),
            subject_id: subject_id.to_string(),
            capabilities,
            workspace_id: workspace_id.clone(),
            tools: Vec::new(),
            valid_from_ms: now,
            valid_until_ms: ttl_ms.map(|ttl| now + ttl),
        };
        self.agent_grants
            .insert_grant(&grant, ctx.actor_id.as_str())
            .map_err(CommandError::from)?;
        Ok(grant.id)
    }

    pub fn revoke_agent_grant(
        &self,
        workspace_id: &WorkspaceId,
        grant_id: &str,
    ) -> Result<(), CommandError> {
        let ctx = self.context();
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &paw_application::ports::authorization::ProtectedAction::RevokeAgentGrant {
                workspace_id: workspace_id.clone(),
            },
            Some(workspace_id),
            "agent_grant",
            Some(grant_id),
        )?;
        self.agent_grants
            .delete_grant(grant_id, workspace_id)
            .map_err(CommandError::from)
    }

    pub fn list_agent_grants(
        &self,
        workspace_id: &WorkspaceId,
        subject_id: Option<&str>,
    ) -> Result<Vec<paw_application::ports::authorization::AgentGrant>, CommandError> {
        self.agent_grants
            .list_grants(workspace_id, subject_id)
            .map_err(CommandError::from)
    }
}

// ---------------------------------------------------------------------------
// Tauri command bridge (E1 Tauri-shell unit; E2 command surface extension)
// ---------------------------------------------------------------------------

/// Tauri-safe error crossing the WebView boundary (Gate C — safe errors).
///
/// The frontend receives a stable error code only; driver messages, file
/// system paths, checksums and stack details never cross the boundary.
/// Variants are part of the published UI contract: add new variants only
/// with an explicit contract decision (E2 added the per-entity not-found
/// codes and `task_status_invalid` for the state-machine rule).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandError {
    /// A required name was empty or whitespace-only after trimming.
    EmptyName,
    /// A name or title exceeded its domain limit.
    NameTooLong,
    /// Free-form notes exceeded their limit.
    NotesTooLong,
    /// The referenced workspace does not exist.
    WorkspaceNotFound,
    /// The referenced project does not exist (or is invisible to the
    /// workspace scope).
    ProjectNotFound,
    /// The referenced area does not exist.
    AreaNotFound,
    /// The referenced goal does not exist.
    GoalNotFound,
    /// The referenced task does not exist.
    TaskNotFound,
    /// The workspace identifier was not a valid UUID.
    InvalidWorkspaceId,
    /// A task status change refused by the domain state machine.
    TaskStatusInvalid,
    /// The search query was empty or carried no searchable terms (E2-03).
    InvalidSearchQuery,
    /// The referenced attachment does not exist (E2-04).
    AttachmentNotFound,
    /// The attachment's owner record does not exist (E2-04).
    OwnerNotFound,
    /// The referenced operation does not exist (E3-04).
    OperationNotFound,
    /// The referenced learning record does not exist (E4).
    LearningRecordNotFound,
    /// The referenced learning item does not exist (E6 gap confirmation).
    ItemNotFound,
    /// The referenced knowledge reference does not exist (E4).
    KnowledgeRefNotFound,
    /// The referenced agent registration does not exist (E5).
    AgentRegistrationNotFound,
    /// The referenced agent execution does not exist (E5).
    AgentExecutionNotFound,
    /// The referenced approval does not exist (E5).
    ApprovalNotFound,
    /// No active approval exists for this execution (E5).
    ApprovalNotActive,
    /// The capability id or contract version is not published (E5).
    CapabilityUnsupported,
    /// The referenced skill does not exist (E6).
    SkillNotFound,
    /// The referenced creator record does not exist (E6).
    CreatorNotFound,
    /// A creator state transition was refused by the domain machine (E6).
    CreatorStateInvalid,
    /// Fresh authorization refused the protected side effect (E1-09).
    Denied,
    /// Any other failure; details stay on the Rust side.
    Unexpected,
}

impl From<WorkManagementError> for CommandError {
    fn from(err: WorkManagementError) -> Self {
        match err {
            WorkManagementError::Validation(ValidationError::EmptyName) => Self::EmptyName,
            WorkManagementError::Validation(ValidationError::NameTooLong { .. }) => {
                Self::NameTooLong
            }
            WorkManagementError::Validation(ValidationError::NotesTooLong { .. }) => {
                Self::NotesTooLong
            }
            WorkManagementError::Repository(WorkspaceRepositoryError::WorkspaceNotFound) => {
                Self::WorkspaceNotFound
            }
            WorkManagementError::Repository(WorkspaceRepositoryError::ProjectNotFound) => {
                Self::ProjectNotFound
            }
            WorkManagementError::Area(
                paw_application::ports::area_repository::AreaRepositoryError::AreaNotFound,
            ) => Self::AreaNotFound,
            WorkManagementError::Goal(
                paw_application::ports::goal_repository::GoalRepositoryError::GoalNotFound,
            ) => Self::GoalNotFound,
            WorkManagementError::Task(
                paw_application::ports::task_repository::TaskRepositoryError::TaskNotFound,
            ) => Self::TaskNotFound,
            WorkManagementError::Authorization(_) => Self::Denied,
            WorkManagementError::TaskTransition { .. } => Self::TaskStatusInvalid,
            WorkManagementError::Attachment(
                paw_application::ports::attachment_repository::AttachmentRepositoryError::AttachmentNotFound,
            ) => Self::AttachmentNotFound,
            WorkManagementError::Attachment(
                paw_application::ports::attachment_repository::AttachmentRepositoryError::OwnerNotFound,
            ) => Self::OwnerNotFound,
            // Invariant violations and repository-level invariant failures
            // stay generic on the boundary.
            _ => Self::Unexpected,
        }
    }
}

impl From<paw_application::use_cases::learning::LearningError> for CommandError {
    fn from(err: paw_application::use_cases::learning::LearningError) -> Self {
        use paw_application::use_cases::learning::LearningError as E;
        match err {
            // Typed not-found codes, opaque invariant failures.
            E::Repository(e) => Self::from_learning(e),
            // Domain validation keeps its existing boundary codes.
            E::Validation(v) => Self::from(WorkManagementError::Validation(v)),
            E::Authorization(_) => Self::Denied,
            // The learning item machine mirrors the task machine; the
            // published UI code stays `task_status_invalid`.
            E::ItemTransition { .. } => Self::TaskStatusInvalid,
            // Delegated Work Management failures (PAW-LG-006 task creation)
            // map exactly as a direct Work Management error.
            E::WorkManagement(e) => Self::from(e),
        }
    }
}

impl From<paw_application::use_cases::knowledge::KnowledgeError> for CommandError {
    fn from(err: paw_application::use_cases::knowledge::KnowledgeError) -> Self {
        use paw_application::use_cases::knowledge::KnowledgeError as E;
        match err {
            E::Knowledge(e) => Self::from_knowledge(e),
            E::Validation(v) => Self::from(WorkManagementError::Validation(v)),
            E::Authorization(_) => Self::Denied,
        }
    }
}

impl From<paw_application::ParseError> for CommandError {
    fn from(_: paw_application::ParseError) -> Self {
        Self::InvalidWorkspaceId
    }
}

/// Map an infrastructure OperationError to the safe boundary code.
impl From<paw_application::ports::learning_repository::LearningRepositoryError> for CommandError {
    fn from(e: paw_application::ports::learning_repository::LearningRepositoryError) -> Self {
        use paw_application::ports::learning_repository::LearningRepositoryError as E;
        match e {
            E::RoadmapNotFound | E::PlanNotFound | E::ItemNotFound | E::SessionNotFound => {
                Self::LearningRecordNotFound
            }
            E::Invariant(_) => Self::Unexpected,
        }
    }
}

fn map_operation_error(e: paw_infrastructure::operations::OperationError) -> CommandError {
    match e {
        paw_infrastructure::operations::OperationError::NotFound(_) => {
            CommandError::OperationNotFound
        }
        paw_infrastructure::operations::OperationError::InvalidTransition { .. } => {
            CommandError::TaskStatusInvalid
        }
        _ => CommandError::Unexpected,
    }
}

fn map_classification_error(
    e: paw_infrastructure::operations::ClassificationError,
) -> CommandError {
    match e {
        paw_infrastructure::operations::ClassificationError::NotFound(_) => {
            CommandError::OperationNotFound
        }
        paw_infrastructure::operations::ClassificationError::InvalidTransition { .. } => {
            CommandError::TaskStatusInvalid
        }
        _ => CommandError::Unexpected,
    }
}

/// E5 pipeline failures map to typed, non-leaky boundary codes.
impl From<paw_application::use_cases::agent_invocation::AgentInvocationError> for CommandError {
    fn from(e: paw_application::use_cases::agent_invocation::AgentInvocationError) -> Self {
        use paw_application::use_cases::agent_invocation::AgentInvocationError as E;
        match e {
            E::RegistrationNotFound => Self::AgentRegistrationNotFound,
            E::RegistrationNotActive { .. } => Self::AgentRegistrationNotFound,
            E::CapabilityUnsupported => Self::CapabilityUnsupported,
            E::Denied => Self::Denied,
            E::ExecutionNotFound => Self::AgentExecutionNotFound,
            E::ExecutionState { .. } => Self::TaskStatusInvalid,
            E::ApprovalNotActive => Self::ApprovalNotActive,
            E::Governance(_) | E::Store(_) | E::Adapter(_) | E::Operation(_) => Self::Unexpected,
        }
    }
}

impl From<AgentStoreError> for CommandError {
    fn from(e: AgentStoreError) -> Self {
        match e {
            AgentStoreError::NotFound => Self::AgentRegistrationNotFound,
            AgentStoreError::WorkspaceMismatch => Self::AgentRegistrationNotFound,
            AgentStoreError::Invariant(_) | AgentStoreError::Storage(_) => Self::Unexpected,
        }
    }
}

impl From<paw_application::ports::skill_repository::SkillStoreError> for CommandError {
    fn from(e: paw_application::ports::skill_repository::SkillStoreError) -> Self {
        use paw_application::ports::skill_repository::SkillStoreError as E;
        match e {
            E::NotFound => Self::SkillNotFound,
            E::WorkspaceMismatch => Self::SkillNotFound,
            E::Invariant(_) | E::Storage(_) => Self::Unexpected,
        }
    }
}

impl From<paw_application::ports::creator_repository::CreatorStoreError> for CommandError {
    fn from(e: paw_application::ports::creator_repository::CreatorStoreError) -> Self {
        use paw_application::ports::creator_repository::CreatorStoreError as E;
        match e {
            E::NotFound => Self::CreatorNotFound,
            E::WorkspaceMismatch => Self::CreatorNotFound,
            E::Invariant(_) | E::Storage(_) => Self::Unexpected,
        }
    }
}

impl From<paw_application::SkillsError> for CommandError {
    fn from(e: paw_application::SkillsError) -> Self {
        use paw_application::SkillsError as E;
        match e {
            E::EmptyName => Self::EmptyName,
            E::NameTooLong { .. } | E::TextTooLong { .. } => Self::NameTooLong,
            E::AssessmentRequiresEvidence => Self::Unexpected,
            E::InvalidLevel { .. } => Self::Unexpected,
        }
    }
}

impl From<paw_application::CreatorError> for CommandError {
    fn from(e: paw_application::CreatorError) -> Self {
        use paw_application::CreatorError as E;
        match e {
            E::EmptyTitle => Self::EmptyName,
            E::TooLong { .. } => Self::NameTooLong,
            E::InvalidTransition { .. } | E::VersionNotFound => Self::CreatorStateInvalid,
        }
    }
}

/// Parse a `major.minor.patch` contract version string.
fn parse_contract_version(value: &str) -> Result<(u32, u32, u32), CommandError> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 3 {
        return Err(CommandError::CapabilityUnsupported);
    }
    let major: u32 = parts[0]
        .parse()
        .map_err(|_| CommandError::CapabilityUnsupported)?;
    let minor: u32 = parts[1]
        .parse()
        .map_err(|_| CommandError::CapabilityUnsupported)?;
    let patch: u32 = parts[2]
        .parse()
        .map_err(|_| CommandError::CapabilityUnsupported)?;
    if major == 0 {
        return Err(CommandError::CapabilityUnsupported);
    }
    Ok((major, minor, patch))
}

/// Serialized pipeline outcomes crossing the Tauri boundary (E5-06).
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProposalStatus {
    Completed,
    AwaitingApproval,
}

#[derive(Debug, serde::Serialize)]
pub struct OperationEnvelopeDto {
    pub id: String,
    pub status: String,
    pub correlation_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ApprovalDisplayDto {
    pub execution_id: String,
    pub capability_id: String,
    pub capability_contract_version: String,
    pub tool_adapter: String,
    pub target_identity: String,
    pub side_effect_set: Vec<String>,
    pub risk_class: String,
    pub expires_at_ms: i64,
    pub attempt_policy: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AgentProposalOutcome {
    pub status: AgentProposalStatus,
    pub execution: AgentExecution,
    pub operation: Option<OperationEnvelopeDto>,
    pub display: Option<ApprovalDisplayDto>,
    pub summary: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Completed,
    Failed,
    Unresolved,
}

#[derive(Debug, serde::Serialize)]
pub struct AgentRunOutcomeDto {
    pub status: AgentRunStatus,
    pub execution: AgentExecution,
    pub operation: OperationEnvelopeDto,
    pub summary: String,
}

impl From<paw_application::use_cases::agent_invocation::RunOutcome> for AgentRunOutcomeDto {
    fn from(outcome: paw_application::use_cases::agent_invocation::RunOutcome) -> Self {
        use paw_application::use_cases::agent_invocation::RunOutcome as O;
        match outcome {
            O::Completed {
                execution,
                operation,
                summary,
            } => Self {
                status: AgentRunStatus::Completed,
                execution,
                operation: OperationEnvelopeDto {
                    id: operation.id,
                    status: operation.status.as_str().to_string(),
                    correlation_id: operation.correlation_id,
                },
                summary,
            },
            O::Failed {
                execution,
                operation,
                reason,
            } => Self {
                status: AgentRunStatus::Failed,
                execution,
                operation: OperationEnvelopeDto {
                    id: operation.id,
                    status: operation.status.as_str().to_string(),
                    correlation_id: operation.correlation_id,
                },
                summary: reason,
            },
            O::Unresolved {
                execution,
                operation,
            } => Self {
                status: AgentRunStatus::Unresolved,
                execution,
                operation: OperationEnvelopeDto {
                    id: operation.id,
                    status: operation.status.as_str().to_string(),
                    correlation_id: operation.correlation_id,
                },
                summary: "unresolved: reconciliation required".to_string(),
            },
        }
    }
}

/// Desktop-side gate: authorize `action`; on denial record the audited
/// refusal and surface `denied`. Used by the E3 flows that operate directly
/// on the coordination services instead of through application use cases.
#[allow(clippy::too_many_arguments)]
fn check_gate(
    authorizer: &impl paw_application::ports::authorization::Authorizer,
    audit: &SqliteAuditSink,
    ctx: &ExecutionContext,
    action: &paw_application::ports::authorization::ProtectedAction,
    workspace_id: Option<&WorkspaceId>,
    resource_type: &'static str,
    resource_id: Option<&str>,
) -> Result<(), CommandError> {
    if authorizer.authorize(ctx, action).is_err() {
        audit.record(&paw_application::ports::audit::AuditEvent {
            workspace_id: workspace_id.cloned(),
            actor_id: ctx.actor_id.clone(),
            action: action.action_key(),
            resource_type,
            resource_id: resource_id.map(str::to_string),
            outcome: paw_application::ports::audit::AuditOutcome::Denied,
            correlation_id: ctx.correlation_id.clone(),
        });
        return Err(CommandError::Denied);
    }
    Ok(())
}

impl CommandError {
    /// Storage-level IO failures surface as the opaque `unexpected` code.
    fn from_io(e: std::io::Error) -> Self {
        let _ = e;
        Self::Unexpected
    }

    /// Learning record failures: typed not-found codes, else opaque.
    fn from_learning(
        e: paw_application::ports::learning_repository::LearningRepositoryError,
    ) -> Self {
        use paw_application::ports::learning_repository::LearningRepositoryError as E;
        match e {
            E::RoadmapNotFound | E::PlanNotFound | E::ItemNotFound | E::SessionNotFound => {
                Self::LearningRecordNotFound
            }
            E::Invariant(_) => Self::Unexpected,
        }
    }

    /// Knowledge reference failures: typed not-found code, else opaque.
    fn from_knowledge(
        e: paw_application::ports::knowledge_repository::KnowledgeRepositoryError,
    ) -> Self {
        use paw_application::ports::knowledge_repository::KnowledgeRepositoryError as E;
        match e {
            E::KnowledgeRefNotFound => Self::KnowledgeRefNotFound,
            E::Invariant(_) => Self::Unexpected,
        }
    }
}

impl From<paw_application::ports::search::SearchError> for CommandError {
    fn from(err: paw_application::ports::search::SearchError) -> Self {
        match err {
            paw_application::ports::search::SearchError::InvalidQuery => Self::InvalidSearchQuery,
            // Storage failures stay opaque on the boundary.
            paw_application::ports::search::SearchError::Invariant(_) => Self::Unexpected,
        }
    }
}

/// `#[tauri::command]` wrappers: the attribute layer only moves data across
/// the WebView boundary and delegates to `WorkManagementApi`. JS arguments
/// use camelCase keys (`workspaceId`) and are mapped to the snake_case Rust
/// parameters by the Tauri command machinery.
///
/// The API is wrapped in a `Mutex` because a rusqlite connection is not
/// `Sync` (SQLite serializes access itself, but Rust still requires a lock
/// to share the handles across the WebView message-loop threads).
#[tauri::command]
pub fn list_workspaces(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
) -> Result<Vec<Workspace>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_workspaces()?)
}

#[tauri::command]
pub fn create_workspace(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    name: String,
) -> Result<Workspace, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_workspace(&name)?)
}

#[tauri::command]
pub fn create_project(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    name: String,
) -> Result<Project, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_project(&workspace_id, &name)?)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_project(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: ProjectId,
    name: String,
    description: Option<String>,
    status: ProjectStatus,
    goal_id: Option<GoalId>,
    area_id: Option<AreaId>,
) -> Result<Project, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_project(
        &workspace_id,
        &project_id,
        &name,
        description.as_deref(),
        status,
        goal_id,
        area_id,
    )?)
}

#[tauri::command]
pub fn delete_project(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: ProjectId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_project(&workspace_id, &project_id)?)
}

#[tauri::command]
pub fn list_projects(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<Project>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_projects(&workspace_id)?)
}

#[tauri::command]
pub fn create_area(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    name: String,
    description: Option<String>,
) -> Result<Area, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_area(&workspace_id, &name, description.as_deref())?)
}

#[tauri::command]
pub fn update_area(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: AreaId,
    name: String,
    description: Option<String>,
) -> Result<Area, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_area(&workspace_id, &area_id, &name, description.as_deref())?)
}

#[tauri::command]
pub fn delete_area(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: AreaId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_area(&workspace_id, &area_id)?)
}

#[tauri::command]
pub fn list_areas(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<Area>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_areas(&workspace_id)?)
}

#[tauri::command]
pub fn create_goal(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: Option<AreaId>,
    name: String,
    description: Option<String>,
) -> Result<Goal, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_goal(&workspace_id, area_id, &name, description.as_deref())?)
}

#[tauri::command]
pub fn update_goal(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    goal_id: GoalId,
    area_id: Option<AreaId>,
    name: String,
    description: Option<String>,
) -> Result<Goal, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_goal(
        &workspace_id,
        &goal_id,
        area_id,
        &name,
        description.as_deref(),
    )?)
}

#[tauri::command]
pub fn delete_goal(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    goal_id: GoalId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_goal(&workspace_id, &goal_id)?)
}

#[tauri::command]
pub fn list_goals(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: Option<AreaId>,
) -> Result<Vec<Goal>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_goals(&workspace_id, area_id.as_ref())?)
}

#[tauri::command]
pub fn create_task(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: Option<ProjectId>,
    title: String,
    notes: Option<String>,
) -> Result<Task, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_task(&workspace_id, project_id, &title, notes.as_deref())?)
}

#[tauri::command]
pub fn update_task_details(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    task_id: TaskId,
    title: String,
    notes: Option<String>,
    project_id: Option<ProjectId>,
) -> Result<Task, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_task_details(
        &workspace_id,
        &task_id,
        &title,
        notes.as_deref(),
        project_id,
    )?)
}

#[tauri::command]
pub fn transition_task(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    task_id: TaskId,
    to: TaskStatus,
) -> Result<Task, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.transition_task(&workspace_id, &task_id, to)?)
}

#[tauri::command]
pub fn delete_task(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    task_id: TaskId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_task(&workspace_id, &task_id)?)
}

#[tauri::command]
pub fn list_tasks(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: Option<ProjectId>,
    status: Option<TaskStatus>,
) -> Result<Vec<Task>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_tasks(&workspace_id, project_id.as_ref(), status)?)
}

#[tauri::command]
pub fn search(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<paw_application::ports::search::SearchHit>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.search(&workspace_id, &query, limit.unwrap_or(20))
}

#[tauri::command]
pub fn dashboard(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<paw_application::ports::dashboard::DashboardSummary, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.dashboard(&workspace_id)
}

// -- Learning & Growth (E4) ------------------------------------------------

#[tauri::command]
pub fn list_roadmaps(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<Roadmap>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_roadmaps(&workspace_id)
}

#[tauri::command]
pub fn create_roadmap(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    name: String,
    description: Option<String>,
) -> Result<Roadmap, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_roadmap(&workspace_id, &name, description.as_deref())
}

#[tauri::command]
pub fn delete_roadmap(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    roadmap_id: RoadmapId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.delete_roadmap(&workspace_id, &roadmap_id)
}

#[tauri::command]
pub fn list_plans(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    roadmap_id: Option<RoadmapId>,
) -> Result<Vec<Plan>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_plans(&workspace_id, roadmap_id.as_ref())
}

#[tauri::command]
pub fn create_plan(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    roadmap_id: RoadmapId,
    name: String,
    description: Option<String>,
) -> Result<Plan, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_plan(&workspace_id, &roadmap_id, &name, description.as_deref())
}

#[tauri::command]
pub fn update_plan_status(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    plan_id: PlanId,
    status: PlanStatus,
) -> Result<Plan, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.update_plan_status(&workspace_id, &plan_id, status)
}

#[tauri::command]
pub fn list_items(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    plan_id: Option<PlanId>,
) -> Result<Vec<Item>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_items(&workspace_id, plan_id.as_ref())
}

#[tauri::command]
pub fn create_item(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    plan_id: PlanId,
    kind: ItemKind,
    title: String,
    expected_outcome: Option<String>,
    acceptance_criteria: Option<String>,
) -> Result<Item, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_item(
        &workspace_id,
        &plan_id,
        kind,
        &title,
        expected_outcome.as_deref(),
        acceptance_criteria.as_deref(),
    )
}

#[tauri::command]
pub fn transition_item(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    item_id: ItemId,
    to: ItemStatus,
) -> Result<Item, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.transition_item(&workspace_id, &item_id, to)
}

#[tauri::command]
pub fn link_item_to_project(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    item_id: ItemId,
    project_id: ProjectId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.link_item_to_project(&workspace_id, &item_id, &project_id)
}

#[tauri::command]
pub fn create_task_for_item(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    item_id: ItemId,
    title: String,
) -> Result<Task, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_task_for_item(&workspace_id, &item_id, &title)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn append_session(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    item_id: ItemId,
    method: Option<String>,
    duration_minutes: Option<i64>,
    summary: Option<String>,
    difficulty: Option<String>,
    takeaway: Option<String>,
    next_step: Option<String>,
) -> Result<Session, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.append_session(
        &workspace_id,
        &item_id,
        method.as_deref(),
        duration_minutes,
        summary.as_deref(),
        difficulty.as_deref(),
        takeaway.as_deref(),
        next_step.as_deref(),
    )
}

#[tauri::command]
pub fn list_sessions(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    item_id: Option<ItemId>,
) -> Result<Vec<Session>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_sessions(&workspace_id, item_id.as_ref())
}

// -- Knowledge & Context (E4) ------------------------------------------------

#[tauri::command]
pub fn list_knowledge_refs(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    availability: Option<paw_application::Availability>,
) -> Result<Vec<paw_application::KnowledgeRef>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_knowledge_refs(&workspace_id, availability)
}

#[tauri::command]
pub fn create_knowledge_ref(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    title: String,
    source: paw_application::KnowledgeSource,
    external_uri: Option<String>,
    locator: Option<String>,
    version_label: Option<String>,
) -> Result<paw_application::KnowledgeRef, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_knowledge_ref(
        &workspace_id,
        &title,
        source,
        external_uri.as_deref(),
        locator.as_deref(),
        version_label.as_deref(),
    )
}

#[tauri::command]
pub fn set_knowledge_availability(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    knowledge_ref_id: paw_application::KnowledgeRefId,
    availability: paw_application::Availability,
) -> Result<paw_application::KnowledgeRef, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.set_knowledge_availability(&workspace_id, &knowledge_ref_id, availability)
}

#[tauri::command]
pub fn delete_knowledge_ref(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    knowledge_ref_id: paw_application::KnowledgeRefId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.delete_knowledge_ref(&workspace_id, &knowledge_ref_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_attachment(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    owner_type: String,
    owner_id: String,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<paw_application::ports::attachment_repository::Attachment, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    let owner = paw_application::ports::attachment_repository::AttachmentOwner::parse(&owner_type)
        .ok_or(CommandError::Unexpected)?;
    Ok(api.add_attachment(&workspace_id, owner, &owner_id, &file_name, &bytes)?)
}

#[tauri::command]
pub fn list_attachments(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    owner_type: String,
    owner_id: String,
) -> Result<Vec<paw_application::ports::attachment_repository::Attachment>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    let owner = paw_application::ports::attachment_repository::AttachmentOwner::parse(&owner_type)
        .ok_or(CommandError::Unexpected)?;
    Ok(api.list_attachments(&workspace_id, owner, &owner_id)?)
}

#[tauri::command]
pub fn read_attachment(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<paw_application::ports::attachment_repository::Attachment, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    // Metadata returns as the command result; bytes are delivered through
    // the read_attachment_bytes command so a huge payload never serializes
    // together with the metadata.
    let (attachment, _bytes) = api.read_attachment(&workspace_id, &attachment_id)?;
    Ok(attachment)
}

#[tauri::command]
pub fn read_attachment_bytes(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<Vec<u8>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    let (_attachment, bytes) = api.read_attachment(&workspace_id, &attachment_id)?;
    Ok(bytes)
}

/// Export the attachment to a temp file and open it with the OS default
/// application (WebView2 blocks programmatic blob downloads; E2-04 UI fix).
#[tauri::command]
pub fn open_attachment_externally(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<String, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.open_attachment_externally(&workspace_id, &attachment_id)?)
}

#[tauri::command]
pub fn remove_attachment(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.remove_attachment(&workspace_id, &attachment_id)?)
}

// -- Operation queue (E3-04) ------------------------------------------------

#[tauri::command]
pub fn list_operations(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<paw_infrastructure::operations::OperationRow>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_operations(&workspace_id)
}

#[tauri::command]
pub fn request_operation_cancellation(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    operation_id: String,
) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.request_operation_cancellation(&workspace_id, &operation_id)
}

#[tauri::command]
pub fn resume_operation(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    operation_id: String,
    note: String,
) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.resume_operation(&workspace_id, &operation_id, &note)
}

#[tauri::command]
pub fn list_quarantined_messages(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<paw_infrastructure::outbox::QuarantinedMessage>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_quarantined(&workspace_id)
}

#[tauri::command]
pub fn redrive_message(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    message_id: String,
) -> Result<String, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.redrive_message(&workspace_id, &message_id)
}

#[tauri::command]
pub fn export_operations(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<String, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.export_operations_to_temp(&workspace_id)
}

#[tauri::command]
pub fn purge_expired_data(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
) -> Result<(usize, usize, usize), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.purge_expired()
}

// -- Agent governance (E5-06) ------------------------------------------------

#[tauri::command]
pub fn register_agent(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    display_name: String,
    adapter_kind: String,
) -> Result<AgentRegistration, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.register_agent(&workspace_id, &display_name, &adapter_kind)
}

#[tauri::command]
pub fn update_agent_registration(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    registration_id: String,
    new_status: String,
) -> Result<AgentRegistration, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.update_agent_registration(&workspace_id, &registration_id, &new_status)
}

#[tauri::command]
pub fn list_agents(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<AgentRegistration>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_agents(&workspace_id)
}

#[tauri::command]
pub fn propose_agent_invocation(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    registration_id: String,
    capability_id: String,
    contract_version: String,
    parameters_json: String,
) -> Result<AgentProposalOutcome, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.propose_agent_invocation(
        &workspace_id,
        &registration_id,
        &capability_id,
        &contract_version,
        &parameters_json,
    )
}

#[tauri::command]
pub fn decide_approval(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    execution_id: String,
    approve: bool,
) -> Result<Option<AgentRunOutcomeDto>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.decide_approval(&workspace_id, &execution_id, approve)
}

#[tauri::command]
pub fn revoke_approval(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    approval_id: String,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.revoke_approval(&workspace_id, &approval_id)
}

#[tauri::command]
pub fn reconcile_agent_execution(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    execution_id: String,
    complete: bool,
) -> Result<AgentExecution, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.reconcile_agent_execution(&workspace_id, &execution_id, complete)
}

#[tauri::command]
pub fn list_agent_executions(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    limit: usize,
) -> Result<Vec<AgentExecution>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_agent_executions(&workspace_id, limit)
}

#[tauri::command]
pub fn list_pending_approvals(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<Approval>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_pending_approvals(&workspace_id)
}

#[tauri::command]
pub fn create_agent_grant(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    subject_id: String,
    capabilities: Vec<String>,
    ttl_ms: Option<i64>,
) -> Result<String, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_agent_grant(&workspace_id, &subject_id, capabilities, ttl_ms)
}

#[tauri::command]
pub fn revoke_agent_grant(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    grant_id: String,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.revoke_agent_grant(&workspace_id, &grant_id)
}

#[tauri::command]
pub fn list_agent_grants(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    subject_id: Option<String>,
) -> Result<Vec<paw_application::ports::authorization::AgentGrant>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_agent_grants(&workspace_id, subject_id.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_error_maps_typed_errors_to_safe_codes() {
        // Validation errors keep their dedicated codes.
        assert_eq!(
            CommandError::from(WorkManagementError::Validation(ValidationError::EmptyName)),
            CommandError::EmptyName
        );
        assert_eq!(
            CommandError::from(WorkManagementError::Validation(
                ValidationError::NameTooLong { max: 120 }
            )),
            CommandError::NameTooLong
        );
        // Not-found keeps its code; authorization refusals surface as
        // `denied` (E1-09 contract decision), never as a generic failure.
        assert_eq!(
            CommandError::from(WorkManagementError::Repository(
                WorkspaceRepositoryError::WorkspaceNotFound
            )),
            CommandError::WorkspaceNotFound
        );
        assert_eq!(
            CommandError::from(WorkManagementError::Repository(
                WorkspaceRepositoryError::ProjectNotFound
            )),
            CommandError::ProjectNotFound
        );
        assert_eq!(
            CommandError::from(WorkManagementError::Authorization(
                paw_application::ports::authorization::AuthorizationError::Denied
            )),
            CommandError::Denied
        );
        // The E2 state-machine rejection is a dedicated code.
        assert_eq!(
            CommandError::from(WorkManagementError::TaskTransition {
                from: TaskStatus::Done,
                to: TaskStatus::Todo,
            }),
            CommandError::TaskStatusInvalid
        );
    }

    #[test]
    fn command_error_never_carries_driver_text() {
        // An invariant failure (which wraps a driver message internally)
        // crosses the boundary as the opaque `unexpected` code only.
        let err = WorkManagementError::Repository(WorkspaceRepositoryError::Invariant(
            "UNIQUE constraint failed: workspaces.id (leaky detail)".into(),
        ));
        let code = CommandError::from(err);
        assert_eq!(code, CommandError::Unexpected);
        let serialized = serde_json::to_string(&code).unwrap();
        assert!(!serialized.contains("UNIQUE"));
        assert!(!serialized.contains("leaky"));
    }
}

// -- Skill matrix & learning gaps (E6) --------------------------------------

#[tauri::command]
pub fn create_skill(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    name: String,
    category: String,
    notes: Option<String>,
) -> Result<Skill, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_skill(&workspace_id, &name, &category, notes.as_deref())
}

#[tauri::command]
pub fn update_skill(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    skill: Skill,
) -> Result<Skill, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.update_skill(&skill)
}

#[tauri::command]
pub fn list_skills(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    category: Option<String>,
) -> Result<Vec<Skill>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_skills(&workspace_id, category.as_deref())
}

#[tauri::command]
pub fn add_skill_evidence(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    skill_id: String,
    kind: String,
    source_kind: String,
    source_id: String,
    summary: Option<String>,
) -> Result<SkillEvidence, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.add_skill_evidence(
        &workspace_id,
        &skill_id,
        &kind,
        &source_kind,
        &source_id,
        summary.as_deref(),
    )
}

#[tauri::command]
pub fn list_skill_evidence(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    skill_id: String,
) -> Result<Vec<SkillEvidence>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_skill_evidence(&workspace_id, &skill_id)
}

#[tauri::command]
pub fn create_assessment(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    skill_id: String,
    level: u8,
    confidence: String,
    rationale: Option<String>,
    evidence_ids: Vec<String>,
) -> Result<Assessment, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_assessment(
        &workspace_id,
        &skill_id,
        level,
        &confidence,
        rationale.as_deref(),
        &evidence_ids,
    )
}

#[tauri::command]
pub fn list_assessments(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    skill_id: String,
) -> Result<Vec<Assessment>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_assessments(&workspace_id, &skill_id)
}

#[tauri::command]
pub fn propose_learning_gap(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: String,
    gap: String,
) -> Result<LearningGapProposal, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.propose_learning_gap(&workspace_id, &project_id, &gap)
}

#[tauri::command]
pub fn confirm_learning_gap(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    gap_id: String,
    item_id: String,
) -> Result<LearningGapProposal, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.confirm_learning_gap(&workspace_id, &gap_id, &item_id)
}

#[tauri::command]
pub fn dismiss_learning_gap(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    gap_id: String,
) -> Result<LearningGapProposal, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.dismiss_learning_gap(&workspace_id, &gap_id)
}

#[tauri::command]
pub fn list_learning_gaps(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    status: Option<String>,
) -> Result<Vec<LearningGapProposal>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    let status = status
        .as_deref()
        .and_then(paw_application::GapProposalStatus::parse);
    api.list_learning_gaps(&workspace_id, status)
}

// -- Creator content pipeline (E6) -------------------------------------------

#[tauri::command]
pub fn create_creator_topic(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    title: String,
    notes: Option<String>,
) -> Result<CreatorTopic, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_creator_topic(&workspace_id, &title, notes.as_deref())
}

#[tauri::command]
pub fn update_creator_topic(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    topic: CreatorTopic,
) -> Result<CreatorTopic, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.update_creator_topic(&topic)
}

#[tauri::command]
pub fn list_creator_topics(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<CreatorTopic>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_creator_topics(&workspace_id)
}

#[tauri::command]
pub fn create_creator_draft(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    topic_id: String,
    title: String,
) -> Result<CreatorDraft, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.create_creator_draft(&workspace_id, &topic_id, &title)
}

#[tauri::command]
pub fn append_creator_draft_version(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    draft_id: String,
    body: String,
) -> Result<CreatorDraft, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.append_creator_draft_version(&workspace_id, &draft_id, &body)
}

#[tauri::command]
pub fn transition_creator_draft(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    draft_id: String,
    new_status: String,
) -> Result<CreatorDraft, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.transition_creator_draft(&workspace_id, &draft_id, &new_status)
}

#[tauri::command]
pub fn submit_creator_review(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    draft_id: String,
    version_no: u32,
    decision: String,
    comment: Option<String>,
) -> Result<CreatorDraftReview, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.submit_creator_review(
        &workspace_id,
        &draft_id,
        version_no,
        &decision,
        comment.as_deref(),
    )
}

#[tauri::command]
pub fn list_creator_drafts(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<CreatorDraft>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_creator_drafts(&workspace_id)
}

#[tauri::command]
pub fn list_creator_reviews(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    draft_id: String,
) -> Result<Vec<CreatorDraftReview>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_creator_reviews(&workspace_id, &draft_id)
}
