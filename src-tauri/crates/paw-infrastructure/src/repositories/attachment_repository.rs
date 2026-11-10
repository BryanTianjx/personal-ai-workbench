//! SQLite adapter for the [`AttachmentRepository`] port: metadata for files
//! stored in the managed file store. The polymorphic owner reference is
//! verified here (owner existence inside the workspace) since it cannot be
//! a SQL foreign key.

use std::path::Path;

use paw_application::ports::attachment_repository::{
    Attachment, AttachmentOwner, AttachmentRepository, AttachmentRepositoryError,
};
use paw_domain::work_management::WorkspaceId;

use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    AttachmentNotFound,
    OwnerNotFound,
    /// Pre-formatted scope/constraint rejection with context.
    Invariant(String),
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

impl From<RepoError> for AttachmentRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => AttachmentRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => AttachmentRepositoryError::Invariant(e.to_string()),
            RepoError::AttachmentNotFound => AttachmentRepositoryError::AttachmentNotFound,
            RepoError::OwnerNotFound => AttachmentRepositoryError::OwnerNotFound,
            RepoError::Invariant(msg) => AttachmentRepositoryError::Invariant(msg),
        }
    }
}

pub struct SqliteAttachmentRepository {
    conn: rusqlite::Connection,
}

impl SqliteAttachmentRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const ATTACHMENT_COLUMNS: &str =
    "id, workspace_id, owner_type, owner_id, managed_id, file_name, size_bytes, sha256, created_at";

fn row_to_attachment(row: &rusqlite::Row<'_>) -> Result<Attachment, RepoError> {
    let owner_type: String = row.get("owner_type")?;
    Ok(Attachment {
        id: row.get("id")?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)
            .map_err(|e| RepoError::Invariant(e.to_string()))?,
        owner: AttachmentOwner::parse(&owner_type)
            .expect("attachment owner types are constrained by the schema CHECK"),
        owner_id: row.get("owner_id")?,
        managed_id: row.get("managed_id")?,
        file_name: row.get("file_name")?,
        size_bytes: row.get::<_, i64>("size_bytes")? as u64,
        sha256: row.get("sha256")?,
        created_at: row.get("created_at")?,
    })
}

impl AttachmentRepository for SqliteAttachmentRepository {
    fn insert_attachment(&self, attachment: &Attachment) -> Result<(), AttachmentRepositoryError> {
        self.insert_attachment_inner(attachment)
            .map_err(AttachmentRepositoryError::from)
    }

    fn find_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<Option<Attachment>, AttachmentRepositoryError> {
        self.find_attachment_inner(workspace_id, attachment_id)
            .map_err(AttachmentRepositoryError::from)
    }

    fn list_for_owner(
        &self,
        workspace_id: &WorkspaceId,
        owner: AttachmentOwner,
        owner_id: &str,
    ) -> Result<Vec<Attachment>, AttachmentRepositoryError> {
        self.list_for_owner_inner(workspace_id, owner, owner_id)
            .map_err(AttachmentRepositoryError::from)
    }

    fn count_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<i64, AttachmentRepositoryError> {
        self.count_for_workspace_inner(workspace_id)
            .map_err(AttachmentRepositoryError::from)
    }

    fn delete_attachment(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<(), AttachmentRepositoryError> {
        self.delete_attachment_inner(workspace_id, attachment_id)
            .map_err(AttachmentRepositoryError::from)
    }

    fn verify_owner(
        &self,
        workspace_id: &WorkspaceId,
        owner: AttachmentOwner,
        owner_id: &str,
    ) -> Result<(), AttachmentRepositoryError> {
        self.verify_owner_inner(workspace_id, owner, owner_id)
            .map_err(AttachmentRepositoryError::from)
    }
}

impl SqliteAttachmentRepository {
    fn insert_attachment_inner(&self, attachment: &Attachment) -> Result<(), RepoError> {
        // Polymorphic-owner scope check before the write: the owner must
        // exist inside this workspace.
        self.verify_owner_inner(
            &attachment.workspace_id,
            attachment.owner,
            &attachment.owner_id,
        )?;
        self.conn
            .execute(
                "INSERT INTO attachments (id, workspace_id, owner_type, owner_id, managed_id, \
                 file_name, size_bytes, sha256, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    attachment.id,
                    attachment.workspace_id.as_str(),
                    attachment.owner.as_str(),
                    attachment.owner_id,
                    attachment.managed_id,
                    attachment.file_name,
                    attachment.size_bytes as i64,
                    attachment.sha256,
                    sqlite::now_ms()?,
                ],
            )
            .map_err(RepoError::from)?;
        Ok(())
    }

    fn find_attachment_inner(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<Option<Attachment>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {ATTACHMENT_COLUMNS} FROM attachments WHERE id = ?1 AND workspace_id = ?2"
        ))?;
        let mut rows = stmt.query(rusqlite::params![attachment_id, workspace_id.as_str()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row_to_attachment(row)?))
    }

    fn list_for_owner_inner(
        &self,
        workspace_id: &WorkspaceId,
        owner: AttachmentOwner,
        owner_id: &str,
    ) -> Result<Vec<Attachment>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {ATTACHMENT_COLUMNS} FROM attachments
             WHERE owner_type = ?1 AND owner_id = ?2 AND workspace_id = ?3
             ORDER BY created_at ASC"
        ))?;
        let mut rows = stmt.query(rusqlite::params![
            owner.as_str(),
            owner_id,
            workspace_id.as_str()
        ])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_attachment(row)?);
        }
        Ok(out)
    }

    fn count_for_workspace_inner(&self, workspace_id: &WorkspaceId) -> Result<i64, RepoError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM attachments WHERE workspace_id = ?1",
            [workspace_id.as_str()],
            |r| r.get(0),
        )?)
    }

    fn delete_attachment_inner(
        &self,
        workspace_id: &WorkspaceId,
        attachment_id: &str,
    ) -> Result<(), RepoError> {
        let changed = self.conn.execute(
            "DELETE FROM attachments WHERE id = ?1 AND workspace_id = ?2",
            rusqlite::params![attachment_id, workspace_id.as_str()],
        )?;
        if changed == 0 {
            return Err(RepoError::AttachmentNotFound);
        }
        Ok(())
    }

    fn verify_owner_inner(
        &self,
        workspace_id: &WorkspaceId,
        owner: AttachmentOwner,
        owner_id: &str,
    ) -> Result<(), RepoError> {
        let table = match owner {
            AttachmentOwner::Project => "projects",
            AttachmentOwner::Task => "tasks",
        };
        // owner_id is a parameter; the table name comes from the parsed
        // enum, never from user input.
        let present: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE id = ?1 AND workspace_id = ?2"),
            rusqlite::params![owner_id, workspace_id.as_str()],
            |r| r.get(0),
        )?;
        if present == 0 {
            return Err(RepoError::OwnerNotFound);
        }
        Ok(())
    }
}
