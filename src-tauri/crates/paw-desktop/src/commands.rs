//! Typed desktop command layer (E1-05, extended in E2): the boundary the UI
//! calls.
//!
//! The core methods are deliberately free of Tauri types so the
//! UI-to-SQLite integration tests in `paw-desktop/tests/` can drive the
//! exact code path the UI drives. `#[tauri::command]` wrappers delegate to
//! this module; no business logic may be added at the attribute layer.
//!
//! As the composition root, paw-desktop is the only crate that may hold
//! concrete repositories; use cases see ports only. Each repository opens
//! its own WAL connection (single-user desktop; SQLite serializes writers,
//! the busy timeout absorbs contention).

use std::path::Path;

use paw_application::ports::authorization::ExecutionContext;
use paw_application::ports::workspace_repository::WorkspaceRepositoryError;
use paw_application::use_cases::work_management::{self as use_cases, WorkManagementError};
use paw_application::{
    Area, AreaId, Goal, GoalId, Project, ProjectId, ProjectStatus, Task, TaskId, TaskStatus,
    ValidationError, Workspace, WorkspaceId,
};
use paw_infrastructure::repositories::area_repository::SqliteAreaRepository;
use paw_infrastructure::repositories::goal_repository::SqliteGoalRepository;
use paw_infrastructure::repositories::task_repository::SqliteTaskRepository;
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::security::{LOCAL_ACTOR_ID, LocalSingleUserAuthorizer, SqliteAuditSink};

/// Work Management command surface constructed once at startup over the
/// authoritative local database (ADR-006: one `app.sqlite`). It owns the
/// concrete repositories plus the E1-09 authorization policy and audit
/// sink; use cases see ports only.
pub struct WorkManagementApi {
    workspaces: SqliteWorkspaceRepository,
    areas: SqliteAreaRepository,
    goals: SqliteGoalRepository,
    tasks: SqliteTaskRepository,
    search: paw_infrastructure::search::SqliteSearchService,
    authorizer: LocalSingleUserAuthorizer,
    audit: SqliteAuditSink,
}

impl WorkManagementApi {
    /// Open the command surface over `db_path`. The database must already
    /// carry the current schema (migrations run before this call).
    pub fn open(db_path: &Path) -> Result<Self, paw_infrastructure::sqlite::Error> {
        Ok(Self {
            workspaces: SqliteWorkspaceRepository::open(db_path)?,
            areas: SqliteAreaRepository::open(db_path)?,
            goals: SqliteGoalRepository::open(db_path)?,
            tasks: SqliteTaskRepository::open(db_path)?,
            search: paw_infrastructure::search::SqliteSearchService::open(db_path)?,
            authorizer: LocalSingleUserAuthorizer,
            audit: SqliteAuditSink::open(db_path)?,
        })
    }

    /// Execution context for one command invocation: the local actor and a
    /// fresh correlation id (E1-09 audit trail).
    fn context(&self) -> ExecutionContext {
        ExecutionContext {
            actor_id: LOCAL_ACTOR_ID.into(),
            correlation_id: uuid::Uuid::now_v7().to_string(),
        }
    }

    // -- Workspaces ---------------------------------------------------------

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, WorkManagementError> {
        use_cases::list_workspaces(&self.workspaces)
    }

    pub fn create_workspace(&self, name: &str) -> Result<Workspace, WorkManagementError> {
        let ctx = self.context();
        use_cases::workspace::create_workspace(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            name,
        )
    }

    // -- Projects -----------------------------------------------------------

    pub fn create_project(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
    ) -> Result<Project, WorkManagementError> {
        let ctx = self.context();
        use_cases::project::create_project(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            name,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_project(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
        name: &str,
        description: Option<&str>,
        status: ProjectStatus,
        goal_id: Option<GoalId>,
        area_id: Option<AreaId>,
    ) -> Result<Project, WorkManagementError> {
        let ctx = self.context();
        use_cases::project::update_project(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            project_id,
            name,
            description,
            status,
            goal_id,
            area_id,
        )
    }

    pub fn delete_project(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::project::delete_project(
            &self.workspaces,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            project_id,
        )
    }

    pub fn list_projects(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Project>, WorkManagementError> {
        use_cases::workspace::list_projects(&self.workspaces, workspace_id)
    }

    // -- Areas --------------------------------------------------------------

    pub fn create_area(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Area, WorkManagementError> {
        let ctx = self.context();
        use_cases::area::create_area(
            &self.areas,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            name,
            description,
        )
    }

    pub fn update_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Area, WorkManagementError> {
        let ctx = self.context();
        use_cases::area::update_area(
            &self.areas,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            area_id,
            name,
            description,
        )
    }

    pub fn delete_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::area::delete_area(
            &self.areas,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            area_id,
        )
    }

    pub fn list_areas(&self, workspace_id: &WorkspaceId) -> Result<Vec<Area>, WorkManagementError> {
        use_cases::area::list_areas(&self.areas, workspace_id)
    }

    // -- Goals --------------------------------------------------------------

    pub fn create_goal(
        &self,
        workspace_id: &WorkspaceId,
        area_id: Option<AreaId>,
        name: &str,
        description: Option<&str>,
    ) -> Result<Goal, WorkManagementError> {
        let ctx = self.context();
        use_cases::goal::create_goal(
            &self.goals,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            area_id,
            name,
            description,
        )
    }

    pub fn update_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
        area_id: Option<AreaId>,
        name: &str,
        description: Option<&str>,
    ) -> Result<Goal, WorkManagementError> {
        let ctx = self.context();
        use_cases::goal::update_goal(
            &self.goals,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            goal_id,
            area_id,
            name,
            description,
        )
    }

    pub fn delete_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::goal::delete_goal(
            &self.goals,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            goal_id,
        )
    }

    pub fn list_goals(
        &self,
        workspace_id: &WorkspaceId,
        area_id: Option<&AreaId>,
    ) -> Result<Vec<Goal>, WorkManagementError> {
        use_cases::goal::list_goals(&self.goals, workspace_id, area_id)
    }

    // -- Search (E2-03) -----------------------------------------------------

    /// Scoped full-text search over projects and tasks.
    pub fn search(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<paw_application::ports::search::SearchHit>, CommandError> {
        use_cases::search(&self.search, workspace_id, query, limit).map_err(CommandError::from)
    }

    // -- Tasks --------------------------------------------------------------

    pub fn create_task(
        &self,
        workspace_id: &WorkspaceId,
        project_id: Option<ProjectId>,
        title: &str,
        notes: Option<&str>,
    ) -> Result<Task, WorkManagementError> {
        let ctx = self.context();
        use_cases::task::create_task(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            project_id,
            title,
            notes,
        )
    }

    pub fn update_task_details(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
        title: &str,
        notes: Option<&str>,
        project_id: Option<ProjectId>,
    ) -> Result<Task, WorkManagementError> {
        let ctx = self.context();
        use_cases::task::update_task_details(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            task_id,
            title,
            notes,
            project_id,
        )
    }

    pub fn transition_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
        to: TaskStatus,
    ) -> Result<Task, WorkManagementError> {
        let ctx = self.context();
        use_cases::task::transition_task(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            task_id,
            to,
        )
    }

    pub fn delete_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::task::delete_task(
            &self.tasks,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            task_id,
        )
    }

    pub fn list_tasks(
        &self,
        workspace_id: &WorkspaceId,
        project_id: Option<&ProjectId>,
        status: Option<TaskStatus>,
    ) -> Result<Vec<Task>, WorkManagementError> {
        use_cases::task::list_tasks(&self.tasks, workspace_id, project_id, status)
    }
}

// ---------------------------------------------------------------------------
// Tauri command bridge (E1 Tauri-shell unit; E2 command surface extension)
// ---------------------------------------------------------------------------

/// Tauri-safe error crossing the WebView boundary (Gate C — safe errors).
///
/// The frontend receives a stable error code only; driver messages, file
/// system paths, checksums and stack details never cross the boundary.
/// Variants are part of the published UI contract: add new variants only
/// with an explicit contract decision (E2 added the per-entity not-found
/// codes and `task_status_invalid` for the state-machine rule).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandError {
    /// A required name was empty or whitespace-only after trimming.
    EmptyName,
    /// A name or title exceeded its domain limit.
    NameTooLong,
    /// Free-form notes exceeded their limit.
    NotesTooLong,
    /// The referenced workspace does not exist.
    WorkspaceNotFound,
    /// The referenced project does not exist (or is invisible to the
    /// workspace scope).
    ProjectNotFound,
    /// The referenced area does not exist.
    AreaNotFound,
    /// The referenced goal does not exist.
    GoalNotFound,
    /// The referenced task does not exist.
    TaskNotFound,
    /// The workspace identifier was not a valid UUID.
    InvalidWorkspaceId,
    /// A task status change refused by the domain state machine.
    TaskStatusInvalid,
    /// The search query was empty or carried no searchable terms (E2-03).
    InvalidSearchQuery,
    /// Fresh authorization refused the protected side effect (E1-09).
    Denied,
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
            WorkManagementError::Validation(ValidationError::NotesTooLong { .. }) => {
                Self::NotesTooLong
            }
            WorkManagementError::Repository(WorkspaceRepositoryError::WorkspaceNotFound) => {
                Self::WorkspaceNotFound
            }
            WorkManagementError::Repository(WorkspaceRepositoryError::ProjectNotFound) => {
                Self::ProjectNotFound
            }
            WorkManagementError::Area(
                paw_application::ports::area_repository::AreaRepositoryError::AreaNotFound,
            ) => Self::AreaNotFound,
            WorkManagementError::Goal(
                paw_application::ports::goal_repository::GoalRepositoryError::GoalNotFound,
            ) => Self::GoalNotFound,
            WorkManagementError::Task(
                paw_application::ports::task_repository::TaskRepositoryError::TaskNotFound,
            ) => Self::TaskNotFound,
            WorkManagementError::Authorization(_) => Self::Denied,
            WorkManagementError::TaskTransition { .. } => Self::TaskStatusInvalid,
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

impl From<paw_application::ports::search::SearchError> for CommandError {
    fn from(err: paw_application::ports::search::SearchError) -> Self {
        match err {
            paw_application::ports::search::SearchError::InvalidQuery => Self::InvalidSearchQuery,
            // Storage failures stay opaque on the boundary.
            paw_application::ports::search::SearchError::Invariant(_) => Self::Unexpected,
        }
    }
}

/// `#[tauri::command]` wrappers: the attribute layer only moves data across
/// the WebView boundary and delegates to `WorkManagementApi`. JS arguments
/// use camelCase keys (`workspaceId`) and are mapped to the snake_case Rust
/// parameters by the Tauri command machinery.
///
/// The API is wrapped in a `Mutex` because a rusqlite connection is not
/// `Sync` (SQLite serializes access itself, but Rust still requires a lock
/// to share the handles across the WebView message-loop threads).
#[tauri::command]
pub fn list_workspaces(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
) -> Result<Vec<Workspace>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_workspaces()?)
}

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
#[allow(clippy::too_many_arguments)]
pub fn update_project(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: ProjectId,
    name: String,
    description: Option<String>,
    status: ProjectStatus,
    goal_id: Option<GoalId>,
    area_id: Option<AreaId>,
) -> Result<Project, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_project(
        &workspace_id,
        &project_id,
        &name,
        description.as_deref(),
        status,
        goal_id,
        area_id,
    )?)
}

#[tauri::command]
pub fn delete_project(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: ProjectId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_project(&workspace_id, &project_id)?)
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

#[tauri::command]
pub fn create_area(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    name: String,
    description: Option<String>,
) -> Result<Area, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_area(&workspace_id, &name, description.as_deref())?)
}

#[tauri::command]
pub fn update_area(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: AreaId,
    name: String,
    description: Option<String>,
) -> Result<Area, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_area(&workspace_id, &area_id, &name, description.as_deref())?)
}

#[tauri::command]
pub fn delete_area(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: AreaId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_area(&workspace_id, &area_id)?)
}

#[tauri::command]
pub fn list_areas(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<Area>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_areas(&workspace_id)?)
}

#[tauri::command]
pub fn create_goal(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: Option<AreaId>,
    name: String,
    description: Option<String>,
) -> Result<Goal, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_goal(&workspace_id, area_id, &name, description.as_deref())?)
}

#[tauri::command]
pub fn update_goal(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    goal_id: GoalId,
    area_id: Option<AreaId>,
    name: String,
    description: Option<String>,
) -> Result<Goal, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_goal(
        &workspace_id,
        &goal_id,
        area_id,
        &name,
        description.as_deref(),
    )?)
}

#[tauri::command]
pub fn delete_goal(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    goal_id: GoalId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_goal(&workspace_id, &goal_id)?)
}

#[tauri::command]
pub fn list_goals(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    area_id: Option<AreaId>,
) -> Result<Vec<Goal>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_goals(&workspace_id, area_id.as_ref())?)
}

#[tauri::command]
pub fn create_task(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: Option<ProjectId>,
    title: String,
    notes: Option<String>,
) -> Result<Task, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.create_task(&workspace_id, project_id, &title, notes.as_deref())?)
}

#[tauri::command]
pub fn update_task_details(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    task_id: TaskId,
    title: String,
    notes: Option<String>,
    project_id: Option<ProjectId>,
) -> Result<Task, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.update_task_details(
        &workspace_id,
        &task_id,
        &title,
        notes.as_deref(),
        project_id,
    )?)
}

#[tauri::command]
pub fn transition_task(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    task_id: TaskId,
    to: TaskStatus,
) -> Result<Task, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.transition_task(&workspace_id, &task_id, to)?)
}

#[tauri::command]
pub fn delete_task(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    task_id: TaskId,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.delete_task(&workspace_id, &task_id)?)
}

#[tauri::command]
pub fn list_tasks(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    project_id: Option<ProjectId>,
    status: Option<TaskStatus>,
) -> Result<Vec<Task>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.list_tasks(&workspace_id, project_id.as_ref(), status)?)
}

#[tauri::command]
pub fn search(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<paw_application::ports::search::SearchHit>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.search(&workspace_id, &query, limit.unwrap_or(20))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_error_maps_typed_errors_to_safe_codes() {
        // Validation errors keep their dedicated codes.
        assert_eq!(
            CommandError::from(WorkManagementError::Validation(ValidationError::EmptyName)),
            CommandError::EmptyName
        );
        assert_eq!(
            CommandError::from(WorkManagementError::Validation(
                ValidationError::NameTooLong { max: 120 }
            )),
            CommandError::NameTooLong
        );
        // Not-found keeps its code; authorization refusals surface as
        // `denied` (E1-09 contract decision), never as a generic failure.
        assert_eq!(
            CommandError::from(WorkManagementError::Repository(
                WorkspaceRepositoryError::WorkspaceNotFound
            )),
            CommandError::WorkspaceNotFound
        );
        assert_eq!(
            CommandError::from(WorkManagementError::Repository(
                WorkspaceRepositoryError::ProjectNotFound
            )),
            CommandError::ProjectNotFound
        );
        assert_eq!(
            CommandError::from(WorkManagementError::Authorization(
                paw_application::ports::authorization::AuthorizationError::Denied
            )),
            CommandError::Denied
        );
        // The E2 state-machine rejection is a dedicated code.
        assert_eq!(
            CommandError::from(WorkManagementError::TaskTransition {
                from: TaskStatus::Done,
                to: TaskStatus::Todo,
            }),
            CommandError::TaskStatusInvalid
        );
    }

    #[test]
    fn command_error_never_carries_driver_text() {
        // An invariant failure (which wraps a driver message internally)
        // crosses the boundary as the opaque `unexpected` code only.
        let err = WorkManagementError::Repository(WorkspaceRepositoryError::Invariant(
            "UNIQUE constraint failed: workspaces.id (leaky detail)".into(),
        ));
        let code = CommandError::from(err);
        assert_eq!(code, CommandError::Unexpected);
        let serialized = serde_json::to_string(&code).unwrap();
        assert!(!serialized.contains("UNIQUE"));
        assert!(!serialized.contains("leaky"));
    }
}
