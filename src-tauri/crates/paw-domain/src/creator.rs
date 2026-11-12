//! Creator domain entities: Topic, Draft, DraftVersion, DraftReview
//! (E6-03, Beta 0.5; PAW-CR scope per product-scope.md: 选题、草稿、版本、审核).
//!
//! Ownership: the Creator domain owns the content pipeline. Publishing
//! automation (微信公众号自动发布) is explicitly out of scope until official
//! API verification (product-scope 非目标; feature-matrix note 4) — E6
//! delivers the local content workflow only.
//!
//! Invariants:
//! - draft versions are append-only snapshots; a review binds one exact
//!   version, and approval applies to that version, not the draft head;
//! - draft status transitions mirror review decisions but remain the
//!   draft's own state machine (no shared machine with reviews);
//! - all records are workspace-consistent.

use serde::{Deserialize, Serialize};

use crate::work_management::{WorkspaceId, opaque_id, validate_name};

opaque_id!(
    CreatorTopicId,
    "Opaque creator topic identifier (UUIDv7 text)."
);
opaque_id!(
    CreatorDraftId,
    "Opaque creator draft identifier (UUIDv7 text)."
);
opaque_id!(
    CreatorDraftVersionId,
    "Opaque creator draft version identifier (UUIDv7 text)."
);
opaque_id!(
    CreatorDraftReviewId,
    "Opaque creator draft review identifier (UUIDv7 text)."
);

/// Creator domain invariant violations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatorError {
    /// A required title was empty or whitespace-only after trimming.
    EmptyTitle,
    /// A title or body exceeded its limit after trimming.
    TooLong { max: usize },
    /// A draft transition the state machine does not publish.
    InvalidTransition { from: String, to: String },
    /// The requested version is not a draft version.
    VersionNotFound,
}

impl std::fmt::Display for CreatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTitle => write!(f, "title must not be empty"),
            Self::TooLong { max } => write!(f, "text exceeds {max} characters"),
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid transition {from} -> {to}")
            }
            Self::VersionNotFound => write!(f, "draft version not found"),
        }
    }
}

impl From<crate::work_management::ValidationError> for CreatorError {
    fn from(e: crate::work_management::ValidationError) -> Self {
        match e {
            crate::work_management::ValidationError::EmptyName => Self::EmptyTitle,
            crate::work_management::ValidationError::NameTooLong { max } => Self::TooLong { max },
            crate::work_management::ValidationError::NotesTooLong { max } => Self::TooLong { max },
        }
    }
}

/// Topic lifecycle (选题): an idea becomes selected for drafting, or is
/// parked. Parked topics can return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopicStatus {
    Idea,
    Selected,
    Parked,
}

impl TopicStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idea => "Idea",
            Self::Selected => "Selected",
            Self::Parked => "Parked",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Idea" => Self::Idea,
            "Selected" => Self::Selected,
            "Parked" => Self::Parked,
            _ => return None,
        })
    }

    /// Idea → Selected | Parked; Selected → Parked; Parked → Idea | Selected.
    pub fn can_transition_to(self, to: Self) -> bool {
        matches!(
            (self, to),
            (Self::Idea, Self::Selected)
                | (Self::Idea, Self::Parked)
                | (Self::Selected, Self::Parked)
                | (Self::Parked, Self::Idea)
                | (Self::Parked, Self::Selected)
        )
    }
}

/// A content topic (选题).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorTopic {
    pub id: CreatorTopicId,
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub notes: Option<String>,
    pub status: TopicStatus,
}

impl CreatorTopic {
    pub fn new(
        id: CreatorTopicId,
        workspace_id: WorkspaceId,
        title: &str,
        notes: Option<&str>,
    ) -> Result<Self, CreatorError> {
        Ok(Self {
            id,
            workspace_id,
            title: validate_name(title, 200)?,
            notes: crate::work_management::validate_name(notes.unwrap_or(""), 500)
                .map(|n| if n.is_empty() { None } else { Some(n) })
                .or::<CreatorError>(Ok(None))?,
            status: TopicStatus::Idea,
        })
    }
}

/// Draft lifecycle (草稿/审核): writing → in review → approved (for the
/// reviewed version) or back to writing after rejection; a publish record
/// moves the draft to the terminal `Published` state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DraftStatus {
    Writing,
    InReview,
    Approved,
    Published,
}

impl DraftStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Writing => "Writing",
            Self::InReview => "InReview",
            Self::Approved => "Approved",
            Self::Published => "Published",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Writing" => Self::Writing,
            "InReview" => Self::InReview,
            "Approved" => Self::Approved,
            "Published" => Self::Published,
            _ => return None,
        })
    }

    /// Writing → InReview; InReview → Approved | Writing (rejection or
    /// revision); Approved → Writing (a new version starts a new cycle) or
    /// Published (terminal, via a publish record).
    pub fn can_transition_to(self, to: Self) -> bool {
        matches!(
            (self, to),
            (Self::Writing, Self::InReview)
                | (Self::InReview, Self::Approved)
                | (Self::InReview, Self::Writing)
                | (Self::Approved, Self::Writing)
                | (Self::Approved, Self::Published)
        )
    }
}

/// A draft bound to one topic. `current_version_no` tracks the latest
/// appended version; versions themselves are immutable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorDraft {
    pub id: CreatorDraftId,
    pub workspace_id: WorkspaceId,
    pub topic_id: CreatorTopicId,
    pub title: String,
    pub current_version_no: u32,
    pub status: DraftStatus,
}

impl CreatorDraft {
    pub fn new(
        id: CreatorDraftId,
        workspace_id: WorkspaceId,
        topic_id: CreatorTopicId,
        title: &str,
    ) -> Result<Self, CreatorError> {
        Ok(Self {
            id,
            workspace_id,
            topic_id,
            title: validate_name(title, 200)?,
            current_version_no: 0,
            status: DraftStatus::Writing,
        })
    }

    /// Advance the draft status through a published transition.
    pub fn transition(&mut self, to: DraftStatus) -> Result<(), CreatorError> {
        if !self.status.can_transition_to(to) {
            return Err(CreatorError::InvalidTransition {
                from: self.status.as_str().to_string(),
                to: to.as_str().to_string(),
            });
        }
        self.status = to;
        Ok(())
    }
}

/// An immutable version snapshot of a draft (版本). Versions start at 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorDraftVersion {
    pub id: CreatorDraftVersionId,
    pub workspace_id: WorkspaceId,
    pub draft_id: CreatorDraftId,
    pub version_no: u32,
    /// The full body of this version (versions are local content).
    pub body: String,
}

impl CreatorDraftVersion {
    pub fn new(
        id: CreatorDraftVersionId,
        workspace_id: WorkspaceId,
        draft_id: CreatorDraftId,
        version_no: u32,
        body: &str,
    ) -> Result<Self, CreatorError> {
        if version_no == 0 {
            return Err(CreatorError::VersionNotFound);
        }
        if body.chars().count() > 100_000 {
            return Err(CreatorError::TooLong { max: 100_000 });
        }
        Ok(Self {
            id,
            workspace_id,
            draft_id,
            version_no,
            body: body.to_string(),
        })
    }
}

/// The review decision (审核): Approved | Rejected for one exact version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewDecision {
    Approved,
    Rejected,
}

impl ReviewDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "Approved",
            Self::Rejected => "Rejected",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Approved" => Self::Approved,
            "Rejected" => Self::Rejected,
            _ => return None,
        })
    }
}

/// A review record bound to one draft version (审核历史).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorDraftReview {
    pub id: CreatorDraftReviewId,
    pub workspace_id: WorkspaceId,
    pub draft_id: CreatorDraftId,
    pub version_no: u32,
    pub decision: ReviewDecision,
    pub comment: Option<String>,
}

impl CreatorDraftReview {
    pub fn new(
        id: CreatorDraftReviewId,
        workspace_id: WorkspaceId,
        draft_id: CreatorDraftId,
        version_no: u32,
        decision: ReviewDecision,
        comment: Option<&str>,
    ) -> Result<Self, CreatorError> {
        if version_no == 0 {
            return Err(CreatorError::VersionNotFound);
        }
        Ok(Self {
            id,
            workspace_id,
            draft_id,
            version_no,
            decision,
            comment: crate::work_management::validate_name(comment.unwrap_or(""), 500)
                .map(|c| if c.is_empty() { None } else { Some(c) })
                .or::<CreatorError>(Ok(None))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> WorkspaceId {
        WorkspaceId::new_v7()
    }

    #[test]
    fn topic_lifecycle() {
        let mut topic = CreatorTopic::new(
            CreatorTopicId::new_v7(),
            ws(),
            "Agent 治理实践",
            Some("选题"),
        )
        .expect("valid");
        assert_eq!(topic.status, TopicStatus::Idea);
        assert!(topic.status.can_transition_to(TopicStatus::Selected));
        topic.status = TopicStatus::Selected;
        assert!(topic.status.can_transition_to(TopicStatus::Parked));
        topic.status = TopicStatus::Parked;
        assert!(topic.status.can_transition_to(TopicStatus::Idea));
        assert!(
            !topic
                .status
                .can_transition_to(TopicStatus::Idea)
                .then_some(())
                .is_none()
        );
    }

    #[test]
    fn draft_review_cycle_binds_versions() {
        let mut draft = CreatorDraft::new(
            CreatorDraftId::new_v7(),
            ws(),
            CreatorTopicId::new_v7(),
            "ADR-022 落地记",
        )
        .expect("valid draft");
        assert_eq!(draft.current_version_no, 0);
        let v1 = CreatorDraftVersion::new(
            CreatorDraftVersionId::new_v7(),
            ws(),
            draft.id.clone(),
            1,
            "第一版正文",
        )
        .expect("valid version");
        assert_eq!(v1.version_no, 1);

        draft
            .transition(DraftStatus::InReview)
            .expect("writing → in review");
        let review = CreatorDraftReview::new(
            CreatorDraftReviewId::new_v7(),
            ws(),
            draft.id.clone(),
            1,
            ReviewDecision::Rejected,
            Some("结构需要重排"),
        )
        .expect("review recorded");
        assert_eq!(review.version_no, 1);
        draft
            .transition(DraftStatus::Writing)
            .expect("rejection returns to writing");
        // A new version starts the next cycle.
        let _v2 = CreatorDraftVersion::new(
            CreatorDraftVersionId::new_v7(),
            ws(),
            draft.id.clone(),
            2,
            "第二版正文",
        )
        .expect("valid version");
        draft
            .transition(DraftStatus::InReview)
            .expect("writing → in review");
        draft.transition(DraftStatus::Approved).expect("approved");
        // A new version after approval restarts the cycle.
        draft
            .transition(DraftStatus::Writing)
            .expect("approved → writing");
    }

    #[test]
    fn draft_rejects_unpublished_transitions_and_zero_versions() {
        let mut draft = CreatorDraft::new(
            CreatorDraftId::new_v7(),
            ws(),
            CreatorTopicId::new_v7(),
            "t",
        )
        .expect("valid");
        assert!(draft.transition(DraftStatus::Approved).is_err());
        assert!(
            CreatorDraftVersion::new(
                CreatorDraftVersionId::new_v7(),
                ws(),
                draft.id.clone(),
                0,
                "body"
            )
            .is_err()
        );
        assert!(
            CreatorDraftReview::new(
                CreatorDraftReviewId::new_v7(),
                ws(),
                draft.id,
                0,
                ReviewDecision::Approved,
                None
            )
            .is_err()
        );
    }
}

/// A publish record (发布记录): the manual fact that an approved version
/// went out on an external channel. Publishing automation stays gated
/// (feature-matrix note 4) — this record is user-entered evidence of an
/// external effect, never an automated action. The record binds one exact
/// draft version, mirroring the review discipline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorPublishRecord {
    pub id: CreatorPublishRecordId,
    pub workspace_id: WorkspaceId,
    pub draft_id: CreatorDraftId,
    pub version_no: u32,
    pub channel: PublishChannel,
    /// External locator (URL / article id) as opaque text.
    pub locator: Option<String>,
    pub notes: Option<String>,
}

opaque_id!(
    CreatorPublishRecordId,
    "Opaque creator publish record identifier (UUIDv7 text)."
);

/// Publication channels. WeChat is the PRD's named channel; others exist
/// for manual records of off-platform publishing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublishChannel {
    WeChat,
    Manual,
}

impl PublishChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WeChat => "wechat",
            Self::Manual => "manual",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "wechat" => Self::WeChat,
            "manual" => Self::Manual,
            _ => return None,
        })
    }
}

impl CreatorPublishRecord {
    pub fn new(
        id: CreatorPublishRecordId,
        workspace_id: WorkspaceId,
        draft_id: CreatorDraftId,
        version_no: u32,
        channel: PublishChannel,
        locator: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Self, CreatorError> {
        if version_no == 0 {
            return Err(CreatorError::VersionNotFound);
        }
        Ok(Self {
            id,
            workspace_id,
            draft_id,
            version_no,
            channel,
            locator: crate::work_management::validate_name(locator.unwrap_or(""), 500)
                .map(|l| if l.is_empty() { None } else { Some(l) })
                .or::<CreatorError>(Ok(None))?,
            notes: crate::work_management::validate_name(notes.unwrap_or(""), 500)
                .map(|n| if n.is_empty() { None } else { Some(n) })
                .or::<CreatorError>(Ok(None))?,
        })
    }
}
