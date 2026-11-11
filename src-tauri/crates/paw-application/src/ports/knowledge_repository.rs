//! Knowledge & Context repository port (PAW-K-001): KnowledgeRef metadata.
//!
//! Availability changes are recorded atomically with a
//! `KnowledgeReferenceAvailabilityChanged.v1` outbox event by the adapter
//! (cross-domain-data-contracts §6); learning records referencing the source
//! are never touched by an availability change (§7).

use paw_domain::knowledge::{Availability, KnowledgeRef};
use paw_domain::work_management::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeRepositoryError {
    KnowledgeRefNotFound,
    /// A constraint/scope invariant was violated.
    Invariant(String),
}

impl std::fmt::Display for KnowledgeRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KnowledgeRefNotFound => write!(f, "knowledge reference not found"),
            Self::Invariant(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for KnowledgeRepositoryError {}

pub trait KnowledgeRefRepository {
    fn insert_knowledge_ref(
        &self,
        knowledge_ref: &KnowledgeRef,
    ) -> Result<(), KnowledgeRepositoryError>;

    fn find_knowledge_ref(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &paw_domain::knowledge::KnowledgeRefId,
    ) -> Result<Option<KnowledgeRef>, KnowledgeRepositoryError>;

    fn list_knowledge_refs(
        &self,
        workspace_id: &WorkspaceId,
        availability: Option<Availability>,
    ) -> Result<Vec<KnowledgeRef>, KnowledgeRepositoryError>;

    /// Persist metadata changes (title, locator, version, notes).
    fn update_knowledge_ref(
        &self,
        knowledge_ref: &KnowledgeRef,
    ) -> Result<(), KnowledgeRepositoryError>;

    /// Change availability. The adapter records the change atomically with a
    /// versioned outbox event when the value actually changes.
    fn set_availability(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &paw_domain::knowledge::KnowledgeRefId,
        availability: Availability,
    ) -> Result<KnowledgeRef, KnowledgeRepositoryError>;

    fn delete_knowledge_ref(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &paw_domain::knowledge::KnowledgeRefId,
    ) -> Result<(), KnowledgeRepositoryError>;
}
