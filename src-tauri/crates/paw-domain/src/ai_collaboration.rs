//! AI Collaboration domain entities: Agent registrations, Agent executions,
//! and approval records (E5, PAW-INF-005/006).
//!
//! Ownership per ADR-026 (five-layer model) and `agent-adapter-architecture.md`:
//! AI Collaboration owns the Agent-facing lifecycle — registrations, execution
//! records and approval state. The adapter owns provider protocol translation
//! only; the external Agent Runtime is untrusted and never appears as a domain
//! entity. The Operation envelope (ADR-019) stays Application Coordination-
//! owned: an execution references its operation and the two state machines
//! never inherit each other's transitions (E5-implementation-specification §7).
//!
//! This module is pure: no SQL, filesystem, OS, Tauri, or provider SDK
//! dependency (ADR-017/018). The SQLite CHECK constraints in migration 0009
//! mirror these state strings exactly.

use serde::{Deserialize, Serialize};

use crate::work_management::{WorkspaceId, opaque_id};

opaque_id!(
    AgentRegistrationId,
    "Opaque agent registration identifier (UUIDv7 text)."
);
opaque_id!(
    AgentExecutionId,
    "Opaque agent execution identifier (UUIDv7 text)."
);
opaque_id!(
    ApprovalId,
    "Opaque approval record identifier (UUIDv7 text)."
);

/// Governance invariant violations for the AI Collaboration module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceError {
    /// A canonical/plan/material digest is not 64 lowercase hex characters.
    InvalidDigest,
    /// A capability id is empty or contains characters outside the
    /// `domain.action` namespace (`[a-z0-9._]`).
    InvalidCapabilityId,
    /// A contract version is not `major.minor.patch` with a nonzero major.
    InvalidContractVersion,
    /// An execution state transition is not published.
    InvalidTransition { from: String, to: String },
    /// The approval is not active (consumed/expired/revoked).
    ApprovalNotActive { status: String },
    /// The approval is active in status but its validity window has passed.
    ApprovalExpired,
    /// The approval's attempt policy is exhausted.
    ApprovalExhausted { used: u32, allowed: u32 },
    /// The presented invocation does not match the approval's binding set.
    ApprovalBindingMismatch,
    /// A registration is not in the state required by the action
    /// (e.g. invoking a suspended or retired agent).
    RegistrationNotActive { status: String },
}

impl std::fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDigest => write!(f, "digest must be 64 lowercase hex characters"),
            Self::InvalidCapabilityId => {
                write!(f, "capability id must be non-empty `[a-z0-9._]`")
            }
            Self::InvalidContractVersion => {
                write!(f, "contract version must be `major.minor.patch`, major > 0")
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid transition {from} -> {to}")
            }
            Self::ApprovalNotActive { status } => write!(f, "approval is {status}"),
            Self::ApprovalExpired => write!(f, "approval validity window has passed"),
            Self::ApprovalExhausted { used, allowed } => {
                write!(f, "approval attempts exhausted ({used}/{allowed})")
            }
            Self::ApprovalBindingMismatch => {
                write!(f, "invocation does not match the approval binding")
            }
            Self::RegistrationNotActive { status } => write!(f, "agent registration is {status}"),
        }
    }
}

impl std::error::Error for GovernanceError {}

/// Validate a canonical digest: exactly 64 lowercase hex characters
/// (SHA-256 output, `E5-implementation-specification` §5).
pub fn validate_digest(value: &str) -> Result<(), GovernanceError> {
    let ok = value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if ok {
        Ok(())
    } else {
        Err(GovernanceError::InvalidDigest)
    }
}

/// Validate a capability id: non-empty, `[a-z0-9._]`, at least one dot
/// (`domain.action` namespace, `E5-implementation-specification` §4).
pub fn validate_capability_id(value: &str) -> Result<(), GovernanceError> {
    let ok = !value.is_empty()
        && value.contains('.')
        && value
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_'));
    if ok {
        Ok(())
    } else {
        Err(GovernanceError::InvalidCapabilityId)
    }
}

/// Validate a capability contract version string (`major.minor.patch`,
/// major > 0 — unknown-major contracts fail closed per ADR-028 §4.3).
pub fn validate_contract_version(value: &str) -> Result<(), GovernanceError> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 3 {
        return Err(GovernanceError::InvalidContractVersion);
    }
    let major: u32 = parts[0]
        .parse()
        .map_err(|_| GovernanceError::InvalidContractVersion)?;
    parts[1]
        .parse::<u32>()
        .map_err(|_| GovernanceError::InvalidContractVersion)?;
    parts[2]
        .parse::<u32>()
        .map_err(|_| GovernanceError::InvalidContractVersion)?;
    if major == 0 {
        return Err(GovernanceError::InvalidContractVersion);
    }
    Ok(())
}

/// How the agent reaches the provider (schema CHECK strings mirror these).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterKind {
    /// In-process fake adapter (E5 conformance surface; no external effect).
    Fake,
    /// Hermes CLI transport (gated behind ADR-022; not enabled in E5).
    CliHermes,
    /// Codex CLI transport (gated behind ADR-022; not enabled in E5).
    CliCodex,
    /// Generic documented API transport (gated behind ADR-022).
    ApiCustom,
}

impl AdapterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fake => "fake",
            Self::CliHermes => "cli_hermes",
            Self::CliCodex => "cli_codex",
            Self::ApiCustom => "api_custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "fake" => Self::Fake,
            "cli_hermes" => Self::CliHermes,
            "cli_codex" => Self::CliCodex,
            "api_custom" => Self::ApiCustom,
            _ => return None,
        })
    }
}

/// Registration lifecycle (`E5-implementation-specification` §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationStatus {
    Active,
    Suspended,
    Retired,
}

impl RegistrationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Suspended => "Suspended",
            Self::Retired => "Retired",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Active" => Self::Active,
            "Suspended" => Self::Suspended,
            "Retired" => Self::Retired,
            _ => return None,
        })
    }

    /// Published transitions. Retired is terminal; history is preserved
    /// (a retired registration's executions stay queryable).
    pub fn can_transition_to(self, to: Self) -> bool {
        matches!(
            (self, to),
            (Self::Active, Self::Suspended)
                | (Self::Active, Self::Retired)
                | (Self::Suspended, Self::Active)
                | (Self::Suspended, Self::Retired)
        )
    }
}

/// A workbench-owned agent registration (`agent_registrations`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub id: AgentRegistrationId,
    pub workspace_id: WorkspaceId,
    pub display_name: String,
    pub adapter_kind: AdapterKind,
    /// Capability ids the registration declares it may be proposed for.
    /// Declarative metadata only — never a grant (ADR-026 R1).
    pub declared_capabilities: Vec<String>,
    pub status: RegistrationStatus,
}

impl AgentRegistration {
    pub fn new(
        id: AgentRegistrationId,
        workspace_id: WorkspaceId,
        display_name: &str,
        adapter_kind: AdapterKind,
        declared_capabilities: Vec<String>,
    ) -> Result<Self, GovernanceError> {
        for capability in &declared_capabilities {
            validate_capability_id(capability)?;
        }
        Ok(Self {
            id,
            workspace_id,
            display_name: display_name.trim().to_string(),
            adapter_kind,
            declared_capabilities,
            status: RegistrationStatus::Active,
        })
    }

    /// Guard for invocation: only an active registration may be proposed.
    pub fn ensure_active(&self) -> Result<(), GovernanceError> {
        if self.status == RegistrationStatus::Active {
            Ok(())
        } else {
            Err(GovernanceError::RegistrationNotActive {
                status: self.status.as_str().to_string(),
            })
        }
    }
}

/// Agent execution lifecycle (`agent-adapter-architecture.md` §3; schema
/// CHECK strings mirror these exactly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Created,
    Validating,
    AwaitingApproval,
    Approved,
    Submitted,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    OutcomeUnknown,
}

impl ExecutionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Validating => "Validating",
            Self::AwaitingApproval => "AwaitingApproval",
            Self::Approved => "Approved",
            Self::Submitted => "Submitted",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
            Self::TimedOut => "TimedOut",
            Self::OutcomeUnknown => "OutcomeUnknown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Created" => Self::Created,
            "Validating" => Self::Validating,
            "AwaitingApproval" => Self::AwaitingApproval,
            "Approved" => Self::Approved,
            "Submitted" => Self::Submitted,
            "Running" => Self::Running,
            "Completed" => Self::Completed,
            "Failed" => Self::Failed,
            "Cancelled" => Self::Cancelled,
            "TimedOut" => Self::TimedOut,
            "OutcomeUnknown" => Self::OutcomeUnknown,
            _ => return None,
        })
    }

    /// Terminal: the execution outcome is established. `TimedOut` and
    /// `OutcomeUnknown` are deliberately NOT terminal — an unresolved
    /// external outcome must be reconciled, never inferred (ADR-029 §4.5).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Allowed execution transitions (`agent-adapter-architecture.md` §3 with
/// ADR-019 reconciliation semantics; `E5-implementation-specification` §7):
/// - `Created → Validating | Failed | Cancelled`
/// - `Validating → AwaitingApproval | Approved | Failed | Cancelled`
///   (direct approval when the capability needs none)
/// - `AwaitingApproval → Approved | Failed | Cancelled`
/// - `Approved → Submitted | Failed | Cancelled`
/// - `Submitted → Running | OutcomeUnknown | Failed | Cancelled`
///   (a submit timeout after possible provider acceptance is indeterminate)
/// - `Running → Completed | Failed | Cancelled | TimedOut | OutcomeUnknown`
/// - `TimedOut → Completed | Failed | OutcomeUnknown | Cancelled`
/// - `OutcomeUnknown → Completed | Failed | Cancelled` — resuming into
///   `Running` is intentionally absent: an unresolved outcome blocks a
///   non-idempotent rerun until explicit reconciliation.
pub fn is_valid_execution_transition(from: ExecutionStatus, to: ExecutionStatus) -> bool {
    use ExecutionStatus::*;
    if from.is_terminal() {
        return false;
    }
    matches!(
        (from, to),
        (Created, Validating)
            | (Created, Failed)
            | (Created, Cancelled)
            | (Validating, AwaitingApproval)
            | (Validating, Approved)
            | (Validating, Failed)
            | (Validating, Cancelled)
            | (AwaitingApproval, Approved)
            | (AwaitingApproval, Failed)
            | (AwaitingApproval, Cancelled)
            | (Approved, Submitted)
            | (Approved, Failed)
            | (Approved, Cancelled)
            | (Submitted, Running)
            | (Submitted, OutcomeUnknown)
            | (Submitted, Failed)
            | (Submitted, Cancelled)
            | (Running, Completed)
            | (Running, Failed)
            | (Running, Cancelled)
            | (Running, TimedOut)
            | (Running, OutcomeUnknown)
            | (TimedOut, Completed)
            | (TimedOut, Failed)
            | (TimedOut, OutcomeUnknown)
            | (TimedOut, Cancelled)
            | (OutcomeUnknown, Completed)
            | (OutcomeUnknown, Failed)
            | (OutcomeUnknown, Cancelled)
    )
}

/// A workbench-owned agent execution record (`agent_executions`). The record
/// is the provider-facing lifecycle; the durable Operation is the cross-cutting
/// envelope and is linked by id without sharing state machines (ADR-019).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecution {
    pub id: AgentExecutionId,
    pub registration_id: AgentRegistrationId,
    pub workspace_id: WorkspaceId,
    pub capability_id: String,
    pub capability_contract_version: String,
    /// Canonical invocation plan digest (SHA-256 hex; §5 of the spec).
    pub plan_digest: String,
    pub status: ExecutionStatus,
    /// The durable Operation envelope opened for this execution, set once
    /// at pipeline open. Ids are opaque; the operation owns its own state.
    pub operation_id: Option<String>,
}

impl AgentExecution {
    pub fn new(
        id: AgentExecutionId,
        registration_id: AgentRegistrationId,
        workspace_id: WorkspaceId,
        capability_id: &str,
        capability_contract_version: &str,
        plan_digest: &str,
    ) -> Result<Self, GovernanceError> {
        validate_capability_id(capability_id)?;
        validate_contract_version(capability_contract_version)?;
        validate_digest(plan_digest)?;
        Ok(Self {
            id,
            registration_id,
            workspace_id,
            capability_id: capability_id.to_string(),
            capability_contract_version: capability_contract_version.to_string(),
            plan_digest: plan_digest.to_string(),
            status: ExecutionStatus::Created,
            operation_id: None,
        })
    }

    /// Link the durable Operation envelope (once, at pipeline open).
    pub fn associate_operation(&mut self, operation_id: &str) -> Result<(), GovernanceError> {
        if self.operation_id.is_some() {
            return Err(GovernanceError::InvalidTransition {
                from: format!("operation={:?}", self.operation_id),
                to: "operation re-association".to_string(),
            });
        }
        self.operation_id = Some(operation_id.to_string());
        Ok(())
    }

    /// Advance the lifecycle through a published transition.
    pub fn transition(&mut self, to: ExecutionStatus) -> Result<(), GovernanceError> {
        if !is_valid_execution_transition(self.status, to) {
            return Err(GovernanceError::InvalidTransition {
                from: self.status.as_str().to_string(),
                to: to.as_str().to_string(),
            });
        }
        self.status = to;
        Ok(())
    }
}

/// Attempt policy bound into an approval (ADR-029 §4.3: one-time or
/// explicitly bounded attempt set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttemptPolicy {
    Single,
    Bounded(u32),
}

impl AttemptPolicy {
    /// (schema CHECK string, allowed attempt count).
    pub fn as_db(self) -> (&'static str, u32) {
        match self {
            Self::Single => ("single", 1),
            Self::Bounded(n) => ("bounded", n.max(1)),
        }
    }

    pub fn parse_db(policy: &str, allowed: u32) -> Option<Self> {
        Some(match policy {
            "single" => Self::Single,
            "bounded" => Self::Bounded(allowed.max(1)),
            _ => return None,
        })
    }
}

/// Approval record lifecycle. `Expired` is time-derived at use time: a row
/// whose validity window has passed is refused even while still `Active` in
/// storage (`E5-implementation-specification` §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Active,
    Consumed,
    Expired,
    Revoked,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Consumed => "Consumed",
            Self::Expired => "Expired",
            Self::Revoked => "Revoked",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Active" => Self::Active,
            "Consumed" => Self::Consumed,
            "Expired" => Self::Expired,
            "Revoked" => Self::Revoked,
            _ => return None,
        })
    }
}

/// The invocation identity an approval binds to (ADR-029 §4.3: actor, agent,
/// data workspace, capability + contract version, tool/adapter, target and
/// the canonical digest of the immutable plan; material parameters enter the
/// plan itself and therefore the digest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBinding {
    pub canonical_digest: String,
    pub actor_id: String,
    pub agent_registration_id: Option<AgentRegistrationId>,
    pub workspace_id: WorkspaceId,
    pub capability_id: String,
    pub capability_contract_version: String,
    pub tool_adapter: String,
    pub target_identity: String,
    /// Digest over the material parameter payload (64-hex; the raw values
    /// are never copied into the approval record).
    pub material_params_digest: String,
}

/// A human approval record (`approvals`). Secret values are never stored —
/// only opaque references and digests (ADR-029 §4.3/§4.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub id: ApprovalId,
    pub workspace_id: WorkspaceId,
    /// The canonical digest this approval is bound to; any material change
    /// to the plan invalidates the approval.
    pub canonical_digest: String,
    pub actor_id: String,
    pub agent_registration_id: Option<AgentRegistrationId>,
    pub capability_id: String,
    pub capability_contract_version: String,
    pub tool_adapter: String,
    pub target_identity: String,
    pub material_params_digest: String,
    /// Declared side-effect set (capability-scoped effect identifiers).
    pub side_effect_set: Vec<String>,
    pub risk_class: String,
    /// Unix epoch milliseconds after which the approval is unusable.
    pub expires_at_ms: i64,
    pub attempt_policy: AttemptPolicy,
    pub consumed_attempts: u32,
    pub status: ApprovalStatus,
}

impl Approval {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ApprovalId,
        binding: ApprovalBinding,
        side_effect_set: Vec<String>,
        risk_class: &str,
        expires_at_ms: i64,
        attempt_policy: AttemptPolicy,
    ) -> Result<Self, GovernanceError> {
        validate_digest(&binding.canonical_digest)?;
        validate_digest(&binding.material_params_digest)?;
        validate_capability_id(&binding.capability_id)?;
        validate_contract_version(&binding.capability_contract_version)?;
        Ok(Self {
            id,
            // Single source of truth: the approval is scoped to exactly the
            // workspace its binding presents, so the two can never diverge.
            workspace_id: binding.workspace_id,
            canonical_digest: binding.canonical_digest,
            actor_id: binding.actor_id,
            agent_registration_id: binding.agent_registration_id,
            capability_id: binding.capability_id,
            capability_contract_version: binding.capability_contract_version,
            tool_adapter: binding.tool_adapter,
            target_identity: binding.target_identity,
            material_params_digest: binding.material_params_digest,
            side_effect_set,
            risk_class: risk_class.to_string(),
            expires_at_ms,
            consumed_attempts: 0,
            status: ApprovalStatus::Active,
            attempt_policy,
        })
    }

    /// Time-derived status: an `Active` approval past its window reads as
    /// `Expired` (consumption and validation refuse it).
    pub fn effective_status(&self, now_ms: i64) -> ApprovalStatus {
        if self.status == ApprovalStatus::Active && now_ms >= self.expires_at_ms {
            ApprovalStatus::Expired
        } else {
            self.status
        }
    }

    /// Validate a presented invocation against this approval's binding set.
    /// A mismatch is refused without mutating the record.
    pub fn validate_binding(&self, binding: &ApprovalBinding) -> Result<(), GovernanceError> {
        let matches = self.canonical_digest == binding.canonical_digest
            && self.actor_id == binding.actor_id
            && self.agent_registration_id == binding.agent_registration_id
            && self.workspace_id == binding.workspace_id
            && self.capability_id == binding.capability_id
            && self.capability_contract_version == binding.capability_contract_version
            && self.tool_adapter == binding.tool_adapter
            && self.target_identity == binding.target_identity
            && self.material_params_digest == binding.material_params_digest;
        if matches {
            Ok(())
        } else {
            Err(GovernanceError::ApprovalBindingMismatch)
        }
    }

    /// Consume one attempt of an active, unexpired, under-quota approval.
    /// Reaching the attempt allowance marks the approval `Consumed`.
    pub fn consume(&mut self, now_ms: i64) -> Result<(), GovernanceError> {
        match self.effective_status(now_ms) {
            ApprovalStatus::Active => {}
            ApprovalStatus::Expired => return Err(GovernanceError::ApprovalExpired),
            other => {
                return Err(GovernanceError::ApprovalNotActive {
                    status: other.as_str().to_string(),
                });
            }
        }
        let (_, allowed) = self.attempt_policy.as_db();
        if self.consumed_attempts >= allowed {
            return Err(GovernanceError::ApprovalExhausted {
                used: self.consumed_attempts,
                allowed,
            });
        }
        self.consumed_attempts += 1;
        if self.consumed_attempts >= allowed {
            self.status = ApprovalStatus::Consumed;
        }
        Ok(())
    }

    /// Explicit revocation (owner action). Revoking a non-active approval
    /// is refused; expiry/consumption states are facts, not decisions.
    pub fn revoke(&mut self) -> Result<(), GovernanceError> {
        if self.status != ApprovalStatus::Active {
            return Err(GovernanceError::ApprovalNotActive {
                status: self.status.as_str().to_string(),
            });
        }
        self.status = ApprovalStatus::Revoked;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> WorkspaceId {
        WorkspaceId::new_v7()
    }

    fn reg() -> AgentRegistration {
        AgentRegistration::new(
            AgentRegistrationId::new_v7(),
            ws(),
            "fake-agent",
            AdapterKind::Fake,
            vec!["work_management.task.create".to_string()],
        )
        .expect("valid registration")
    }

    fn digest() -> String {
        "a".repeat(64)
    }

    #[test]
    fn registration_lifecycle_and_invoke_guard() {
        let mut r = reg();
        assert!(r.ensure_active().is_ok());
        assert!(r.status.can_transition_to(RegistrationStatus::Suspended));
        r.status = RegistrationStatus::Suspended;
        assert!(r.ensure_active().is_err());
        assert!(r.status.can_transition_to(RegistrationStatus::Active));
        r.status = RegistrationStatus::Active;
        assert!(r.status.can_transition_to(RegistrationStatus::Retired));
        r.status = RegistrationStatus::Retired;
        assert!(!r.status.can_transition_to(RegistrationStatus::Active));
        assert!(r.ensure_active().is_err());
    }

    #[test]
    fn registration_rejects_bad_capability_ids() {
        let err = AgentRegistration::new(
            AgentRegistrationId::new_v7(),
            ws(),
            "x",
            AdapterKind::Fake,
            vec!["not a capability".to_string()],
        )
        .expect_err("invalid capability id must be rejected");
        assert_eq!(err, GovernanceError::InvalidCapabilityId);
    }

    #[test]
    fn execution_lifecycle_happy_path() {
        let mut e = AgentExecution::new(
            AgentExecutionId::new_v7(),
            reg().id,
            ws(),
            "work_management.task.create",
            "1.0.0",
            &digest(),
        )
        .expect("valid execution");
        assert_eq!(e.status, ExecutionStatus::Created);
        for step in [
            ExecutionStatus::Validating,
            ExecutionStatus::AwaitingApproval,
            ExecutionStatus::Approved,
            ExecutionStatus::Submitted,
            ExecutionStatus::Running,
            ExecutionStatus::Completed,
        ] {
            e.transition(step).expect("published transition");
        }
        assert!(e.status.is_terminal());
    }

    #[test]
    fn execution_unknown_outcome_blocks_blind_rerun() {
        let mut e = AgentExecution::new(
            AgentExecutionId::new_v7(),
            reg().id,
            ws(),
            "work_management.task.create",
            "1.0.0",
            &digest(),
        )
        .expect("valid execution");
        for step in [
            ExecutionStatus::Validating,
            ExecutionStatus::Approved,
            ExecutionStatus::Submitted,
            ExecutionStatus::OutcomeUnknown,
        ] {
            e.transition(step).expect("published transition");
        }
        // The unsafe rerun (back to Running) is blocked at state level.
        assert!(!is_valid_execution_transition(
            ExecutionStatus::OutcomeUnknown,
            ExecutionStatus::Running
        ));
        // Reconciliation resolves to an explicit terminal outcome.
        e.transition(ExecutionStatus::Completed)
            .expect("reconciliation resolves");
    }

    #[test]
    fn execution_terminal_frozen() {
        for terminal in [
            ExecutionStatus::Completed,
            ExecutionStatus::Failed,
            ExecutionStatus::Cancelled,
        ] {
            assert!(!is_valid_execution_transition(
                terminal,
                ExecutionStatus::Running
            ));
        }
    }

    #[test]
    fn execution_operation_associated_once() {
        let mut e = AgentExecution::new(
            AgentExecutionId::new_v7(),
            reg().id,
            ws(),
            "work_management.task.create",
            "1.0.0",
            &digest(),
        )
        .expect("valid execution");
        e.associate_operation("op-1").expect("first link");
        assert!(e.associate_operation("op-2").is_err());
    }

    #[test]
    fn execution_rejects_bad_inputs() {
        assert_eq!(
            AgentExecution::new(
                AgentExecutionId::new_v7(),
                reg().id,
                ws(),
                "no-dot",
                "1.0.0",
                &digest()
            )
            .expect_err("capability id rejected"),
            GovernanceError::InvalidCapabilityId
        );
        assert_eq!(
            AgentExecution::new(
                AgentExecutionId::new_v7(),
                reg().id,
                ws(),
                "work.task",
                "0.1.0",
                &digest()
            )
            .expect_err("zero-major contract rejected"),
            GovernanceError::InvalidContractVersion
        );
        assert_eq!(
            AgentExecution::new(
                AgentExecutionId::new_v7(),
                reg().id,
                ws(),
                "work.task",
                "1.0.0",
                "xyz"
            )
            .expect_err("digest rejected"),
            GovernanceError::InvalidDigest
        );
    }

    fn binding(
        digest_hex: &str,
        registration_id: AgentRegistrationId,
        workspace_id: WorkspaceId,
    ) -> ApprovalBinding {
        ApprovalBinding {
            canonical_digest: digest_hex.to_string(),
            actor_id: "local-user".to_string(),
            agent_registration_id: Some(registration_id),
            workspace_id,
            capability_id: "work_management.task.create".to_string(),
            capability_contract_version: "1.0.0".to_string(),
            tool_adapter: "fake".to_string(),
            target_identity: "provider:fake".to_string(),
            material_params_digest: "b".repeat(64),
        }
    }

    #[test]
    fn approval_binding_exact_match_required() {
        let reg_id = reg().id;
        let ws_id = ws();
        let approval = Approval::new(
            ApprovalId::new_v7(),
            binding(&digest(), reg_id.clone(), ws_id.clone()),
            vec!["effect".to_string()],
            "High",
            1_000,
            AttemptPolicy::Single,
        )
        .expect("valid approval");
        let mut presented = binding(&digest(), reg_id.clone(), ws_id.clone());
        assert!(approval.validate_binding(&presented).is_ok());
        // Any material change invalidates the approval.
        presented.target_identity = "provider:other".to_string();
        assert_eq!(
            approval.validate_binding(&presented),
            Err(GovernanceError::ApprovalBindingMismatch)
        );
        presented = binding(&digest(), reg_id.clone(), ws_id.clone());
        presented.material_params_digest = "c".repeat(64);
        assert_eq!(
            approval.validate_binding(&presented),
            Err(GovernanceError::ApprovalBindingMismatch)
        );
    }

    #[test]
    fn approval_consume_respects_expiry_and_policy() {
        let reg_id = reg().id;
        let ws_id = ws();
        let mut approval = Approval::new(
            ApprovalId::new_v7(),
            binding(&digest(), reg_id.clone(), ws_id.clone()),
            vec![],
            "High",
            500,
            AttemptPolicy::Single,
        )
        .expect("valid approval");
        assert_eq!(approval.effective_status(400), ApprovalStatus::Active);
        assert_eq!(approval.effective_status(500), ApprovalStatus::Expired);
        approval.consume(400).expect("within window");
        assert_eq!(approval.status, ApprovalStatus::Consumed);
        assert_eq!(
            approval.consume(400),
            Err(GovernanceError::ApprovalNotActive {
                status: "Consumed".to_string()
            })
        );
    }

    #[test]
    fn approval_bounded_attempts() {
        let reg_id = reg().id;
        let ws_id = ws();
        let mut approval = Approval::new(
            ApprovalId::new_v7(),
            binding(&digest(), reg_id.clone(), ws_id.clone()),
            vec![],
            "Medium",
            10_000,
            AttemptPolicy::Bounded(2),
        )
        .expect("valid approval");
        approval.consume(1).expect("attempt 1");
        assert_eq!(approval.status, ApprovalStatus::Active);
        approval.consume(1).expect("attempt 2");
        assert_eq!(approval.status, ApprovalStatus::Consumed);
        assert_eq!(
            approval.consume(1),
            Err(GovernanceError::ApprovalNotActive {
                status: "Consumed".to_string()
            })
        );
    }

    #[test]
    fn approval_revoke_only_active() {
        let reg_id = reg().id;
        let ws_id = ws();
        let mut approval = Approval::new(
            ApprovalId::new_v7(),
            binding(&digest(), reg_id.clone(), ws_id.clone()),
            vec![],
            "High",
            10_000,
            AttemptPolicy::Single,
        )
        .expect("valid approval");
        approval.revoke().expect("active revocation");
        assert_eq!(approval.status, ApprovalStatus::Revoked);
        assert_eq!(
            approval.revoke(),
            Err(GovernanceError::ApprovalNotActive {
                status: "Revoked".to_string()
            })
        );
    }

    #[test]
    fn attempt_policy_db_roundtrip() {
        for policy in [AttemptPolicy::Single, AttemptPolicy::Bounded(3)] {
            let (text, allowed) = policy.as_db();
            assert_eq!(AttemptPolicy::parse_db(text, allowed), Some(policy));
        }
    }
}
