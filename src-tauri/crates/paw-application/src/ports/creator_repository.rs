//! Creator content pipeline persistence ports (E6-03; 选题、草稿、版本、审核).

use paw_domain::creator::{
    CreatorDraft, CreatorDraftId, CreatorDraftReview, CreatorDraftVersion, CreatorTopic,
    CreatorTopicId,
};
use paw_domain::work_management::WorkspaceId;

/// Non-leaky persistence errors for the creator surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatorStoreError {
    NotFound,
    WorkspaceMismatch,
    Invariant(String),
    Storage(String),
}

impl std::fmt::Display for CreatorStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "record not found"),
            Self::WorkspaceMismatch => write!(f, "record outside the requested workspace"),
            Self::Invariant(msg) => write!(f, "invariant violated: {msg}"),
            Self::Storage(msg) => write!(f, "storage failure: {msg}"),
        }
    }
}

impl std::error::Error for CreatorStoreError {}

/// Topics (选题).
pub trait CreatorTopicRepository {
    fn insert_topic(&self, topic: &CreatorTopic) -> Result<(), CreatorStoreError>;
    fn get_topic(
        &self,
        id: &CreatorTopicId,
        workspace_id: &WorkspaceId,
    ) -> Result<CreatorTopic, CreatorStoreError>;
    fn list_topics(
        &self,
        workspace_id: &WorkspaceId,
        status: Option<paw_domain::creator::TopicStatus>,
    ) -> Result<Vec<CreatorTopic>, CreatorStoreError>;
    fn update_topic(&self, topic: &CreatorTopic) -> Result<(), CreatorStoreError>;
}

/// Drafts with their immutable versions and review history (草稿/版本/审核).
/// Version appends bump the draft's `current_version_no` in one transaction.
pub trait CreatorDraftRepository {
    fn insert_draft(&self, draft: &CreatorDraft) -> Result<(), CreatorStoreError>;
    fn get_draft(
        &self,
        id: &CreatorDraftId,
        workspace_id: &WorkspaceId,
    ) -> Result<CreatorDraft, CreatorStoreError>;
    fn list_drafts(
        &self,
        workspace_id: &WorkspaceId,
        topic_id: Option<&CreatorTopicId>,
    ) -> Result<Vec<CreatorDraft>, CreatorStoreError>;
    /// Persist a status transition (and title edits if any).
    fn update_draft(&self, draft: &CreatorDraft) -> Result<(), CreatorStoreError>;
    /// Append an immutable version snapshot and bump `current_version_no`
    /// atomically.
    fn append_version(&self, version: &CreatorDraftVersion) -> Result<(), CreatorStoreError>;
    fn get_version(
        &self,
        draft_id: &CreatorDraftId,
        version_no: u32,
        workspace_id: &WorkspaceId,
    ) -> Result<CreatorDraftVersion, CreatorStoreError>;
    fn list_versions(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &CreatorDraftId,
    ) -> Result<Vec<CreatorDraftVersion>, CreatorStoreError>;
    /// Record a review decision for one exact version (history).
    fn insert_review(&self, review: &CreatorDraftReview) -> Result<(), CreatorStoreError>;
    fn list_reviews(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &CreatorDraftId,
    ) -> Result<Vec<CreatorDraftReview>, CreatorStoreError>;
}

/// Publish records (发布记录): manual facts bound to exact versions.
pub trait CreatorPublishRecordRepository {
    fn insert_publish_record(
        &self,
        record: &paw_domain::creator::CreatorPublishRecord,
    ) -> Result<(), CreatorStoreError>;
    fn list_publish_records(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &CreatorDraftId,
    ) -> Result<Vec<paw_domain::creator::CreatorPublishRecord>, CreatorStoreError>;
}
