//! Workspace use cases: create (gated + audited) and the workspace picker
//! listing; project listing lives with the workspace read surface.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::work_management::{Project, Workspace, WorkspaceId};

/// Create a workspace with a validated name. The generated UUIDv7 id is
/// opaque; callers must not infer ordering or authorization from it.
pub fn create_workspace(
    repo: &dyn WorkspaceRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    name: &str,
) -> Result<Workspace, WorkManagementError> {
    let action = ProtectedAction::CreateWorkspace;
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        None,
        "workspace",
        None,
    )?;
    let workspace = Workspace::new(WorkspaceId::new_v7(), name)?;
    match repo.insert_workspace(&workspace) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                None,
                "workspace",
                Some(workspace.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(workspace)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                None,
                "workspace",
                Some(workspace.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// List every workspace (workspace picker; see `list_workspaces` in the
/// module root for the authorization rationale).
pub fn list_workspaces(
    repo: &dyn WorkspaceRepository,
) -> Result<Vec<Workspace>, WorkManagementError> {
    Ok(repo.list_workspaces()?)
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
    use crate::use_cases::work_management::test_support::{
        FakeAudit, FakeAuthorizer, FakeWorkspaceRepo, ctx,
    };

    #[test]
    fn create_workspace_validates_name_before_repository() {
        let repo = FakeWorkspaceRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let err = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "   ").unwrap_err();
        assert!(matches!(err, WorkManagementError::Validation(_)));
        assert!(repo.workspaces.borrow().is_empty());

        let ws = create_workspace(&repo, &authz, &audit, &ctx("local-user"), "  主空间  ").unwrap();
        assert_eq!(ws.name, "主空间");
        assert_eq!(repo.workspaces.borrow().len(), 1);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("workspace.create".into(), AuditOutcome::Allowed))
        );
    }

    #[test]
    fn denied_authorization_blocks_the_side_effect_and_is_audited() {
        let repo = FakeWorkspaceRepo::default();
        let authz = FakeAuthorizer::denying();
        let audit = FakeAudit::default();

        let result = create_workspace(&repo, &authz, &audit, &ctx("intruder"), "alpha");
        assert!(matches!(result, Err(WorkManagementError::Authorization(_))));
        assert!(repo.workspaces.borrow().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("workspace.create".into(), AuditOutcome::Denied))
        );
    }

    #[test]
    fn list_projects_requires_existing_workspace() {
        let repo = FakeWorkspaceRepo::default();
        let ghost = WorkspaceId::new_v7();
        assert!(matches!(
            list_projects(&repo, &ghost),
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::WorkspaceNotFound
            ))
        ));

        let ws = Workspace::new(WorkspaceId::new_v7(), "alpha").unwrap();
        let repo = FakeWorkspaceRepo::with_workspace(&ws);
        assert!(list_projects(&repo, &ws.id).unwrap().is_empty());
    }

    #[test]
    fn list_workspaces_returns_every_workspace_unscoped() {
        let repo = FakeWorkspaceRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        create_workspace(&repo, &authz, &audit, &ctx("local-user"), "w1").unwrap();
        create_workspace(&repo, &authz, &audit, &ctx("local-user"), "w2").unwrap();
        let all = list_workspaces(&repo).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|w| w.name == "w1"));
        assert!(all.iter().any(|w| w.name == "w2"));
    }
}
