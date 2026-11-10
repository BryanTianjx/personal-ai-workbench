//! SQLite adapter for the [`DashboardPort`]: real-count aggregation over
//! the Work Management tables, always workspace-scoped.

use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

use paw_application::ports::dashboard::{DashboardError, DashboardPort, DashboardSummary};
use paw_domain::work_management::WorkspaceId;

use crate::sqlite::{self, Error};

pub struct SqliteDashboard {
    conn: Connection,
}

impl SqliteDashboard {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }

    fn count(&self, sql: &str, workspace_id: &WorkspaceId) -> Result<i64, DashboardError> {
        self.conn
            .query_row(sql, [workspace_id.as_str()], |r| r.get(0))
            .map_err(|e| DashboardError::Invariant(e.to_string()))
    }
}

impl DashboardPort for SqliteDashboard {
    fn summary(&self, workspace_id: &WorkspaceId) -> Result<DashboardSummary, DashboardError> {
        let area_count = self.count(
            "SELECT COUNT(*) FROM areas WHERE workspace_id = ?1",
            workspace_id,
        )?;
        let goal_count = self.count(
            "SELECT COUNT(*) FROM goals WHERE workspace_id = ?1",
            workspace_id,
        )?;
        let attachment_count = self.count(
            "SELECT COUNT(*) FROM attachments WHERE workspace_id = ?1",
            workspace_id,
        )?;

        // Status breakdowns: every known status appears (zero-filled) so the
        // UI renders stable rows without inferring from absence.
        let mut projects_by_status: Vec<(String, i64)> = Vec::new();
        let mut stmt = self
            .conn
            .prepare(
                "SELECT status, COUNT(*) FROM projects WHERE workspace_id = ?1 GROUP BY status",
            )
            .map_err(|e| DashboardError::Invariant(e.to_string()))?;
        let rows = stmt
            .query_map([workspace_id.as_str()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(|e| DashboardError::Invariant(e.to_string()))?;
        let mut observed: Vec<(String, i64)> = rows.filter_map(|r| r.ok()).collect();

        let mut tasks_by_status: Vec<(String, i64)> = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM tasks WHERE workspace_id = ?1 GROUP BY status")
            .map_err(|e| DashboardError::Invariant(e.to_string()))?;
        let rows = stmt
            .query_map([workspace_id.as_str()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(|e| DashboardError::Invariant(e.to_string()))?;
        observed.extend(rows.filter_map(|r| r.ok()));

        // Zero-fill the known statuses in canonical order.
        for status in ["active", "paused", "completed", "archived"] {
            let n = observed
                .iter()
                .find(|(s, _)| s == status)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            projects_by_status.push((status.into(), n));
        }
        for status in ["todo", "in_progress", "done", "cancelled"] {
            let n = observed
                .iter()
                .find(|(s, _)| s == status)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            tasks_by_status.push((status.into(), n));
        }

        Ok(DashboardSummary {
            workspace_id: workspace_id.clone(),
            area_count,
            goal_count,
            projects_by_status,
            tasks_by_status,
            attachment_count,
        })
    }
}

// OptionalExtension is used by count()-style single-row reads elsewhere;
// keep the import active via a tiny usage so clippy stays quiet if the
// helper evolves.
#[allow(unused_imports)]
use OptionalExtension as _OptionalExtension;
