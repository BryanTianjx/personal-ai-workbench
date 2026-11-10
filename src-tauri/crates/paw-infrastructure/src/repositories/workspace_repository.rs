//! SQLite repository implementations of the paw-application workspace ports.
//!
//! These adapters enforce workspace isolation at the repository layer: every
//! query is filtered by the requested workspace_id, and cross-workspace
//! mutations are refused at the port boundary. SQL row types and driver
//! errors stay private to this module: foreign driver errors cannot impl
//! `From<rusqlite::Error> for WorkspaceRepositoryError` (orphan rule), so a
//! private `RepoError` absorbs them and each trait method converts once at
//! its boundary. The port contract only ever sees `WorkspaceRepositoryError`
//! (see `E1-rust-workspace-layout.md` §4).

use std::path::Path;

use paw_application::ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};
use paw_domain::work_management::{
    AreaId, GoalId, ParseError, Project, ProjectId, ProjectStatus, Workspace, WorkspaceId,
};

use crate::search;
use crate::sqlite::{self, Error};

use rusqlite::params;

/// Private error type absorbing foreign driver/domain errors so `?` works
/// inside the adapter without leaking SQL or driver types into the port
/// trait signatures.
enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
    /// Pre-formatted constraint rejection (unique/FK) with record context.
    Constraint(String),
    /// The target workspace row does not exist.
    NotFound,
    /// The target project row does not exist in the workspace.
    ProjectNotFound,
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

impl From<RepoError> for WorkspaceRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => WorkspaceRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => WorkspaceRepositoryError::Invariant(e.to_string()),
            RepoError::Parse(e) => WorkspaceRepositoryError::Invariant(e.to_string()),
            RepoError::Constraint(msg) => WorkspaceRepositoryError::Invariant(msg),
            RepoError::NotFound => WorkspaceRepositoryError::WorkspaceNotFound,
            RepoError::ProjectNotFound => WorkspaceRepositoryError::ProjectNotFound,
        }
    }
}

/// SQLite reports a UNIQUE/FK violation as ConstraintViolation; both are
/// table-level and translate to the same typed, context-carrying rejection.
fn repo_constraint_error(e: rusqlite::Error, id: &str) -> RepoError {
    if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        RepoError::Constraint(format!(
            "unique/foreign-key constraint failed for {id}: {e}"
        ))
    } else {
        RepoError::Sql(e)
    }
}

/// Row shape: id, workspace_id, goal_id, area_id, name, description, status.
fn row_to_project(row: &rusqlite::Row<'_>) -> Result<Project, RepoError> {
    let goal_id: Option<String> = row.get("goal_id")?;
    let area_id: Option<String> = row.get("area_id")?;
    let status: String = row.get("status")?;
    Ok(Project {
        id: ProjectId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        goal_id: match goal_id {
            Some(v) => Some(GoalId::new(v)?),
            None => None,
        },
        area_id: match area_id {
            Some(v) => Some(AreaId::new(v)?),
            None => None,
        },
        name: row.get("name")?,
        description: row.get("description")?,
        status: ProjectStatus::parse(&status)
            .expect("project status values are constrained by the schema CHECK"),
    })
}

pub struct SqliteWorkspaceRepository {
    conn: rusqlite::Connection,
}

impl SqliteWorkspaceRepository {
    /// Open the repository at `db_path`. Callers usually open a connection,
    /// run `sqlite::migrate`, drop that connection, then open this repository.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let conn = sqlite::open(path)?;
        Ok(Self { conn })
    }
}

impl WorkspaceRepository for SqliteWorkspaceRepository {
    fn find_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Option<Workspace>, WorkspaceRepositoryError> {
        self.find_workspace_inner(workspace_id)
            .map_err(WorkspaceRepositoryError::from)
    }

    fn list_projects(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Project>, WorkspaceRepositoryError> {
        self.list_projects_inner(workspace_id)
            .map_err(WorkspaceRepositoryError::from)
    }

    fn insert_workspace(&self, workspace: &Workspace) -> Result<(), WorkspaceRepositoryError> {
        self.insert_workspace_inner(workspace)
            .map_err(WorkspaceRepositoryError::from)
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceRepositoryError> {
        self.list_workspaces_inner()
            .map_err(WorkspaceRepositoryError::from)
    }

    fn insert_project(
        &self,
        workspace_id: &WorkspaceId,
        project: &Project,
    ) -> Result<(), WorkspaceRepositoryError> {
        self.insert_project_inner(workspace_id, project)
            .map_err(WorkspaceRepositoryError::from)
    }

    fn update_project(
        &self,
        workspace_id: &WorkspaceId,
        project: &Project,
    ) -> Result<(), WorkspaceRepositoryError> {
        self.update_project_inner(workspace_id, project)
            .map_err(WorkspaceRepositoryError::from)
    }

    fn delete_project(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
    ) -> Result<(), WorkspaceRepositoryError> {
        self.delete_project_inner(workspace_id, project_id)
            .map_err(WorkspaceRepositoryError::from)
    }

    fn delete_workspace(&self, workspace_id: &WorkspaceId) -> Result<(), WorkspaceRepositoryError> {
        self.delete_workspace_inner(workspace_id)
            .map_err(WorkspaceRepositoryError::from)
    }
}

impl SqliteWorkspaceRepository {
    fn list_workspaces_inner(&self) -> Result<Vec<Workspace>, RepoError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM workspaces ORDER BY created_at ASC")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let id = WorkspaceId::new(row.get::<_, String>(0)?)?;
            let name: String = row.get(1)?;
            out.push(Workspace { id, name });
        }
        Ok(out)
    }

    fn find_workspace_inner(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Option<Workspace>, RepoError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM workspaces WHERE id = ?1")?;
        let mut rows = stmt.query([workspace_id.as_str()])?;
        let Some(row) = rows.next()? else {
            // Port contract: an absent workspace is `Ok(None)`, not an error.
            return Ok(None);
        };
        let id = WorkspaceId::new(row.get::<_, String>(0)?)?;
        let name: String = row.get(1)?;
        Ok(Some(Workspace { id, name }))
    }

    fn list_projects_inner(&self, workspace_id: &WorkspaceId) -> Result<Vec<Project>, RepoError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, goal_id, area_id, name, description, status FROM projects
             WHERE workspace_id = ?1
             ORDER BY created_at ASC",
        )?;
        let mut rows = stmt.query([workspace_id.as_str()])?;
        let mut out = Vec::new();
        // Propagate iteration errors: `while let Ok(...)` would silently
        // truncate the list on a mid-iteration failure.
        while let Some(row) = rows.next()? {
            out.push(row_to_project(row)?);
        }
        Ok(out)
    }

    fn insert_workspace_inner(&self, workspace: &Workspace) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO workspaces (id, name, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)",
                params![workspace.id.as_str(), workspace.name, sqlite::now_ms()?,],
            )
            .map_err(|e| repo_constraint_error(e, workspace.id.as_str()))?;
        Ok(())
    }

    fn insert_project_inner(
        &self,
        workspace_id: &WorkspaceId,
        project: &Project,
    ) -> Result<(), RepoError> {
        // Enforce the port's invariant: a project must belong to the requested
        // workspace. The composite UNIQUE(id, workspace_id) in the schema
        // additionally prevents cross-workspace identifier collisions, and
        // the composite goal/area FKs refuse links into other workspaces.
        if project.workspace_id != *workspace_id {
            return Err(RepoError::Constraint(format!(
                "project {} does not belong to workspace {}",
                project.id.as_str(),
                workspace_id.as_str()
            )));
        }
        self.conn
            .execute(
                "INSERT INTO projects (id, workspace_id, goal_id, area_id, name, description, \
                 status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                params![
                    project.id.as_str(),
                    project.workspace_id.as_str(),
                    project.goal_id.as_ref().map(|g| g.as_str().to_string()),
                    project.area_id.as_ref().map(|a| a.as_str().to_string()),
                    project.name,
                    project.description,
                    project.status.as_str(),
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint_error(e, project.id.as_str()))?;
        search::index_project(&self.conn, project);
        Ok(())
    }

    fn update_project_inner(
        &self,
        workspace_id: &WorkspaceId,
        project: &Project,
    ) -> Result<(), RepoError> {
        if project.workspace_id != *workspace_id {
            return Err(RepoError::Constraint(format!(
                "project {} does not belong to workspace {}",
                project.id.as_str(),
                workspace_id.as_str()
            )));
        }
        let changed = self
            .conn
            .execute(
                "UPDATE projects SET goal_id = ?1, area_id = ?2, name = ?3, description = ?4, \
                 status = ?5, updated_at = ?6
                 WHERE id = ?7 AND workspace_id = ?8",
                params![
                    project.goal_id.as_ref().map(|g| g.as_str().to_string()),
                    project.area_id.as_ref().map(|a| a.as_str().to_string()),
                    project.name,
                    project.description,
                    project.status.as_str(),
                    sqlite::now_ms()?,
                    project.id.as_str(),
                    workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint_error(e, project.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::ProjectNotFound);
        }
        // Keep the rebuildable FTS projection in step (same transaction).
        search::deindex_project(&self.conn, project.id.as_str());
        search::index_project(&self.conn, project);
        Ok(())
    }

    fn delete_project_inner(
        &self,
        workspace_id: &WorkspaceId,
        project_id: &ProjectId,
    ) -> Result<(), RepoError> {
        // Tasks referencing the project hold a RESTRICT composite FK: the
        // caller unbinds/deletes them explicitly first — no silent cascade.
        let changed = self
            .conn
            .execute(
                "DELETE FROM projects WHERE id = ?1 AND workspace_id = ?2",
                params![project_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::ProjectNotFound);
        }
        search::deindex_project(&self.conn, project_id.as_str());
        Ok(())
    }

    fn delete_workspace_inner(&self, workspace_id: &WorkspaceId) -> Result<(), RepoError> {
        // Port contract: delete the workspace and all its records, in child
        // dependency order (tasks → projects → goals → areas → workspace).
        // Everything commits atomically; a failure rolls the whole deletion
        // back. Audit rows (migration 0003) hold a RESTRICT FK and are not
        // deleted: a workspace with audit history is refused until the E3
        // audit retention policy lands.
        let tx = self.conn.unchecked_transaction()?;
        // Rebuildable projections are cleaned inside the same transaction.
        tx.execute(
            "DELETE FROM fts_projects WHERE workspace_id = ?1",
            params![workspace_id.as_str()],
        )?;
        tx.execute(
            "DELETE FROM fts_tasks WHERE workspace_id = ?1",
            params![workspace_id.as_str()],
        )?;
        tx.execute(
            "DELETE FROM tasks WHERE workspace_id = ?1",
            params![workspace_id.as_str()],
        )?;
        tx.execute(
            "DELETE FROM projects WHERE workspace_id = ?1",
            params![workspace_id.as_str()],
        )?;
        tx.execute(
            "DELETE FROM goals WHERE workspace_id = ?1",
            params![workspace_id.as_str()],
        )?;
        tx.execute(
            "DELETE FROM areas WHERE workspace_id = ?1",
            params![workspace_id.as_str()],
        )?;
        tx.execute(
            "DELETE FROM workspaces WHERE id = ?1",
            params![workspace_id.as_str()],
        )?;
        if tx.changes() == 0 {
            // No workspace row was deleted; the transaction (including any
            // child deletes) is rolled back when `tx` drops.
            return Err(RepoError::NotFound);
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn cross_workspace_denial() {
        let db = temp_db("e1-04-cross-workspace");
        let guard = TempDbGuard::new(db.clone());

        let mut conn = sqlite::open(&db).unwrap();
        sqlite::migrate(&mut conn, &sqlite::migrations_dir()).unwrap();
        drop(conn);

        let repo = SqliteWorkspaceRepository::open(&db).unwrap();

        let w1 = WorkspaceId::new_v7();
        let w2 = WorkspaceId::new_v7();
        let proj_in_w1 = ProjectId::new_v7();
        let proj_in_w2 = ProjectId::new_v7();

        repo.insert_workspace(&Workspace {
            id: w1.clone(),
            name: "alpha".into(),
        })
        .unwrap();
        repo.insert_workspace(&Workspace {
            id: w2.clone(),
            name: "beta".into(),
        })
        .unwrap();

        repo.insert_project(
            &w1,
            &Project {
                id: proj_in_w1.clone(),
                workspace_id: w1.clone(),
                goal_id: None,
                area_id: None,
                description: None,
                status: ProjectStatus::Active,
                name: "p1".into(),
            },
        )
        .unwrap();

        // Inserting a project whose workspace_id does not match the requested
        // workspace is refused, even when the raw UUID itself is valid.
        let result = repo.insert_project(
            &w2,
            &Project {
                id: proj_in_w2.clone(),
                workspace_id: w1.clone(),
                goal_id: None,
                area_id: None,
                description: None,
                status: ProjectStatus::Active,
                name: "p2".into(),
            },
        );
        assert!(matches!(
            result,
            Err(WorkspaceRepositoryError::Invariant(_))
        ));

        // find_workspace succeeds for an existing workspace and returns
        // Ok(None) for an absent one (port contract).
        assert!(repo.find_workspace(&w1).unwrap().is_some());
        assert!(
            repo.find_workspace(&WorkspaceId::new_v7())
                .unwrap()
                .is_none()
        );

        // list_projects returns only the rows of the requested workspace.
        let projects = repo.list_projects(&w1).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, proj_in_w1);
        let projects_w2 = repo.list_projects(&w2).unwrap();
        assert!(projects_w2.is_empty());

        // Temporal isolation: deleting w2 must not erase w1's data.
        repo.delete_workspace(&w2).unwrap();
        repo.insert_project(
            &w1,
            &Project {
                id: proj_in_w2.clone(),
                workspace_id: w1.clone(),
                goal_id: None,
                area_id: None,
                description: None,
                status: ProjectStatus::Active,
                name: "p3".into(),
            },
        )
        .unwrap();

        // Deleting an absent workspace is a typed rejection.
        assert_eq!(
            repo.delete_workspace(&WorkspaceId::new_v7()),
            Err(WorkspaceRepositoryError::WorkspaceNotFound)
        );

        // Cascade: deleting w1 removes its projects; the workspace row is gone.
        repo.delete_workspace(&w1).unwrap();
        assert!(repo.list_projects(&w1).unwrap().is_empty());
        assert!(repo.find_workspace(&w1).unwrap().is_none());

        drop(guard);
    }

    /// Removes the test database directory when dropped, including WAL/SHM
    /// sidecar files.
    struct TempDbGuard {
        dir: PathBuf,
    }

    impl TempDbGuard {
        fn new(db_path: PathBuf) -> Self {
            Self {
                dir: db_path
                    .parent()
                    .expect("db path has a parent")
                    .to_path_buf(),
            }
        }
    }

    impl Drop for TempDbGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn temp_db(suffix: &str) -> PathBuf {
        // Test-thread names contain `::`, which is illegal in Windows file
        // names; use a process-unique counter instead.
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/tmp-e1-04-{}-{seq}-{}",
            std::process::id(),
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("test.db")
    }
}
