//! Security adapters for the E1-09 gates: the SQLite audit sink and the
//! local single-user authorization policy.
//!
//! - `SqliteAuditSink` persists audit events into `audit_events`
//!   (migration 0003). Recording is best-effort by port contract: an audit
//!   write failure never fails the business action it observes. Rows carry
//!   only stable keys, opaque ids and outcomes — no driver messages, no
//!   payloads — so the trail cannot leak secrets by construction.
//! - `LocalSingleUserAuthorizer` is the E1 policy: the single local actor
//!   holds the Work Management capabilities; every other (actor, action)
//!   pair is denied. Deny-by-default; the capability set grows only with
//!   explicit contracts (PAW-S-004 governs the Alpha 0.2 Agent gate).

use rusqlite::{Connection, params};

use paw_application::ports::audit::{AuditEvent, AuditSink};
use paw_application::ports::authorization::{
    AuthorizationError, Authorizer, ExecutionContext, GrantStore, ProtectedAction,
};

use crate::sqlite;

/// Actor id of the single local user for E1. Real principals and
/// credential-backed identity arrive with later gates; the id is an
/// initiating principal, never a grant (`E0.3.2` §2).
pub const LOCAL_ACTOR_ID: &str = "local-user";

/// SQLite-backed audit sink over its own connection (WAL allows a second
/// short-lived connection alongside the repository's).
pub struct SqliteAuditSink {
    conn: Connection,
}

impl SqliteAuditSink {
    pub fn open(db_path: &std::path::Path) -> Result<Self, sqlite::Error> {
        Ok(Self {
            conn: sqlite::open(db_path)?,
        })
    }
}

impl AuditSink for SqliteAuditSink {
    fn record(&self, event: &AuditEvent) {
        let result = (|| -> Result<(), rusqlite::Error> {
            self.conn.execute(
                "INSERT INTO audit_events (id, workspace_id, actor_id, action, resource_type, \
                 resource_id, outcome, correlation_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    uuid::Uuid::now_v7().to_string(),
                    event.workspace_id.as_ref().map(|w| w.as_str().to_string()),
                    event.actor_id,
                    event.action,
                    event.resource_type,
                    event.resource_id,
                    event.outcome.as_str(),
                    event.correlation_id,
                    sqlite::now_ms().unwrap_or_default(),
                ],
            )?;
            Ok(())
        })();
        // Best-effort per the port contract; never propagate into the flow.
        if let Err(_e) = result {
            // Deliberately swallowed: audit is a hook, not a gate.
        }
    }
}

/// E1 authorization policy: the single local actor holds the Work
/// Management capabilities; deny everything else.
pub struct LocalSingleUserAuthorizer;

/// The local single-user policy body (E1/E2/E3/E4 surface plus the E5
/// agent-administration actions). Shared by [`LocalSingleUserAuthorizer`]
/// and [`GovernedAuthorizer`]; `CapabilityInvoke` is deliberately absent —
/// the human owner invokes capabilities through their concrete commands,
/// never through the agent pipeline identity.
fn local_policy(action: &ProtectedAction) -> Result<(), AuthorizationError> {
    match action {
        ProtectedAction::CreateWorkspace
        | ProtectedAction::CreateProject { .. }
        | ProtectedAction::UpdateProject { .. }
        | ProtectedAction::DeleteProject { .. }
        | ProtectedAction::DeleteWorkspace { .. }
        | ProtectedAction::CreateArea { .. }
        | ProtectedAction::UpdateArea { .. }
        | ProtectedAction::DeleteArea { .. }
        | ProtectedAction::CreateGoal { .. }
        | ProtectedAction::UpdateGoal { .. }
        | ProtectedAction::DeleteGoal { .. }
        | ProtectedAction::CreateTask { .. }
        | ProtectedAction::UpdateTask { .. }
        | ProtectedAction::TransitionTask { .. }
        | ProtectedAction::DeleteTask { .. }
        | ProtectedAction::CreateAttachment { .. }
        | ProtectedAction::DeleteAttachment { .. }
        | ProtectedAction::CancelOperation { .. }
        | ProtectedAction::ResumeOperation { .. }
        | ProtectedAction::RedriveMessage { .. }
        | ProtectedAction::ExportWorkspaceData { .. }
        | ProtectedAction::CreateLearningRoadmap { .. }
        | ProtectedAction::UpdateLearningRoadmap { .. }
        | ProtectedAction::DeleteLearningRoadmap { .. }
        | ProtectedAction::CreateLearningPlan { .. }
        | ProtectedAction::UpdateLearningPlan { .. }
        | ProtectedAction::DeleteLearningPlan { .. }
        | ProtectedAction::CreateLearningItem { .. }
        | ProtectedAction::UpdateLearningItem { .. }
        | ProtectedAction::DeleteLearningItem { .. }
        | ProtectedAction::AppendLearningSession { .. }
        | ProtectedAction::LinkLearningItemProject { .. }
        | ProtectedAction::CreateTaskForLearningItem { .. }
        | ProtectedAction::CreateKnowledgeRef { .. }
        | ProtectedAction::UpdateKnowledgeRef { .. }
        | ProtectedAction::DeleteKnowledgeRef { .. }
        | ProtectedAction::SetKnowledgeAvailability { .. }
        | ProtectedAction::RegisterAgent { .. }
        | ProtectedAction::UpdateAgentRegistration { .. }
        | ProtectedAction::ProposeAgentInvocation { .. }
        | ProtectedAction::DecideApproval { .. }
        | ProtectedAction::RevokeApproval { .. }
        | ProtectedAction::ReconcileAgentExecution { .. }
        | ProtectedAction::CreateAgentGrant { .. }
        | ProtectedAction::RevokeAgentGrant { .. }
        | ProtectedAction::PurgeExpiredData => Ok(()),
        // The pipeline identity is reserved for agent subjects; the human
        // owner's concrete commands carry their own actions.
        ProtectedAction::CapabilityInvoke { .. } => Err(AuthorizationError::Denied),
    }
}

impl Authorizer for LocalSingleUserAuthorizer {
    fn authorize(
        &self,
        ctx: &ExecutionContext,
        action: &ProtectedAction,
    ) -> Result<(), AuthorizationError> {
        if ctx.actor_id != LOCAL_ACTOR_ID {
            return Err(AuthorizationError::Denied);
        }
        local_policy(action)
    }
}

/// E5 authorization policy (PAW-INF-006; ADR-029 §4.1/§4.4): the local
/// human subject keeps the local policy; an agent subject (`agent:<id>`)
/// is default-denied except for `CapabilityInvoke`, which is evaluated
/// against grants read **fresh** on every check — revocation therefore
/// takes effect at the next side effect. Grant-store failures deny.
pub struct GovernedAuthorizer<G: GrantStore> {
    grants: G,
}

impl<G: GrantStore> GovernedAuthorizer<G> {
    pub fn new(grants: G) -> Self {
        Self { grants }
    }
}

impl<G: GrantStore> Authorizer for GovernedAuthorizer<G> {
    fn authorize(
        &self,
        ctx: &ExecutionContext,
        action: &ProtectedAction,
    ) -> Result<(), AuthorizationError> {
        if ctx.actor_id == LOCAL_ACTOR_ID {
            return local_policy(action);
        }
        match action {
            ProtectedAction::CapabilityInvoke {
                workspace_id,
                capability_id,
            } => {
                let now = sqlite::now_ms().map_err(|_| AuthorizationError::Denied)?;
                let grants = self.grants.grants_for_subject(&ctx.actor_id)?;
                // Shared policy: scope intersection evaluated on fresh rows.
                if paw_application::ports::authorization::grant_allows(
                    &grants,
                    &ctx.actor_id,
                    capability_id,
                    workspace_id,
                    now,
                ) {
                    Ok(())
                } else {
                    Err(AuthorizationError::Denied)
                }
            }
            // Agent subjects hold no other authority: registration, grant
            // administration and approvals are human actions.
            _ => Err(AuthorizationError::Denied),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_policy_grants_local_actor_and_denies_others() {
        let authz = LocalSingleUserAuthorizer;
        let local = ExecutionContext {
            actor_id: LOCAL_ACTOR_ID.into(),
            correlation_id: "c1".into(),
        };
        let other = ExecutionContext {
            actor_id: "someone-else".into(),
            correlation_id: "c2".into(),
        };

        assert!(
            authz
                .authorize(&local, &ProtectedAction::CreateWorkspace)
                .is_ok()
        );
        assert_eq!(
            authz.authorize(&other, &ProtectedAction::CreateWorkspace),
            Err(AuthorizationError::Denied)
        );
    }
}
