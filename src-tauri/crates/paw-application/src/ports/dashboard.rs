//! Dashboard read-model port (E2-05): cross-domain read-only composition
//! over the Work Management records of one workspace.
//!
//! The Dashboard owns no authoritative records (PRD §4.1, system-
//! architecture §4); this port only aggregates counts from the owning
//! repositories' tables through the adapter, and the numbers are always
//! real queries — never fabricated progress indicators.

use paw_domain::work_management::WorkspaceId;

/// Real-data summary for one workspace (PAW-F-004).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardSummary {
    pub workspace_id: WorkspaceId,
    pub area_count: i64,
    pub goal_count: i64,
    /// Projects grouped by status key (active/paused/completed/archived).
    pub projects_by_status: Vec<(String, i64)>,
    /// Tasks grouped by status key (todo/in_progress/done/cancelled).
    pub tasks_by_status: Vec<(String, i64)>,
    pub attachment_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardError {
    /// A storage-level failure, non-leaky by contract.
    Invariant(String),
}

impl std::fmt::Display for DashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invariant(msg) => write!(f, "dashboard failure: {msg}"),
        }
    }
}

impl std::error::Error for DashboardError {}

pub trait DashboardPort {
    fn summary(&self, workspace_id: &WorkspaceId) -> Result<DashboardSummary, DashboardError>;
}
