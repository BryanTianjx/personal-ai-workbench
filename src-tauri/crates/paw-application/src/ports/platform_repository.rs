//! Platform composition persistence ports (E7-02/E7-03/E7-04;
//! PAW-INF-007/008/009).

use paw_domain::platform::{CompositionId, ProfileId, RoleProfile, WorkspaceComposition};
use paw_domain::work_management::WorkspaceId;

/// Non-leaky persistence errors for the platform surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformStoreError {
    NotFound,
    WorkspaceMismatch,
    Invariant(String),
    Storage(String),
}

impl std::fmt::Display for PlatformStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "record not found"),
            Self::WorkspaceMismatch => write!(f, "record outside the requested workspace"),
            Self::Invariant(msg) => write!(f, "invariant violated: {msg}"),
            Self::Storage(msg) => write!(f, "storage failure: {msg}"),
        }
    }
}

impl std::error::Error for PlatformStoreError {}

/// Plugin disable preference (ADR-023 §R1 dimension: composition/availability
/// preference; NOT registration or authorization). Reads are fresh so a
/// disable takes effect at the next capability resolution.
pub trait PluginStateRepository {
    fn set_disabled(
        &self,
        workspace_id: &WorkspaceId,
        plugin_id: &str,
        disabled: bool,
    ) -> Result<(), PlatformStoreError>;
    /// `true` when the plugin is currently disabled in this workspace.
    fn is_disabled(
        &self,
        workspace_id: &WorkspaceId,
        plugin_id: &str,
    ) -> Result<bool, PlatformStoreError>;
    fn list_disabled(&self, workspace_id: &WorkspaceId) -> Result<Vec<String>, PlatformStoreError>;
}

/// Role profiles (ADR-024).
pub trait RoleProfileRepository {
    fn insert_profile(&self, profile: &RoleProfile) -> Result<(), PlatformStoreError>;
    fn get_profile(
        &self,
        id: &ProfileId,
        workspace_id: &WorkspaceId,
    ) -> Result<RoleProfile, PlatformStoreError>;
    fn list_profiles(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<RoleProfile>, PlatformStoreError>;
    fn update_profile(&self, profile: &RoleProfile) -> Result<(), PlatformStoreError>;
}

/// Workspace compositions (ADR-025). Scoped to one Data Workspace; every
/// command/query still resolves the actual workspace and authorization.
pub trait CompositionRepository {
    fn insert_composition(
        &self,
        composition: &WorkspaceComposition,
    ) -> Result<(), PlatformStoreError>;
    fn get_composition(
        &self,
        id: &CompositionId,
        workspace_id: &WorkspaceId,
    ) -> Result<WorkspaceComposition, PlatformStoreError>;
    fn list_compositions(
        &self,
        workspace_id: &WorkspaceId,
        actor_id: Option<&str>,
    ) -> Result<Vec<WorkspaceComposition>, PlatformStoreError>;
    fn update_composition(
        &self,
        composition: &WorkspaceComposition,
    ) -> Result<(), PlatformStoreError>;
}
