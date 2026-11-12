//! SQLite adapters for the platform composition surface (E7-02/03/04).

use std::path::Path;

use paw_application::ports::platform_repository::{
    CompositionRepository, PlatformStoreError, PluginStateRepository, RoleProfileRepository,
};
use paw_domain::platform::{CompositionId, ProfileId, RoleProfile, WorkspaceComposition};
use paw_domain::work_management::{ParseError, WorkspaceId};

use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Json(serde_json::Error),
    Parse(ParseError),
    NotFound,
    // WorkspaceMismatch is decided directly against the fetched row and
    // returned as PlatformStoreError::WorkspaceMismatch (no driver source).
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

impl From<serde_json::Error> for RepoError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<ParseError> for RepoError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<RepoError> for PlatformStoreError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => PlatformStoreError::Invariant(e.to_string()),
            RepoError::Now(e) => PlatformStoreError::Storage(e.to_string()),
            RepoError::Json(e) => PlatformStoreError::Invariant(e.to_string()),
            RepoError::Parse(e) => PlatformStoreError::Invariant(e.to_string()),
            RepoError::NotFound => PlatformStoreError::NotFound,
        }
    }
}

fn store_error(e: RepoError) -> PlatformStoreError {
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

fn now() -> Result<i64, PlatformStoreError> {
    sqlite::now_ms().map_err(|e| PlatformStoreError::Storage(e.to_string()))
}

fn json_array(values: &[String]) -> Result<String, RepoError> {
    serde_json::to_string(values).map_err(RepoError::Json)
}

fn parse_json_array(text: String) -> Result<Vec<String>, RepoError> {
    serde_json::from_str(&text).map_err(RepoError::Json)
}

pub struct SqlitePluginStateRepository {
    conn: rusqlite::Connection,
}

impl SqlitePluginStateRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

impl PluginStateRepository for SqlitePluginStateRepository {
    fn set_disabled(
        &self,
        workspace_id: &WorkspaceId,
        plugin_id: &str,
        disabled: bool,
    ) -> Result<(), PlatformStoreError> {
        let now = now()?;
        let disabled_at = if disabled { Some(now) } else { None };
        self.conn
            .execute(
                "INSERT INTO plugin_states (plugin_id, workspace_id, disabled, disabled_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(plugin_id, workspace_id) DO UPDATE SET
                   disabled = excluded.disabled, disabled_at = excluded.disabled_at,
                   updated_at = excluded.updated_at",
                rusqlite::params![
                    plugin_id,
                    workspace_id.as_str(),
                    disabled as i64,
                    disabled_at,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn is_disabled(
        &self,
        workspace_id: &WorkspaceId,
        plugin_id: &str,
    ) -> Result<bool, PlatformStoreError> {
        let disabled: Option<i64> = self
            .conn
            .query_row(
                "SELECT disabled FROM plugin_states
                 WHERE plugin_id = ?1 AND workspace_id = ?2",
                rusqlite::params![plugin_id, workspace_id.as_str()],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    Ok(None)
                } else {
                    Err(RepoError::Sql(e))
                }
            })
            .map_err(store_error)?;
        Ok(disabled.unwrap_or(0) == 1)
    }

    fn list_disabled(&self, workspace_id: &WorkspaceId) -> Result<Vec<String>, PlatformStoreError> {
        let stmt = self.conn.prepare(
            "SELECT plugin_id FROM plugin_states
             WHERE workspace_id = ?1 AND disabled = 1 ORDER BY plugin_id",
        );
        let rows = query_all(stmt, rusqlite::params![workspace_id.as_str()], |row| {
            Ok(row.get::<_, String>(0)?)
        })
        .map_err(store_error)?;
        Ok(rows)
    }
}

pub struct SqliteRoleProfileRepository {
    conn: rusqlite::Connection,
}

impl SqliteRoleProfileRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const PROFILE_COLUMNS: &str =
    "id, workspace_id, name, plugin_preferences, dashboard_layout, agent_profile_refs";

fn row_to_profile(row: &rusqlite::Row<'_>) -> Result<RoleProfile, RepoError> {
    Ok(RoleProfile {
        id: ProfileId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        name: row.get("name")?,
        plugin_preferences: parse_json_array(row.get("plugin_preferences")?)?,
        dashboard_layout: row.get("dashboard_layout")?,
        agent_profile_refs: parse_json_array(row.get("agent_profile_refs")?)?,
    })
}

impl RoleProfileRepository for SqliteRoleProfileRepository {
    fn insert_profile(&self, profile: &RoleProfile) -> Result<(), PlatformStoreError> {
        let now = now()?;
        let prefs = json_array(&profile.plugin_preferences).map_err(store_error)?;
        let refs = json_array(&profile.agent_profile_refs).map_err(store_error)?;
        self.conn
            .execute(
                "INSERT INTO role_profiles
                 (id, workspace_id, name, plugin_preferences, dashboard_layout, agent_profile_refs, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                rusqlite::params![
                    profile.id.as_str(),
                    profile.workspace_id.as_str(),
                    profile.name,
                    prefs,
                    profile.dashboard_layout,
                    refs,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_profile(
        &self,
        id: &ProfileId,
        workspace_id: &WorkspaceId,
    ) -> Result<RoleProfile, PlatformStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {PROFILE_COLUMNS} FROM role_profiles WHERE id = ?1"
        ));
        let found =
            query_one(stmt, rusqlite::params![id.as_str()], row_to_profile).map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(PlatformStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_profiles(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<RoleProfile>, PlatformStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {PROFILE_COLUMNS} FROM role_profiles
             WHERE workspace_id = ?1 ORDER BY updated_at DESC, id"
        ));
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str()],
            row_to_profile,
        )
        .map_err(store_error)
    }

    fn update_profile(&self, profile: &RoleProfile) -> Result<(), PlatformStoreError> {
        let now = now()?;
        let prefs = json_array(&profile.plugin_preferences).map_err(store_error)?;
        let refs = json_array(&profile.agent_profile_refs).map_err(store_error)?;
        let changed = self
            .conn
            .execute(
                "UPDATE role_profiles SET name = ?1, plugin_preferences = ?2, dashboard_layout = ?3,
                 agent_profile_refs = ?4, updated_at = ?5
                 WHERE id = ?6 AND workspace_id = ?7",
                rusqlite::params![
                    profile.name,
                    prefs,
                    profile.dashboard_layout,
                    refs,
                    now,
                    profile.id.as_str(),
                    profile.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(PlatformStoreError::NotFound);
        }
        Ok(())
    }
}

pub struct SqliteCompositionRepository {
    conn: rusqlite::Connection,
}

impl SqliteCompositionRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const COMPOSITION_COLUMNS: &str =
    "id, workspace_id, actor_id, name, profile_ref, plugin_selection, navigation_config";

fn row_to_composition(row: &rusqlite::Row<'_>) -> Result<WorkspaceComposition, RepoError> {
    Ok(WorkspaceComposition {
        id: CompositionId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        actor_id: row.get("actor_id")?,
        name: row.get("name")?,
        profile_ref: row
            .get::<_, Option<String>>("profile_ref")?
            .map(ProfileId::new)
            .transpose()?,
        plugin_selection: parse_json_array(row.get("plugin_selection")?)?,
        navigation_config: row.get("navigation_config")?,
    })
}

impl CompositionRepository for SqliteCompositionRepository {
    fn insert_composition(
        &self,
        composition: &WorkspaceComposition,
    ) -> Result<(), PlatformStoreError> {
        let now = now()?;
        let selection = json_array(&composition.plugin_selection).map_err(store_error)?;
        self.conn
            .execute(
                "INSERT INTO workspace_compositions
                 (id, workspace_id, actor_id, name, profile_ref, plugin_selection, navigation_config, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                rusqlite::params![
                    composition.id.as_str(),
                    composition.workspace_id.as_str(),
                    composition.actor_id,
                    composition.name,
                    composition.profile_ref.as_ref().map(|p| p.as_str()),
                    selection,
                    composition.navigation_config,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_composition(
        &self,
        id: &CompositionId,
        workspace_id: &WorkspaceId,
    ) -> Result<WorkspaceComposition, PlatformStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {COMPOSITION_COLUMNS} FROM workspace_compositions WHERE id = ?1"
        ));
        let found = query_one(stmt, rusqlite::params![id.as_str()], row_to_composition)
            .map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(PlatformStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_compositions(
        &self,
        workspace_id: &WorkspaceId,
        actor_id: Option<&str>,
    ) -> Result<Vec<WorkspaceComposition>, PlatformStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {COMPOSITION_COLUMNS} FROM workspace_compositions
             WHERE workspace_id = ?1 AND (?2 IS NULL OR actor_id = ?2)
             ORDER BY updated_at DESC, id"
        ));
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), actor_id],
            row_to_composition,
        )
        .map_err(store_error)
    }

    fn update_composition(
        &self,
        composition: &WorkspaceComposition,
    ) -> Result<(), PlatformStoreError> {
        let now = now()?;
        let selection = json_array(&composition.plugin_selection).map_err(store_error)?;
        let changed = self
            .conn
            .execute(
                "UPDATE workspace_compositions SET name = ?1, profile_ref = ?2,
                 plugin_selection = ?3, navigation_config = ?4, updated_at = ?5
                 WHERE id = ?6 AND workspace_id = ?7",
                rusqlite::params![
                    composition.name,
                    composition.profile_ref.as_ref().map(|p| p.as_str()),
                    selection,
                    composition.navigation_config,
                    now,
                    composition.id.as_str(),
                    composition.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(PlatformStoreError::NotFound);
        }
        Ok(())
    }
}
