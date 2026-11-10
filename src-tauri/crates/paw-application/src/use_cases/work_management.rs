//! Work Management use cases for the E1 vertical slice: create a workspace,
//! create a project inside a workspace, list a workspace's projects.
//!
//! Application use cases own orchestration (existence checks, ID generation)
//! while domain invariants stay in paw-domain constructors and repository
//! guarantees (scoping, constraints) stay in the adapter.

use crate::ports::audit::{AuditEvent, AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};
use paw_domain::work_management::{Project, ProjectId, ValidationError, Workspace, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkManagementError {
    /// Typed repository rejection: not-found, cross-workspace refusal,
    /// constraint violation or other invariant failure.
    Repository(WorkspaceRepositoryError),
    /// Input failed a domain invariant before touching the repository.
    Validation(ValidationError),
    /// Fresh authorization was refused for the protected side effect (E1-09).
    Authorization(crate::ports::authorization::AuthorizationError),
}

impl std::fmt::Display for WorkManagementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Repository(e) => write!(f, "{e}"),
            Self::Validation(e) => write!(f, "{e}"),
            Self::Authorization(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for WorkManagementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Repository(e) => Some(e),
            Self::Validation(e) => Some(e),
            Self::Authorization(e) => Some(e),
        }
    }
}

impl From<WorkspaceRepositoryError> for WorkManagementError {
    fn from(e: WorkspaceRepositoryError) -> Self {
        Self::Repository(e)
    }
}

impl From<ValidationError> for WorkManagementError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<crate::ports::authorization::AuthorizationError> for WorkManagementError {
    fn from(e: crate::ports::authorization::AuthorizationError) -> Self {
        Self::Authorization(e)
    }
}

/// Audit one attempted action; failures of the audit hook never fail the
/// business flow (port contract).
fn audit(sink: &dyn AuditSink, event: AuditEvent) {
    sink.record(&event);
}

/// Create a workspace with a validated name. The generated UUIDv7 id is
/// opaque; callers must not infer ordering or authorization from it.
///
/// The protected side effect (insert) is preceded by a fresh authorization
/// check, and both the allowed and the refused attempt are audited (E1-09).
pub fn create_workspace(
    repo: &dyn WorkspaceRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    name: &str,
) -> Result<Workspace, WorkManagementError> {
    let action = ProtectedAction::CreateWorkspace;
    if let Err(denied) = authorizer.authorize(ctx, &action) {
        audit(
            audit_sink,
            AuditEvent {
                workspace_id: None,
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "workspace",
                resource_id: None,
                outcome: AuditOutcome::Denied,
                correlation_id: ctx.correlation_id.clone(),
            },
        );
        return Err(denied.into());
    }
    let workspace = Workspace::new(WorkspaceId::new_v7(), name)?;
    match repo.insert_workspace(&workspace) {
        Ok(()) => {
            audit(
                audit_sink,
                AuditEvent {
                    workspace_id: None,
                    actor_id: ctx.actor_id.clone(),
                    action: action.action_key(),
                    resource_type: "workspace",
                    resource_id: Some(workspace.id.as_str().to_string()),
                    outcome: AuditOutcome::Allowed,
                    correlation_id: ctx.correlation_id.clone(),
                },
            );
            Ok(workspace)
        }
        Err(e) => {
            audit(
                audit_sink,
                AuditEvent {
                    workspace_id: None,
                    actor_id: ctx.actor_id.clone(),
                    action: action.action_key(),
                    resource_type: "workspace",
                    resource_id: Some(workspace.id.as_str().to_string()),
                    outcome: AuditOutcome::Error,
                    correlation_id: ctx.correlation_id.clone(),
                },
            );
            Err(e.into())
        }
    }
}

/// Create a project inside `workspace_id`. The workspace must exist (checked
/// before the insert so the caller gets a typed not-found instead of a
/// constraint error), and the project is bound to that workspace by
/// construction.
pub fn create_project(
    repo: &dyn WorkspaceRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    name: &str,
) -> Result<Project, WorkManagementError> {
    let action = ProtectedAction::CreateProject {
        workspace_id: workspace_id.clone(),
    };
    if let Err(denied) = authorizer.authorize(ctx, &action) {
        audit(
            audit_sink,
            AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "project",
                resource_id: None,
                outcome: AuditOutcome::Denied,
                correlation_id: ctx.correlation_id.clone(),
            },
        );
        return Err(denied.into());
    }
    if repo.find_workspace(workspace_id)?.is_none() {
        return Err(WorkManagementError::Repository(
            WorkspaceRepositoryError::WorkspaceNotFound,
        ));
    }
    let project = Project::new(ProjectId::new_v7(), workspace_id.clone(), name)?;
    match repo.insert_project(workspace_id, &project) {
        Ok(()) => {
            audit(
                audit_sink,
                AuditEvent {
                    workspace_id: Some(workspace_id.clone()),
                    actor_id: ctx.actor_id.clone(),
                    action: action.action_key(),
                    resource_type: "project",
                    resource_id: Some(project.id.as_str().to_string()),
                    outcome: AuditOutcome::Allowed,
                    correlation_id: ctx.correlation_id.clone(),
                },
            );
            Ok(project)
        }
        Err(e) => {
            audit(
                audit_sink,
                AuditEvent {
                    workspace_id: Some(workspace_id.clone()),
                    actor_id: ctx.actor_id.clone(),
                    action: action.action_key(),
                    resource_type: "project",
                    resource_id: Some(project.id.as_str().to_string()),
                    outcome: AuditOutcome::Error,
                    correlation_id: ctx.correlation_id.clone(),
                },
            );
            Err(e.into())
        }
    }
}

/// List the projects of `workspace_id`. Cross-workspace records are never
/// returned by the repository; a missing workspace is a typed not-found.
pub fn list_projects(
    repo: &dyn WorkspaceRepository,
    workspace_id: &WorkspaceId,
) -> Result<Vec<Project>, WorkManagementError> {
    if repo.find_workspace(workspace_id)?.is_none() {
        return Err(WorkManagementError::Repository(
            WorkspaceRepositoryError::WorkspaceNotFound,
        ));
    }
    Ok(repo.list_projects(workspace_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::authorization::AuthorizationError;
    use std::cell::RefCell;

    /// In-memory fake implementing the port; records inserted rows and
    /// simulates workspace existence by inserted id only.
    #[derive(Default)]
    struct FakeRepo {
        workspaces: RefCell<Vec<Workspace>>,
        projects: RefCell<Vec<Project>>,
    }

    impl WorkspaceRepository for FakeRepo {
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

        fn delete_workspace(
            &self,
            _workspace_id: &WorkspaceId,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::WorkspaceNotFound)
        }

        fn update_project(
            &self,
            _workspace_id: &WorkspaceId,
            _project: &Project,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::Invariant("not supported".into()))
        }

        fn delete_project(
            &self,
            _workspace_id: &WorkspaceId,
            _project_id: &ProjectId,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::Invariant("not supported".into()))
        }
    }

    /// Authorization fake: grants `grant_actor`, denies everyone else, and
    /// records every check it sees.
    struct FakeAuthorizer {
        grant_actor: String,
        pub checks: RefCell<Vec<(String, &'static str)>>,
    }

    impl FakeAuthorizer {
        fn granting(actor: &str) -> Self {
            Self {
                grant_actor: actor.into(),
                checks: RefCell::new(Vec::new()),
            }
        }

        fn denying() -> Self {
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

    /// Audit fake: records every event it sees.
    #[derive(Default)]
    struct FakeAudit {
        pub events: RefCell<Vec<(String, AuditOutcome)>>,
    }

    impl AuditSink for FakeAudit {
        fn record(&self, event: &AuditEvent) {
            self.events
                .borrow_mut()
                .push((event.action.to_string(), event.outcome));
        }
    }

    fn ctx(actor: &str) -> ExecutionContext {
        ExecutionContext {
            actor_id: actor.into(),
            correlation_id: "corr-test".into(),
        }
    }

    #[test]
    fn create_workspace_validates_name_before_repository() {
        let repo = FakeRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let err = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "   ").unwrap_err();
        assert!(matches!(err, WorkManagementError::Validation(_)));
        assert!(repo.workspaces.borrow().is_empty());

        let ws = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "  主空间  ").unwrap();
        assert_eq!(ws.name, "主空间");
        assert_eq!(repo.workspaces.borrow().len(), 1);
    }

    #[test]
    fn create_project_requires_existing_workspace() {
        let repo = FakeRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ghost = WorkspaceId::new_v7();
        let result = create_project(&repo, &authz, &audit, &ctx("local-user"), &ghost, "p");
        assert!(matches!(
            result,
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::WorkspaceNotFound
            ))
        ));

        let ws = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "alpha").unwrap();
        let project =
            create_project(&repo, &authz, &audit, &ctx("local-user"), &ws.id, "p1").unwrap();
        assert_eq!(project.workspace_id, ws.id);
        assert_eq!(repo.projects.borrow().len(), 1);
    }

    #[test]
    fn list_projects_requires_existing_workspace() {
        let repo = FakeRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ghost = WorkspaceId::new_v7();
        assert!(matches!(
            list_projects(&repo, &ghost),
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::WorkspaceNotFound
            ))
        ));

        let ws = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "alpha").unwrap();
        create_project(&repo, &authz, &audit, &ctx("local-user"), &ws.id, "p1").unwrap();
        assert_eq!(list_projects(&repo, &ws.id).unwrap().len(), 1);
    }

    #[test]
    fn denied_authorization_blocks_the_side_effect_and_is_audited() {
        let repo = FakeRepo::default();
        let authz = FakeAuthorizer::denying();
        let audit = FakeAudit::default();

        let result = create_workspace(&repo, &authz, &audit, &ctx("intruder"), "alpha");
        assert!(matches!(
            result,
            Err(WorkManagementError::Authorization(
                AuthorizationError::Denied
            ))
        ));
        // Deny-by-default: nothing was written, and the refusal is audited.
        assert!(repo.workspaces.borrow().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("workspace.create".into(), AuditOutcome::Denied))
        );

        // The check happened before the repository was touched; an allowed
        // actor proceeds and is audited as allowed.
        let ws = create_workspace(&repo, &authz, &audit, &ctx("__nobody__"), "alpha").unwrap();
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("workspace.create".into(), AuditOutcome::Allowed))
        );
        assert_eq!(ws.name, "alpha");
        // Two authorize() checks were performed for three calls (denied one
        // plus the granted one); the granted actor here is the deny-actor,
        // proving the fake's grant list drives the decision, not the action.
        assert_eq!(authz.checks.borrow().len(), 2);
    }

    #[test]
    fn authorization_is_rechecked_per_side_effect_not_once() {
        let repo = FakeRepo::default();
        let audit = FakeAudit::default();
        // Actor that is granted only for the first call by flipping the
        // grant between calls.
        let authz = FakeAuthorizer::granting("local-user");

        let ws = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "alpha").unwrap();
        // Second side effect with a different (denied) actor: the use case
        // must re-evaluate, not inherit the earlier grant.
        let second = create_project(&repo, &authz, &audit, &ctx("other"), &ws.id, "p1");
        assert!(matches!(
            second,
            Err(WorkManagementError::Authorization(
                AuthorizationError::Denied
            ))
        ));
        assert!(repo.projects.borrow().is_empty());
        assert_eq!(authz.checks.borrow().len(), 2);
    }

    /// Repository fake whose inserts always fail (simulates a constraint or
    /// driver failure behind the port).
    struct FailingRepo;

    impl WorkspaceRepository for FailingRepo {
        fn find_workspace(
            &self,
            _workspace_id: &WorkspaceId,
        ) -> Result<Option<Workspace>, WorkspaceRepositoryError> {
            Ok(None)
        }

        fn list_projects(
            &self,
            _workspace_id: &WorkspaceId,
        ) -> Result<Vec<Project>, WorkspaceRepositoryError> {
            Ok(Vec::new())
        }

        fn insert_workspace(&self, _workspace: &Workspace) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::Invariant(
                "UNIQUE constraint failed: workspaces.id (simulated driver detail)".into(),
            ))
        }

        fn insert_project(
            &self,
            _workspace_id: &WorkspaceId,
            _project: &Project,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::Invariant("simulated".into()))
        }

        fn delete_workspace(
            &self,
            _workspace_id: &WorkspaceId,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::WorkspaceNotFound)
        }

        fn update_project(
            &self,
            _workspace_id: &WorkspaceId,
            _project: &Project,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::Invariant("not supported".into()))
        }

        fn delete_project(
            &self,
            _workspace_id: &WorkspaceId,
            _project_id: &ProjectId,
        ) -> Result<(), WorkspaceRepositoryError> {
            Err(WorkspaceRepositoryError::Invariant("not supported".into()))
        }
    }

    #[test]
    fn failing_side_effect_audits_error_outcome_without_leaking_details() {
        let repo = FailingRepo;
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();

        let result = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "alpha");
        assert!(matches!(
            result,
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::Invariant(_)
            ))
        ));

        // The failure is audited as `error`; the audit event itself carries
        // only stable keys and outcomes — no driver message crosses into it.
        let events = audit.events.borrow();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], ("workspace.create".into(), AuditOutcome::Error));
    }
}
