//! Work Management use cases for the E1 vertical slice: create a workspace,
//! create a project inside a workspace, list a workspace's projects.
//!
//! Application use cases own orchestration (existence checks, ID generation)
//! while domain invariants stay in paw-domain constructors and repository
//! guarantees (scoping, constraints) stay in the adapter.

use crate::ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};
use paw_domain::work_management::{Project, ProjectId, ValidationError, Workspace, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkManagementError {
    /// Typed repository rejection: not-found, cross-workspace refusal,
    /// constraint violation or other invariant failure.
    Repository(WorkspaceRepositoryError),
    /// Input failed a domain invariant before touching the repository.
    Validation(ValidationError),
}

impl std::fmt::Display for WorkManagementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Repository(e) => write!(f, "{e}"),
            Self::Validation(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for WorkManagementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Repository(e) => Some(e),
            Self::Validation(e) => Some(e),
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

/// Create a workspace with a validated name. The generated UUIDv7 id is
/// opaque; callers must not infer ordering or authorization from it.
pub fn create_workspace(
    repo: &dyn WorkspaceRepository,
    name: &str,
) -> Result<Workspace, WorkManagementError> {
    let workspace = Workspace::new(WorkspaceId::new_v7(), name)?;
    repo.insert_workspace(&workspace)?;
    Ok(workspace)
}

/// Create a project inside `workspace_id`. The workspace must exist (checked
/// before the insert so the caller gets a typed not-found instead of a
/// constraint error), and the project is bound to that workspace by
/// construction.
pub fn create_project(
    repo: &dyn WorkspaceRepository,
    workspace_id: &WorkspaceId,
    name: &str,
) -> Result<Project, WorkManagementError> {
    if repo.find_workspace(workspace_id)?.is_none() {
        return Err(WorkManagementError::Repository(
            WorkspaceRepositoryError::WorkspaceNotFound,
        ));
    }
    let project = Project::new(ProjectId::new_v7(), workspace_id.clone(), name)?;
    repo.insert_project(workspace_id, &project)?;
    Ok(project)
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
    }

    #[test]
    fn create_workspace_validates_name_before_repository() {
        let repo = FakeRepo::default();
        let err = create_workspace(&repo, "   ").unwrap_err();
        assert!(matches!(err, WorkManagementError::Validation(_)));
        assert!(repo.workspaces.borrow().is_empty());

        let ws = create_workspace(&repo, "  主空间  ").unwrap();
        assert_eq!(ws.name, "主空间");
        assert_eq!(repo.workspaces.borrow().len(), 1);
    }

    #[test]
    fn create_project_requires_existing_workspace() {
        let repo = FakeRepo::default();
        let ghost = WorkspaceId::new_v7();
        let result = create_project(&repo, &ghost, "p");
        assert!(matches!(
            result,
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::WorkspaceNotFound
            ))
        ));

        let ws = create_workspace(&repo, "alpha").unwrap();
        let project = create_project(&repo, &ws.id, "p1").unwrap();
        assert_eq!(project.workspace_id, ws.id);
        assert_eq!(repo.projects.borrow().len(), 1);
    }

    #[test]
    fn list_projects_requires_existing_workspace() {
        let repo = FakeRepo::default();
        let ghost = WorkspaceId::new_v7();
        assert!(matches!(
            list_projects(&repo, &ghost),
            Err(WorkManagementError::Repository(
                WorkspaceRepositoryError::WorkspaceNotFound
            ))
        ));

        let ws = create_workspace(&repo, "alpha").unwrap();
        create_project(&repo, &ws.id, "p1").unwrap();
        assert_eq!(list_projects(&repo, &ws.id).unwrap().len(), 1);
    }
}
