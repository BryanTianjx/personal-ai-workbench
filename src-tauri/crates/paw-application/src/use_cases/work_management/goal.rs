//! Goal use cases: create, update, delete and list (workspace-scoped,
//! optional Area filter). Area links are validated by the composite schema
//! keys — a cross-workspace link surfaces as a typed invariant rejection.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::goal_repository::GoalRepository;
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::work_management::{AreaId, Goal, GoalId, WorkspaceId, validate_name};

/// Create a goal inside `workspace_id`, optionally bound to an Area of the
/// same workspace.
#[allow(clippy::too_many_arguments)]
pub fn create_goal(
    repo: &dyn GoalRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    area_id: Option<AreaId>,
    name: &str,
    description: Option<&str>,
) -> Result<Goal, WorkManagementError> {
    let action = ProtectedAction::CreateGoal {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "goal",
        None,
    )?;
    let goal = Goal::new(
        GoalId::new_v7(),
        workspace_id.clone(),
        area_id,
        name,
        description,
    )?;
    match repo.insert_goal(&goal) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "goal",
                Some(goal.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(goal)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "goal",
                Some(goal.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Update an existing goal: Area link, name and description. Loads first
/// (typed not-found); the workspace binding is immutable.
#[allow(clippy::too_many_arguments)]
pub fn update_goal(
    repo: &dyn GoalRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    goal_id: &GoalId,
    area_id: Option<AreaId>,
    name: &str,
    description: Option<&str>,
) -> Result<Goal, WorkManagementError> {
    let action = ProtectedAction::UpdateGoal {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "goal",
        Some(goal_id.as_str()),
    )?;
    let mut goal = repo
        .find_goal(workspace_id, goal_id)?
        .ok_or(WorkManagementError::Goal(
            crate::ports::goal_repository::GoalRepositoryError::GoalNotFound,
        ))?;
    goal.area_id = area_id;
    goal.name = validate_name(name, 200)?;
    goal.description = description.map(str::to_string);
    match repo.update_goal(&goal) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "goal",
                Some(goal.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(goal)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "goal",
                Some(goal.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Delete a goal. Projects referencing it keep their records (RESTRICT
/// surfaces a typed refusal); unbind explicitly first.
pub fn delete_goal(
    repo: &dyn GoalRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    goal_id: &GoalId,
) -> Result<(), WorkManagementError> {
    let action = ProtectedAction::DeleteGoal {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "goal",
        Some(goal_id.as_str()),
    )?;
    match repo.delete_goal(workspace_id, goal_id) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "goal",
                Some(goal_id.as_str()),
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
                "goal",
                Some(goal_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// List the workspace's goals, optionally filtered by Area.
pub fn list_goals(
    repo: &dyn GoalRepository,
    workspace_id: &WorkspaceId,
    area_id: Option<&AreaId>,
) -> Result<Vec<Goal>, WorkManagementError> {
    Ok(repo.list_goals(workspace_id, area_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::audit::AuditOutcome;
    use crate::use_cases::work_management::test_support::{
        FakeAudit, FakeAuthorizer, FakeGoalRepo, ctx,
    };

    #[test]
    fn goal_crud_round_trip_with_audit() {
        let repo = FakeGoalRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let area = AreaId::new_v7();

        let goal = create_goal(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            Some(area.clone()),
            "发布 Alpha",
            None,
        )
        .unwrap();
        assert_eq!(goal.area_id, Some(area.clone()));
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("goal.create".into(), AuditOutcome::Allowed))
        );

        let updated = update_goal(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &goal.id,
            None,
            "发布 Alpha 0.2",
            None,
        )
        .unwrap();
        assert_eq!(updated.area_id, None);
        assert_eq!(updated.name, "发布 Alpha 0.2");

        assert_eq!(list_goals(&repo, &ws, None).unwrap().len(), 1);

        delete_goal(&repo, &authz, &audit, &ctx("local-user"), &ws, &goal.id).unwrap();
        assert!(repo.goals.borrow().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("goal.delete".into(), AuditOutcome::Allowed))
        );
    }
}
