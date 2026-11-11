//! Learning Session use cases (PAW-LG-004): one actually-occurred learning
//! activity.
//!
//! Sessions are historical records: they are appended, never updated or
//! deleted (cross-domain-data-contracts §7). The port therefore has no
//! update/delete method, and this module exposes only append and list. The
//! occurrence timestamp is supplied by the boundary (it is a clock reading,
//! not a business decision), so the use case stays free of infrastructure
//! time sources.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::learning_repository::SessionRepository;
use crate::use_cases::learning::{LearningError, audit_outcome, gate};
use paw_domain::learning::{ItemId, Session, SessionId};
use paw_domain::work_management::WorkspaceId;

/// Audit resource type used for every session action.
const RESOURCE: &str = "learning_session";

/// Append one learning session for `item_id`. Multiple sessions of the same
/// item never overwrite each other (append-only history).
#[allow(clippy::too_many_arguments)]
pub fn append_session(
    repo: &dyn SessionRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    item_id: &ItemId,
    method: Option<&str>,
    duration_minutes: Option<i64>,
    summary: Option<&str>,
    difficulty: Option<&str>,
    takeaway: Option<&str>,
    next_step: Option<&str>,
    occurred_at: i64,
) -> Result<Session, LearningError> {
    let session = Session::new(
        SessionId::new_v7(),
        workspace_id.clone(),
        item_id.clone(),
        None,
        None,
        method,
        duration_minutes,
        summary,
        difficulty,
        takeaway,
        next_step,
        occurred_at,
    )?;
    let action = ProtectedAction::AppendLearningSession {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        RESOURCE,
        Some(session.id.as_str()),
    )?;
    match repo.append_session(&session) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(session.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(session)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                RESOURCE,
                Some(session.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// List sessions of one item, or the whole workspace when `item_id` is
/// `None` (read: workspace-scoped by the port, not authorization-gated).
pub fn list_sessions(
    repo: &dyn SessionRepository,
    workspace_id: &WorkspaceId,
    item_id: Option<&ItemId>,
) -> Result<Vec<Session>, LearningError> {
    Ok(repo.list_sessions(workspace_id, item_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::learning_repository::ItemRepository;
    use crate::use_cases::learning::test_support::{FakeItemRepo, FakeSessionRepo};
    use crate::use_cases::work_management::test_support::{FakeAudit, FakeAuthorizer, ctx};
    use paw_domain::learning::ItemKind;
    use paw_domain::work_management::ValidationError;

    fn seed_item(items: &FakeItemRepo, ws: &WorkspaceId) -> ItemId {
        let item = paw_domain::learning::Item::new(
            ItemId::new_v7(),
            ws.clone(),
            paw_domain::learning::PlanId::new_v7(),
            ItemKind::Reading,
            "阅读",
            None,
            None,
            None,
        )
        .unwrap();
        let id = item.id.clone();
        items.insert_item(&item).unwrap();
        id
    }

    #[test]
    fn sessions_append_only_and_stay_workspace_scoped() {
        let items = FakeItemRepo::default();
        let sessions = FakeSessionRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let other = WorkspaceId::new_v7();
        let item_id = seed_item(&items, &ws);

        let first = append_session(
            &sessions,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &item_id,
            Some("实验"),
            Some(45),
            Some("第一次实验"),
            Some("恢复路径不明确"),
            Some("理解了 checkpoint 语义"),
            Some("补一次崩溃恢复"),
            1_000,
        )
        .unwrap();
        append_session(
            &sessions,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &item_id,
            Some("阅读"),
            None,
            Some("第二次学习"),
            None,
            None,
            None,
            2_000,
        )
        .unwrap();

        let listed = list_sessions(&sessions, &ws, Some(&item_id)).unwrap();
        assert_eq!(listed.len(), 2, "sessions never overwrite each other");
        assert_eq!(listed[0].id, first.id);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.session.append".into(), AuditOutcome::Allowed))
        );
        assert!(list_sessions(&sessions, &other, None).unwrap().is_empty());
    }

    #[test]
    fn session_validation_and_denial_are_typed() {
        let sessions = FakeSessionRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let item_id = ItemId::new_v7();

        // Negative duration is refused by the domain before the gate.
        assert!(matches!(
            append_session(
                &sessions,
                &authz,
                &audit,
                &ctx("local-user"),
                &ws,
                &item_id,
                None,
                Some(-1),
                None,
                None,
                None,
                None,
                1
            ),
            Err(LearningError::Validation(
                ValidationError::NotesTooLong { .. }
            ))
        ));

        // Denied actor: audited refusal, nothing appended.
        assert_eq!(
            append_session(
                &sessions,
                &authz,
                &audit,
                &ctx("intruder"),
                &ws,
                &item_id,
                None,
                None,
                Some("非法记录"),
                None,
                None,
                None,
                1
            ),
            Err(LearningError::Authorization(
                crate::ports::authorization::AuthorizationError::Denied
            ))
        );
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("learning.session.append".into(), AuditOutcome::Denied))
        );
        assert!(sessions.sessions.borrow().is_empty());
    }
}
