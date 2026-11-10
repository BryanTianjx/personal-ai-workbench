//! Goal repository port: workspace-scoped persistence for Goal records.
//!
//! Same contract shape as `workspace_repository`: every method takes an
//! explicit `WorkspaceId`, cross-workspace records are never returned, and
//! rejections are typed and non-leaky.

use paw_domain::work_management::{Goal, GoalId, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalRepositoryError {
    /// The referenced goal does not exist in the workspace.
    GoalNotFound,
    /// A domain-level or constraint-level invariant was violated (including
    /// references into another workspace, which the composite schema keys
    /// refuse).
    Invariant(String),
}

impl std::fmt::Display for GoalRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GoalNotFound => write!(f, "goal not found"),
            Self::Invariant(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for GoalRepositoryError {}

pub trait GoalRepository {
    fn insert_goal(&self, goal: &Goal) -> Result<(), GoalRepositoryError>;

    fn find_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<Option<Goal>, GoalRepositoryError>;

    /// List the workspace's goals, optionally filtered by Area.
    fn list_goals(
        &self,
        workspace_id: &WorkspaceId,
        area_id: Option<&paw_domain::work_management::AreaId>,
    ) -> Result<Vec<Goal>, GoalRepositoryError>;

    /// Persist the full state of an existing goal (name, description,
    /// Area link). The workspace binding is immutable.
    fn update_goal(&self, goal: &Goal) -> Result<(), GoalRepositoryError>;

    fn delete_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<(), GoalRepositoryError>;
}
