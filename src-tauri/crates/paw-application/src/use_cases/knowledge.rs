//! Knowledge & Context use cases (PAW-K-001): KnowledgeRef metadata —
//! source, identifier, title, location, version and availability.
//!
//! External sources stay authoritative for their content; the workbench owns
//! only the reference metadata (ADR-013/ADR-016). An availability change is
//! recorded atomically with a versioned outbox event by the repository
//! adapter (cross-domain-data-contracts §6) and never touches the learning
//! records that reference the source (§7) — this use case therefore only
//! gates, delegates and audits.

use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{
    AuthorizationError, Authorizer, ExecutionContext, ProtectedAction,
};
use crate::ports::knowledge_repository::{KnowledgeRefRepository, KnowledgeRepositoryError};
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::knowledge::{Availability, KnowledgeRef, KnowledgeRefId, KnowledgeSource};
use paw_domain::work_management::{ValidationError, WorkspaceId};

/// Audit resource type used for every knowledge reference action.
const RESOURCE: &str = "knowledge_ref";

/// Typed failure of a Knowledge & Context use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeError {
    /// Typed knowledge repository rejection.
    Knowledge(KnowledgeRepositoryError),
    /// Input failed a domain invariant before touching the repository.
    Validation(ValidationError),
    /// Fresh authorization was refused for the protected side effect.
    Authorization(AuthorizationError),
}

impl std::fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Knowledge(e) => write!(f, "{e}"),
            Self::Validation(e) => write!(f, "{e}"),
            Self::Authorization(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for KnowledgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Knowledge(e) => Some(e),
            Self::Validation(e) => Some(e),
            Self::Authorization(e) => Some(e),
        }
    }
}

impl From<KnowledgeRepositoryError> for KnowledgeError {
    fn from(e: KnowledgeRepositoryError) -> Self {
        Self::Knowledge(e)
    }
}

impl From<ValidationError> for KnowledgeError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

/// The shared E1-09 gate helper returns a Work Management error; only its
/// authorization variant can escape, so the shapes stay aligned.
impl From<WorkManagementError> for KnowledgeError {
    fn from(e: WorkManagementError) -> Self {
        match e {
            WorkManagementError::Authorization(a) => Self::Authorization(a),
            WorkManagementError::Validation(v) => Self::Validation(v),
            other => Self::Knowledge(KnowledgeRepositoryError::Invariant(other.to_string())),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn gate(
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    action: &ProtectedAction,
    workspace_id: &WorkspaceId,
    resource_id: Option<&str>,
) -> Result<(), KnowledgeError> {
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        action,
        Some(workspace_id),
        RESOURCE,
        resource_id,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn audit_outcome(
    sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    action: &ProtectedAction,
    workspace_id: &WorkspaceId,
    resource_id: Option<&str>,
    outcome: AuditOutcome,
) {
    audit_action(
        sink,
        ctx,
        action,
        Some(workspace_id),
        RESOURCE,
        resource_id,
        outcome,
    );
}

/// List the workspace's knowledge references, optionally filtered by
/// availability (read: workspace-scoped by the port, not authorization-gated).
pub fn list_knowledge_refs(
    repo: &dyn KnowledgeRefRepository,
    workspace_id: &WorkspaceId,
    availability: Option<Availability>,
) -> Result<Vec<KnowledgeRef>, KnowledgeError> {
    Ok(repo.list_knowledge_refs(workspace_id, availability)?)
}

/// Record one reference. New references start [`Availability::Available`];
/// nothing is fetched or stored beyond the metadata.
#[allow(clippy::too_many_arguments)]
pub fn create_knowledge_ref(
    repo: &dyn KnowledgeRefRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    title: &str,
    source: KnowledgeSource,
    external_uri: Option<&str>,
    locator: Option<&str>,
    version_label: Option<&str>,
) -> Result<KnowledgeRef, KnowledgeError> {
    let knowledge_ref = KnowledgeRef::new(
        KnowledgeRefId::new_v7(),
        workspace_id.clone(),
        title,
        source,
        external_uri,
        locator,
        version_label,
        None,
    )?;
    let action = ProtectedAction::CreateKnowledgeRef {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        Some(knowledge_ref.id.as_str()),
    )?;
    match repo.insert_knowledge_ref(&knowledge_ref) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                Some(knowledge_ref.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(knowledge_ref)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                Some(knowledge_ref.id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Change a reference's availability. The repository records the change and
/// its versioned outbox event in one transaction and leaves every learning
/// record untouched (contract §6/§7).
pub fn set_knowledge_availability(
    repo: &dyn KnowledgeRefRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    knowledge_ref_id: &KnowledgeRefId,
    availability: Availability,
) -> Result<KnowledgeRef, KnowledgeError> {
    let action = ProtectedAction::SetKnowledgeAvailability {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        Some(knowledge_ref_id.as_str()),
    )?;
    match repo.set_availability(workspace_id, knowledge_ref_id, availability) {
        Ok(updated) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                Some(knowledge_ref_id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(updated)
        }
        Err(e) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                Some(knowledge_ref_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// Delete a reference's metadata. Learning sessions that cited the source
/// keep their records; only the association disappears (contract §7).
pub fn delete_knowledge_ref(
    repo: &dyn KnowledgeRefRepository,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    knowledge_ref_id: &KnowledgeRefId,
) -> Result<(), KnowledgeError> {
    let action = ProtectedAction::DeleteKnowledgeRef {
        workspace_id: workspace_id.clone(),
    };
    gate(
        authorizer,
        audit_sink,
        ctx,
        &action,
        workspace_id,
        Some(knowledge_ref_id.as_str()),
    )?;
    match repo.delete_knowledge_ref(workspace_id, knowledge_ref_id) {
        Ok(()) => {
            audit_outcome(
                audit_sink,
                ctx,
                &action,
                workspace_id,
                Some(knowledge_ref_id.as_str()),
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
                Some(knowledge_ref_id.as_str()),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::use_cases::work_management::test_support::{FakeAudit, FakeAuthorizer, ctx};

    /// Minimal in-memory knowledge repository fake (records availability
    /// changes so the test can assert what the adapter was asked to do).
    #[derive(Default)]
    struct FakeKnowledgeRepo {
        refs: RefCell<Vec<KnowledgeRef>>,
        availability_calls: RefCell<Vec<Availability>>,
    }

    impl KnowledgeRefRepository for FakeKnowledgeRepo {
        fn insert_knowledge_ref(
            &self,
            knowledge_ref: &KnowledgeRef,
        ) -> Result<(), KnowledgeRepositoryError> {
            self.refs.borrow_mut().push(knowledge_ref.clone());
            Ok(())
        }

        fn find_knowledge_ref(
            &self,
            workspace_id: &WorkspaceId,
            knowledge_ref_id: &KnowledgeRefId,
        ) -> Result<Option<KnowledgeRef>, KnowledgeRepositoryError> {
            Ok(self
                .refs
                .borrow()
                .iter()
                .find(|r| r.id == *knowledge_ref_id && r.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_knowledge_refs(
            &self,
            workspace_id: &WorkspaceId,
            availability: Option<Availability>,
        ) -> Result<Vec<KnowledgeRef>, KnowledgeRepositoryError> {
            Ok(self
                .refs
                .borrow()
                .iter()
                .filter(|r| {
                    r.workspace_id == *workspace_id
                        && availability.is_none_or(|a| r.availability == a)
                })
                .cloned()
                .collect())
        }

        fn update_knowledge_ref(
            &self,
            _knowledge_ref: &KnowledgeRef,
        ) -> Result<(), KnowledgeRepositoryError> {
            Ok(())
        }

        fn set_availability(
            &self,
            workspace_id: &WorkspaceId,
            knowledge_ref_id: &KnowledgeRefId,
            availability: Availability,
        ) -> Result<KnowledgeRef, KnowledgeRepositoryError> {
            self.availability_calls.borrow_mut().push(availability);
            let mut refs = self.refs.borrow_mut();
            let slot = refs
                .iter_mut()
                .find(|r| r.id == *knowledge_ref_id && r.workspace_id == *workspace_id)
                .ok_or(KnowledgeRepositoryError::KnowledgeRefNotFound)?;
            slot.availability = availability;
            Ok(slot.clone())
        }

        fn delete_knowledge_ref(
            &self,
            workspace_id: &WorkspaceId,
            knowledge_ref_id: &KnowledgeRefId,
        ) -> Result<(), KnowledgeRepositoryError> {
            let mut refs = self.refs.borrow_mut();
            let before = refs.len();
            refs.retain(|r| !(r.id == *knowledge_ref_id && r.workspace_id == *workspace_id));
            if refs.len() == before {
                return Err(KnowledgeRepositoryError::KnowledgeRefNotFound);
            }
            Ok(())
        }
    }

    fn create(repo: &FakeKnowledgeRepo, authz: &FakeAuthorizer, audit: &FakeAudit) -> KnowledgeRef {
        create_knowledge_ref(
            repo,
            authz,
            audit,
            &ctx("local-user"),
            &WorkspaceId::new_v7(),
            "Durable Execution 综述",
            KnowledgeSource::Web,
            Some("https://example.com/durable-execution"),
            Some("section-2"),
            Some("v2026-10"),
        )
        .unwrap_or_else(|e| panic!("create failed: {e}"))
    }

    #[test]
    fn knowledge_ref_lifecycle_is_gated_and_audited() {
        let repo = FakeKnowledgeRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();

        let created = create_knowledge_ref(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            "Durable Execution 综述",
            KnowledgeSource::Web,
            Some("https://example.com/durable-execution"),
            Some("section-2"),
            Some("v2026-10"),
        )
        .unwrap();
        assert_eq!(created.availability, Availability::Available);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("knowledge.ref.create".into(), AuditOutcome::Allowed))
        );

        let stale = set_knowledge_availability(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &created.id,
            Availability::Stale,
        )
        .unwrap();
        assert_eq!(stale.availability, Availability::Stale);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&(
                "knowledge.ref.set_availability".into(),
                AuditOutcome::Allowed
            ))
        );
        assert_eq!(
            repo.availability_calls.borrow().as_slice(),
            &[Availability::Stale]
        );

        assert_eq!(
            list_knowledge_refs(&repo, &ws, Some(Availability::Stale))
                .unwrap()
                .len(),
            1
        );
        assert!(
            list_knowledge_refs(&repo, &ws, Some(Availability::Available))
                .unwrap()
                .is_empty()
        );

        delete_knowledge_ref(&repo, &authz, &audit, &ctx("local-user"), &ws, &created.id).unwrap();
        assert!(repo.refs.borrow().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("knowledge.ref.delete".into(), AuditOutcome::Allowed))
        );
    }

    #[test]
    fn knowledge_ref_denial_and_scope_are_typed() {
        let repo = FakeKnowledgeRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();
        let other = WorkspaceId::new_v7();
        let created = create_knowledge_ref(
            &repo,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            "参考资料",
            KnowledgeSource::Local,
            None,
            None,
            None,
        )
        .unwrap();

        // Cross-workspace availability change: typed not-found, refusal
        // audited as an error against the requested workspace.
        assert_eq!(
            set_knowledge_availability(
                &repo,
                &authz,
                &audit,
                &ctx("local-user"),
                &other,
                &created.id,
                Availability::Unavailable
            ),
            Err(KnowledgeError::Knowledge(
                KnowledgeRepositoryError::KnowledgeRefNotFound
            ))
        );
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("knowledge.ref.set_availability".into(), AuditOutcome::Error))
        );

        // Denied actor: audited refusal, no availability call reaches the
        // repository.
        let calls_before = repo.availability_calls.borrow().len();
        assert_eq!(
            set_knowledge_availability(
                &repo,
                &authz,
                &audit,
                &ctx("intruder"),
                &ws,
                &created.id,
                Availability::Unavailable
            ),
            Err(KnowledgeError::Authorization(AuthorizationError::Denied))
        );
        assert_eq!(
            audit.events.borrow().last(),
            Some(&(
                "knowledge.ref.set_availability".into(),
                AuditOutcome::Denied
            ))
        );
        assert_eq!(repo.availability_calls.borrow().len(), calls_before);

        // Empty title is a domain validation error.
        assert!(matches!(
            create_knowledge_ref(
                &repo,
                &authz,
                &audit,
                &ctx("local-user"),
                &ws,
                "  ",
                KnowledgeSource::Web,
                None,
                None,
                None
            ),
            Err(KnowledgeError::Validation(ValidationError::EmptyName))
        ));
    }

    #[test]
    fn create_helper_builds_a_reference() {
        let repo = FakeKnowledgeRepo::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        assert_eq!(create(&repo, &authz, &audit).source, KnowledgeSource::Web);
        assert_eq!(repo.refs.borrow().len(), 1);
    }
}
