//! Workspace-scoped repository ports published by the application layer.
//!
//! These traits are the *only* way the application layer may touch workspace
//! and project records. The repository adapter (paw-infrastructure) enforces
//! workspace scoping; the UI never touches repositories directly.

use paw_domain::work_management::{Project, ProjectId, Workspace, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceRepositoryError {
    /// The requested workspace does not exist.
    WorkspaceNotFound,
    /// The requested project does not exist in the workspace.
    ProjectNotFound,
    /// A domain-level invariant was violated.
    Invariant(String),
}

impl std::fmt::Display for WorkspaceRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceNotFound => write!(f, "workspace not found"),
            Self::ProjectNotFound => write!(f, "project not found"),
            Self::Invariant(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for WorkspaceRepositoryError {}

impl From<paw_domain::work_management::ParseError> for WorkspaceRepositoryError {
    fn from(e: paw_domain::work_management::ParseError) -> Self {
        // Defined here because WorkspaceRepositoryError is local to this
        // crate (orphan rule); infrastructure converts its own driver errors
        // through its private adapter error type instead.
        Self::Invariant(e.to_string())
    }
}

/// Workspace-scoped repository contract.
///
/// Every method takes an explicit `workspace_id`. A caller that hands a
/// workspace_id that does not own a target record gets a typed, non-leaky
/// rejection. Workspace isolation is enforced here, not by UI filters.
pub trait WorkspaceRepository {
    /// Load a workspace by id. Returns `Ok(None)` when absent.
    fn find_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Option<Workspace>, WorkspaceRepositoryError>;

    /// List every workspace (E2 workspace picker). Deliberately unscoped:
    /// choosing which workspace to enter happens before any workspace
    /// context exists. The returned records carry only id and name.
    fn list_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceRepositoryError>;

    /// List all projects in a workspace. Cross-workspace records are never
    /// returned: a project whose `workspace_id` differs from the requested
    /// workspace is invisible to this call.
    fn list_projects(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Project>, WorkspaceRepositoryError>;

    /// Insert a new workspace. The caller supplies the workspace id (UUIDv7);
    /// the repository stores it verbatim and enforces uniqueness.
    fn insert_workspace(&self, workspace: &Workspace) -> Result<(), WorkspaceRepositoryError>;

    /// Insert a project scoped to `workspace_id`. The repository asserts
    /// `project.workspace_id == *workspace_id`.
    fn insert_project(
        &self,
        workspace_id: &WorkspaceId,
        project: &Project,
    ) -> Result<(), WorkspaceRepositoryError>;

    /// Persist the full state of an existing project (E2: name, status,
    /// Goal/Area links). The workspace binding is immutable; the project
    /// must exist inside `workspace_id`.
    fn update_project(
        &self,
        workspace_id: &WorkspaceId,
        project: &Project,
    ) -> Result<(), WorkspaceRepositoryError>;

    /// Delete one project inside `workspace_id`. Referential protections
    /// (tasks, later attachments) surface as typed rejections, never as
    /// silent cascades.
    fn delete_project(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
    ) -> Result<(), WorkspaceRepositoryError>;

    /// Delete a workspace and all its records (cascade in dependency order).
    /// Cross-workspace deletion is refused: `workspace_id` must be a
    /// workspace this adapter actually owns.
    fn delete_workspace(&self, workspace_id: &WorkspaceId) -> Result<(), WorkspaceRepositoryError>;
}
