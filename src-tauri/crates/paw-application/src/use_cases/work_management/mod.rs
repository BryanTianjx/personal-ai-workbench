//! Work Management use cases: named operations over Workspace, Area, Goal,
//! Project and Task records (PRD §4.2), published to the desktop boundary.
//!
//! Every mutating use case follows the E1-09 gate pattern: fresh
//! authorization before the protected side effect, domain validation before
//! the repository, and an audit record for allowed, denied and error
//! outcomes. Reads are workspace-scoped by the repository ports and are not
//! authorization-gated in E2.

pub mod area;
pub mod goal;
pub mod project;
pub mod task;
pub mod workspace;

use crate::ports::area_repository::AreaRepositoryError;
use crate::ports::audit::{AuditEvent, AuditOutcome, AuditSink};
use crate::ports::authorization::AuthorizationError;
use crate::ports::goal_repository::GoalRepositoryError;
use crate::ports::task_repository::TaskRepositoryError;
use crate::ports::workspace_repository::WorkspaceRepositoryError;
use paw_domain::work_management::{TaskStatus, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkManagementError {
    /// Typed workspace/project repository rejection.
    Repository(WorkspaceRepositoryError),
    /// Typed area repository rejection.
    Area(AreaRepositoryError),
    /// Typed goal repository rejection.
    Goal(GoalRepositoryError),
    /// Typed task repository rejection.
    Task(TaskRepositoryError),
    /// Input failed a domain invariant before touching the repository.
    Validation(paw_domain::work_management::ValidationError),
    /// Fresh authorization was refused for the protected side effect.
    Authorization(AuthorizationError),
    /// A task status change that the domain state machine refuses.
    TaskTransition { from: TaskStatus, to: TaskStatus },
}

impl std::fmt::Display for WorkManagementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Repository(e) => write!(f, "{e}"),
            Self::Area(e) => write!(f, "{e}"),
            Self::Goal(e) => write!(f, "{e}"),
            Self::Task(e) => write!(f, "{e}"),
            Self::Validation(e) => write!(f, "{e}"),
            Self::Authorization(e) => write!(f, "{e}"),
            Self::TaskTransition { from, to } => {
                write!(f, "cannot move a task from {from} to {to}")
            }
        }
    }
}

impl std::error::Error for WorkManagementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Repository(e) => Some(e),
            Self::Area(e) => Some(e),
            Self::Goal(e) => Some(e),
            Self::Task(e) => Some(e),
            Self::Validation(e) => Some(e),
            Self::Authorization(e) => Some(e),
            Self::TaskTransition { .. } => None,
        }
    }
}

impl From<WorkspaceRepositoryError> for WorkManagementError {
    fn from(e: WorkspaceRepositoryError) -> Self {
        Self::Repository(e)
    }
}

impl From<AreaRepositoryError> for WorkManagementError {
    fn from(e: AreaRepositoryError) -> Self {
        Self::Area(e)
    }
}

impl From<GoalRepositoryError> for WorkManagementError {
    fn from(e: GoalRepositoryError) -> Self {
        Self::Goal(e)
    }
}

impl From<TaskRepositoryError> for WorkManagementError {
    fn from(e: TaskRepositoryError) -> Self {
        Self::Task(e)
    }
}

impl From<AuthorizationError> for WorkManagementError {
    fn from(e: AuthorizationError) -> Self {
        Self::Authorization(e)
    }
}

impl From<paw_domain::work_management::ValidationError> for WorkManagementError {
    fn from(e: paw_domain::work_management::ValidationError) -> Self {
        Self::Validation(e)
    }
}

/// Audit one attempted action; failures of the audit hook never fail the
/// business flow (port contract).
pub(crate) fn audit(sink: &dyn AuditSink, event: AuditEvent) {
    sink.record(&event);
}

/// Build and record one audit event for an attempted action (shared by all
/// Work Management use cases).
pub(crate) fn audit_action(
    sink: &dyn AuditSink,
    ctx: &crate::ports::authorization::ExecutionContext,
    action: &crate::ports::authorization::ProtectedAction,
    workspace_id: Option<&WorkspaceId>,
    resource_type: &'static str,
    resource_id: Option<&str>,
    outcome: AuditOutcome,
) {
    audit(
        sink,
        AuditEvent {
            workspace_id: workspace_id.cloned(),
            actor_id: ctx.actor_id.clone(),
            action: action.action_key(),
            resource_type,
            resource_id: resource_id.map(str::to_string),
            outcome,
            correlation_id: ctx.correlation_id.clone(),
        },
    );
}

/// Gate helper: run the fresh authorization check for `action`; on denial
/// record the audited refusal and return the typed error. Every mutating
/// use case starts with this call.
pub(crate) fn check_authorization(
    authorizer: &dyn crate::ports::authorization::Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &crate::ports::authorization::ExecutionContext,
    action: &crate::ports::authorization::ProtectedAction,
    workspace_id: Option<&WorkspaceId>,
    resource_type: &'static str,
    resource_id: Option<&str>,
) -> Result<(), WorkManagementError> {
    if let Err(denied) = authorizer.authorize(ctx, action) {
        audit_action(
            audit_sink,
            ctx,
            action,
            workspace_id,
            resource_type,
            resource_id,
            AuditOutcome::Denied,
        );
        return Err(denied.into());
    }
    Ok(())
}

/// List every workspace for the workspace picker (E2: repairs the Gate E
/// observation that the UI could not re-select a persisted workspace).
/// Deliberately unscoped and ungated: this read happens before any
/// workspace context exists and exposes only id and name. Re-exported from
/// the workspace module; kept here as the named operation entry point.
pub use workspace::list_workspaces;

#[cfg(test)]
pub(crate) mod test_support {
    //! Shared in-memory fakes for the Work Management use-case tests.

    use crate::ports::area_repository::{AreaRepository, AreaRepositoryError};
    use crate::ports::audit::{AuditEvent, AuditOutcome, AuditSink};
    use crate::ports::authorization::{
        AuthorizationError, Authorizer, ExecutionContext, ProtectedAction,
    };
    use crate::ports::goal_repository::{GoalRepository, GoalRepositoryError};
    use crate::ports::task_repository::{TaskRepository, TaskRepositoryError};
    use crate::ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};
    use paw_domain::work_management::{
        Area, AreaId, Goal, GoalId, Project, ProjectId, Task, TaskId, TaskStatus, Workspace,
        WorkspaceId,
    };
    use std::cell::RefCell;

    pub fn ctx(actor: &str) -> ExecutionContext {
        ExecutionContext {
            actor_id: actor.into(),
            correlation_id: "corr-test".into(),
        }
    }

    #[derive(Default)]
    pub struct FakeWorkspaceRepo {
        pub workspaces: RefCell<Vec<Workspace>>,
        pub projects: RefCell<Vec<Project>>,
    }

    impl FakeWorkspaceRepo {
        pub fn with_workspace(ws: &Workspace) -> Self {
            let repo = Self::default();
            repo.workspaces.borrow_mut().push(ws.clone());
            repo
        }
    }

    impl WorkspaceRepository for FakeWorkspaceRepo {
        fn find_workspace(
            &self,
            workspace_id: &WorkspaceId,
        ) -> Result<Option<Workspace>, WorkspaceRepositoryError> {
            Ok(self
                .workspaces
                .borrow()
                .iter()
                .find(|w| w.id == *workspace_id)
                .cloned())
        }

        fn list_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceRepositoryError> {
            Ok(self.workspaces.borrow().clone())
        }

        fn list_projects(
            &self,
            workspace_id: &WorkspaceId,
        ) -> Result<Vec<Project>, WorkspaceRepositoryError> {
            Ok(self
                .projects
                .borrow()
                .iter()
                .filter(|p| p.workspace_id == *workspace_id)
                .cloned()
                .collect())
        }

        fn insert_workspace(&self, workspace: &Workspace) -> Result<(), WorkspaceRepositoryError> {
            self.workspaces.borrow_mut().push(workspace.clone());
            Ok(())
        }

        fn insert_project(
            &self,
            _workspace_id: &WorkspaceId,
            project: &Project,
        ) -> Result<(), WorkspaceRepositoryError> {
            self.projects.borrow_mut().push(project.clone());
            Ok(())
        }

        fn update_project(
            &self,
            workspace_id: &WorkspaceId,
            project: &Project,
        ) -> Result<(), WorkspaceRepositoryError> {
            let mut projects = self.projects.borrow_mut();
            let existing = projects
                .iter_mut()
                .find(|p| p.id == project.id && p.workspace_id == *workspace_id);
            match existing {
                Some(slot) => {
                    *slot = project.clone();
                    Ok(())
                }
                None => Err(WorkspaceRepositoryError::ProjectNotFound),
            }
        }

        fn delete_project(
            &self,
            workspace_id: &WorkspaceId,
            project_id: &ProjectId,
        ) -> Result<(), WorkspaceRepositoryError> {
            let mut projects = self.projects.borrow_mut();
            let len_before = projects.len();
            projects.retain(|p| !(p.id == *project_id && p.workspace_id == *workspace_id));
            if projects.len() == len_before {
                Err(WorkspaceRepositoryError::ProjectNotFound)
            } else {
                Ok(())
            }
        }

        fn delete_workspace(
            &self,
            _workspace_id: &WorkspaceId,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::WorkspaceNotFound)
        }
    }

    #[derive(Default)]
    pub struct FakeAreaRepo {
        pub areas: RefCell<Vec<Area>>,
    }

    impl AreaRepository for FakeAreaRepo {
        fn insert_area(&self, area: &Area) -> Result<(), AreaRepositoryError> {
            self.areas.borrow_mut().push(area.clone());
            Ok(())
        }

        fn find_area(
            &self,
            workspace_id: &WorkspaceId,
            area_id: &AreaId,
        ) -> Result<Option<Area>, AreaRepositoryError> {
            Ok(self
                .areas
                .borrow()
                .iter()
                .find(|a| a.id == *area_id && a.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_areas(&self, workspace_id: &WorkspaceId) -> Result<Vec<Area>, AreaRepositoryError> {
            Ok(self
                .areas
                .borrow()
                .iter()
                .filter(|a| a.workspace_id == *workspace_id)
                .cloned()
                .collect())
        }

        fn update_area(&self, area: &Area) -> Result<(), AreaRepositoryError> {
            let mut areas = self.areas.borrow_mut();
            match areas.iter_mut().find(|a| a.id == area.id) {
                Some(slot) => {
                    *slot = area.clone();
                    Ok(())
                }
                None => Err(AreaRepositoryError::AreaNotFound),
            }
        }

        fn delete_area(
            &self,
            workspace_id: &WorkspaceId,
            area_id: &AreaId,
        ) -> Result<(), AreaRepositoryError> {
            let mut areas = self.areas.borrow_mut();
            let len_before = areas.len();
            areas.retain(|a| !(a.id == *area_id && a.workspace_id == *workspace_id));
            if areas.len() == len_before {
                Err(AreaRepositoryError::AreaNotFound)
            } else {
                Ok(())
            }
        }
    }

    #[derive(Default)]
    pub struct FakeGoalRepo {
        pub goals: RefCell<Vec<Goal>>,
    }

    impl GoalRepository for FakeGoalRepo {
        fn insert_goal(&self, goal: &Goal) -> Result<(), GoalRepositoryError> {
            self.goals.borrow_mut().push(goal.clone());
            Ok(())
        }

        fn find_goal(
            &self,
            workspace_id: &WorkspaceId,
            goal_id: &GoalId,
        ) -> Result<Option<Goal>, GoalRepositoryError> {
            Ok(self
                .goals
                .borrow()
                .iter()
                .find(|g| g.id == *goal_id && g.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_goals(
            &self,
            workspace_id: &WorkspaceId,
            area_id: Option<&AreaId>,
        ) -> Result<Vec<Goal>, GoalRepositoryError> {
            Ok(self
                .goals
                .borrow()
                .iter()
                .filter(|g| {
                    g.workspace_id == *workspace_id
                        && match area_id {
                            Some(a) => g.area_id.as_ref() == Some(a),
                            None => true,
                        }
                })
                .cloned()
                .collect())
        }

        fn update_goal(&self, goal: &Goal) -> Result<(), GoalRepositoryError> {
            let mut goals = self.goals.borrow_mut();
            match goals.iter_mut().find(|g| g.id == goal.id) {
                Some(slot) => {
                    *slot = goal.clone();
                    Ok(())
                }
                None => Err(GoalRepositoryError::GoalNotFound),
            }
        }

        fn delete_goal(
            &self,
            workspace_id: &WorkspaceId,
            goal_id: &GoalId,
        ) -> Result<(), GoalRepositoryError> {
            let mut goals = self.goals.borrow_mut();
            let len_before = goals.len();
            goals.retain(|g| !(g.id == *goal_id && g.workspace_id == *workspace_id));
            if goals.len() == len_before {
                Err(GoalRepositoryError::GoalNotFound)
            } else {
                Ok(())
            }
        }
    }

    #[derive(Default)]
    pub struct FakeTaskRepo {
        pub tasks: RefCell<Vec<Task>>,
    }

    impl TaskRepository for FakeTaskRepo {
        fn insert_task(&self, task: &Task) -> Result<(), TaskRepositoryError> {
            self.tasks.borrow_mut().push(task.clone());
            Ok(())
        }

        fn find_task(
            &self,
            workspace_id: &WorkspaceId,
            task_id: &TaskId,
        ) -> Result<Option<Task>, TaskRepositoryError> {
            Ok(self
                .tasks
                .borrow()
                .iter()
                .find(|t| t.id == *task_id && t.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_tasks(
            &self,
            workspace_id: &WorkspaceId,
            project_id: Option<&ProjectId>,
            status: Option<TaskStatus>,
        ) -> Result<Vec<Task>, TaskRepositoryError> {
            Ok(self
                .tasks
                .borrow()
                .iter()
                .filter(|t| {
                    t.workspace_id == *workspace_id
                        && match project_id {
                            Some(p) => t.project_id.as_ref() == Some(p),
                            None => true,
                        }
                        && match status {
                            Some(s) => t.status == s,
                            None => true,
                        }
                })
                .cloned()
                .collect())
        }

        fn update_task(&self, task: &Task) -> Result<(), TaskRepositoryError> {
            let mut tasks = self.tasks.borrow_mut();
            match tasks.iter_mut().find(|t| t.id == task.id) {
                Some(slot) => {
                    *slot = task.clone();
                    Ok(())
                }
                None => Err(TaskRepositoryError::TaskNotFound),
            }
        }

        fn delete_task(
            &self,
            workspace_id: &WorkspaceId,
            task_id: &TaskId,
        ) -> Result<(), TaskRepositoryError> {
            let mut tasks = self.tasks.borrow_mut();
            let len_before = tasks.len();
            tasks.retain(|t| !(t.id == *task_id && t.workspace_id == *workspace_id));
            if tasks.len() == len_before {
                Err(TaskRepositoryError::TaskNotFound)
            } else {
                Ok(())
            }
        }
    }

    /// Authorization fake: grants `grant_actor`, denies everyone else, and
    /// records every check it sees.
    pub struct FakeAuthorizer {
        grant_actor: String,
        pub checks: RefCell<Vec<(String, &'static str)>>,
    }

    impl FakeAuthorizer {
        pub fn granting(actor: &str) -> Self {
            Self {
                grant_actor: actor.into(),
                checks: RefCell::new(Vec::new()),
            }
        }

        pub fn denying() -> Self {
            Self::granting("__nobody__")
        }
    }

    impl Authorizer for FakeAuthorizer {
        fn authorize(
            &self,
            ctx: &ExecutionContext,
            action: &ProtectedAction,
        ) -> Result<(), AuthorizationError> {
            self.checks
                .borrow_mut()
                .push((ctx.actor_id.clone(), action.action_key()));
            if ctx.actor_id == self.grant_actor {
                Ok(())
            } else {
                Err(AuthorizationError::Denied)
            }
        }
    }

    /// Audit fake: records (action, outcome) pairs.
    #[derive(Default)]
    pub struct FakeAudit {
        pub events: RefCell<Vec<(String, AuditOutcome)>>,
    }

    impl AuditSink for FakeAudit {
        fn record(&self, event: &AuditEvent) {
            self.events
                .borrow_mut()
                .push((event.action.to_string(), event.outcome));
        }
    }
}
