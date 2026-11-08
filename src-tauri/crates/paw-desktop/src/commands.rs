//! Typed desktop command layer (E1-05): the boundary the UI calls.
//!
//! These functions are deliberately free of Tauri types so the
//! UI-to-SQLite integration tests in `paw-desktop/tests/` can drive the
//! exact code path the UI will drive. When the Tauri shell lands (next
//! bounded unit), `#[tauri::command]` wrappers attach here and delegate to
//! this module; no business logic may be added at the attribute layer.
//!
//! As the composition root, paw-desktop is the only crate that may hold a
//! concrete repository; use cases see ports only.

use std::path::Path;

use paw_application::use_cases::work_management::{self as use_cases, WorkManagementError};
// Domain contract types re-exported by the application layer: paw-desktop
// must not depend on paw-domain directly (E1-rust-workspace-layout §2).
use paw_application::{Project, Workspace, WorkspaceId};
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;

/// Work Management command surface constructed once at startup over the
/// authoritative local database (ADR-006: one `app.sqlite`).
pub struct WorkManagementApi {
    repo: SqliteWorkspaceRepository,
}

impl WorkManagementApi {
    /// Open the command surface over `db_path`. The database must already
    /// carry the current schema (migrations run before this call).
    pub fn open(db_path: &Path) -> Result<Self, paw_infrastructure::sqlite::Error> {
        Ok(Self {
            repo: SqliteWorkspaceRepository::open(db_path)?,
        })
    }

    /// Create a workspace with a validated name; returns the stored record.
    pub fn create_workspace(&self, name: &str) -> Result<Workspace, WorkManagementError> {
        use_cases::create_workspace(&self.repo, name)
    }

    /// Create a project inside `workspace_id`.
    pub fn create_project(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
    ) -> Result<Project, WorkManagementError> {
        use_cases::create_project(&self.repo, workspace_id, name)
    }

    /// List the projects of `workspace_id`.
    pub fn list_projects(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Project>, WorkManagementError> {
        use_cases::list_projects(&self.repo, workspace_id)
    }
}
