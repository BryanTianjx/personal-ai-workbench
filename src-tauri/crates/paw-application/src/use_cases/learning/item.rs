//! Learning Item use cases (PAW-LG-003, PAW-LG-006): a learnable,
//! practicable and verifiable target under a plan.
//!
//! Create/list/transition/link are Learning-owned operations over the
//! `ItemRepository` port. `request_task_for_item` is the cross-domain
//! action-step flow: the Task is created by the Work Management use case
//! (its own authorization gate and audit row) and only the Item → Task link
//! is written by Learning, so Task ownership stays with Work Management
//! (PAW-LG-006, ADR-017).

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::learning_repository::{ItemRepository, LearningRepositoryError};
use crate::ports::task_repository::TaskRepository;
use crate::use_cases::learning::{LearningError, audit_outcome, gate};
use crate::use_cases::work_management::task::create_task as create_work_management_task;
use paw_domain::learning::{Item, ItemId, ItemKind, ItemStatus, PlanId};
use paw_domain::work_management::{ProjectId, Task, WorkspaceId};

/// Audit resource type used for every item action.
const RESOURCE: &str = "learning_item";

/// List the workspace's items, optionally filtered by plan (read:
/// workspace-scoped by the port, not authorization-gated). The status filter
/// is not exposed on this command surface, so it stays `None`.
pub fn list_items(
    repo: &dyn ItemRepository,
    workspace_id: &WorkspaceId,
    plan_id: Option<&PlanId>,
) -> Result<Vec<Item>, LearningError> {
    Ok(repo.list_items(workspace_id, plan_id, None)?)
}

/// Create an item under `plan_id`. New items always start in
/// [`ItemStatus::Todo`].
#[allow(clippy::too_many_arguments)]
pub fn create_item(
    repo: &dyn ItemRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    plan_id: &PlanId,
    kind: ItemKind,
    title: &str,
    expected_outcome: Option<&str>,
    acceptance_criteria: Option<&str>,
) -> Result<Item, LearningError> {
    let item = Item::new(
        ItemId::new_v7(),
        workspace_id.clone(),
        plan_id.clone(),
        kind,
        title,
        expected_outcome,
        acceptance_criteria,
        None,
    )?;
    let action = ProtectedAction::CreateLearningItem {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(item.id.as_str()),
    )?;
    match repo.insert_item(&item) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(item)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Move an item through its status machine. The domain decides legality; an
/// illegal move is a typed, audited rejection before any repository write.
pub fn transition_item(
    repo: &dyn ItemRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    item_id: &ItemId,
    to: ItemStatus,
) -> Result<Item, LearningError> {
    let mut item = repo
        .find_item(workspace_id, item_id)?
        .ok_or(LearningError::Repository(
            LearningRepositoryError::ItemNotFound,
        ))?;
    if !item.status.can_transition_to(to) {
        return Err(LearningError::ItemTransition {
            from: item.status,
            to,
        });
    }
    item.status = to;
    let action = ProtectedAction::UpdateLearningItem {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(item.id.as_str()),
    )?;
    match repo.update_item(&item) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(item)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Record the `applies_to` link between an item and a Work Management
/// project. The project must live in the same workspace (composite key); a
/// cross-workspace id is refused by the schema and surfaces as a typed
/// invariant rejection.
pub fn link_item_to_project(
    repo: &dyn ItemRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    item_id: &ItemId,
    project_id: &ProjectId,
) -> Result<(), LearningError> {
    let action = ProtectedAction::LinkLearningItemProject {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(item_id.as_str()),
    )?;
    match repo.attach_item_to_project(workspace_id, item_id, project_id) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item_id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(())
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// PAW-LG-006 (CreateTaskForLearningItem semantics): create a Work
/// Management task for a learning item and record the action-step link.
///
/// The task is created through the Work Management use case, so the
/// `task.create` gate and audit row are the same as for any other task; this
/// use case adds its own `learning.item.create_task` gate. The first linked
/// project (if any) becomes the task's project. The task stays
/// Work-Management-owned: Learning never mutates it.
#[allow(clippy::too_many_arguments)]
pub fn request_task_for_item(
    items: &dyn ItemRepository,
    tasks: &dyn TaskRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    item_id: &ItemId,
    title: &str,
) -> Result<Task, LearningError> {
    let action = ProtectedAction::CreateTaskForLearningItem {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(item_id.as_str()),
    )?;
    let item = items
        .find_item(workspace_id, item_id)?
        .ok_or(LearningError::Repository(
            LearningRepositoryError::ItemNotFound,
        ))?;
    let projects = items.list_projects_for_item(workspace_id, item_id)?;
    // Delegated Work Management use case: it runs the `task.create` gate and
    // writes its own audit rows. A failure is audited here as an error for
    // this action and surfaces typed.
    let task = match create_work_management_task(
        tasks,
        authorizer,
        audit_sink,
        ctx,
        workspace_id,
        projects.first().cloned(),
        title,
        None,
    ) {
        Ok(task) => task,
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item.id.as_str()),
                AuditOutcome::Error,
            );
            return Err(e.into());
        }
    };
    match items.attach_task_to_item(workspace_id, item_id, &task.id) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(task)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(item.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::learning_repository::PlanRepository;
    use crate::use_cases::learning::test_support::{FakeItemRepo, FakePlanRepo};
    use crate::use_cases::work_management::test_support::{
        FakeAudit, FakeAuthorizer, FakeTaskRepo, ctx,
    };

    fn seed_plan(plans: &FakePlanRepo, ws: &WorkspaceId) -> PlanId {
        let roadmap_id = paw_domain::learning::RoadmapId::new_v7();
        let plan =
            paw_domain::learning::Plan::new(PlanId::new_v7(), ws.clone(), roadmap_id, "计划", None)
                .unwrap();
        let id = plan.id.clone();
        plans.insert_plan(&plan).unwrap();
        id
    }

    #[test]
    fn item_create_and_transition_follow_the_state_machine() {
        let plans = FakePlanRepo::default();
        let items = FakeItemRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let other = WorkspaceId::new_v7();
        let plan_id = seed_plan(&plans, &ws);

        let item = create_item(
            &items,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &plan_id,
            ItemKind::Implementation,
            "实现 durable execution 最小示例",
            Some("可运行的示例 + 测试"),
            Some("至少覆盖一次恢复路径"),
        )
        .unwrap();
        assert_eq!(item.status, ItemStatus::Todo);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.item.create".into(), AuditOutcome::Allowed))
        );
        assert_eq!(list_items(&items, &ws, Some(&plan_id)).unwrap().len(), 1);
        assert!(list_items(&items, &other, None).unwrap().is_empty());

        let started = transition_item(
            &items,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &item.id,
            ItemStatus::InProgress,
        )
        .unwrap();
        assert_eq!(started.status, ItemStatus::InProgress);

        // Terminal state is frozen by the domain machine: typed rejection,
        // no write, no Allowed audit row.
        let done = transition_item(
            &items,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &item.id,
            ItemStatus::Done,
        )
        .unwrap();
        assert_eq!(done.status, ItemStatus::Done);
        assert_eq!(
            transition_item(
                &items,
                &authz,
                &audit,
                &ctx("local-user"),
                &ws,
                &item.id,
                ItemStatus::Todo
            ),
            Err(LearningError::ItemTransition {
                from: ItemStatus::Done,
                to: ItemStatus::Todo,
            })
        );

        // Scope: another workspace cannot transition the item.
        assert_eq!(
            transition_item(
                &items,
                &authz,
                &audit,
                &ctx("local-user"),
                &other,
                &item.id,
                ItemStatus::Todo
            ),
            Err(LearningError::Repository(
                LearningRepositoryError::ItemNotFound
            ))
        );
    }

    #[test]
    fn item_project_link_is_gated_and_audited() {
        let plans = FakePlanRepo::default();
        let items = FakeItemRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let plan_id = seed_plan(&plans, &ws);
        let item = create_item(
            &items,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &plan_id,
            ItemKind::Experiment,
            "实验",
            None,
            None,
        )
        .unwrap();
        let project = ProjectId::new_v7();

        link_item_to_project(
            &items,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &item.id,
            &project,
        )
        .unwrap();
        assert_eq!(
            items.list_projects_for_item(&ws, &item.id).unwrap(),
            vec![project.clone()]
        );
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.item.link_project".into(), AuditOutcome::Allowed))
        );

        // Denied actor: audited refusal, no new link.
        assert_eq!(
            link_item_to_project(
                &items,
                &authz,
                &audit,
                &ctx("intruder"),
                &ws,
                &item.id,
                &ProjectId::new_v7()
            ),
            Err(LearningError::Authorization(
                crate::ports::authorization::AuthorizationError::Denied
            ))
        );
        assert_eq!(
            items.list_projects_for_item(&ws, &item.id).unwrap().len(),
            1
        );
    }

    #[test]
    fn request_task_for_item_keeps_task_in_work_management() {
        let plans = FakePlanRepo::default();
        let items = FakeItemRepo::default();
        let tasks = FakeTaskRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let plan_id = seed_plan(&plans, &ws);
        let item = create_item(
            &items,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &plan_id,
            ItemKind::Implementation,
            "实现示例",
            None,
            None,
        )
        .unwrap();
        let project = ProjectId::new_v7();
        items
            .attach_item_to_project(&ws, &item.id, &project)
            .unwrap();

        let task = request_task_for_item(
            &items,
            &tasks,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &item.id,
            "学习项动作：最小示例",
        )
        .unwrap();

        // The task exists in the Work Management repository, bound to the
        // first linked project, and the action-step link is recorded.
        assert_eq!(tasks.tasks.borrow().len(), 1);
        assert_eq!(tasks.tasks.borrow()[0].id, task.id);
        assert_eq!(task.project_id, Some(project));
        assert_eq!(
            items.list_tasks_for_item(&ws, &item.id).unwrap(),
            vec![task.id.clone()]
        );
        // Both gates ran: the Work Management task gate and the Learning
        // orchestration gate.
        {
            let checks = authz.checks.borrow();
            let keys: Vec<&str> = checks.iter().map(|(_, key)| *key).collect();
            assert!(keys.contains(&"task.create"));
            assert!(keys.contains(&"learning.item.create_task"));
        }
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.item.create_task".into(), AuditOutcome::Allowed))
        );

        // A denied actor creates nothing: no task row, no link.
        let denying = FakeAuthorizer::denying();
        let before = tasks.tasks.borrow().len();
        assert_eq!(
            request_task_for_item(
                &items,
                &tasks,
                &denying,
                &audit,
                &ctx("intruder"),
                &ws,
                &item.id,
                "不该出现的任务"
            ),
            Err(LearningError::Authorization(
                crate::ports::authorization::AuthorizationError::Denied
            ))
        );
        assert_eq!(tasks.tasks.borrow().len(), before);

        // A task for an item outside the workspace is refused before any
        // task is created.
        assert_eq!(
            request_task_for_item(
                &items,
                &tasks,
                &authz,
                &audit,
                &ctx("local-user"),
                &WorkspaceId::new_v7(),
                &item.id,
                "越界任务"
            ),
            Err(LearningError::Repository(
                LearningRepositoryError::ItemNotFound
            ))
        );
        assert_eq!(tasks.tasks.borrow().len(), before);
    }
}
