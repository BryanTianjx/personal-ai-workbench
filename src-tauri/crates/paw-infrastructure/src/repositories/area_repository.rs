//! SQLite adapter for the [`AreaRepository`] port.

use std::path::Path;

use paw_application::ports::area_repository::{AreaRepository, AreaRepositoryError};
use paw_domain::work_management::{Area, AreaId, ParseError, WorkspaceId};

use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
    Constraint(String),
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

impl From<RepoError> for AreaRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => AreaRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => AreaRepositoryError::Invariant(e.to_string()),
            RepoError::Parse(e) => AreaRepositoryError::Invariant(e.to_string()),
            RepoError::Constraint(msg) => AreaRepositoryError::Invariant(msg),
            RepoError::NotFound => AreaRepositoryError::AreaNotFound,
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

pub struct SqliteAreaRepository {
    conn: rusqlite::Connection,
}

impl SqliteAreaRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const AREA_COLUMNS: &str = "id, workspace_id, name, description";

fn row_to_area(row: &rusqlite::Row<'_>) -> Result<Area, RepoError> {
    Ok(Area {
        id: AreaId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        name: row.get("name")?,
        description: row.get("description")?,
    })
}

impl AreaRepository for SqliteAreaRepository {
    fn insert_area(&self, area: &Area) -> Result<(), AreaRepositoryError> {
        self.insert_area_inner(area)
            .map_err(AreaRepositoryError::from)
    }

    fn find_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<Option<Area>, AreaRepositoryError> {
        self.find_area_inner(workspace_id, area_id)
            .map_err(AreaRepositoryError::from)
    }

    fn list_areas(&self, workspace_id: &WorkspaceId) -> Result<Vec<Area>, AreaRepositoryError> {
        self.list_areas_inner(workspace_id)
            .map_err(AreaRepositoryError::from)
    }

    fn update_area(&self, area: &Area) -> Result<(), AreaRepositoryError> {
        self.update_area_inner(area)
            .map_err(AreaRepositoryError::from)
    }

    fn delete_area(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<(), AreaRepositoryError> {
        self.delete_area_inner(workspace_id, area_id)
            .map_err(AreaRepositoryError::from)
    }
}

impl SqliteAreaRepository {
    fn insert_area_inner(&self, area: &Area) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO areas (id, workspace_id, name, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                rusqlite::params![
                    area.id.as_str(),
                    area.workspace_id.as_str(),
                    area.name,
                    area.description,
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, area.id.as_str()))?;
        Ok(())
    }

    fn find_area_inner(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<Option<Area>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {AREA_COLUMNS} FROM areas WHERE id = ?1 AND workspace_id = ?2"
        ))?;
        let mut rows = stmt.query(rusqlite::params![area_id.as_str(), workspace_id.as_str()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row_to_area(row)?))
    }

    fn list_areas_inner(&self, workspace_id: &WorkspaceId) -> Result<Vec<Area>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {AREA_COLUMNS} FROM areas WHERE workspace_id = ?1 ORDER BY created_at ASC"
        ))?;
        let mut rows = stmt.query([workspace_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_area(row)?);
        }
        Ok(out)
    }

    fn update_area_inner(&self, area: &Area) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "UPDATE areas SET name = ?1, description = ?2, updated_at = ?3
                 WHERE id = ?4 AND workspace_id = ?5",
                rusqlite::params![
                    area.name,
                    area.description,
                    sqlite::now_ms()?,
                    area.id.as_str(),
                    area.workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint(e, area.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn delete_area_inner(
        &self,
        workspace_id: &WorkspaceId,
        area_id: &AreaId,
    ) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM areas WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![area_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
