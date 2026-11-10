//! SQLite adapter for the [`TaskRepository`] port.

use std::path::Path;

use paw_application::ports::task_repository::{TaskRepository, TaskRepositoryError};
use paw_domain::work_management::{ParseError, ProjectId, Task, TaskId, TaskStatus, WorkspaceId};

use crate::search;
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

impl From<RepoError> for TaskRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => TaskRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => TaskRepositoryError::Invariant(e.to_string()),
            RepoError::Parse(e) => TaskRepositoryError::Invariant(e.to_string()),
            RepoError::Constraint(msg) => TaskRepositoryError::Invariant(msg),
            RepoError::NotFound => TaskRepositoryError::TaskNotFound,
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

pub struct SqliteTaskRepository {
    conn: rusqlite::Connection,
}

impl SqliteTaskRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const TASK_COLUMNS: &str = "id, workspace_id, project_id, title, notes, status";

fn row_to_task(row: &rusqlite::Row<'_>) -> Result<Task, RepoError> {
    let project_id: Option<String> = row.get("project_id")?;
    let status: String = row.get("status")?;
    Ok(Task {
        id: TaskId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        project_id: match project_id {
            Some(v) => Some(ProjectId::new(v)?),
            None => None,
        },
        title: row.get("title")?,
        notes: row.get("notes")?,
        status: TaskStatus::parse(&status)
            .expect("task status values are constrained by the schema CHECK"),
    })
}

impl TaskRepository for SqliteTaskRepository {
    fn insert_task(&self, task: &Task) -> Result<(), TaskRepositoryError> {
        self.insert_task_inner(task)
            .map_err(TaskRepositoryError::from)
    }

    fn find_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<Option<Task>, TaskRepositoryError> {
        self.find_task_inner(workspace_id, task_id)
            .map_err(TaskRepositoryError::from)
    }

    fn list_tasks(
        &self,
        workspace_id: &WorkspaceId,
        project_id: Option<&ProjectId>,
        status: Option<TaskStatus>,
    ) -> Result<Vec<Task>, TaskRepositoryError> {
        self.list_tasks_inner(workspace_id, project_id, status)
            .map_err(TaskRepositoryError::from)
    }

    fn update_task(&self, task: &Task) -> Result<(), TaskRepositoryError> {
        self.update_task_inner(task)
            .map_err(TaskRepositoryError::from)
    }

    fn delete_task(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<(), TaskRepositoryError> {
        self.delete_task_inner(workspace_id, task_id)
            .map_err(TaskRepositoryError::from)
    }
}

impl SqliteTaskRepository {
    fn insert_task_inner(&self, task: &Task) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO tasks (id, workspace_id, project_id, title, notes, status, \
                 created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                rusqlite::params![
                    task.id.as_str(),
                    task.workspace_id.as_str(),
                    task.project_id.as_ref().map(|p| p.as_str().to_string()),
                    task.title,
                    task.notes,
                    task.status.as_str(),
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, task.id.as_str()))?;
        search::index_task(&self.conn, task);
        Ok(())
    }

    fn find_task_inner(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<Option<Task>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?1 AND workspace_id = ?2"
        ))?;
        let mut rows = stmt.query(rusqlite::params![task_id.as_str(), workspace_id.as_str()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row_to_task(row)?))
    }

    fn list_tasks_inner(
        &self,
        workspace_id: &WorkspaceId,
        project_id: Option<&ProjectId>,
        status: Option<TaskStatus>,
    ) -> Result<Vec<Task>, RepoError> {
        // Parameter count must match the SQL shape exactly: rusqlite rejects
        // binding an unused parameter.
        let sql = match (project_id.is_some(), status.is_some()) {
            (true, true) => format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE workspace_id = ?1 \
                 AND project_id = ?2 AND status = ?3 ORDER BY created_at ASC"
            ),
            (true, false) => format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE workspace_id = ?1 \
                 AND project_id = ?2 ORDER BY created_at ASC"
            ),
            (false, true) => format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE workspace_id = ?1 \
                 AND status = ?2 ORDER BY created_at ASC"
            ),
            (false, false) => format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE workspace_id = ?1 \
                 ORDER BY created_at ASC"
            ),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let project_value = project_id.map(|p| p.as_str().to_string());
        let status_value = status.map(|s| s.as_str().to_string());
        let mut rows = match (project_id.is_some(), status.is_some()) {
            (true, true) => stmt.query(rusqlite::params![
                workspace_id.as_str(),
                project_value,
                status_value
            ])?,
            (true, false) => stmt.query(rusqlite::params![workspace_id.as_str(), project_value])?,
            (false, true) => stmt.query(rusqlite::params![workspace_id.as_str(), status_value])?,
            (false, false) => stmt.query([workspace_id.as_str()])?,
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_task(row)?);
        }
        Ok(out)
    }

    fn update_task_inner(&self, task: &Task) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "UPDATE tasks SET project_id = ?1, title = ?2, notes = ?3, status = ?4, \
                 updated_at = ?5 WHERE id = ?6 AND workspace_id = ?7",
                rusqlite::params![
                    task.project_id.as_ref().map(|p| p.as_str().to_string()),
                    task.title,
                    task.notes,
                    task.status.as_str(),
                    sqlite::now_ms()?,
                    task.id.as_str(),
                    task.workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint(e, task.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        search::deindex_task(&self.conn, task.id.as_str());
        search::index_task(&self.conn, task);
        Ok(())
    }

    fn delete_task_inner(
        &self,
        workspace_id: &WorkspaceId,
        task_id: &TaskId,
    ) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM tasks WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![task_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::NotFound);
        }
        search::deindex_task(&self.conn, task_id.as_str());
        Ok(())
    }
}
