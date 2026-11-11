//! SQLite adapters for the Learning & Growth ports (PAW-LG-001～006).
//!
//! Four repositories over the migration-0008 tables. All queries carry the
//! workspace predicate, cross-workspace references are refused by the
//! composite schema keys, and sessions are append-only historical records.

use std::path::Path;

use paw_application::ports::learning_repository::{
    ItemRepository, LearningRepositoryError, PlanRepository, RoadmapRepository, SessionRepository,
};
use paw_domain::learning::{
    Item, ItemId, ItemKind, ItemStatus, Plan, PlanId, PlanStatus, Roadmap, RoadmapId, Session,
    SessionId,
};
use paw_domain::work_management::{ParseError, ProjectId, TaskId, WorkspaceId};

use crate::sqlite::{self, Error};

/// Shared private error for the four learning adapters: one enum, one
/// not-found-per-entity mapping (mirrors the E1/E2 adapters' pattern).
enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
    Constraint(String),
    RoadmapNotFound,
    PlanNotFound,
    ItemNotFound,
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

impl From<RepoError> for LearningRepositoryError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => LearningRepositoryError::Invariant(e.to_string()),
            RepoError::Now(e) => LearningRepositoryError::Invariant(e.to_string()),
            RepoError::Parse(e) => LearningRepositoryError::Invariant(e.to_string()),
            RepoError::Constraint(msg) => LearningRepositoryError::Invariant(msg),
            RepoError::RoadmapNotFound => LearningRepositoryError::RoadmapNotFound,
            RepoError::PlanNotFound => LearningRepositoryError::PlanNotFound,
            RepoError::ItemNotFound => LearningRepositoryError::ItemNotFound,
        }
    }
}

impl From<Error> for LearningRepositoryError {
    fn from(e: Error) -> Self {
        LearningRepositoryError::Invariant(e.to_string())
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

macro_rules! impl_repo_open {
    ($repo:ident) => {
        impl $repo {
            pub fn open(path: &Path) -> Result<Self, Error> {
                Ok(Self {
                    conn: sqlite::open(path)?,
                })
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Roadmap
// ---------------------------------------------------------------------------

pub struct SqliteRoadmapRepository {
    conn: rusqlite::Connection,
}

impl_repo_open!(SqliteRoadmapRepository);

fn row_to_roadmap(row: &rusqlite::Row<'_>) -> Result<Roadmap, RepoError> {
    Ok(Roadmap {
        id: RoadmapId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        name: row.get("name")?,
        description: row.get("description")?,
    })
}

impl RoadmapRepository for SqliteRoadmapRepository {
    fn insert_roadmap(&self, roadmap: &Roadmap) -> Result<(), LearningRepositoryError> {
        self.conn
            .execute(
                "INSERT INTO learning_roadmaps (id, workspace_id, name, description, \
                 created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                rusqlite::params![
                    roadmap.id.as_str(),
                    roadmap.workspace_id.as_str(),
                    roadmap.name,
                    roadmap.description,
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, roadmap.id.as_str()))?;
        Ok(())
    }

    fn find_roadmap(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: &RoadmapId,
    ) -> Result<Option<Roadmap>, LearningRepositoryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, workspace_id, name, description FROM learning_roadmaps
             WHERE id = ?1 AND workspace_id = ?2",
            )
            .map_err(RepoError::from)?;
        let mut rows = stmt
            .query(rusqlite::params![
                roadmap_id.as_str(),
                workspace_id.as_str()
            ])
            .map_err(RepoError::from)?;
        let Some(row) = rows.next().map_err(RepoError::from)? else {
            return Ok(None);
        };
        Ok(Some(row_to_roadmap(row)?))
    }

    fn list_roadmaps(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Roadmap>, LearningRepositoryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, workspace_id, name, description FROM learning_roadmaps
             WHERE workspace_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(RepoError::from)?;
        let mut rows = stmt
            .query([workspace_id.as_str()])
            .map_err(RepoError::from)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(RepoError::from)? {
            out.push(row_to_roadmap(row)?);
        }
        Ok(out)
    }

    fn update_roadmap(&self, roadmap: &Roadmap) -> Result<(), LearningRepositoryError> {
        let changed = self
            .conn
            .execute(
                "UPDATE learning_roadmaps SET name = ?1, description = ?2, updated_at = ?3
                 WHERE id = ?4 AND workspace_id = ?5",
                rusqlite::params![
                    roadmap.name,
                    roadmap.description,
                    sqlite::now_ms()?,
                    roadmap.id.as_str(),
                    roadmap.workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint(e, roadmap.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::RoadmapNotFound.into());
        }
        Ok(())
    }

    fn delete_roadmap(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: &RoadmapId,
    ) -> Result<(), LearningRepositoryError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM learning_roadmaps WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![roadmap_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::RoadmapNotFound.into());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

pub struct SqlitePlanRepository {
    conn: rusqlite::Connection,
}

impl_repo_open!(SqlitePlanRepository);

fn row_to_plan(row: &rusqlite::Row<'_>) -> Result<Plan, RepoError> {
    let status: String = row.get("status")?;
    Ok(Plan {
        id: PlanId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        roadmap_id: RoadmapId::new(row.get::<_, String>("roadmap_id")?)?,
        name: row.get("name")?,
        description: row.get("description")?,
        status: PlanStatus::parse(&status)
            .expect("plan status values are constrained by the schema CHECK"),
    })
}

impl PlanRepository for SqlitePlanRepository {
    fn insert_plan(&self, plan: &Plan) -> Result<(), LearningRepositoryError> {
        self.conn
            .execute(
                "INSERT INTO learning_plans (id, workspace_id, roadmap_id, name, description, \
                 status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                rusqlite::params![
                    plan.id.as_str(),
                    plan.workspace_id.as_str(),
                    plan.roadmap_id.as_str(),
                    plan.name,
                    plan.description,
                    plan.status.as_str(),
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, plan.id.as_str()))?;
        Ok(())
    }

    fn find_plan(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: &PlanId,
    ) -> Result<Option<Plan>, LearningRepositoryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, workspace_id, roadmap_id, name, description, status FROM learning_plans
             WHERE id = ?1 AND workspace_id = ?2",
            )
            .map_err(RepoError::from)?;
        let mut rows = stmt
            .query(rusqlite::params![plan_id.as_str(), workspace_id.as_str()])
            .map_err(RepoError::from)?;
        let Some(row) = rows.next().map_err(RepoError::from)? else {
            return Ok(None);
        };
        Ok(Some(row_to_plan(row)?))
    }

    fn list_plans(
        &self,
        workspace_id: &WorkspaceId,
        roadmap_id: Option<&RoadmapId>,
    ) -> Result<Vec<Plan>, LearningRepositoryError> {
        // Parameter count must match the SQL shape exactly (rusqlite rejects
        // binding an unused parameter).
        let mut stmt = self
            .conn
            .prepare(if roadmap_id.is_some() {
                "SELECT id, workspace_id, roadmap_id, name, description, status FROM learning_plans
             WHERE workspace_id = ?1 AND roadmap_id = ?2 ORDER BY created_at ASC"
            } else {
                "SELECT id, workspace_id, roadmap_id, name, description, status FROM learning_plans
             WHERE workspace_id = ?1 ORDER BY created_at ASC"
            })
            .map_err(RepoError::from)?;
        let roadmap_value = roadmap_id.map(|r| r.as_str().to_string());
        let mut rows = if roadmap_id.is_some() {
            stmt.query(rusqlite::params![workspace_id.as_str(), roadmap_value])
                .map_err(RepoError::from)?
        } else {
            stmt.query([workspace_id.as_str()])
                .map_err(RepoError::from)?
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(RepoError::from)? {
            out.push(row_to_plan(row)?);
        }
        Ok(out)
    }

    fn update_plan(&self, plan: &Plan) -> Result<(), LearningRepositoryError> {
        let changed = self
            .conn
            .execute(
                "UPDATE learning_plans SET name = ?1, description = ?2, status = ?3, \
                 updated_at = ?4 WHERE id = ?5 AND workspace_id = ?6",
                rusqlite::params![
                    plan.name,
                    plan.description,
                    plan.status.as_str(),
                    sqlite::now_ms()?,
                    plan.id.as_str(),
                    plan.workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint(e, plan.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::PlanNotFound.into());
        }
        Ok(())
    }

    fn delete_plan(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: &PlanId,
    ) -> Result<(), LearningRepositoryError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM learning_plans WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![plan_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::PlanNotFound.into());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Item
// ---------------------------------------------------------------------------

pub struct SqliteItemRepository {
    conn: rusqlite::Connection,
}

impl_repo_open!(SqliteItemRepository);

const ITEM_COLUMNS: &str =
    "id, workspace_id, plan_id, kind, title, expected_outcome, acceptance_criteria, notes, status";

fn row_to_item(row: &rusqlite::Row<'_>) -> Result<Item, RepoError> {
    let kind: String = row.get("kind")?;
    let status: String = row.get("status")?;
    Ok(Item {
        id: ItemId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        plan_id: PlanId::new(row.get::<_, String>("plan_id")?)?,
        kind: ItemKind::parse(&kind).expect("item kind values are constrained by the schema CHECK"),
        title: row.get("title")?,
        expected_outcome: row.get("expected_outcome")?,
        acceptance_criteria: row.get("acceptance_criteria")?,
        notes: row.get("notes")?,
        status: ItemStatus::parse(&status)
            .expect("item status values are constrained by the schema CHECK"),
    })
}

impl ItemRepository for SqliteItemRepository {
    fn insert_item(&self, item: &Item) -> Result<(), LearningRepositoryError> {
        self.insert_item_inner(item)
            .map_err(LearningRepositoryError::from)
    }

    fn find_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<Option<Item>, LearningRepositoryError> {
        self.find_item_inner(workspace_id, item_id)
            .map_err(LearningRepositoryError::from)
    }

    fn list_items(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: Option<&PlanId>,
        status: Option<ItemStatus>,
    ) -> Result<Vec<Item>, LearningRepositoryError> {
        self.list_items_inner(workspace_id, plan_id, status)
            .map_err(LearningRepositoryError::from)
    }

    fn update_item(&self, item: &Item) -> Result<(), LearningRepositoryError> {
        self.update_item_inner(item)
            .map_err(LearningRepositoryError::from)
    }

    fn delete_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<(), LearningRepositoryError> {
        self.delete_item_inner(workspace_id, item_id)
            .map_err(LearningRepositoryError::from)
    }

    fn attach_item_to_project(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        project_id: &ProjectId,
    ) -> Result<(), LearningRepositoryError> {
        self.attach_item_to_project_inner(workspace_id, item_id, project_id)
            .map_err(LearningRepositoryError::from)
    }

    fn detach_item_from_project(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        project_id: &ProjectId,
    ) -> Result<(), LearningRepositoryError> {
        self.detach_item_from_project_inner(workspace_id, item_id, project_id)
            .map_err(LearningRepositoryError::from)
    }

    fn list_projects_for_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<Vec<ProjectId>, LearningRepositoryError> {
        self.list_projects_for_item_inner(workspace_id, item_id)
            .map_err(LearningRepositoryError::from)
    }

    fn attach_task_to_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        task_id: &TaskId,
    ) -> Result<(), LearningRepositoryError> {
        self.attach_task_to_item_inner(workspace_id, item_id, task_id)
            .map_err(LearningRepositoryError::from)
    }

    fn detach_task_from_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        task_id: &TaskId,
    ) -> Result<(), LearningRepositoryError> {
        self.detach_task_from_item_inner(workspace_id, item_id, task_id)
            .map_err(LearningRepositoryError::from)
    }

    fn list_tasks_for_item(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<Vec<TaskId>, LearningRepositoryError> {
        self.list_tasks_for_item_inner(workspace_id, item_id)
            .map_err(LearningRepositoryError::from)
    }
}

impl SqliteItemRepository {
    fn insert_item_inner(&self, item: &Item) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO learning_items (id, workspace_id, plan_id, kind, title, \
                 expected_outcome, acceptance_criteria, notes, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                rusqlite::params![
                    item.id.as_str(),
                    item.workspace_id.as_str(),
                    item.plan_id.as_str(),
                    item.kind.as_str(),
                    item.title,
                    item.expected_outcome,
                    item.acceptance_criteria,
                    item.notes,
                    item.status.as_str(),
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, item.id.as_str()))?;
        Ok(())
    }

    fn find_item_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<Option<Item>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {ITEM_COLUMNS} FROM learning_items WHERE id = ?1 AND workspace_id = ?2"
        ))?;
        let mut rows = stmt.query(rusqlite::params![item_id.as_str(), workspace_id.as_str()])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row_to_item(row)?))
    }

    fn list_items_inner(
        &self,
        workspace_id: &WorkspaceId,
        plan_id: Option<&PlanId>,
        status: Option<ItemStatus>,
    ) -> Result<Vec<Item>, RepoError> {
        let sql = match (plan_id.is_some(), status.is_some()) {
            (true, true) => format!(
                "SELECT {ITEM_COLUMNS} FROM learning_items WHERE workspace_id = ?1 \
                 AND plan_id = ?2 AND status = ?3 ORDER BY created_at ASC"
            ),
            (true, false) => format!(
                "SELECT {ITEM_COLUMNS} FROM learning_items WHERE workspace_id = ?1 \
                 AND plan_id = ?2 ORDER BY created_at ASC"
            ),
            (false, true) => format!(
                "SELECT {ITEM_COLUMNS} FROM learning_items WHERE workspace_id = ?1 \
                 AND status = ?2 ORDER BY created_at ASC"
            ),
            (false, false) => format!(
                "SELECT {ITEM_COLUMNS} FROM learning_items WHERE workspace_id = ?1 \
                 ORDER BY created_at ASC"
            ),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let plan_value = plan_id.map(|p| p.as_str().to_string());
        let status_value = status.map(|s| s.as_str().to_string());
        let mut rows = match (plan_id.is_some(), status.is_some()) {
            (true, true) => stmt.query(rusqlite::params![
                workspace_id.as_str(),
                plan_value,
                status_value
            ])?,
            (true, false) => stmt.query(rusqlite::params![workspace_id.as_str(), plan_value])?,
            (false, true) => stmt.query(rusqlite::params![workspace_id.as_str(), status_value])?,
            (false, false) => stmt.query([workspace_id.as_str()])?,
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_item(row)?);
        }
        Ok(out)
    }

    fn update_item_inner(&self, item: &Item) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "UPDATE learning_items SET title = ?1, expected_outcome = ?2, \
                 acceptance_criteria = ?3, notes = ?4, status = ?5, updated_at = ?6
                 WHERE id = ?7 AND workspace_id = ?8",
                rusqlite::params![
                    item.title,
                    item.expected_outcome,
                    item.acceptance_criteria,
                    item.notes,
                    item.status.as_str(),
                    sqlite::now_ms()?,
                    item.id.as_str(),
                    item.workspace_id.as_str(),
                ],
            )
            .map_err(|e| repo_constraint(e, item.id.as_str()))?;
        if changed == 0 {
            return Err(RepoError::ItemNotFound);
        }
        Ok(())
    }

    fn delete_item_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<(), RepoError> {
        // Sessions are historical records (RESTRICT composite FK): they must
        // be removed explicitly first — never a silent cascade.
        let changed = self
            .conn
            .execute(
                "DELETE FROM learning_items WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![item_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::ItemNotFound);
        }
        Ok(())
    }

    fn attach_item_to_project_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        project_id: &ProjectId,
    ) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO learning_item_projects (item_id, workspace_id, project_id, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    item_id.as_str(),
                    workspace_id.as_str(),
                    project_id.as_str(),
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, item_id.as_str()))?;
        Ok(())
    }

    fn detach_item_from_project_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        project_id: &ProjectId,
    ) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM learning_item_projects
                 WHERE item_id = ?1 AND project_id = ?2 AND workspace_id = ?3",
                rusqlite::params![item_id.as_str(), project_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::ItemNotFound);
        }
        Ok(())
    }

    fn list_projects_for_item_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<Vec<ProjectId>, RepoError> {
        let mut stmt = self.conn.prepare(
            "SELECT project_id FROM learning_item_projects
             WHERE item_id = ?1 AND workspace_id = ?2 ORDER BY created_at ASC",
        )?;
        let mut rows = stmt.query(rusqlite::params![item_id.as_str(), workspace_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(ProjectId::new(row.get::<_, String>(0)?)?);
        }
        Ok(out)
    }

    fn attach_task_to_item_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        task_id: &TaskId,
    ) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO learning_item_tasks (item_id, workspace_id, task_id, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    item_id.as_str(),
                    workspace_id.as_str(),
                    task_id.as_str(),
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, item_id.as_str()))?;
        Ok(())
    }

    fn detach_task_from_item_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
        task_id: &TaskId,
    ) -> Result<(), RepoError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM learning_item_tasks
                 WHERE item_id = ?1 AND task_id = ?2 AND workspace_id = ?3",
                rusqlite::params![item_id.as_str(), task_id.as_str(), workspace_id.as_str()],
            )
            .map_err(RepoError::from)?;
        if changed == 0 {
            return Err(RepoError::ItemNotFound);
        }
        Ok(())
    }

    fn list_tasks_for_item_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: &ItemId,
    ) -> Result<Vec<TaskId>, RepoError> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id FROM learning_item_tasks
             WHERE item_id = ?1 AND workspace_id = ?2 ORDER BY created_at ASC",
        )?;
        let mut rows = stmt.query(rusqlite::params![item_id.as_str(), workspace_id.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(TaskId::new(row.get::<_, String>(0)?)?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Session (append-only historical records)
// ---------------------------------------------------------------------------

pub struct SqliteSessionRepository {
    conn: rusqlite::Connection,
}

impl_repo_open!(SqliteSessionRepository);

const SESSION_COLUMNS: &str = "id, workspace_id, item_id, task_id, project_id, method, \r
     duration_minutes, summary, difficulty, takeaway, next_step, occurred_at, created_at";

fn row_to_session(row: &rusqlite::Row<'_>) -> Result<Session, RepoError> {
    let task_id: Option<String> = row.get("task_id")?;
    let project_id: Option<String> = row.get("project_id")?;
    Ok(Session {
        id: SessionId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        item_id: ItemId::new(row.get::<_, String>("item_id")?)?,
        task_id: match task_id {
            Some(v) => Some(TaskId::new(v)?),
            None => None,
        },
        project_id: match project_id {
            Some(v) => Some(ProjectId::new(v)?),
            None => None,
        },
        method: row.get("method")?,
        duration_minutes: row.get("duration_minutes")?,
        summary: row.get("summary")?,
        difficulty: row.get("difficulty")?,
        takeaway: row.get("takeaway")?,
        next_step: row.get("next_step")?,
        occurred_at: row.get("occurred_at")?,
    })
}

impl SessionRepository for SqliteSessionRepository {
    fn append_session(&self, session: &Session) -> Result<(), LearningRepositoryError> {
        self.append_session_inner(session)
            .map_err(LearningRepositoryError::from)
    }

    fn find_session(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &SessionId,
    ) -> Result<Option<Session>, LearningRepositoryError> {
        self.find_session_inner(workspace_id, session_id)
            .map_err(LearningRepositoryError::from)
    }

    fn list_sessions(
        &self,
        workspace_id: &WorkspaceId,
        item_id: Option<&ItemId>,
    ) -> Result<Vec<Session>, LearningRepositoryError> {
        self.list_sessions_inner(workspace_id, item_id)
            .map_err(LearningRepositoryError::from)
    }
}

impl SqliteSessionRepository {
    fn append_session_inner(&self, session: &Session) -> Result<(), RepoError> {
        self.conn
            .execute(
                "INSERT INTO learning_sessions (id, workspace_id, item_id, task_id, project_id, \
                 method, duration_minutes, summary, difficulty, takeaway, next_step, \
                 occurred_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params![
                    session.id.as_str(),
                    session.workspace_id.as_str(),
                    session.item_id.as_str(),
                    session.task_id.as_ref().map(|t| t.as_str().to_string()),
                    session.project_id.as_ref().map(|p| p.as_str().to_string()),
                    session.method,
                    session.duration_minutes,
                    session.summary,
                    session.difficulty,
                    session.takeaway,
                    session.next_step,
                    session.occurred_at,
                    sqlite::now_ms()?,
                ],
            )
            .map_err(|e| repo_constraint(e, session.id.as_str()))?;
        Ok(())
    }

    fn find_session_inner(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &SessionId,
    ) -> Result<Option<Session>, RepoError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {SESSION_COLUMNS} FROM learning_sessions WHERE id = ?1 AND workspace_id = ?2"
        ))?;
        let mut rows = stmt.query(rusqlite::params![
            session_id.as_str(),
            workspace_id.as_str()
        ])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row_to_session(row)?))
    }

    fn list_sessions_inner(
        &self,
        workspace_id: &WorkspaceId,
        item_id: Option<&ItemId>,
    ) -> Result<Vec<Session>, RepoError> {
        let sql = if item_id.is_some() {
            format!(
                "SELECT {SESSION_COLUMNS} FROM learning_sessions
                 WHERE workspace_id = ?1 AND item_id = ?2 ORDER BY occurred_at DESC"
            )
        } else {
            format!(
                "SELECT {SESSION_COLUMNS} FROM learning_sessions
                 WHERE workspace_id = ?1 ORDER BY occurred_at DESC"
            )
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let item_value = item_id.map(|i| i.as_str().to_string());
        let mut rows = if item_id.is_some() {
            stmt.query(rusqlite::params![workspace_id.as_str(), item_value])?
        } else {
            stmt.query([workspace_id.as_str()])?
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row_to_session(row)?);
        }
        Ok(out)
    }
}
