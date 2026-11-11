//! SQLite adapter for the [`KnowledgeRefRepository`] port (PAW-K-001).
//!
//! `set_availability` records the change **atomically** with a
//! `KnowledgeReferenceAvailabilityChanged.v1` outbox event (schema_version 1,
//! minimal payload: knowledge_ref_id, from, to) — the first real producer on
//! the E3 transactional outbox (cross-domain-data-contracts §6/§12).

use std::path::Path;

use paw_application::ports::knowledge_repository::{
    KnowledgeRefRepository, KnowledgeRepositoryError,
};
use paw_domain::knowledge::{Availability, KnowledgeRef, KnowledgeRefId};
use paw_domain::work_management::{ParseError, WorkspaceId};

use crate::outbox::NewOutboxEvent;
use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
    Constraint(String),
    /// Outbox enqueue failure while recording an availability event.
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

impl From<crate::outbox::OutboxError> for RepoError {
    fn from(e: crate::outbox::OutboxError) -> Self {
        Self::Outbox(e)
    }
}

impl From<ParseError> for RepoError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<RepoError> for KnowledgeRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => KnowledgeRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => KnowledgeRepositoryError::Invariant(e.to_string()),
            RepoError::Parse(e) => KnowledgeRepositoryError::Invariant(e.to_string()),
            RepoError::Constraint(msg) => KnowledgeRepositoryError::Invariant(msg),
            RepoError::Outbox(e) => KnowledgeRepositoryError::Invariant(e.to_string()),
            RepoError::NotFound => KnowledgeRepositoryError::KnowledgeRefNotFound,
        }
    }
}

fn repo_constraint(e: rusqlite::Error, id: &str) -> RepoError {
    if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        RepoError::Constraint(format!(
            "unique/foreign-key constraint failed for {id}: {e}"
        ))
    } else {
        RepoError::Sql(e)
    }
}

pub struct SqliteKnowledgeRefRepository {
    conn: rusqlite::Connection,
}

impl SqliteKnowledgeRefRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const KNOWLEDGE_COLUMNS: &str = "id, workspace_id, title, source, external_uri, locator, \
     version_label, notes, availability";

fn row_to_knowledge_ref(row: &rusqlite::Row<'_>) -> Result<KnowledgeRef, RepoError> {
    let source: String = row.get("source")?;
    let availability: String = row.get("availability")?;
    Ok(KnowledgeRef {
        id: KnowledgeRefId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        title: row.get("title")?,
        source: paw_domain::knowledge::KnowledgeSource::parse(&source)
            .expect("knowledge source values are constrained by the schema CHECK"),
        external_uri: row.get("external_uri")?,
        locator: row.get("locator")?,
        version_label: row.get("version_label")?,
        notes: row.get("notes")?,
        availability: Availability::parse(&availability)
            .expect("availability values are constrained by the schema CHECK"),
    })
}

impl KnowledgeRefRepository for SqliteKnowledgeRefRepository {
    fn insert_knowledge_ref(
        &self,
        knowledge_ref: &KnowledgeRef,
    ) -> Result<(), KnowledgeRepositoryError> {
        self.insert_knowledge_ref_inner(knowledge_ref)
            .map_err(KnowledgeRepositoryError::from)
    }

    fn find_knowledge_ref(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
    ) -> Result<Option<KnowledgeRef>, KnowledgeRepositoryError> {
        self.find_knowledge_ref_inner(workspace_id, knowledge_ref_id)
            .map_err(KnowledgeRepositoryError::from)
    }

    fn list_knowledge_refs(
        &self,
        workspace_id: &WorkspaceId,
        availability: Option<Availability>,
    ) -> Result<Vec<KnowledgeRef>, KnowledgeRepositoryError> {
        self.list_knowledge_refs_inner(workspace_id, availability)
            .map_err(KnowledgeRepositoryError::from)
    }

    fn update_knowledge_ref(
        &self,
        knowledge_ref: &KnowledgeRef,
    ) -> Result<(), KnowledgeRepositoryError> {
        self.update_knowledge_ref_inner(knowledge_ref)
            .map_err(KnowledgeRepositoryError::from)
    }

    fn set_availability(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
        availability: Availability,
    ) -> Result<KnowledgeRef, KnowledgeRepositoryError> {
        self.set_availability_inner(workspace_id, knowledge_ref_id, availability)
            .map_err(KnowledgeRepositoryError::from)
    }

    fn delete_knowledge_ref(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
    ) -> Result<(), KnowledgeRepositoryError> {
        self.delete_knowledge_ref_inner(workspace_id, knowledge_ref_id)
            .map_err(KnowledgeRepositoryError::from)
    }
}

impl SqliteKnowledgeRefRepository {
    fn insert_knowledge_ref_inner(&self, knowledge_ref: &KnowledgeRef) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO knowledge_refs (id, workspace_id, title, source, external_uri, \
                 locator, version_label, notes, availability, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                rusqlite::params![
                    knowledge_ref.id.as_str(),
                    knowledge_ref.workspace_id.as_str(),
                    knowledge_ref.title,
                    knowledge_ref.source.as_str(),
                    knowledge_ref.external_uri,
                    knowledge_ref.locator,
                    knowledge_ref.version_label,
                    knowledge_ref.notes,
                    knowledge_ref.availability.as_str(),
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, knowledge_ref.id.as_str()))?;
        Ok(())
    }

    fn find_knowledge_ref_inner(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
    ) -> Result<Option<KnowledgeRef>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {KNOWLEDGE_COLUMNS} FROM knowledge_refs WHERE id = ?1 AND workspace_id = ?2"
        ))?;
        let mut rows = stmt.query(rusqlite::params![
            knowledge_ref_id.as_str(),
            workspace_id.as_str()
        ])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row_to_knowledge_ref(row)?))
    }

    fn list_knowledge_refs_inner(
        &self,
        workspace_id: &WorkspaceId,
        availability: Option<Availability>,
    ) -> Result<Vec<KnowledgeRef>, RepoError> {
        let sql = if availability.is_some() {
            format!(
                "SELECT {KNOWLEDGE_COLUMNS} FROM knowledge_refs
                 WHERE workspace_id = ?1 AND availability = ?2 ORDER BY created_at ASC"
            )
        } else {
            format!(
                "SELECT {KNOWLEDGE_COLUMNS} FROM knowledge_refs
                 WHERE workspace_id = ?1 ORDER BY created_at ASC"
            )
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let availability_value = availability.map(|a| a.as_str().to_string());
        let mut rows = if availability.is_some() {
            stmt.query(rusqlite::params![workspace_id.as_str(), availability_value])?
        } else {
            stmt.query([workspace_id.as_str()])?
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_knowledge_ref(row)?);
        }
        Ok(out)
    }

    fn update_knowledge_ref_inner(&self, knowledge_ref: &KnowledgeRef) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "UPDATE knowledge_refs SET title = ?1, locator = ?2, version_label = ?3, \
                 notes = ?4, updated_at = ?5 WHERE id = ?6 AND workspace_id = ?7",
                rusqlite::params![
                    knowledge_ref.title,
                    knowledge_ref.locator,
                    knowledge_ref.version_label,
                    knowledge_ref.notes,
                    sqlite::now_ms()?,
                    knowledge_ref.id.as_str(),
                    knowledge_ref.workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint(e, knowledge_ref.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn set_availability_inner(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
        availability: Availability,
    ) -> Result<KnowledgeRef, RepoError> {
        let current = self
            .find_knowledge_ref_inner(workspace_id, knowledge_ref_id)?
            .ok_or(RepoError::NotFound)?;
        if current.availability == availability {
            return Ok(current);
        }
        let now = sqlite::now_ms()?;
        // Availability change + versioned outbox event commit atomically
        // (cross-domain-data-contracts §6/§12): at-least-once delivery to
        // future consumers; the learning records referencing this source
        // are never touched by this change (§7).
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE knowledge_refs SET availability = ?1, updated_at = ?2
             WHERE id = ?3 AND workspace_id = ?4",
            rusqlite::params![
                availability.as_str(),
                now,
                knowledge_ref_id.as_str(),
                workspace_id.as_str()
            ],
        )?;
        crate::outbox::enqueue_in_tx(
            &tx,
            &NewOutboxEvent {
                workspace_id: Some(workspace_id.as_str().to_string()),
                producer_domain: "knowledge".into(),
                aggregate_type: "knowledge_ref".into(),
                aggregate_id: knowledge_ref_id.as_str().to_string(),
                event_type: "KnowledgeReferenceAvailabilityChanged".into(),
                schema_version: 1,
                payload_json: format!(
                    "{{\"knowledge_ref_id\":\"{}\",\"from\":\"{}\",\"to\":\"{}\"}}",
                    knowledge_ref_id.as_str(),
                    current.availability.as_str(),
                    availability.as_str()
                ),
                correlation_id: format!("knowledge-availability-{now}"),
                causation_id: None,
            },
        )?;
        tx.commit()?;

        self.find_knowledge_ref_inner(workspace_id, knowledge_ref_id)?
            .ok_or(RepoError::NotFound)
    }

    fn delete_knowledge_ref_inner(
        &self,
        workspace_id: &WorkspaceId,
        knowledge_ref_id: &KnowledgeRefId,
    ) -> Result<(), RepoError> {
        // Learning sessions referencing this source keep their records; the
        // session_refs association cascades (availability marker, not data).
        let changed = self
            .conn
            .execute(
                "DELETE FROM knowledge_refs WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![knowledge_ref_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
