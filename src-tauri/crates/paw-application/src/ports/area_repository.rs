//! Area repository port: workspace-scoped persistence for Area records.

use paw_domain::work_management::{Area, AreaId, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaRepositoryError {
    /// The referenced area does not exist in the workspace.
    AreaNotFound,
    /// A domain-level or constraint-level invariant was violated (including
    /// references into another workspace, which the composite schema keys
    /// refuse).
    Invariant(String),
}

impl std::fmt::Display for AreaRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AreaNotFound => write!(f, "area not found"),
            Self::Invariant(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for AreaRepositoryError {}

pub trait AreaRepository {
    fn insert_area(&self, area: &Area) -> Result<(), AreaRepositoryError>;

    fn find_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<Option<Area>, AreaRepositoryError>;

    fn list_areas(&self, workspace_id: &WorkspaceId) -> Result<Vec<Area>, AreaRepositoryError>;

    /// Persist the full state of an existing area (name, description).
    fn update_area(&self, area: &Area) -> Result<(), AreaRepositoryError>;

    /// Delete an area. Goals/projects referencing it keep their records —
    /// the schema's RESTRICT surfaces a typed refusal instead of a silent
    /// cascade; the caller unbinds references explicitly.
    fn delete_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<(), AreaRepositoryError>;
}
