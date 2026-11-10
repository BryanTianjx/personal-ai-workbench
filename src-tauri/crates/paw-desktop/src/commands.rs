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
use paw_application::ports::workspace_repository::WorkspaceRepositoryError;
use paw_application::{Project, ValidationError, Workspace, WorkspaceId};
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

// ---------------------------------------------------------------------------
// Tauri command bridge (E1 Tauri-shell unit)
// ---------------------------------------------------------------------------

/// Tauri-safe error crossing the WebView boundary (Gate C — safe errors).
///
/// The frontend receives a stable error code only; driver messages, file
/// system paths, checksums and stack details never cross the boundary.
/// Variants are part of the published UI contract: add new variants only
/// with an explicit contract decision.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandError {
    /// A required name was empty or whitespace-only after trimming.
    EmptyName,
    /// A name exceeded its domain limit.
    NameTooLong,
    /// The referenced workspace does not exist.
    WorkspaceNotFound,
    /// The workspace identifier was not a valid UUID.
    InvalidWorkspaceId,
    /// Any other failure; details stay on the Rust side.
    Unexpected,
}

impl From<WorkManagementError> for CommandError {
    fn from(err: WorkManagementError) -> Self {
        match err {
            WorkManagementError::Validation(ValidationError::EmptyName) => Self::EmptyName,
            WorkManagementError::Validation(ValidationError::NameTooLong { .. }) => {
                Self::NameTooLong
            }
            WorkManagementError::Repository(WorkspaceRepositoryError::WorkspaceNotFound) => {
                Self::WorkspaceNotFound
            }
            // Invariant violations and repository-level invariant failures
            // stay generic on the boundary.
            _ => Self::Unexpected,
        }
    }
}

impl From<paw_application::ParseError> for CommandError {
    fn from(_: paw_application::ParseError) -> Self {
        Self::InvalidWorkspaceId
    }
}

/// `#[tauri::command]` wrappers: the attribute layer only moves data across
/// the WebView boundary and delegates to `WorkManagementApi`; no business
/// logic lives here (E1-rust-workspace-layout §4). JS arguments use
/// camelCase keys (`workspaceId`) and are mapped to the snake_case Rust
/// parameters by the Tauri command machinery.
///
/// The API is wrapped in a `Mutex` because a rusqlite connection is not
/// `Sync` (SQLite serializes access itself, but Rust still requires a lock
/// to share the handle across the WebView message-loop threads).
#[tauri::command]
pub fn create_workspace(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    name: String,
) -> Result<Workspace, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_workspace(&name)?)
}

#[tauri::command]
pub fn create_project(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    name: String,
) -> Result<Project, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_project(&workspace_id, &name)?)
}

#[tauri::command]
pub fn list_projects(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<Project>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_projects(&workspace_id)?)
}
