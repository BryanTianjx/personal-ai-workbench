//! Task use cases: create, update details, status transitions, delete and
//! list (workspace-scoped, optional project/status filters). Status
//! transition legality is enforced by the domain state machine; the use case
//! turns an illegal move into a typed `TaskTransition` error.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::task_repository::TaskRepository;
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::work_management::{
    ProjectId, TASK_NOTES_MAX, Task, TaskId, TaskStatus, WorkspaceId, validate_name,
};

/// Create a task inside `workspace_id`, optionally linked to a project.
/// New tasks always start in [`TaskStatus::Todo`].
#[allow(clippy::too_many_arguments)]
pub fn create_task(
    repo: &dyn TaskRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    project_id: Option<ProjectId>,
    title: &str,
    notes: Option<&str>,
) -> Result<Task, WorkManagementError> {
    let action = ProtectedAction::CreateTask {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "task",
        None,
    )?;
    let task = Task::new(
        TaskId::new_v7(),
        workspace_id.clone(),
        project_id,
        title,
        notes,
    )?;
    match repo.insert_task(&task) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(task)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Update an existing task's title, notes and project link (status moves go
/// through [`transition_task`]). Loads first; the workspace binding is
/// immutable.
#[allow(clippy::too_many_arguments)]
pub fn update_task_details(
    repo: &dyn TaskRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    task_id: &TaskId,
    title: &str,
    notes: Option<&str>,
    project_id: Option<ProjectId>,
) -> Result<Task, WorkManagementError> {
    let action = ProtectedAction::UpdateTask {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "task",
        Some(task_id.as_str()),
    )?;
    let mut task = repo
        .find_task(workspace_id, task_id)?
        .ok_or(WorkManagementError::Task(
            crate::ports::task_repository::TaskRepositoryError::TaskNotFound,
        ))?;
    task.title = validate_name(title, 200)?;
    task.notes = match notes {
        None => None,
        Some(text) if text.trim().chars().count() > TASK_NOTES_MAX => {
            return Err(WorkManagementError::Validation(
                paw_domain::work_management::ValidationError::NotesTooLong {
                    max: TASK_NOTES_MAX,
                },
            ));
        }
        Some(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
    };
    task.project_id = project_id;
    match repo.update_task(&task) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(task)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Move a task through its status machine. The domain decides legality; an
/// illegal move is a typed, audited rejection before any repository write.
pub fn transition_task(
    repo: &dyn TaskRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    task_id: &TaskId,
    to: TaskStatus,
) -> Result<Task, WorkManagementError> {
    let action = ProtectedAction::TransitionTask {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "task",
        Some(task_id.as_str()),
    )?;
    let mut task = repo
        .find_task(workspace_id, task_id)?
        .ok_or(WorkManagementError::Task(
            crate::ports::task_repository::TaskRepositoryError::TaskNotFound,
        ))?;
    if !task.status.can_transition_to(to) {
        return Err(WorkManagementError::TaskTransition {
            from: task.status,
            to,
        });
    }
    task.status = to;
    match repo.update_task(&task) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(task)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Delete a task inside `workspace_id`.
pub fn delete_task(
    repo: &dyn TaskRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    task_id: &TaskId,
) -> Result<(), WorkManagementError> {
    let action = ProtectedAction::DeleteTask {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "task",
        Some(task_id.as_str()),
    )?;
    match repo.delete_task(workspace_id, task_id) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task_id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(())
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "task",
                Some(task_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// List the workspace's tasks, optionally filtered by project and/or status.
pub fn list_tasks(
    repo: &dyn TaskRepository,
    workspace_id: &WorkspaceId,
    project_id: Option<&ProjectId>,
    status: Option<TaskStatus>,
) -> Result<Vec<Task>, WorkManagementError> {
    Ok(repo.list_tasks(workspace_id, project_id, status)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::audit::AuditOutcome;
    use crate::use_cases::work_management::test_support::{
        FakeAudit, FakeAuthorizer, FakeTaskRepo, ctx,
    };

    #[test]
    fn task_lifecycle_through_the_state_machine() {
        let repo = FakeTaskRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let project = ProjectId::new_v7();

        let task = create_task(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            Some(project.clone()),
            "实现领域模型",
            Some("注释"),
        )
        .unwrap();
        assert_eq!(task.status, TaskStatus::Todo);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("task.create".into(), AuditOutcome::Allowed))
        );

        let started = transition_task(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &task.id,
            TaskStatus::InProgress,
        )
        .unwrap();
        assert_eq!(started.status, TaskStatus::InProgress);

        // Rework path: in_progress -> todo is legal.
        transition_task(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &task.id,
            TaskStatus::Todo,
        )
        .unwrap();
        transition_task(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &task.id,
            TaskStatus::InProgress,
        )
        .unwrap();
        let done = transition_task(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &task.id,
            TaskStatus::Done,
        )
        .unwrap();
        assert_eq!(done.status, TaskStatus::Done);

        // Terminal freeze: done -> anything is a typed TaskTransition error.
        assert_eq!(
            transition_task(
                &repo,
                &authz,
                &audit,
                &ctx("local-user"),
                &ws,
                &task.id,
                TaskStatus::Todo
            ),
            Err(WorkManagementError::TaskTransition {
                from: TaskStatus::Done,
                to: TaskStatus::Todo,
            })
        );

        assert_eq!(
            audit.events.borrow().last(),
            Some(&("task.transition".into(), AuditOutcome::Allowed))
        );
    }

    #[test]
    fn task_details_update_and_filters() {
        let repo = FakeTaskRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let project = ProjectId::new_v7();

        let task = create_task(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            None,
            "独立任务",
            None,
        )
        .unwrap();

        let linked = update_task_details(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &task.id,
            "独立任务改",
            Some("  新注释  "),
            Some(project.clone()),
        )
        .unwrap();
        assert_eq!(linked.project_id, Some(project.clone()));
        assert_eq!(linked.notes.as_deref(), Some("新注释"));

        assert_eq!(
            list_tasks(&repo, &ws, Some(&project), None).unwrap().len(),
            1
        );
        assert!(
            list_tasks(&repo, &ws, None, Some(TaskStatus::Done))
                .unwrap()
                .is_empty()
        );

        delete_task(&repo, &authz, &audit, &ctx("local-user"), &ws, &task.id).unwrap();
        assert!(repo.tasks.borrow().is_empty());
    }
}
