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

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;

use paw_application::ports::audit::AuditSink as _;
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
    dashboard: paw_infrastructure::dashboard::SqliteDashboard,
    attachments:
        paw_infrastructure::repositories::attachment_repository::SqliteAttachmentRepository,
    file_store: paw_infrastructure::filesystem::FsManagedFileStore,
    /// Coordination connection for the durable-operations services (E3):
    /// operations, outbox and retention operate on raw rusqlite connections.
    coordination: rusqlite::Connection,
    authorizer: LocalSingleUserAuthorizer,
    audit: SqliteAuditSink,
}

impl WorkManagementApi {
    /// Open the command surface over `db_path`. The database must already
    /// carry the current schema (migrations run before this call). Managed
    /// attachment bytes live in a sibling `attachments/` directory (E2-04).
    pub fn open(db_path: &Path) -> Result<Self, paw_infrastructure::sqlite::Error> {
        Ok(Self {
            workspaces: SqliteWorkspaceRepository::open(db_path)?,
            areas: SqliteAreaRepository::open(db_path)?,
            goals: SqliteGoalRepository::open(db_path)?,
            tasks: SqliteTaskRepository::open(db_path)?,
            search: paw_infrastructure::search::SqliteSearchService::open(db_path)?,
            dashboard: paw_infrastructure::dashboard::SqliteDashboard::open(db_path)?,
            attachments:
                paw_infrastructure::repositories::attachment_repository::SqliteAttachmentRepository::open(
                    db_path,
                )?,
            file_store: paw_infrastructure::filesystem::FsManagedFileStore::open(
                db_path
                    .parent()
                    .expect("db_path lives inside the app data directory"),
            )
            .map_err(|e| {
                paw_infrastructure::sqlite::Error::Io {
                    source: std::io::Error::other(e.to_string()),
                }
            })?,
            coordination: paw_infrastructure::sqlite::open(db_path)?,
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

    // -- Dashboard (E2-05) ----------------------------------------------------

    /// Real-data summary for one workspace (PAW-F-004).
    pub fn dashboard(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<paw_application::ports::dashboard::DashboardSummary, CommandError> {
        use paw_application::ports::dashboard::DashboardPort;
        self.dashboard.summary(workspace_id).map_err(|e| match e {
            paw_application::ports::dashboard::DashboardError::Invariant(_) => {
                CommandError::Unexpected
            }
        })
    }

    // -- Operation queue (E3-04) ------------------------------------------------

    /// Workspace-scoped operation queue view. Read-only (reads are not
    /// authorization-gated in this codebase's current policy).
    pub fn list_operations(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<paw_infrastructure::operations::OperationRow>, CommandError> {
        paw_infrastructure::operations::list_operations(
            &self.coordination,
            workspace_id.as_str(),
            None,
        )
        .map_err(|_| CommandError::Unexpected)
    }

    /// Request cooperative cancellation (audited, gated).
    pub fn request_operation_cancellation(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &str,
    ) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::CancelOperation {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "operation",
            Some(operation_id),
        )?;
        let row =
            paw_infrastructure::operations::request_cancellation(&self.coordination, operation_id)
                .map_err(map_classification_error)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "operation",
                resource_id: Some(operation_id.to_string()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(row)
    }

    /// Reconcile an OutcomeUnknown operation back into Running (audited).
    pub fn resume_operation(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &str,
        note: &str,
    ) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::ResumeOperation {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "operation",
            Some(operation_id),
        )?;
        let row = paw_infrastructure::operations::resume_after_reconciliation(
            &self.coordination,
            operation_id,
            note,
        )
        .map_err(map_operation_error)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "operation",
                resource_id: Some(operation_id.to_string()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(row)
    }

    /// List quarantined messages of a workspace (plus system-scope rows are
    /// excluded: only the unfiltered system view sees those).
    pub fn list_quarantined(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<paw_infrastructure::outbox::QuarantinedMessage>, CommandError> {
        paw_infrastructure::outbox::list_quarantined(
            &self.coordination,
            Some(workspace_id.as_str()),
        )
        .map_err(|_| CommandError::Unexpected)
    }

    /// Re-drive a quarantined message (gated, audited; operator recorded).
    pub fn redrive_message(
        &self,
        workspace_id: &WorkspaceId,
        message_id: &str,
    ) -> Result<String, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::RedriveMessage {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "message",
            Some(message_id),
        )?;
        let now = paw_infrastructure::sqlite::now_ms().map_err(|_| CommandError::Unexpected)?;
        let id = paw_infrastructure::outbox::redrive_quarantined(
            &self.coordination,
            message_id,
            &ctx.actor_id,
            now,
        )
        .map_err(|_| CommandError::Unexpected)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "message",
                resource_id: Some(id.clone()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(id)
    }

    /// Export the workspace's durable-operation history to a temp JSON file
    /// and return its path (pre-purge user-initiated step; audited).
    pub fn export_operations_to_temp(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<String, CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::ExportWorkspaceData {
            workspace_id: workspace_id.clone(),
        };
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            Some(workspace_id),
            "operation",
            None,
        )?;
        let json = paw_infrastructure::retention::export_operations_json(
            &self.coordination,
            workspace_id.as_str(),
        )
        .map_err(|_| CommandError::Unexpected)?;
        let dir = std::env::temp_dir().join("paw-exports");
        std::fs::create_dir_all(&dir).map_err(CommandError::from_io)?;
        let path = dir.join(format!(
            "operations-{}-{}.json",
            workspace_id.as_str(),
            paw_infrastructure::sqlite::now_ms().map_err(|_| CommandError::Unexpected)?
        ));
        std::fs::write(&path, json).map_err(CommandError::from_io)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: Some(workspace_id.clone()),
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "export",
                resource_id: Some(path.to_string_lossy().into_owned()),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok(path.to_string_lossy().into_owned())
    }

    /// Purge expired operations and audit rows under the default policy
    /// (gated, audited, explicit — no background purge exists).
    pub fn purge_expired(&self) -> Result<(usize, usize, usize), CommandError> {
        let ctx = self.context();
        let action = paw_application::ports::authorization::ProtectedAction::PurgeExpiredData;
        check_gate(
            &self.authorizer,
            &self.audit,
            &ctx,
            &action,
            None,
            "data",
            None,
        )?;
        let now = paw_infrastructure::sqlite::now_ms().map_err(|_| CommandError::Unexpected)?;
        let (operations, attempts) = paw_infrastructure::retention::purge_expired_operations(
            &self.coordination,
            now,
            &Default::default(),
        )
        .map_err(|_| CommandError::Unexpected)?;
        let audit_rows = paw_infrastructure::retention::purge_expired_audit_events(
            &self.coordination,
            now,
            &Default::default(),
        )
        .map_err(|_| CommandError::Unexpected)?;
        self.audit
            .record(&paw_application::ports::audit::AuditEvent {
                workspace_id: None,
                actor_id: ctx.actor_id.clone(),
                action: action.action_key(),
                resource_type: "data",
                resource_id: Some(format!(
                    "operations={operations},attempts={attempts},audit={audit_rows}"
                )),
                outcome: paw_application::ports::audit::AuditOutcome::Allowed,
                correlation_id: ctx.correlation_id.clone(),
            });
        Ok((operations, attempts, audit_rows))
    }

    // -- Attachments (E2-04) --------------------------------------------------

    pub fn add_attachment(
        &self,
        workspace_id: &WorkspaceId,
        owner: paw_application::ports::attachment_repository::AttachmentOwner,
        owner_id: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<paw_application::ports::attachment_repository::Attachment, WorkManagementError>
    {
        let ctx = self.context();
        use_cases::attachment::add_attachment(
            &self.attachments,
            &self.file_store,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            owner,
            owner_id,
            file_name,
            bytes,
        )
    }

    pub fn list_attachments(
        &self,
        workspace_id: &WorkspaceId,
        owner: paw_application::ports::attachment_repository::AttachmentOwner,
        owner_id: &str,
    ) -> Result<Vec<paw_application::ports::attachment_repository::Attachment>, WorkManagementError>
    {
        use_cases::attachment::list_attachments(&self.attachments, workspace_id, owner, owner_id)
    }

    pub fn read_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<
        (
            paw_application::ports::attachment_repository::Attachment,
            Vec<u8>,
        ),
        WorkManagementError,
    > {
        use_cases::attachment::read_attachment(
            &self.attachments,
            &self.file_store,
            workspace_id,
            attachment_id,
        )
    }

    /// Open an attachment with the OS default application (E2-04 UI fix:
    /// WebView2 swallows programmatic `<a download>` clicks, so bytes are
    /// exported to a temp file and handed to the shell instead). The copy
    /// lives under the user's temp directory; removing it is left to OS
    /// temp cleanup and nothing is executed.
    pub fn open_attachment_externally(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<String, WorkManagementError> {
        let (attachment, bytes) = self.read_attachment(workspace_id, attachment_id)?;
        let temp_dir = std::env::temp_dir().join("paw-attachments");
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            WorkManagementError::Repository(
                paw_application::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                    e.to_string(),
                ),
            )
        })?;
        // The attachment id prefixes the name: unique per attachment and
        // free of path components (the stored file name was validated at
        // add time).
        let path = temp_dir.join(format!("{}_{}", attachment.id, attachment.file_name));
        std::fs::write(&path, &bytes).map_err(|e| {
            WorkManagementError::Repository(
                paw_application::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                    e.to_string(),
                ),
            )
        })?;
        // `start "" "path"` opens with the default application; the empty
        // title argument guards paths with spaces.
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: no console flash
            .spawn()
            .map_err(|e| {
                WorkManagementError::Repository(
                    paw_application::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                        e.to_string(),
                    ),
                )
            })?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub fn remove_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<(), WorkManagementError> {
        let ctx = self.context();
        use_cases::attachment::remove_attachment(
            &self.attachments,
            &self.file_store,
            &self.authorizer,
            &self.audit,
            &ctx,
            workspace_id,
            attachment_id,
        )
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
    /// The referenced attachment does not exist (E2-04).
    AttachmentNotFound,
    /// The attachment's owner record does not exist (E2-04).
    OwnerNotFound,
    /// The referenced operation does not exist (E3-04).
    OperationNotFound,
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
            WorkManagementError::Attachment(
                paw_application::ports::attachment_repository::AttachmentRepositoryError::AttachmentNotFound,
            ) => Self::AttachmentNotFound,
            WorkManagementError::Attachment(
                paw_application::ports::attachment_repository::AttachmentRepositoryError::OwnerNotFound,
            ) => Self::OwnerNotFound,
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

/// Map an infrastructure OperationError to the safe boundary code.
fn map_operation_error(e: paw_infrastructure::operations::OperationError) -> CommandError {
    match e {
        paw_infrastructure::operations::OperationError::NotFound(_) => {
            CommandError::OperationNotFound
        }
        paw_infrastructure::operations::OperationError::InvalidTransition { .. } => {
            CommandError::TaskStatusInvalid
        }
        _ => CommandError::Unexpected,
    }
}

fn map_classification_error(
    e: paw_infrastructure::operations::ClassificationError,
) -> CommandError {
    match e {
        paw_infrastructure::operations::ClassificationError::NotFound(_) => {
            CommandError::OperationNotFound
        }
        paw_infrastructure::operations::ClassificationError::InvalidTransition { .. } => {
            CommandError::TaskStatusInvalid
        }
        _ => CommandError::Unexpected,
    }
}

/// Desktop-side gate: authorize `action`; on denial record the audited
/// refusal and surface `denied`. Used by the E3 flows that operate directly
/// on the coordination services instead of through application use cases.
#[allow(clippy::too_many_arguments)]
fn check_gate(
    authorizer: &LocalSingleUserAuthorizer,
    audit: &SqliteAuditSink,
    ctx: &ExecutionContext,
    action: &paw_application::ports::authorization::ProtectedAction,
    workspace_id: Option<&WorkspaceId>,
    resource_type: &'static str,
    resource_id: Option<&str>,
) -> Result<(), CommandError> {
    use paw_application::ports::authorization::Authorizer;
    if authorizer.authorize(ctx, action).is_err() {
        audit.record(&paw_application::ports::audit::AuditEvent {
            workspace_id: workspace_id.cloned(),
            actor_id: ctx.actor_id.clone(),
            action: action.action_key(),
            resource_type,
            resource_id: resource_id.map(str::to_string),
            outcome: paw_application::ports::audit::AuditOutcome::Denied,
            correlation_id: ctx.correlation_id.clone(),
        });
        return Err(CommandError::Denied);
    }
    Ok(())
}

impl CommandError {
    /// Storage-level IO failures surface as the opaque `unexpected` code.
    fn from_io(e: std::io::Error) -> Self {
        let _ = e;
        Self::Unexpected
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

#[tauri::command]
pub fn dashboard(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<paw_application::ports::dashboard::DashboardSummary, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.dashboard(&workspace_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_attachment(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    owner_type: String,
    owner_id: String,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<paw_application::ports::attachment_repository::Attachment, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    let owner = paw_application::ports::attachment_repository::AttachmentOwner::parse(&owner_type)
        .ok_or(CommandError::Unexpected)?;
    Ok(api.add_attachment(&workspace_id, owner, &owner_id, &file_name, &bytes)?)
}

#[tauri::command]
pub fn list_attachments(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    owner_type: String,
    owner_id: String,
) -> Result<Vec<paw_application::ports::attachment_repository::Attachment>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    let owner = paw_application::ports::attachment_repository::AttachmentOwner::parse(&owner_type)
        .ok_or(CommandError::Unexpected)?;
    Ok(api.list_attachments(&workspace_id, owner, &owner_id)?)
}

#[tauri::command]
pub fn read_attachment(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<paw_application::ports::attachment_repository::Attachment, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    // Metadata returns as the command result; bytes are delivered through
    // the read_attachment_bytes command so a huge payload never serializes
    // together with the metadata.
    let (attachment, _bytes) = api.read_attachment(&workspace_id, &attachment_id)?;
    Ok(attachment)
}

#[tauri::command]
pub fn read_attachment_bytes(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<Vec<u8>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    let (_attachment, bytes) = api.read_attachment(&workspace_id, &attachment_id)?;
    Ok(bytes)
}

/// Export the attachment to a temp file and open it with the OS default
/// application (WebView2 blocks programmatic blob downloads; E2-04 UI fix).
#[tauri::command]
pub fn open_attachment_externally(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<String, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.open_attachment_externally(&workspace_id, &attachment_id)?)
}

#[tauri::command]
pub fn remove_attachment(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    attachment_id: String,
) -> Result<(), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    Ok(api.remove_attachment(&workspace_id, &attachment_id)?)
}

// -- Operation queue (E3-04) ------------------------------------------------

#[tauri::command]
pub fn list_operations(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<paw_infrastructure::operations::OperationRow>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_operations(&workspace_id)
}

#[tauri::command]
pub fn request_operation_cancellation(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    operation_id: String,
) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.request_operation_cancellation(&workspace_id, &operation_id)
}

#[tauri::command]
pub fn resume_operation(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    operation_id: String,
    note: String,
) -> Result<paw_infrastructure::operations::OperationRow, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.resume_operation(&workspace_id, &operation_id, &note)
}

#[tauri::command]
pub fn list_quarantined_messages(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<Vec<paw_infrastructure::outbox::QuarantinedMessage>, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.list_quarantined(&workspace_id)
}

#[tauri::command]
pub fn redrive_message(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
    message_id: String,
) -> Result<String, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.redrive_message(&workspace_id, &message_id)
}

#[tauri::command]
pub fn export_operations(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
    workspace_id: WorkspaceId,
) -> Result<String, CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.export_operations_to_temp(&workspace_id)
}

#[tauri::command]
pub fn purge_expired_data(
    state: tauri::State<'_, std::sync::Mutex<WorkManagementApi>>,
) -> Result<(usize, usize, usize), CommandError> {
    let api = state
        .lock()
        .expect("WorkManagementApi mutex must not be poisoned");
    api.purge_expired()
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
