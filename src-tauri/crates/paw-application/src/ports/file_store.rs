//! Managed file store port (E2-04): raw attachment bytes live outside
//! SQLite under workspace-scoped managed directories
//! (`attachments/<workspace-id>/<managed-uuid>`, data-architecture §2/§6).
//!
//! Implementations must validate every path component (ids are UUIDs —
//! no traversal), write atomically (temp file + rename), enforce a size
//! cap, and refuse reads/writes that resolve outside the workspace
//! directory (symlink escapes).

use paw_domain::work_management::WorkspaceId;

/// Result of storing one file: the opaque managed id and integrity facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFile {
    /// Opaque managed file id (UUIDv7 text) used for later open/remove.
    pub managed_id: String,
    pub size_bytes: u64,
    /// SHA-256 hex of the stored bytes.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStoreError {
    /// The payload exceeded the store's size cap.
    TooLarge { max: usize },
    /// The managed id was not a valid opaque id (traversal attempt).
    InvalidId,
    /// The file does not exist in this workspace's managed directory.
    NotFound,
    /// A storage-level failure, non-leaky by contract.
    Invariant(String),
}

impl std::fmt::Display for FileStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { max } => write!(f, "file exceeds the {max} byte limit"),
            Self::InvalidId => write!(f, "invalid managed file identifier"),
            Self::NotFound => write!(f, "managed file not found"),
            Self::Invariant(msg) => write!(f, "file store failure: {msg}"),
        }
    }
}

impl std::error::Error for FileStoreError {}

pub trait ManagedFileStore {
    /// Store `bytes` under `workspace_id`; returns the managed id.
    fn store(&self, workspace_id: &WorkspaceId, bytes: &[u8])
    -> Result<StoredFile, FileStoreError>;

    /// Read the bytes of one managed file. Only files stored under this
    /// workspace's directory are visible.
    fn open(&self, workspace_id: &WorkspaceId, managed_id: &str)
    -> Result<Vec<u8>, FileStoreError>;

    /// Remove one managed file (idempotent on absence).
    fn remove(&self, workspace_id: &WorkspaceId, managed_id: &str) -> Result<(), FileStoreError>;
}
