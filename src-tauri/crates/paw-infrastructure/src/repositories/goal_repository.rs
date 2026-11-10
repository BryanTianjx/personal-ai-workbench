//! SQLite adapter for the [`GoalRepository`] port.

use std::path::Path;

use paw_application::ports::goal_repository::{GoalRepository, GoalRepositoryError};
use paw_domain::work_management::{AreaId, Goal, GoalId, ParseError, WorkspaceId};

use crate::sqlite::{self, Error};

/// Private error absorbing foreign driver/domain errors (the orphan rule
/// keeps `From<rusqlite::Error> for GoalRepositoryError` off the table).
enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
    /// Pre-formatted constraint rejection (composite UNIQUE/FK) with the
    /// record context; covers cross-workspace link attempts.
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

impl From<RepoError> for GoalRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => GoalRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => GoalRepositoryError::Invariant(e.to_string()),
            RepoError::Parse(e) => GoalRepositoryError::Invariant(e.to_string()),
            RepoError::Constraint(msg) => GoalRepositoryError::Invariant(msg),
            RepoError::NotFound => GoalRepositoryError::GoalNotFound,
        }
    }
}

/// Composite FK/UNIQUE violations (e.g. an area link from another workspace)
/// surface as typed, context-carrying rejections.
fn repo_constraint(e: rusqlite::Error, id: &str) -> RepoError {
    if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        RepoError::Constraint(format!(
            "unique/foreign-key constraint failed for {id}: {e}"
        ))
    } else {
        RepoError::Sql(e)
    }
}

pub struct SqliteGoalRepository {
    conn: rusqlite::Connection,
}

impl SqliteGoalRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_goal(row: &rusqlite::Row<'_>) -> Result<Goal, RepoError> {
    let area_id: Option<String> = row.get("area_id")?;
    Ok(Goal {
        id: GoalId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        area_id: match area_id {
            Some(v) => Some(AreaId::new(v)?),
            None => None,
        },
        name: row.get("name")?,
        description: row.get("description")?,
    })
}

impl GoalRepository for SqliteGoalRepository {
    fn insert_goal(&self, goal: &Goal) -> Result<(), GoalRepositoryError> {
        self.insert_goal_inner(goal)
            .map_err(GoalRepositoryError::from)
    }

    fn find_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<Option<Goal>, GoalRepositoryError> {
        self.find_goal_inner(workspace_id, goal_id)
            .map_err(GoalRepositoryError::from)
    }

    fn list_goals(
        &self,
        workspace_id: &WorkspaceId,
        area_id: Option<&AreaId>,
    ) -> Result<Vec<Goal>, GoalRepositoryError> {
        self.list_goals_inner(workspace_id, area_id)
            .map_err(GoalRepositoryError::from)
    }

    fn update_goal(&self, goal: &Goal) -> Result<(), GoalRepositoryError> {
        self.update_goal_inner(goal)
            .map_err(GoalRepositoryError::from)
    }

    fn delete_goal(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<(), GoalRepositoryError> {
        self.delete_goal_inner(workspace_id, goal_id)
            .map_err(GoalRepositoryError::from)
    }
}

impl SqliteGoalRepository {
    fn insert_goal_inner(&self, goal: &Goal) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO goals (id, workspace_id, area_id, name, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                rusqlite::params![
                    goal.id.as_str(),
                    goal.workspace_id.as_str(),
                    goal.area_id.as_ref().map(|a| a.as_str().to_string()),
                    goal.name,
                    goal.description,
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, goal.id.as_str()))?;
        Ok(())
    }

    fn find_goal_inner(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<Option<Goal>, RepoError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, area_id, name, description
             FROM goals WHERE id = ?1 AND workspace_id = ?2",
        )?;
        let mut rows = stmt.query(rusqlite::params![goal_id.as_str(), workspace_id.as_str()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row_to_goal(row)?))
    }

    fn list_goals_inner(
        &self,
        workspace_id: &WorkspaceId,
        area_id: Option<&AreaId>,
    ) -> Result<Vec<Goal>, RepoError> {
        let sql = if area_id.is_some() {
            "SELECT id, workspace_id, area_id, name, description FROM goals
             WHERE workspace_id = ?1 AND area_id = ?2 ORDER BY created_at ASC"
        } else {
            "SELECT id, workspace_id, area_id, name, description FROM goals
             WHERE workspace_id = ?1 ORDER BY created_at ASC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let area_value = area_id.map(|a| a.as_str().to_string());
        let mut rows = stmt.query(rusqlite::params![workspace_id.as_str(), area_value])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_goal(row)?);
        }
        Ok(out)
    }

    fn update_goal_inner(&self, goal: &Goal) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "UPDATE goals SET area_id = ?1, name = ?2, description = ?3, updated_at = ?4
                 WHERE id = ?5 AND workspace_id = ?6",
                rusqlite::params![
                    goal.area_id.as_ref().map(|a| a.as_str().to_string()),
                    goal.name,
                    goal.description,
                    sqlite::now_ms()?,
                    goal.id.as_str(),
                    goal.workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint(e, goal.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn delete_goal_inner(
        &self,
        workspace_id: &WorkspaceId,
        goal_id: &GoalId,
    ) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM goals WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![goal_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
