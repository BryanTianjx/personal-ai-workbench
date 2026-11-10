//! Attachment metadata port (E2-04): SQLite metadata for files whose bytes
//! live in the managed file store. The owner reference is polymorphic
//! (project|task) and verified by the adapter inside the workspace — it is
//! not a SQL foreign key.

use paw_domain::work_management::WorkspaceId;

/// The kind of record an attachment is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentOwner {
    Project,
    Task,
}

impl AttachmentOwner {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Task => "task",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "project" => Self::Project,
            "task" => Self::Task,
            _ => return None,
        })
    }
}

/// One attachment metadata record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Attachment {
    pub id: String,
    pub workspace_id: WorkspaceId,
    pub owner: AttachmentOwner,
    /// Opaque id of the owning record (project or task).
    pub owner_id: String,
    /// Managed-file id binding the metadata row to its bytes.
    pub managed_id: String,
    /// Original file name (validated: no path components, <= 255 chars).
    pub file_name: String,
    pub size_bytes: u64,
    /// SHA-256 hex of the stored bytes.
    pub sha256: String,
    pub created_at: i64,
}

/// Validated attachment metadata supplied by the use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAttachment {
    pub owner: AttachmentOwner,
    pub owner_id: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentRepositoryError {
    /// The referenced attachment does not exist in the workspace.
    AttachmentNotFound,
    /// The owner record does not exist in the workspace.
    OwnerNotFound,
    /// A constraint-level or scope invariant was violated.
    Invariant(String),
}

impl std::fmt::Display for AttachmentRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AttachmentNotFound => write!(f, "attachment not found"),
            Self::OwnerNotFound => write!(f, "attachment owner not found"),
            Self::Invariant(msg) => write!(f, "invariant violation: {msg}"),
        }
    }
}

impl std::error::Error for AttachmentRepositoryError {}

pub trait AttachmentRepository {
    /// Insert metadata. The adapter verifies the owner exists inside the
    /// workspace before writing (polymorphic-owner scope check).
    fn insert_attachment(&self, attachment: &Attachment) -> Result<(), AttachmentRepositoryError>;

    fn find_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<Option<Attachment>, AttachmentRepositoryError>;

    fn list_for_owner(
        &self,
        workspace_id: &WorkspaceId,
        owner: AttachmentOwner,
        owner_id: &str,
    ) -> Result<Vec<Attachment>, AttachmentRepositoryError>;

    /// Count every attachment in the workspace (deletion guard input).
    fn count_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<i64, AttachmentRepositoryError>;

    /// Delete metadata for one attachment. The use case removes the stored
    /// bytes separately (metadata first: an orphaned file is harmless, a
    /// dangling record is not).
    fn delete_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<(), AttachmentRepositoryError>;

    /// Verify the polymorphic owner exists inside the workspace.
    fn verify_owner(
        &self,
        workspace_id: &WorkspaceId,
        owner: AttachmentOwner,
        owner_id: &str,
    ) -> Result<(), AttachmentRepositoryError>;
}
