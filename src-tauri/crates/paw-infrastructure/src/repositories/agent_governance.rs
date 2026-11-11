//! SQLite adapters for the AI Collaboration ports (E5-02c/E5-03/E5-04).
//!
//! Four repositories over migration `0009_agent_governance`:
//! registrations, executions, approvals and agent grants (the grant store
//! backing the governed authorizer). Every query carries a workspace
//! predicate (Gate C); cross-workspace ids surface as `WorkspaceMismatch`
//! and are never returned. JSON array columns are validated on write and
//! `expect`-unpacked on read because the schema CHECK (`json_valid`)
//! constrains what can be stored.
//!
//! Execution history is never cascade-deleted: registrations and workspaces
//! RESTRICT, and statuses `TimedOut`/`OutcomeUnknown` stay queryable until
//! reconciled (ADR-019; `E5-implementation-specification` §10).

use std::path::Path;

use paw_application::ports::agent_repository::{
    AgentExecutionRepository, AgentRegistrationRepository, AgentStoreError, ApprovalRepository,
};
use paw_application::ports::authorization::{AgentGrant, AuthorizationError, GrantStore};
use paw_domain::ai_collaboration::{
    AdapterKind, AgentExecution, AgentExecutionId, AgentRegistration, AgentRegistrationId,
    Approval, ApprovalId, ApprovalStatus, AttemptPolicy, ExecutionStatus, RegistrationStatus,
};
use paw_domain::work_management::{ParseError, WorkspaceId};

use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Json(serde_json::Error),
    Parse(ParseError),
    NotFound,
    // WorkspaceMismatch is decided directly against the fetched row and
    // returned as `AgentStoreError::WorkspaceMismatch`; it has no driver
    // error source of its own.
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

impl From<RepoError> for AgentStoreError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => AgentStoreError::Invariant(e.to_string()),
            RepoError::Now(e) => AgentStoreError::Storage(e.to_string()),
            RepoError::Json(e) => AgentStoreError::Invariant(e.to_string()),
            RepoError::Parse(e) => AgentStoreError::Invariant(e.to_string()),
            RepoError::NotFound => AgentStoreError::NotFound,
        }
    }
}

impl From<RepoError> for AuthorizationError {
    fn from(_: RepoError) -> Self {
        // Grant-store failures fail closed with a non-leaky denial (ADR-029
        // §4.1: missing or unavailable authorization context results in deny).
        AuthorizationError::Denied
    }
}

fn store_error(e: RepoError) -> AgentStoreError {
    e.into()
}

/// Fetch the first row of a prepared query, mapping no-rows to `NotFound`.
/// Accepts the raw `prepare` result so call sites never need error
/// conversion just to hand the statement over.
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

/// Fetch all rows of a prepared query in order.
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

fn json_array(values: &[String]) -> Result<String, RepoError> {
    Ok(serde_json::to_string(values)?)
}

fn parse_json_array(text: String) -> Result<Vec<String>, RepoError> {
    serde_json::from_str(&text).map_err(RepoError::Json)
}

fn now() -> Result<i64, AgentStoreError> {
    sqlite::now_ms().map_err(|e| AgentStoreError::Storage(e.to_string()))
}

// ---------------------------------------------------------------------------
// Registrations
// ---------------------------------------------------------------------------

pub struct SqliteAgentRegistrationRepository {
    conn: rusqlite::Connection,
}

impl SqliteAgentRegistrationRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_registration(row: &rusqlite::Row<'_>) -> Result<AgentRegistration, RepoError> {
    let adapter_kind: String = row.get("adapter_kind")?;
    let status: String = row.get("status")?;
    Ok(AgentRegistration {
        id: AgentRegistrationId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        display_name: row.get("display_name")?,
        adapter_kind: AdapterKind::parse(&adapter_kind)
            .expect("adapter kind values are constrained by the schema CHECK"),
        declared_capabilities: parse_json_array(row.get("declared_capabilities")?)?,
        status: RegistrationStatus::parse(&status)
            .expect("registration status values are constrained by the schema CHECK"),
    })
}

impl AgentRegistrationRepository for SqliteAgentRegistrationRepository {
    fn insert_registration(&self, registration: &AgentRegistration) -> Result<(), AgentStoreError> {
        let now = now()?;
        let declared = json_array(&registration.declared_capabilities).map_err(store_error)?;
        self.conn
            .execute(
                "INSERT INTO agent_registrations
                 (id, workspace_id, display_name, adapter_kind, declared_capabilities, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                rusqlite::params![
                    registration.id.as_str(),
                    registration.workspace_id.as_str(),
                    registration.display_name,
                    registration.adapter_kind.as_str(),
                    declared,
                    registration.status.as_str(),
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_registration(
        &self,
        id: &AgentRegistrationId,
        workspace_id: &WorkspaceId,
    ) -> Result<AgentRegistration, AgentStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, display_name, adapter_kind, declared_capabilities, status
             FROM agent_registrations WHERE id = ?1",
        );
        let found = query_one(stmt, rusqlite::params![id.as_str()], row_to_registration)
            .map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(AgentStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_registrations(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<AgentRegistration>, AgentStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, display_name, adapter_kind, declared_capabilities, status
             FROM agent_registrations WHERE workspace_id = ?1 ORDER BY created_at, id",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str()],
            row_to_registration,
        )
        .map_err(store_error)
    }

    fn update_registration_status(
        &self,
        registration: &AgentRegistration,
    ) -> Result<(), AgentStoreError> {
        let now = now()?;
        let changed = self
            .conn
            .execute(
                "UPDATE agent_registrations SET status = ?1, updated_at = ?2
                 WHERE id = ?3 AND workspace_id = ?4",
                rusqlite::params![
                    registration.status.as_str(),
                    now,
                    registration.id.as_str(),
                    registration.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(AgentStoreError::NotFound);
        }
        Ok(())
    }
}

pub struct SqliteAgentExecutionRepository {
    conn: rusqlite::Connection,
}

impl SqliteAgentExecutionRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const EXECUTION_COLUMNS: &str = "id, registration_id, workspace_id, capability_id, \
     capability_contract_version, plan_digest, status, operation_id, created_at, updated_at";

fn row_to_execution(row: &rusqlite::Row<'_>) -> Result<AgentExecution, RepoError> {
    let status: String = row.get("status")?;
    Ok(AgentExecution {
        id: AgentExecutionId::new(row.get::<_, String>("id")?)?,
        registration_id: AgentRegistrationId::new(row.get::<_, String>("registration_id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        capability_id: row.get("capability_id")?,
        capability_contract_version: row.get("capability_contract_version")?,
        plan_digest: row.get("plan_digest")?,
        status: ExecutionStatus::parse(&status)
            .expect("execution status values are constrained by the schema CHECK"),
        operation_id: row.get("operation_id")?,
    })
}

impl AgentExecutionRepository for SqliteAgentExecutionRepository {
    fn insert_execution(&self, execution: &AgentExecution) -> Result<(), AgentStoreError> {
        let now = now()?;
        self.conn
            .execute(
                "INSERT INTO agent_executions
                 (id, registration_id, workspace_id, capability_id, capability_contract_version,
                  plan_digest, status, operation_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                rusqlite::params![
                    execution.id.as_str(),
                    execution.registration_id.as_str(),
                    execution.workspace_id.as_str(),
                    execution.capability_id,
                    execution.capability_contract_version,
                    execution.plan_digest,
                    execution.status.as_str(),
                    execution.operation_id,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_execution(
        &self,
        id: &AgentExecutionId,
        workspace_id: &WorkspaceId,
    ) -> Result<AgentExecution, AgentStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {EXECUTION_COLUMNS} FROM agent_executions WHERE id = ?1"
        ));
        let found = query_one(stmt, rusqlite::params![id.as_str()], row_to_execution)
            .map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(AgentStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_executions(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<AgentExecution>, AgentStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {EXECUTION_COLUMNS} FROM agent_executions
             WHERE workspace_id = ?1 ORDER BY updated_at DESC, id LIMIT ?2"
        ));
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), limit as i64],
            row_to_execution,
        )
        .map_err(store_error)
    }

    fn update_execution(&self, execution: &AgentExecution) -> Result<(), AgentStoreError> {
        let now = now()?;
        let changed = self
            .conn
            .execute(
                "UPDATE agent_executions SET status = ?1, operation_id = ?2, updated_at = ?3
                 WHERE id = ?4 AND workspace_id = ?5",
                rusqlite::params![
                    execution.status.as_str(),
                    execution.operation_id,
                    now,
                    execution.id.as_str(),
                    execution.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(AgentStoreError::NotFound);
        }
        Ok(())
    }

    fn list_unresolved_executions(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<AgentExecution>, AgentStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {EXECUTION_COLUMNS} FROM agent_executions
             WHERE workspace_id = ?1 AND status IN ('TimedOut','OutcomeUnknown')
             ORDER BY updated_at"
        ));
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str()],
            row_to_execution,
        )
        .map_err(store_error)
    }

    fn count_executions_by_status(
        &self,
        workspace_id: &WorkspaceId,
        status: ExecutionStatus,
    ) -> Result<usize, AgentStoreError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM agent_executions
                 WHERE workspace_id = ?1 AND status = ?2",
                rusqlite::params![workspace_id.as_str(), status.as_str()],
                |row| row.get(0),
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(count as usize)
    }
}

// ---------------------------------------------------------------------------
// Approvals
// ---------------------------------------------------------------------------

pub struct SqliteApprovalRepository {
    conn: rusqlite::Connection,
}

impl SqliteApprovalRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

const APPROVAL_COLUMNS: &str = "id, workspace_id, canonical_digest, actor_id, agent_registration_id, \
     capability_id, capability_contract_version, tool_adapter, target_identity, material_params_digest, \
     side_effect_set, risk_class, expires_at_ms, attempt_policy, attempts_allowed, consumed_attempts, status, created_at";

fn row_to_approval(row: &rusqlite::Row<'_>) -> Result<Approval, RepoError> {
    let registration_id: Option<String> = row.get("agent_registration_id")?;
    let status: String = row.get("status")?;
    let policy: String = row.get("attempt_policy")?;
    let allowed: u32 = row.get::<_, i64>("attempts_allowed")? as u32;
    Ok(Approval {
        id: ApprovalId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        canonical_digest: row.get("canonical_digest")?,
        actor_id: row.get("actor_id")?,
        agent_registration_id: registration_id.map(AgentRegistrationId::new).transpose()?,
        capability_id: row.get("capability_id")?,
        capability_contract_version: row.get("capability_contract_version")?,
        tool_adapter: row.get("tool_adapter")?,
        target_identity: row.get("target_identity")?,
        material_params_digest: row.get("material_params_digest")?,
        side_effect_set: parse_json_array(row.get("side_effect_set")?)?,
        risk_class: row.get("risk_class")?,
        expires_at_ms: row.get("expires_at_ms")?,
        attempt_policy: AttemptPolicy::parse_db(&policy, allowed)
            .expect("attempt policy values are constrained by the schema CHECK"),
        consumed_attempts: row.get::<_, i64>("consumed_attempts")? as u32,
        status: ApprovalStatus::parse(&status)
            .expect("approval status values are constrained by the schema CHECK"),
    })
}

impl ApprovalRepository for SqliteApprovalRepository {
    fn insert_approval(&self, approval: &Approval) -> Result<(), AgentStoreError> {
        let now = now()?;
        let effects = json_array(&approval.side_effect_set).map_err(store_error)?;
        let (policy_text, attempts_allowed) = approval.attempt_policy.as_db();
        self.conn
            .execute(
                "INSERT INTO approvals
                 (id, workspace_id, canonical_digest, actor_id, agent_registration_id,
                  capability_id, capability_contract_version, tool_adapter, target_identity,
                  material_params_digest, side_effect_set, risk_class, expires_at_ms,
                  attempt_policy, attempts_allowed, consumed_attempts, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                rusqlite::params![
                    approval.id.as_str(),
                    approval.workspace_id.as_str(),
                    approval.canonical_digest,
                    approval.actor_id,
                    approval.agent_registration_id.as_ref().map(|id| id.as_str()),
                    approval.capability_id,
                    approval.capability_contract_version,
                    approval.tool_adapter,
                    approval.target_identity,
                    approval.material_params_digest,
                    effects,
                    approval.risk_class,
                    approval.expires_at_ms,
                    policy_text,
                    attempts_allowed,
                    approval.consumed_attempts,
                    approval.status.as_str(),
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_approval(
        &self,
        id: &ApprovalId,
        workspace_id: &WorkspaceId,
    ) -> Result<Approval, AgentStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {APPROVAL_COLUMNS} FROM approvals WHERE id = ?1"
        ));
        let found = query_one(stmt, rusqlite::params![id.as_str()], row_to_approval)
            .map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(AgentStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_pending_approvals(
        &self,
        workspace_id: &WorkspaceId,
        now_ms: i64,
    ) -> Result<Vec<Approval>, AgentStoreError> {
        let stmt = self.conn.prepare(&format!(
            "SELECT {APPROVAL_COLUMNS} FROM approvals
             WHERE workspace_id = ?1 AND status = 'Active' AND expires_at_ms > ?2
             ORDER BY created_at, id"
        ));
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), now_ms],
            row_to_approval,
        )
        .map_err(store_error)
    }

    fn update_approval(&self, approval: &Approval) -> Result<(), AgentStoreError> {
        let changed = self
            .conn
            .execute(
                "UPDATE approvals SET consumed_attempts = ?1, status = ?2
                 WHERE id = ?3 AND workspace_id = ?4",
                rusqlite::params![
                    approval.consumed_attempts,
                    approval.status.as_str(),
                    approval.id.as_str(),
                    approval.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(AgentStoreError::NotFound);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Grants (the GrantStore backing the governed authorizer)
// ---------------------------------------------------------------------------

pub struct SqliteAgentGrantRepository {
    conn: rusqlite::Connection,
}

impl SqliteAgentGrantRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }

    /// Insert a grant. Rows are read fresh on every authorization check, so
    /// creation takes effect at the next side effect (ADR-029 §4.4).
    pub fn insert_grant(
        &self,
        grant: &AgentGrant,
        created_by: &str,
    ) -> Result<(), AgentStoreError> {
        let now = now()?;
        let capabilities = json_array(&grant.capabilities).map_err(store_error)?;
        let tools = json_array(&grant.tools).map_err(store_error)?;
        self.conn
            .execute(
                "INSERT INTO agent_grants
                 (id, subject_id, capabilities, workspace_id, tools, valid_from_ms, valid_until_ms, created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    grant.id,
                    grant.subject_id,
                    capabilities,
                    grant.workspace_id.as_str(),
                    tools,
                    grant.valid_from_ms,
                    grant.valid_until_ms,
                    created_by,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    /// Revoke a grant by deleting its row. Deletion is the revocation
    /// (reads are fresh); the grant id never authorized anything by itself.
    pub fn delete_grant(
        &self,
        grant_id: &str,
        workspace_id: &WorkspaceId,
    ) -> Result<(), AgentStoreError> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM agent_grants WHERE id = ?1 AND workspace_id = ?2",
                rusqlite::params![grant_id, workspace_id.as_str()],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(AgentStoreError::NotFound);
        }
        Ok(())
    }

    /// Grants bound to a workspace (admin listing; subject filter optional).
    pub fn list_grants(
        &self,
        workspace_id: &WorkspaceId,
        subject_id: Option<&str>,
    ) -> Result<Vec<AgentGrant>, AgentStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, subject_id, capabilities, workspace_id, tools, valid_from_ms, valid_until_ms
             FROM agent_grants WHERE workspace_id = ?1 AND (?2 IS NULL OR subject_id = ?2)
             ORDER BY created_at, id",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), subject_id],
            grant_from_row,
        )
        .map_err(store_error)
    }
}

fn grant_from_row(row: &rusqlite::Row<'_>) -> Result<AgentGrant, RepoError> {
    let capabilities: String = row.get("capabilities")?;
    let tools: String = row.get("tools")?;
    Ok(AgentGrant {
        id: row.get("id")?,
        subject_id: row.get("subject_id")?,
        capabilities: serde_json::from_str(&capabilities)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        tools: serde_json::from_str(&tools)?,
        valid_from_ms: row.get("valid_from_ms")?,
        valid_until_ms: row.get("valid_until_ms")?,
    })
}

impl GrantStore for SqliteAgentGrantRepository {
    fn grants_for_subject(&self, subject_id: &str) -> Result<Vec<AgentGrant>, AuthorizationError> {
        let stmt = self.conn.prepare(
            "SELECT id, subject_id, capabilities, workspace_id, tools, valid_from_ms, valid_until_ms
             FROM agent_grants WHERE subject_id = ?1",
        );
        query_all(stmt, rusqlite::params![subject_id], grant_from_row)
            .map_err(AuthorizationError::from)
    }
}
