//! Task repository port: workspace-scoped persistence for Task records.

use paw_domain::work_management::{Task, TaskId, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRepositoryError {
    /// The referenced task does not exist in the workspace.
    TaskNotFound,
    /// A domain-level or constraint-level invariant was violated (including
    /// references into another workspace, which the composite schema keys
    /// refuse).
    Invariant(String),
}

impl std::fmt::Display for TaskRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TaskNotFound => write!(f, "task not found"),
            Self::Invariant(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for TaskRepositoryError {}

pub trait TaskRepository {
    fn insert_task(&self, task: &Task) -> Result<(), TaskRepositoryError>;

    fn find_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<Option<Task>, TaskRepositoryError>;

    /// List the workspace's tasks, optionally filtered by project and/or
    /// status. Both filters are workspace-scoped by construction.
    fn list_tasks(
        &self,
        workspace_id: &WorkspaceId,
        project_id: Option<&paw_domain::work_management::ProjectId>,
        status: Option<paw_domain::work_management::TaskStatus>,
    ) -> Result<Vec<Task>, TaskRepositoryError>;

    /// Persist the full state of an existing task (title, notes, status,
    /// project link). The workspace binding is immutable; status-transition
    /// legality is enforced by the domain, not here.
    fn update_task(&self, task: &Task) -> Result<(), TaskRepositoryError>;

    fn delete_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<(), TaskRepositoryError>;
}
