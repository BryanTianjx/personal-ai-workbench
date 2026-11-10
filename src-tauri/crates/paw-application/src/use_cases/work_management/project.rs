//! Project use cases: create, update (name/description/status/links) and
//! delete, with authorization, audit and workspace existence checks.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::work_management::{
    AreaId, GoalId, Project, ProjectId, ProjectStatus, WorkspaceId, validate_name,
};

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
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "project",
        None,
    )?;
    if repo.find_workspace(workspace_id)?.is_none() {
        return Err(WorkManagementError::Repository(
            WorkspaceRepositoryError::WorkspaceNotFound,
        ));
    }
    let project = Project::new(ProjectId::new_v7(), workspace_id.clone(), name)?;
    match repo.insert_project(workspace_id, &project) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "project",
                Some(project.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(project)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "project",
                Some(project.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Update an existing project: name, description, status and Goal/Area
/// links. The record is loaded first (typed not-found), mutated, and
/// persisted whole; the workspace binding is immutable.
#[allow(clippy::too_many_arguments)]
pub fn update_project(
    repo: &dyn WorkspaceRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    project_id: &ProjectId,
    name: &str,
    description: Option<&str>,
    status: ProjectStatus,
    goal_id: Option<GoalId>,
    area_id: Option<AreaId>,
) -> Result<Project, WorkManagementError> {
    let action = ProtectedAction::UpdateProject {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "project",
        Some(project_id.as_str()),
    )?;
    let mut project = match repo
        .list_projects(workspace_id)?
        .into_iter()
        .find(|p| p.id == *project_id)
    {
        Some(p) => p,
        None => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "project",
                Some(project_id.as_str()),
                AuditOutcome::Error,
            );
            return Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::ProjectNotFound,
            ));
        }
    };
    project.name = validate_name(name, 200)?;
    project.description = description.map(str::to_string);
    project.status = status;
    project.goal_id = goal_id;
    project.area_id = area_id;
    match repo.update_project(workspace_id, &project) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "project",
                Some(project.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(project)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "project",
                Some(project.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Delete a project inside `workspace_id`. Referential protections (tasks)
/// surface as typed rejections — no silent cascade.
pub fn delete_project(
    repo: &dyn WorkspaceRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    project_id: &ProjectId,
) -> Result<(), WorkManagementError> {
    let action = ProtectedAction::DeleteProject {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "project",
        Some(project_id.as_str()),
    )?;
    match repo.delete_project(workspace_id, project_id) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "project",
                Some(project_id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(())
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "project",
                Some(project_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::audit::AuditOutcome;
    use crate::use_cases::work_management::test_support::{
        FakeAudit, FakeAuthorizer, FakeWorkspaceRepo, ctx,
    };
    use paw_domain::work_management::Workspace;

    fn seeded() -> (FakeWorkspaceRepo, Workspace) {
        let ws = Workspace::new(WorkspaceId::new_v7(), "alpha").unwrap();
        let repo = FakeWorkspaceRepo::with_workspace(&ws);
        (repo, ws)
    }

    #[test]
    fn create_project_requires_existing_workspace() {
        let repo = FakeWorkspaceRepo::default();
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

        let (repo, ws) = seeded();
        let project =
            create_project(&repo, &authz, &audit, &ctx("local-user"), &ws.id, "p1").unwrap();
        assert_eq!(project.workspace_id, ws.id);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("project.create".into(), AuditOutcome::Allowed))
        );
    }

    #[test]
    fn update_project_mutates_and_audits() {
        let (repo, ws) = seeded();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let project =
            create_project(&repo, &authz, &audit, &ctx("local-user"), &ws.id, "p1").unwrap();

        let updated = update_project(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws.id,
            &project.id,
            "p1 改名",
            Some("描述"),
            ProjectStatus::Completed,
            None,
            None,
        )
        .unwrap();
        assert_eq!(updated.name, "p1 改名");
        assert_eq!(updated.status, ProjectStatus::Completed);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("project.update".into(), AuditOutcome::Allowed))
        );

        // Unknown project → typed invariant, audited as error.
        let result = update_project(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws.id,
            &ProjectId::new_v7(),
            "ghost",
            None,
            ProjectStatus::Active,
            None,
            None,
        );
        assert!(matches!(
            result,
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::ProjectNotFound,
            ))
        ));
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("project.update".into(), AuditOutcome::Error))
        );
    }

    #[test]
    fn delete_project_audits_and_requires_existence() {
        let (repo, ws) = seeded();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let project =
            create_project(&repo, &authz, &audit, &ctx("local-user"), &ws.id, "p1").unwrap();

        delete_project(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws.id,
            &project.id,
        )
        .unwrap();
        assert!(repo.list_projects(&ws.id).unwrap().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("project.delete".into(), AuditOutcome::Allowed))
        );

        let result = delete_project(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws.id,
            &project.id,
        );
        assert!(matches!(
            result,
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::ProjectNotFound,
            ))
        ));
    }
}
