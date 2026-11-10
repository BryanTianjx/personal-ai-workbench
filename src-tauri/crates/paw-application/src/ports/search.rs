//! Search port published by the application layer (E2-03): scoped full-text
//! lookup over project/task text.
//!
//! The port returns opaque, workspace-scoped hits; the adapter guarantees
//! that a hit's record still exists in the authoritative table (join-back)
//! and that no other workspace's records are ever visible (data-architecture
//! §7: every query enforces workspace authorization).

use paw_domain::work_management::WorkspaceId;

/// The record kind a hit points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    Project,
    Task,
}

/// One scoped search hit. Ids are opaque; the UI navigates through the
/// regular typed commands, never through raw index contents.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SearchHit {
    pub kind: SearchKind,
    /// Opaque record id (project or task).
    pub id: String,
    /// Workspace the record belongs to (always the queried scope).
    pub workspace_id: WorkspaceId,
    /// Display title (project name / task title).
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    /// The query was empty after sanitization or invalid after escaping.
    InvalidQuery,
    /// A storage-level failure, non-leaky by contract.
    Invariant(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidQuery => write!(f, "invalid search query"),
            Self::Invariant(msg) => write!(f, "search failure: {msg}"),
        }
    }
}

impl std::error::Error for SearchError {}

pub trait SearchPort {
    /// Full-text search within one workspace. Returns at most `limit` hits.
    fn search(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError>;
}
