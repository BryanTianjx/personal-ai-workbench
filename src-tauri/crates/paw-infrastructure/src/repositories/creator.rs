//! SQLite adapter for the Creator content pipeline (E6-03).
//!
//! Versions are INSERT-only immutable snapshots (append_version bumps the
//! draft head in one transaction); reviews are append-only history.

use std::path::Path;

use paw_application::ports::creator_repository::{
    CreatorDraftRepository, CreatorStoreError, CreatorTopicRepository,
};
use paw_domain::creator::{
    CreatorDraft, CreatorDraftId, CreatorDraftReview, CreatorDraftVersion, CreatorTopic,
    CreatorTopicId, DraftStatus, ReviewDecision, TopicStatus,
};
use paw_domain::work_management::{ParseError, WorkspaceId};

use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
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

impl From<RepoError> for CreatorStoreError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => CreatorStoreError::Invariant(e.to_string()),
            RepoError::Now(e) => CreatorStoreError::Storage(e.to_string()),
            RepoError::Parse(e) => CreatorStoreError::Invariant(e.to_string()),
            RepoError::NotFound => CreatorStoreError::NotFound,
        }
    }
}

fn store_error(e: RepoError) -> CreatorStoreError {
    e.into()
}

fn query_one<T>(
    stmt: rusqlite::Result<rusqlite::Statement<'_>>,
    params: impl rusqlite::Params,
    map: impl Fn(&rusqlite::Row<'_>) -> Result<T, RepoError>,
) -> Result<T, RepoError> {
    let mut stmt = stmt?;
    let mut rows = stmt.query(params)?;
    let Some(row) = rows.next()? else {
        return Err(RepoError::NotFound);
    };
    map(row)
}

fn query_all<T>(
    stmt: rusqlite::Result<rusqlite::Statement<'_>>,
    params: impl rusqlite::Params,
    map: impl Fn(&rusqlite::Row<'_>) -> Result<T, RepoError>,
) -> Result<Vec<T>, RepoError> {
    let mut stmt = stmt?;
    let mut rows = stmt.query(params)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(map(row)?);
    }
    Ok(out)
}

fn now() -> Result<i64, CreatorStoreError> {
    sqlite::now_ms().map_err(|e| CreatorStoreError::Storage(e.to_string()))
}

pub struct SqliteCreatorTopicRepository {
    conn: rusqlite::Connection,
}

impl SqliteCreatorTopicRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_topic(row: &rusqlite::Row<'_>) -> Result<CreatorTopic, RepoError> {
    let status: String = row.get("status")?;
    Ok(CreatorTopic {
        id: CreatorTopicId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        title: row.get("title")?,
        notes: row.get("notes")?,
        status: TopicStatus::parse(&status)
            .expect("topic status values are constrained by the schema CHECK"),
    })
}

impl CreatorTopicRepository for SqliteCreatorTopicRepository {
    fn insert_topic(&self, topic: &CreatorTopic) -> Result<(), CreatorStoreError> {
        let now = now()?;
        self.conn
            .execute(
                "INSERT INTO creator_topics (id, workspace_id, title, notes, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                rusqlite::params![
                    topic.id.as_str(),
                    topic.workspace_id.as_str(),
                    topic.title,
                    topic.notes,
                    topic.status.as_str(),
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_topic(
        &self,
        id: &CreatorTopicId,
        workspace_id: &WorkspaceId,
    ) -> Result<CreatorTopic, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, title, notes, status FROM creator_topics WHERE id = ?1",
        );
        let found =
            query_one(stmt, rusqlite::params![id.as_str()], row_to_topic).map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(CreatorStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_topics(
        &self,
        workspace_id: &WorkspaceId,
        status: Option<TopicStatus>,
    ) -> Result<Vec<CreatorTopic>, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, title, notes, status FROM creator_topics
             WHERE workspace_id = ?1 AND (?2 IS NULL OR status = ?2)
             ORDER BY updated_at DESC, id",
        );
        query_all(
            stmt,
            rusqlite::params![
                workspace_id.as_str(),
                status.map(|s| s.as_str().to_string())
            ],
            row_to_topic,
        )
        .map_err(store_error)
    }

    fn update_topic(&self, topic: &CreatorTopic) -> Result<(), CreatorStoreError> {
        let now = now()?;
        let changed = self
            .conn
            .execute(
                "UPDATE creator_topics SET title = ?1, notes = ?2, status = ?3, updated_at = ?4
                 WHERE id = ?5 AND workspace_id = ?6",
                rusqlite::params![
                    topic.title,
                    topic.notes,
                    topic.status.as_str(),
                    now,
                    topic.id.as_str(),
                    topic.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(CreatorStoreError::NotFound);
        }
        Ok(())
    }
}

pub struct SqliteCreatorDraftRepository {
    conn: rusqlite::Connection,
}

impl SqliteCreatorDraftRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_draft(row: &rusqlite::Row<'_>) -> Result<CreatorDraft, RepoError> {
    let status: String = row.get("status")?;
    Ok(CreatorDraft {
        id: CreatorDraftId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        topic_id: CreatorTopicId::new(row.get::<_, String>("topic_id")?)?,
        title: row.get("title")?,
        current_version_no: row.get::<_, i64>("current_version_no")? as u32,
        status: DraftStatus::parse(&status)
            .expect("draft status values are constrained by the schema CHECK"),
    })
}

fn row_to_version(row: &rusqlite::Row<'_>) -> Result<CreatorDraftVersion, RepoError> {
    Ok(CreatorDraftVersion {
        id: paw_domain::creator::CreatorDraftVersionId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        draft_id: CreatorDraftId::new(row.get::<_, String>("draft_id")?)?,
        version_no: row.get::<_, i64>("version_no")? as u32,
        body: row.get("body")?,
    })
}

fn row_to_review(row: &rusqlite::Row<'_>) -> Result<CreatorDraftReview, RepoError> {
    let decision: String = row.get("decision")?;
    Ok(CreatorDraftReview {
        id: paw_domain::creator::CreatorDraftReviewId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        draft_id: CreatorDraftId::new(row.get::<_, String>("draft_id")?)?,
        version_no: row.get::<_, i64>("version_no")? as u32,
        decision: ReviewDecision::parse(&decision)
            .expect("review decision values are constrained by the schema CHECK"),
        comment: row.get("comment")?,
    })
}

impl CreatorDraftRepository for SqliteCreatorDraftRepository {
    fn insert_draft(&self, draft: &CreatorDraft) -> Result<(), CreatorStoreError> {
        let now = now()?;
        self.conn
            .execute(
                "INSERT INTO creator_drafts
                 (id, workspace_id, topic_id, title, current_version_no, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                rusqlite::params![
                    draft.id.as_str(),
                    draft.workspace_id.as_str(),
                    draft.topic_id.as_str(),
                    draft.title,
                    draft.current_version_no,
                    draft.status.as_str(),
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_draft(
        &self,
        id: &CreatorDraftId,
        workspace_id: &WorkspaceId,
    ) -> Result<CreatorDraft, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, topic_id, title, current_version_no, status
             FROM creator_drafts WHERE id = ?1",
        );
        let found =
            query_one(stmt, rusqlite::params![id.as_str()], row_to_draft).map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(CreatorStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_drafts(
        &self,
        workspace_id: &WorkspaceId,
        topic_id: Option<&CreatorTopicId>,
    ) -> Result<Vec<CreatorDraft>, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, topic_id, title, current_version_no, status
             FROM creator_drafts
             WHERE workspace_id = ?1 AND (?2 IS NULL OR topic_id = ?2)
             ORDER BY updated_at DESC, id",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), topic_id.map(|t| t.as_str())],
            row_to_draft,
        )
        .map_err(store_error)
    }

    fn update_draft(&self, draft: &CreatorDraft) -> Result<(), CreatorStoreError> {
        let now = now()?;
        let changed = self
            .conn
            .execute(
                "UPDATE creator_drafts SET title = ?1, current_version_no = ?2, status = ?3, updated_at = ?4
                 WHERE id = ?5 AND workspace_id = ?6",
                rusqlite::params![
                    draft.title,
                    draft.current_version_no,
                    draft.status.as_str(),
                    now,
                    draft.id.as_str(),
                    draft.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(CreatorStoreError::NotFound);
        }
        Ok(())
    }

    /// Append the immutable snapshot and bump the draft head atomically.
    fn append_version(&self, version: &CreatorDraftVersion) -> Result<(), CreatorStoreError> {
        let now = now()?;
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        tx.execute(
            "INSERT INTO creator_draft_versions
             (id, workspace_id, draft_id, version_no, body, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                version.id.as_str(),
                version.workspace_id.as_str(),
                version.draft_id.as_str(),
                version.version_no,
                version.body,
                now,
            ],
        )
        .map_err(RepoError::Sql)
        .map_err(store_error)?;
        tx.execute(
            "UPDATE creator_drafts SET current_version_no = ?1, updated_at = ?2
             WHERE id = ?3 AND workspace_id = ?4",
            rusqlite::params![
                version.version_no,
                now,
                version.draft_id.as_str(),
                version.workspace_id.as_str(),
            ],
        )
        .map_err(RepoError::Sql)
        .map_err(store_error)?;
        tx.commit().map_err(RepoError::Sql).map_err(store_error)?;
        Ok(())
    }

    fn get_version(
        &self,
        draft_id: &CreatorDraftId,
        version_no: u32,
        workspace_id: &WorkspaceId,
    ) -> Result<CreatorDraftVersion, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, draft_id, version_no, body
             FROM creator_draft_versions WHERE draft_id = ?1 AND version_no = ?2",
        );
        let found = query_one(
            stmt,
            rusqlite::params![draft_id.as_str(), version_no],
            row_to_version,
        )
        .map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(CreatorStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_versions(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &CreatorDraftId,
    ) -> Result<Vec<CreatorDraftVersion>, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, draft_id, version_no, body
             FROM creator_draft_versions WHERE workspace_id = ?1 AND draft_id = ?2
             ORDER BY version_no",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), draft_id.as_str()],
            row_to_version,
        )
        .map_err(store_error)
    }

    fn insert_review(&self, review: &CreatorDraftReview) -> Result<(), CreatorStoreError> {
        let now = now()?;
        self.conn
            .execute(
                "INSERT INTO creator_draft_reviews
                 (id, workspace_id, draft_id, version_no, decision, comment, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    review.id.as_str(),
                    review.workspace_id.as_str(),
                    review.draft_id.as_str(),
                    review.version_no,
                    review.decision.as_str(),
                    review.comment,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn list_reviews(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &CreatorDraftId,
    ) -> Result<Vec<CreatorDraftReview>, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, draft_id, version_no, decision, comment
             FROM creator_draft_reviews WHERE workspace_id = ?1 AND draft_id = ?2
             ORDER BY created_at, id",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), draft_id.as_str()],
            row_to_review,
        )
        .map_err(store_error)
    }
}

// ---------------------------------------------------------------------------
// Publish records (E6 第二批: 发布辅助 — manual facts, automation gated)
// ---------------------------------------------------------------------------

pub struct SqliteCreatorPublishRecordRepository {
    conn: rusqlite::Connection,
}

impl SqliteCreatorPublishRecordRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_publish_record(
    row: &rusqlite::Row<'_>,
) -> Result<paw_domain::creator::CreatorPublishRecord, RepoError> {
    let channel: String = row.get("channel")?;
    Ok(paw_domain::creator::CreatorPublishRecord {
        id: paw_domain::creator::CreatorPublishRecordId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        draft_id: CreatorDraftId::new(row.get::<_, String>("draft_id")?)?,
        version_no: row.get::<_, i64>("version_no")? as u32,
        channel: paw_domain::creator::PublishChannel::parse(&channel)
            .expect("publish channel values are constrained by the schema CHECK"),
        locator: row.get("locator")?,
        notes: row.get("notes")?,
    })
}

impl paw_application::ports::creator_repository::CreatorPublishRecordRepository
    for SqliteCreatorPublishRecordRepository
{
    fn insert_publish_record(
        &self,
        record: &paw_domain::creator::CreatorPublishRecord,
    ) -> Result<(), CreatorStoreError> {
        let now = now()?;
        self.conn
            .execute(
                "INSERT INTO creator_publish_records
                 (id, workspace_id, draft_id, version_no, channel, locator, notes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    record.id.as_str(),
                    record.workspace_id.as_str(),
                    record.draft_id.as_str(),
                    record.version_no,
                    record.channel.as_str(),
                    record.locator,
                    record.notes,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn list_publish_records(
        &self,
        workspace_id: &WorkspaceId,
        draft_id: &CreatorDraftId,
    ) -> Result<Vec<paw_domain::creator::CreatorPublishRecord>, CreatorStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, draft_id, version_no, channel, locator, notes
             FROM creator_publish_records WHERE workspace_id = ?1 AND draft_id = ?2
             ORDER BY created_at DESC, id",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), draft_id.as_str()],
            row_to_publish_record,
        )
        .map_err(store_error)
    }
}
