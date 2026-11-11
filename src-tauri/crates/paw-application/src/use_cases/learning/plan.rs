//! Plan use cases (PAW-LG-002): a staged learning plan under a roadmap.
//!
//! Create/list/status-change over the `PlanRepository` port. The roadmap
//! binding is immutable and workspace-consistent (composite key); a plan
//! created under a roadmap of another workspace is refused by the schema.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::learning_repository::{LearningRepositoryError, PlanRepository};
use crate::use_cases::learning::{LearningError, audit_outcome, gate};
use paw_domain::learning::{Plan, PlanId, PlanStatus, RoadmapId};
use paw_domain::work_management::WorkspaceId;

/// Audit resource type used for every plan action.
const RESOURCE: &str = "learning_plan";

/// List the workspace's plans, optionally filtered by roadmap (read:
/// workspace-scoped by the port, not authorization-gated).
pub fn list_plans(
    repo: &dyn PlanRepository,
    workspace_id: &WorkspaceId,
    roadmap_id: Option<&RoadmapId>,
) -> Result<Vec<Plan>, LearningError> {
    Ok(repo.list_plans(workspace_id, roadmap_id)?)
}

/// Create a plan under `roadmap_id`. New plans always start in
/// [`PlanStatus::Draft`].
#[allow(clippy::too_many_arguments)]
pub fn create_plan(
    repo: &dyn PlanRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    roadmap_id: &RoadmapId,
    name: &str,
    description: Option<&str>,
) -> Result<Plan, LearningError> {
    let plan = Plan::new(
        PlanId::new_v7(),
        workspace_id.clone(),
        roadmap_id.clone(),
        name,
        description,
    )?;
    let action = ProtectedAction::CreateLearningPlan {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(plan.id.as_str()),
    )?;
    match repo.insert_plan(&plan) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(plan.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(plan)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(plan.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Change a plan's lifecycle status (PRD §3.2). Loads first: a plan outside
/// the workspace scope is a typed not-found, never a cross-workspace write.
pub fn update_plan_status(
    repo: &dyn PlanRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    plan_id: &PlanId,
    status: PlanStatus,
) -> Result<Plan, LearningError> {
    let mut plan = repo
        .find_plan(workspace_id, plan_id)?
        .ok_or(LearningError::Repository(
            LearningRepositoryError::PlanNotFound,
        ))?;
    plan.status = status;
    let action = ProtectedAction::UpdateLearningPlan {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(plan.id.as_str()),
    )?;
    match repo.update_plan(&plan) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(plan.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(plan)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(plan.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::learning::test_support::{FakePlanRepo, FakeRoadmapRepo};
    use crate::use_cases::work_management::test_support::{FakeAudit, FakeAuthorizer, ctx};

    fn seed_roadmap(repo: &FakeRoadmapRepo, ws: &WorkspaceId) -> RoadmapId {
        let roadmap =
            paw_domain::learning::Roadmap::new(RoadmapId::new_v7(), ws.clone(), "路线", None)
                .unwrap();
        let id = roadmap.id.clone();
        repo.roadmaps.borrow_mut().push(roadmap);
        id
    }

    #[test]
    fn plan_lifecycle_is_gated_and_scoped() {
        let roadmaps = FakeRoadmapRepo::default();
        let plans = FakePlanRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let other = WorkspaceId::new_v7();
        let roadmap_id = seed_roadmap(&roadmaps, &ws);

        let plan = create_plan(
            &plans,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &roadmap_id,
            "Q1 计划",
            None,
        )
        .unwrap();
        assert_eq!(plan.status, PlanStatus::Draft);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.plan.create".into(), AuditOutcome::Allowed))
        );

        let active = update_plan_status(
            &plans,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &plan.id,
            PlanStatus::Active,
        )
        .unwrap();
        assert_eq!(active.status, PlanStatus::Active);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.plan.update".into(), AuditOutcome::Allowed))
        );

        // Scope: the other workspace sees nothing and cannot mutate the plan.
        assert!(list_plans(&plans, &other, None).unwrap().is_empty());
        assert_eq!(
            update_plan_status(
                &plans,
                &authz,
                &audit,
                &ctx("local-user"),
                &other,
                &plan.id,
                PlanStatus::Paused
            ),
            Err(LearningError::Repository(
                crate::ports::learning_repository::LearningRepositoryError::PlanNotFound
            ))
        );

        // Denied actor: audited refusal.
        assert_eq!(
            update_plan_status(
                &plans,
                &authz,
                &audit,
                &ctx("intruder"),
                &ws,
                &plan.id,
                PlanStatus::Paused
            ),
            Err(LearningError::Authorization(
                crate::ports::authorization::AuthorizationError::Denied
            ))
        );
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.plan.update".into(), AuditOutcome::Denied))
        );
    }

    #[test]
    fn plan_list_filters_by_roadmap() {
        let plans = FakePlanRepo::default();
        let ws = WorkspaceId::new_v7();
        let r1 = seed_roadmap(&FakeRoadmapRepo::default(), &ws);
        let r2 = seed_roadmap(&FakeRoadmapRepo::default(), &ws);
        for (roadmap_id, name) in [(&r1, "计划一"), (&r2, "计划二")] {
            let plan =
                Plan::new(PlanId::new_v7(), ws.clone(), roadmap_id.clone(), name, None).unwrap();
            plans.plans.borrow_mut().push(plan);
        }
        assert_eq!(list_plans(&plans, &ws, None).unwrap().len(), 2);
        assert_eq!(list_plans(&plans, &ws, Some(&r1)).unwrap().len(), 1);
    }
}
