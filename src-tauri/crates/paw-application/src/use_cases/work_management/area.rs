//! Area use cases: create, update, delete and list (workspace-scoped).

use crate::ports::area_repository::AreaRepository;
use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::work_management::{Area, AreaId, WorkspaceId, validate_name};

/// Create an area inside `workspace_id`.
pub fn create_area(
    repo: &dyn AreaRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    name: &str,
    description: Option<&str>,
) -> Result<Area, WorkManagementError> {
    let action = ProtectedAction::CreateArea {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "area",
        None,
    )?;
    let area = Area::new(AreaId::new_v7(), workspace_id.clone(), name, description)?;
    match repo.insert_area(&area) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "area",
                Some(area.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(area)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "area",
                Some(area.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Update an existing area's name/description. Loads first (typed
/// not-found); the workspace binding is immutable.
#[allow(clippy::too_many_arguments)]
pub fn update_area(
    repo: &dyn AreaRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    area_id: &AreaId,
    name: &str,
    description: Option<&str>,
) -> Result<Area, WorkManagementError> {
    let action = ProtectedAction::UpdateArea {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "area",
        Some(area_id.as_str()),
    )?;
    let mut area = repo
        .find_area(workspace_id, area_id)?
        .ok_or(WorkManagementError::Area(
            crate::ports::area_repository::AreaRepositoryError::AreaNotFound,
        ))?;
    area.name = validate_name(name, 120)?;
    area.description = description.map(str::to_string);
    match repo.update_area(&area) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "area",
                Some(area.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(area)
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "area",
                Some(area.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Delete an area. References from goals/projects surface as typed
/// rejections (RESTRICT); unbind explicitly first.
pub fn delete_area(
    repo: &dyn AreaRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    area_id: &AreaId,
) -> Result<(), WorkManagementError> {
    let action = ProtectedAction::DeleteArea {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "area",
        Some(area_id.as_str()),
    )?;
    match repo.delete_area(workspace_id, area_id) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "area",
                Some(area_id.as_str()),
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
                "area",
                Some(area_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// List the workspace's areas.
pub fn list_areas(
    repo: &dyn AreaRepository,
    workspace_id: &WorkspaceId,
) -> Result<Vec<Area>, WorkManagementError> {
    Ok(repo.list_areas(workspace_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::audit::AuditOutcome;
    use crate::use_cases::work_management::test_support::{
        FakeAreaRepo, FakeAudit, FakeAuthorizer, ctx,
    };

    #[test]
    fn area_crud_round_trip_with_audit() {
        let repo = FakeAreaRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();

        let area =
            create_area(&repo, &authz, &audit, &ctx("local-user"), &ws, "工程", None).unwrap();
        assert_eq!(area.name, "工程");
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("area.create".into(), AuditOutcome::Allowed))
        );

        let updated = update_area(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &area.id,
            "工程改",
            Some("d"),
        )
        .unwrap();
        assert_eq!(updated.name, "工程改");
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("area.update".into(), AuditOutcome::Allowed))
        );

        delete_area(&repo, &authz, &audit, &ctx("local-user"), &ws, &area.id).unwrap();
        assert!(repo.list_areas(&ws).unwrap().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("area.delete".into(), AuditOutcome::Allowed))
        );
    }

    #[test]
    fn area_validation_and_denial() {
        let repo = FakeAreaRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();

        // Validation before the repository.
        assert!(matches!(
            create_area(&repo, &authz, &audit, &ctx("local-user"), &ws, "  ", None),
            Err(WorkManagementError::Validation(_))
        ));
        assert!(repo.areas.borrow().is_empty());

        // Deny-by-default: nothing written, refusal audited.
        assert!(matches!(
            create_area(&repo, &authz, &audit, &ctx("intruder"), &ws, "x", None),
            Err(WorkManagementError::Authorization(_))
        ));
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("area.create".into(), AuditOutcome::Denied))
        );
    }
}
