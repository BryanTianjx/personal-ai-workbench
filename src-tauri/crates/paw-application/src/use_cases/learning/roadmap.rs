//! Roadmap use cases (PAW-LG-001): a long-term learning direction.
//!
//! Create/list/delete over the `RoadmapRepository` port. Hard delete is
//! refused by the repository (RESTRICT) while plans still reference the
//! roadmap — that refusal surfaces as a typed `LearningRepositoryError`.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::learning_repository::RoadmapRepository;
use crate::use_cases::learning::{LearningError, audit_outcome, gate};
use paw_domain::learning::{Roadmap, RoadmapId};
use paw_domain::work_management::WorkspaceId;

/// Audit resource type used for every roadmap action.
const RESOURCE: &str = "learning_roadmap";

/// List the workspace's roadmaps (read: workspace-scoped by the port, not
/// authorization-gated).
pub fn list_roadmaps(
    repo: &dyn RoadmapRepository,
    workspace_id: &WorkspaceId,
) -> Result<Vec<Roadmap>, LearningError> {
    Ok(repo.list_roadmaps(workspace_id)?)
}

/// Create a roadmap inside `workspace_id`. The name is validated by the
/// domain before the protected side effect.
pub fn create_roadmap(
    repo: &dyn RoadmapRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    name: &str,
    description: Option<&str>,
) -> Result<Roadmap, LearningError> {
    let roadmap = Roadmap::new(RoadmapId::new_v7(), workspace_id.clone(), name, description)?;
    let action = ProtectedAction::CreateLearningRoadmap {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(roadmap.id.as_str()),
    )?;
    match repo.insert_roadmap(&roadmap) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(roadmap.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(roadmap)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(roadmap.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Delete a roadmap inside `workspace_id`. A roadmap that still owns plans is
/// RESTRICT-blocked by the schema; the caller sees a typed rejection and the
/// plans are never cascaded away.
pub fn delete_roadmap(
    repo: &dyn RoadmapRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    roadmap_id: &RoadmapId,
) -> Result<(), LearningError> {
    let action = ProtectedAction::DeleteLearningRoadmap {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(roadmap_id.as_str()),
    )?;
    match repo.delete_roadmap(workspace_id, roadmap_id) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(roadmap_id.as_str()),
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
                Some(roadmap_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::work_management::test_support::{FakeAudit, FakeAuthorizer, ctx};
    use paw_domain::work_management::ValidationError;

    use crate::use_cases::learning::test_support::FakeRoadmapRepo;

    #[test]
    fn roadmap_create_is_gated_audited_and_workspace_scoped() {
        let repo = FakeRoadmapRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let other = WorkspaceId::new_v7();

        let created = create_roadmap(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            "Agent & Runtime Engineering",
            Some("长期方向"),
        )
        .unwrap();
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.roadmap.create".into(), AuditOutcome::Allowed))
        );
        assert_eq!(list_roadmaps(&repo, &ws).unwrap().len(), 1);
        // The other workspace never sees the record (scope on every query).
        assert!(list_roadmaps(&repo, &other).unwrap().is_empty());

        // Denied actor: audited refusal, nothing written.
        let denied = create_roadmap(
            &repo,
            &authz,
            &audit,
            &ctx("intruder"),
            &ws,
            "非法路线",
            None,
        );
        assert_eq!(
            denied,
            Err(LearningError::Authorization(
                crate::ports::authorization::AuthorizationError::Denied
            ))
        );
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.roadmap.create".into(), AuditOutcome::Denied))
        );
        assert_eq!(list_roadmaps(&repo, &ws).unwrap().len(), 1);

        // Delete removes exactly one row and is audited.
        delete_roadmap(&repo, &authz, &audit, &ctx("local-user"), &ws, &created.id).unwrap();
        assert!(list_roadmaps(&repo, &ws).unwrap().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.roadmap.delete".into(), AuditOutcome::Allowed))
        );
    }

    #[test]
    fn roadmap_name_validation_and_missing_delete_are_typed() {
        let repo = FakeRoadmapRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();

        assert!(matches!(
            create_roadmap(&repo, &authz, &audit, &ctx("local-user"), &ws, "   ", None),
            Err(LearningError::Validation(ValidationError::EmptyName))
        ));
        assert!(matches!(
            delete_roadmap(
                &repo,
                &authz,
                &audit,
                &ctx("local-user"),
                &ws,
                &RoadmapId::new_v7()
            ),
            Err(LearningError::Repository(
                crate::ports::learning_repository::LearningRepositoryError::RoadmapNotFound
            ))
        ));
    }
}
