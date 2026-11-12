//! SQLite adapter for learning-gap proposals (E6-02, PAW-LG-007).
//!
//! `insert_gap_proposal` emits the versioned `LearningGapProposed.v1`
//! outbox event; `confirm` (through `update_gap_proposal` after the
//! application layer binds the item) emits `LearningGapConfirmed.v1` —
//! both atomically with the state change in the same transaction
//! (ADR-020; the knowledge-repository pattern).

use std::path::Path;

use paw_application::ports::learning_repository::{GapProposalRepository, LearningRepositoryError};
use paw_domain::learning::{GapProposalStatus, LearningGapProposal, LearningGapProposalId};
use paw_domain::work_management::{ParseError, WorkspaceId};

use crate::outbox::NewOutboxEvent;
use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
    Outbox(crate::outbox::OutboxError),
    NotFound,
}

impl From<rusqlite::Error> for RepoError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}

impl From<Error> for RepoError {
    fn from(e: Error) -> Self {
        Self::Now(e)
    }
}

impl From<ParseError> for RepoError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<crate::outbox::OutboxError> for RepoError {
    fn from(e: crate::outbox::OutboxError) -> Self {
        Self::Outbox(e)
    }
}

impl From<RepoError> for LearningRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => LearningRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => LearningRepositoryError::Invariant(e.to_string()),
            RepoError::Parse(e) => LearningRepositoryError::Invariant(e.to_string()),
            RepoError::Outbox(e) => LearningRepositoryError::Invariant(e.to_string()),
            RepoError::NotFound => LearningRepositoryError::ItemNotFound,
        }
    }
}

pub struct SqliteGapProposalRepository {
    conn: rusqlite::Connection,
}

impl SqliteGapProposalRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }

    fn row_to_gap(&self, row: &rusqlite::Row<'_>) -> Result<LearningGapProposal, RepoError> {
        let status: String = row.get("status")?;
        Ok(LearningGapProposal {
            id: LearningGapProposalId::new(row.get::<_, String>("id")?)?,
            workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
            project_id: paw_domain::work_management::ProjectId::new(
                row.get::<_, String>("project_id")?,
            )?,
            gap: row.get("gap")?,
            status: GapProposalStatus::parse(&status)
                .expect("gap status values are constrained by the schema CHECK"),
            confirmed_item_id: row
                .get::<_, Option<String>>("confirmed_item_id")?
                .map(paw_domain::learning::ItemId::new)
                .transpose()?,
        })
    }

    /// Persist + emit in one transaction (ADR-020: state change and event
    /// commit atomically).
    fn write_with_event(
        &self,
        gap: &LearningGapProposal,
        event_type: &str,
    ) -> Result<(), RepoError> {
        let now = sqlite::now_ms()?;
        let tx = self.conn.unchecked_transaction()?;
        let changed = tx.execute(
            "INSERT INTO learning_gap_proposals
             (id, workspace_id, project_id, gap, status, confirmed_item_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(id) DO UPDATE SET
               status = excluded.status, confirmed_item_id = excluded.confirmed_item_id,
               updated_at = excluded.updated_at",
            rusqlite::params![
                gap.id.as_str(),
                gap.workspace_id.as_str(),
                gap.project_id.as_str(),
                gap.gap,
                gap.status.as_str(),
                gap.confirmed_item_id.as_ref().map(|id| id.as_str()),
                now,
            ],
        )?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        // Minimal versioned payload (ADR-020 §4): ids + status, no prose.
        let payload = serde_json::json!({
            "gap_proposal_id": gap.id.as_str(),
            "project_id": gap.project_id.as_str(),
            "status": gap.status.as_str(),
            "confirmed_item_id": gap.confirmed_item_id.as_ref().map(|id| id.as_str()),
        });
        crate::outbox::enqueue_in_tx(
            &tx,
            &NewOutboxEvent {
                workspace_id: Some(gap.workspace_id.as_str().to_string()),
                producer_domain: "learning".to_string(),
                aggregate_type: "learning_gap_proposal".to_string(),
                aggregate_id: gap.id.as_str().to_string(),
                event_type: event_type.to_string(),
                schema_version: 1,
                payload_json: payload.to_string(),
                correlation_id: uuid::Uuid::now_v7().to_string(),
                causation_id: None,
            },
        )?;
        tx.commit()?;
        Ok(())
    }
}

impl GapProposalRepository for SqliteGapProposalRepository {
    fn insert_gap_proposal(
        &self,
        gap: &LearningGapProposal,
    ) -> Result<(), LearningRepositoryError> {
        self.write_with_event(gap, "LearningGapProposed.v1")
            .map_err(LearningRepositoryError::from)
    }

    fn get_gap_proposal(
        &self,
        id: &LearningGapProposalId,
        workspace_id: &WorkspaceId,
    ) -> Result<LearningGapProposal, LearningRepositoryError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, project_id, gap, status, confirmed_item_id
             FROM learning_gap_proposals WHERE id = ?1",
        );
        let found = (|| {
            let mut stmt = stmt?;
            let mut rows = stmt.query(rusqlite::params![id.as_str()])?;
            let Some(row) = rows.next()? else {
                return Err(RepoError::NotFound);
            };
            self.row_to_gap(row)
        })()
        .map_err(LearningRepositoryError::from)?;
        if found.workspace_id != *workspace_id {
            return Err(LearningRepositoryError::Invariant(
                "gap proposal outside the requested workspace".to_string(),
            ));
        }
        Ok(found)
    }

    fn list_gap_proposals(
        &self,
        workspace_id: &WorkspaceId,
        status: Option<GapProposalStatus>,
    ) -> Result<Vec<LearningGapProposal>, LearningRepositoryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, workspace_id, project_id, gap, status, confirmed_item_id
                 FROM learning_gap_proposals
                 WHERE workspace_id = ?1 AND (?2 IS NULL OR status = ?2)
                 ORDER BY updated_at DESC, id",
            )
            .map_err(RepoError::Sql)?;
        let mut rows = stmt
            .query(rusqlite::params![
                workspace_id.as_str(),
                status.map(|s| s.as_str().to_string())
            ])
            .map_err(RepoError::Sql)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(RepoError::Sql)? {
            out.push(self.row_to_gap(row)?);
        }
        Ok(out)
    }

    fn update_gap_proposal(
        &self,
        gap: &LearningGapProposal,
    ) -> Result<(), LearningRepositoryError> {
        let event_type = match gap.status {
            GapProposalStatus::Confirmed => "LearningGapConfirmed.v1",
            _ => "LearningGapDismissed.v1",
        };
        self.write_with_event(gap, event_type)
            .map_err(LearningRepositoryError::from)
    }
}
