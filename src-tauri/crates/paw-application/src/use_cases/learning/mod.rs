//! Learning & Growth use cases (PAW-LG-001～006, E4): Roadmap, Plan, Item and
//! Session records over the learning repository ports.
//!
//! Every mutating use case follows the same E1-09 gate pattern as Work
//! Management: fresh authorization before the protected side effect, domain
//! validation before the repository, and an audit record for allowed, denied
//! and error outcomes. Reads are workspace-scoped by the repository ports and
//! are not authorization-gated (same policy as E2).
//!
//! Cross-domain orchestration lives here too (ADR-017: coordination stays in
//! paw-application). `item::request_task_for_item` holds the `ItemRepository`
//! plus the Work Management `TaskRepository`; the task itself is created by
//! the Work Management use case and stays Work-Management-owned
//! (PAW-LG-006), only the action-step link is Learning's.

pub mod item;
pub mod plan;
pub mod roadmap;
pub mod session;

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{
    AuthorizationError, Authorizer, ExecutionContext, ProtectedAction,
};
use crate::ports::learning_repository::LearningRepositoryError;
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::learning::ItemStatus;
use paw_domain::work_management::{ValidationError, WorkspaceId};

/// Typed failure of a Learning & Growth use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LearningError {
    /// Typed learning repository rejection (including cross-workspace
    /// invariant violations surfaced by the composite schema keys).
    Repository(LearningRepositoryError),
    /// Input failed a domain invariant before touching the repository.
    Validation(ValidationError),
    /// Fresh authorization was refused for the protected side effect.
    Authorization(AuthorizationError),
    /// An item status change that the domain state machine refuses.
    ItemTransition { from: ItemStatus, to: ItemStatus },
    /// A delegated Work Management operation failed (cross-domain
    /// orchestration, PAW-LG-006).
    WorkManagement(WorkManagementError),
}

impl std::fmt::Display for LearningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Repository(e) => write!(f, "{e}"),
            Self::Validation(e) => write!(f, "{e}"),
            Self::Authorization(e) => write!(f, "{e}"),
            Self::WorkManagement(e) => write!(f, "{e}"),
            Self::ItemTransition { from, to } => {
                write!(f, "cannot move a learning item from {from} to {to}")
            }
        }
    }
}

impl std::error::Error for LearningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Repository(e) => Some(e),
            Self::Validation(e) => Some(e),
            Self::Authorization(e) => Some(e),
            Self::WorkManagement(e) => Some(e),
            Self::ItemTransition { .. } => None,
        }
    }
}

impl From<LearningRepositoryError> for LearningError {
    fn from(e: LearningRepositoryError) -> Self {
        Self::Repository(e)
    }
}

impl From<ValidationError> for LearningError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<AuthorizationError> for LearningError {
    fn from(e: AuthorizationError) -> Self {
        Self::Authorization(e)
    }
}

/// Delegated Work Management failures keep their own shape so the desktop
/// boundary can map them exactly as it maps a direct Work Management error
/// (authorization denials stay denials, task transitions stay transitions).
impl From<WorkManagementError> for LearningError {
    fn from(e: WorkManagementError) -> Self {
        match e {
            WorkManagementError::Authorization(a) => Self::Authorization(a),
            WorkManagementError::Validation(v) => Self::Validation(v),
            other => Self::WorkManagement(other),
        }
    }
}

/// Run the fresh authorization check for `action`; on denial record the
/// audited refusal (through the shared Work Management helper) and return
/// the typed error with the Learning error shape.
#[allow(clippy::too_many_arguments)]
pub(crate) fn gate(
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    action: &ProtectedAction,
    workspace_id: &WorkspaceId,
    resource_type: &'static str,
    resource_id: Option<&str>,
) -> Result<(), LearningError> {
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        action,
        Some(workspace_id),
        resource_type,
        resource_id,
    )?;
    Ok(())
}

/// Record the outcome of an attempted Learning action (allowed or error)
/// through the shared audit helper. Audit hook failures never fail the
/// business flow (port contract).
#[allow(clippy::too_many_arguments)]
pub(crate) fn audit_outcome(
    sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    action: &ProtectedAction,
    workspace_id: &WorkspaceId,
    resource_type: &'static str,
    resource_id: Option<&str>,
    outcome: AuditOutcome,
) {
    audit_action(
        sink,
        ctx,
        action,
        Some(workspace_id),
        resource_type,
        resource_id,
        outcome,
    );
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Shared in-memory fakes for the Learning use-case tests. The
    //! authorizer, audit sink, context and task repository fakes are the Work
    //! Management ones (same gate pattern, same audit shape).

    use std::cell::RefCell;

    use crate::ports::learning_repository::{
        ItemRepository, LearningRepositoryError, PlanRepository, RoadmapRepository,
        SessionRepository,
    };
    use paw_domain::learning::{
        Item, ItemId, ItemStatus, Plan, PlanId, Roadmap, RoadmapId, Session, SessionId,
    };
    use paw_domain::work_management::{ProjectId, TaskId, WorkspaceId};

    #[derive(Default)]
    pub struct FakeRoadmapRepo {
        pub roadmaps: RefCell<Vec<Roadmap>>,
    }

    impl RoadmapRepository for FakeRoadmapRepo {
        fn insert_roadmap(&self, roadmap: &Roadmap) -> Result<(), LearningRepositoryError> {
            self.roadmaps.borrow_mut().push(roadmap.clone());
            Ok(())
        }

        fn find_roadmap(
            &self,
            workspace_id: &WorkspaceId,
            roadmap_id: &RoadmapId,
        ) -> Result<Option<Roadmap>, LearningRepositoryError> {
            Ok(self
                .roadmaps
                .borrow()
                .iter()
                .find(|r| r.id == *roadmap_id && r.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_roadmaps(
            &self,
            workspace_id: &WorkspaceId,
        ) -> Result<Vec<Roadmap>, LearningRepositoryError> {
            Ok(self
                .roadmaps
                .borrow()
                .iter()
                .filter(|r| r.workspace_id == *workspace_id)
                .cloned()
                .collect())
        }

        fn update_roadmap(&self, _roadmap: &Roadmap) -> Result<(), LearningRepositoryError> {
            Ok(())
        }

        fn delete_roadmap(
            &self,
            workspace_id: &WorkspaceId,
            roadmap_id: &RoadmapId,
        ) -> Result<(), LearningRepositoryError> {
            let mut roadmaps = self.roadmaps.borrow_mut();
            let before = roadmaps.len();
            roadmaps.retain(|r| !(r.id == *roadmap_id && r.workspace_id == *workspace_id));
            if roadmaps.len() == before {
                Err(LearningRepositoryError::RoadmapNotFound)
            } else {
                Ok(())
            }
        }
    }

    #[derive(Default)]
    pub struct FakePlanRepo {
        pub plans: RefCell<Vec<Plan>>,
    }

    impl PlanRepository for FakePlanRepo {
        fn insert_plan(&self, plan: &Plan) -> Result<(), LearningRepositoryError> {
            self.plans.borrow_mut().push(plan.clone());
            Ok(())
        }

        fn find_plan(
            &self,
            workspace_id: &WorkspaceId,
            plan_id: &PlanId,
        ) -> Result<Option<Plan>, LearningRepositoryError> {
            Ok(self
                .plans
                .borrow()
                .iter()
                .find(|p| p.id == *plan_id && p.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_plans(
            &self,
            workspace_id: &WorkspaceId,
            roadmap_id: Option<&RoadmapId>,
        ) -> Result<Vec<Plan>, LearningRepositoryError> {
            Ok(self
                .plans
                .borrow()
                .iter()
                .filter(|p| {
                    p.workspace_id == *workspace_id && roadmap_id.is_none_or(|r| p.roadmap_id == *r)
                })
                .cloned()
                .collect())
        }

        fn update_plan(&self, plan: &Plan) -> Result<(), LearningRepositoryError> {
            let mut plans = self.plans.borrow_mut();
            match plans
                .iter_mut()
                .find(|p| p.id == plan.id && p.workspace_id == plan.workspace_id)
            {
                Some(slot) => {
                    *slot = plan.clone();
                    Ok(())
                }
                None => Err(LearningRepositoryError::PlanNotFound),
            }
        }

        fn delete_plan(
            &self,
            workspace_id: &WorkspaceId,
            plan_id: &PlanId,
        ) -> Result<(), LearningRepositoryError> {
            let mut plans = self.plans.borrow_mut();
            let before = plans.len();
            plans.retain(|p| !(p.id == *plan_id && p.workspace_id == *workspace_id));
            if plans.len() == before {
                Err(LearningRepositoryError::PlanNotFound)
            } else {
                Ok(())
            }
        }
    }

    #[derive(Default)]
    pub struct FakeItemRepo {
        pub items: RefCell<Vec<Item>>,
        pub item_projects: RefCell<Vec<(ItemId, ProjectId)>>,
        pub item_tasks: RefCell<Vec<(ItemId, TaskId)>>,
    }

    impl ItemRepository for FakeItemRepo {
        fn insert_item(&self, item: &Item) -> Result<(), LearningRepositoryError> {
            self.items.borrow_mut().push(item.clone());
            Ok(())
        }

        fn find_item(
            &self,
            workspace_id: &WorkspaceId,
            item_id: &ItemId,
        ) -> Result<Option<Item>, LearningRepositoryError> {
            Ok(self
                .items
                .borrow()
                .iter()
                .find(|i| i.id == *item_id && i.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_items(
            &self,
            workspace_id: &WorkspaceId,
            plan_id: Option<&PlanId>,
            status: Option<ItemStatus>,
        ) -> Result<Vec<Item>, LearningRepositoryError> {
            Ok(self
                .items
                .borrow()
                .iter()
                .filter(|i| {
                    i.workspace_id == *workspace_id
                        && plan_id.is_none_or(|p| i.plan_id == *p)
                        && status.is_none_or(|s| i.status == s)
                })
                .cloned()
                .collect())
        }

        fn update_item(&self, item: &Item) -> Result<(), LearningRepositoryError> {
            let mut items = self.items.borrow_mut();
            match items
                .iter_mut()
                .find(|i| i.id == item.id && i.workspace_id == item.workspace_id)
            {
                Some(slot) => {
                    *slot = item.clone();
                    Ok(())
                }
                None => Err(LearningRepositoryError::ItemNotFound),
            }
        }

        fn delete_item(
            &self,
            workspace_id: &WorkspaceId,
            item_id: &ItemId,
        ) -> Result<(), LearningRepositoryError> {
            let mut items = self.items.borrow_mut();
            let before = items.len();
            items.retain(|i| !(i.id == *item_id && i.workspace_id == *workspace_id));
            if items.len() == before {
                Err(LearningRepositoryError::ItemNotFound)
            } else {
                Ok(())
            }
        }

        fn attach_item_to_project(
            &self,
            _workspace_id: &WorkspaceId,
            item_id: &ItemId,
            project_id: &ProjectId,
        ) -> Result<(), LearningRepositoryError> {
            self.item_projects
                .borrow_mut()
                .push((item_id.clone(), project_id.clone()));
            Ok(())
        }

        fn detach_item_from_project(
            &self,
            _workspace_id: &WorkspaceId,
            item_id: &ItemId,
            project_id: &ProjectId,
        ) -> Result<(), LearningRepositoryError> {
            let mut links = self.item_projects.borrow_mut();
            let before = links.len();
            links.retain(|(i, p)| !(i == item_id && p == project_id));
            if links.len() == before {
                Err(LearningRepositoryError::ItemNotFound)
            } else {
                Ok(())
            }
        }

        fn list_projects_for_item(
            &self,
            _workspace_id: &WorkspaceId,
            item_id: &ItemId,
        ) -> Result<Vec<ProjectId>, LearningRepositoryError> {
            Ok(self
                .item_projects
                .borrow()
                .iter()
                .filter(|(i, _)| i == item_id)
                .map(|(_, p)| p.clone())
                .collect())
        }

        fn attach_task_to_item(
            &self,
            _workspace_id: &WorkspaceId,
            item_id: &ItemId,
            task_id: &TaskId,
        ) -> Result<(), LearningRepositoryError> {
            self.item_tasks
                .borrow_mut()
                .push((item_id.clone(), task_id.clone()));
            Ok(())
        }

        fn detach_task_from_item(
            &self,
            _workspace_id: &WorkspaceId,
            item_id: &ItemId,
            task_id: &TaskId,
        ) -> Result<(), LearningRepositoryError> {
            let mut links = self.item_tasks.borrow_mut();
            let before = links.len();
            links.retain(|(i, t)| !(i == item_id && t == task_id));
            if links.len() == before {
                Err(LearningRepositoryError::ItemNotFound)
            } else {
                Ok(())
            }
        }

        fn list_tasks_for_item(
            &self,
            _workspace_id: &WorkspaceId,
            item_id: &ItemId,
        ) -> Result<Vec<TaskId>, LearningRepositoryError> {
            Ok(self
                .item_tasks
                .borrow()
                .iter()
                .filter(|(i, _)| i == item_id)
                .map(|(_, t)| t.clone())
                .collect())
        }
    }

    #[derive(Default)]
    pub struct FakeSessionRepo {
        pub sessions: RefCell<Vec<Session>>,
    }

    impl SessionRepository for FakeSessionRepo {
        fn append_session(&self, session: &Session) -> Result<(), LearningRepositoryError> {
            self.sessions.borrow_mut().push(session.clone());
            Ok(())
        }

        fn find_session(
            &self,
            workspace_id: &WorkspaceId,
            session_id: &SessionId,
        ) -> Result<Option<Session>, LearningRepositoryError> {
            Ok(self
                .sessions
                .borrow()
                .iter()
                .find(|s| s.id == *session_id && s.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_sessions(
            &self,
            workspace_id: &WorkspaceId,
            item_id: Option<&ItemId>,
        ) -> Result<Vec<Session>, LearningRepositoryError> {
            Ok(self
                .sessions
                .borrow()
                .iter()
                .filter(|s| {
                    s.workspace_id == *workspace_id && item_id.is_none_or(|i| s.item_id == *i)
                })
                .cloned()
                .collect())
        }
    }
}
