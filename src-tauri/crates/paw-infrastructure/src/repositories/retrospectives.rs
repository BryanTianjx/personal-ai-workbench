//! SQLite adapter for project retrospectives with learning-product backflow
//! (E6 第二批, Beta 0.5; PRD §3.8/§5.3).
//!
//! `record_retrospective` persists the retrospective plus its
//! learning-session backflow links atomically and emits the versioned
//! `ProjectRetrospectiveRecorded.v1` outbox event (ADR-020; minimal payload:
//! ids only). Insert-only discipline — corrections are new records.

use std::path::Path;

use paw_application::ports::learning_repository::{
    LearningRepositoryError, RetrospectiveRepository,
};
use paw_domain::work_management::{
    ParseError, ProjectId, ProjectRetrospective, RetrospectiveId, WorkspaceId,
};

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

pub struct SqliteRetrospectiveRepository {
    conn: rusqlite::Connection,
}

impl SqliteRetrospectiveRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }

    fn row_to_retrospective(
        &self,
        row: &rusqlite::Row<'_>,
        session_refs: Vec<paw_domain::learning::SessionId>,
    ) -> Result<ProjectRetrospective, RepoError> {
        Ok(ProjectRetrospective {
            id: RetrospectiveId::new(row.get::<_, String>("id")?)?,
            workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
            project_id: ProjectId::new(row.get::<_, String>("project_id")?)?,
            summary: row.get("summary")?,
            learning_session_refs: session_refs,
        })
    }

    fn load_session_refs(
        &self,
        retrospective_id: &str,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<paw_domain::learning::SessionId>, RepoError> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id FROM retrospective_session_refs
             WHERE retrospective_id = ?1 AND workspace_id = ?2 ORDER BY session_id",
        )?;
        let mut rows = stmt.query(rusqlite::params![retrospective_id, workspace_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(paw_domain::learning::SessionId::new(
                row.get::<_, String>(0)?,
            )?);
        }
        Ok(out)
    }
}

impl RetrospectiveRepository for SqliteRetrospectiveRepository {
    fn record_retrospective(
        &self,
        retrospective: &ProjectRetrospective,
    ) -> Result<(), LearningRepositoryError> {
        let now = sqlite::now_ms().map_err(RepoError::Now)?;
        let tx = self.conn.unchecked_transaction().map_err(RepoError::Sql)?;
        tx.execute(
            "INSERT INTO project_retrospectives
             (id, workspace_id, project_id, summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                retrospective.id.as_str(),
                retrospective.workspace_id.as_str(),
                retrospective.project_id.as_str(),
                retrospective.summary,
                now,
            ],
        )
        .map_err(RepoError::Sql)?;
        for session_id in &retrospective.learning_session_refs {
            tx.execute(
                "INSERT INTO retrospective_session_refs
                 (retrospective_id, workspace_id, session_id)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    retrospective.id.as_str(),
                    retrospective.workspace_id.as_str(),
                    session_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)?;
        }
        // Minimal versioned payload (ADR-020 §4): ids only, no prose.
        let payload = serde_json::json!({
            "retrospective_id": retrospective.id.as_str(),
            "project_id": retrospective.project_id.as_str(),
            "learning_session_refs": retrospective
                .learning_session_refs
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        });
        crate::outbox::enqueue_in_tx(
            &tx,
            &NewOutboxEvent {
                workspace_id: Some(retrospective.workspace_id.as_str().to_string()),
                producer_domain: "work_management".to_string(),
                aggregate_type: "project_retrospective".to_string(),
                aggregate_id: retrospective.id.as_str().to_string(),
                event_type: "ProjectRetrospectiveRecorded.v1".to_string(),
                schema_version: 1,
                payload_json: payload.to_string(),
                correlation_id: uuid::Uuid::now_v7().to_string(),
                causation_id: None,
            },
        )
        .map_err(RepoError::Outbox)?;
        tx.commit().map_err(RepoError::Sql)?;
        Ok(())
    }

    fn get_retrospective(
        &self,
        id: &RetrospectiveId,
        workspace_id: &WorkspaceId,
    ) -> Result<ProjectRetrospective, LearningRepositoryError> {
        let session_refs = self
            .load_session_refs(id.as_str(), workspace_id)
            .map_err(LearningRepositoryError::from)?;
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, workspace_id, project_id, summary
             FROM project_retrospectives WHERE id = ?1",
            )
            .map_err(RepoError::Sql)?;
        let mut rows = stmt
            .query(rusqlite::params![id.as_str()])
            .map_err(RepoError::Sql)?;
        let Some(row) = rows.next().map_err(RepoError::Sql)? else {
            return Err(RepoError::NotFound.into());
        };
        let found = self
            .row_to_retrospective(row, session_refs)
            .map_err(LearningRepositoryError::from)?;
        if found.workspace_id != *workspace_id {
            return Err(LearningRepositoryError::Invariant(
                "retrospective outside the requested workspace".to_string(),
            ));
        }
        Ok(found)
    }

    fn list_retrospectives(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
    ) -> Result<Vec<ProjectRetrospective>, LearningRepositoryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, workspace_id, project_id, summary
                 FROM project_retrospectives
                 WHERE workspace_id = ?1 AND project_id = ?2
                 ORDER BY created_at DESC, id",
            )
            .map_err(RepoError::Sql)?;
        let mut rows = stmt
            .query(rusqlite::params![
                workspace_id.as_str(),
                project_id.as_str()
            ])
            .map_err(RepoError::Sql)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(RepoError::Sql)? {
            let id: String = row.get("id").map_err(RepoError::Sql)?;
            let session_refs = self
                .load_session_refs(&id, workspace_id)
                .map_err(LearningRepositoryError::from)?;
            out.push(
                self.row_to_retrospective(row, session_refs)
                    .map_err(LearningRepositoryError::from)?,
            );
        }
        Ok(out)
    }
}
