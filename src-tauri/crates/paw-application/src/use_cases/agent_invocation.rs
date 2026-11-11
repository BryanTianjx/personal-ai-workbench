//! Governed agent invocation pipeline (E5-04/E5-05; PAW-INF-005/006).
//!
//! Application Coordination owns the pipeline: authorization, approval
//! policy, durable Operations, correlation and outbox mechanics (ADR-017/
//! 026 R1). The domain owns business invariants and capability execution;
//! the adapter owns provider translation only; the external Agent Runtime
//! is untrusted and never receives repository or database access.
//!
//! Pipeline per `E5-implementation-specification` §8:
//! proposal (untrusted) → parse/validate → resolve actor/agent/workspace →
//! resolve capability descriptor + contract version → evaluate current
//! authorization (default deny) → classify risk/approval → canonicalize
//! plan → validate approval binding (if required) → open durable Operation
//! → **re-authorize immediately before each side effect** → execute via the
//! mediated adapter → persist outcome / OutcomeUnknown → reconcile, audit,
//! return bounded result.
//!
//! Two-phase flow: `propose` parks the execution in `AwaitingApproval` when
//! the risk class or hint requires a human decision; `decide_approval`
//! reconstructs the binding **from persisted execution fields only** (never
//! caller-supplied values), records the approval, and runs. Uncertain
//! non-idempotent outcomes hold at `OutcomeUnknown` for `reconcile` — they
//! are never blindly retried (ADR-019/029 §4.5).

use std::collections::BTreeMap;

use paw_domain::ai_collaboration::{
    AgentExecution, AgentExecutionId, AgentRegistrationId, Approval, ApprovalBinding, ApprovalId,
    AttemptPolicy, ExecutionStatus, GovernanceError,
};
use paw_domain::operation_machine::OperationStatus;
use paw_domain::work_management::WorkspaceId;

use crate::ports::agent_adapter::{AdapterSubmitOutcome, AgentAdapter};
use crate::ports::agent_repository::{
    AgentExecutionRepository, AgentRegistrationRepository, AgentStoreError, ApprovalRepository,
};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::canonical_plan::{CanonicalPlan, PlanValue};
use crate::ports::capability::{ApprovalHint, CapabilityRegistry, Resolution, RiskClass};
use crate::ports::operation_service::{
    AttemptOutcome, AttemptPhase, ExternalAttempt, OpenOperation, OperationEnvelope,
    OperationPortError, OperationService,
};

/// A proposal submitted into the pipeline. Untrusted input: every field is
/// validated against the registry and the persisted registration before any
/// state is created (ADR-029 §4.1).
#[derive(Debug, Clone)]
pub struct AgentProposal {
    pub workspace_id: WorkspaceId,
    pub registration_id: AgentRegistrationId,
    pub capability_id: String,
    pub capability_contract_version: (u32, u32, u32),
    /// Material parameters (small value domain; keys sorted in the plan).
    pub parameters: BTreeMap<String, PlanValue>,
    pub correlation_id: String,
    /// Approval validity window from decision time (epoch millis offset).
    pub approval_ttl_ms: i64,
    pub attempt_policy: AttemptPolicy,
}

/// What the approval dialog displays (`E5-implementation-specification` §6):
/// target, action, scope, data flow, risk, expiry. Secret values never
/// appear — the plan carries only opaque references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalDisplay {
    pub execution_id: String,
    pub capability_id: String,
    pub capability_contract_version: String,
    pub tool_adapter: String,
    pub target_identity: String,
    pub side_effect_set: Vec<String>,
    pub risk_class: String,
    pub expires_at_ms: i64,
    pub attempt_policy: AttemptPolicy,
}

/// Outcome of `propose`.
#[derive(Debug, Clone, PartialEq)]
pub enum ProposalOutcome {
    /// No approval was required: the invocation already ran.
    Completed {
        execution: AgentExecution,
        operation: OperationEnvelope,
        summary: String,
    },
    /// Approval required: the execution is parked; decide via the approval.
    AwaitingApproval {
        execution: AgentExecution,
        display: ApprovalDisplay,
    },
}

/// Outcome of a full run (also returned by `decide_approval`).
#[derive(Debug, Clone, PartialEq)]
pub enum RunOutcome {
    Completed {
        execution: AgentExecution,
        operation: OperationEnvelope,
        summary: String,
    },
    Failed {
        execution: AgentExecution,
        operation: OperationEnvelope,
        reason: String,
    },
    /// The external effect is unresolved (submit indeterminate / provider
    /// unknown / timeout). Non-idempotent work is NOT retried; the owner
    /// reconciles (ADR-019).
    Unresolved {
        execution: AgentExecution,
        operation: OperationEnvelope,
    },
}

/// The human decision on a pending approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approve,
    Deny,
}

/// Resolution chosen by the owner for an unresolved execution (ADR-019).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileResolution {
    /// The external effect provably happened (or its absence is confirmed
    /// harmless) — record Completed.
    Complete,
    /// The external effect failed or must be redone deliberately — record
    /// Failed; any re-execution is a NEW authorized proposal.
    Fail,
}

/// Typed pipeline failures. Non-leaky: no provider payloads, no grant
/// internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentInvocationError {
    /// The registration does not exist for this workspace (or is another
    /// workspace's — indistinguishable to the caller).
    RegistrationNotFound,
    /// The registration is suspended/retired.
    RegistrationNotActive { status: String },
    /// The capability id is unknown, or the requested contract version is
    /// not published (fail closed per ADR-028 §4.3).
    CapabilityUnsupported,
    /// Backend authorization denied the invocation (default-deny path).
    Denied,
    /// The execution does not exist for this workspace.
    ExecutionNotFound,
    /// The execution is not in the state the action requires.
    ExecutionState { expected: String, actual: String },
    /// No pending approval exists for this execution.
    ApprovalNotActive,
    /// Domain governance invariant (digest/capability/transition errors).
    Governance(GovernanceError),
    /// Persistence failure; fail closed.
    Store(AgentStoreError),
    /// Adapter transport failure while the outcome was known (spec §7:
    /// transport facts never carry authorization semantics).
    Adapter(String),
    /// Operation envelope failure; fail closed.
    Operation(OperationPortError),
}

impl std::fmt::Display for AgentInvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegistrationNotFound => write!(f, "agent registration not found"),
            Self::RegistrationNotActive { status } => write!(f, "agent registration is {status}"),
            Self::CapabilityUnsupported => write!(f, "capability or contract version unsupported"),
            Self::Denied => write!(f, "invocation denied by authorization policy"),
            Self::ExecutionNotFound => write!(f, "agent execution not found"),
            Self::ExecutionState { expected, actual } => {
                write!(f, "execution state {actual}; expected {expected}")
            }
            Self::ApprovalNotActive => write!(f, "no active approval for this execution"),
            Self::Governance(e) => write!(f, "governance invariant: {e}"),
            Self::Store(e) => write!(f, "agent store failure: {e}"),
            Self::Adapter(e) => write!(f, "adapter failure: {e}"),
            Self::Operation(e) => write!(f, "operation envelope failure: {e}"),
        }
    }
}

impl std::error::Error for AgentInvocationError {}

impl From<crate::ports::agent_adapter::AdapterError> for AgentInvocationError {
    fn from(e: crate::ports::agent_adapter::AdapterError) -> Self {
        Self::Adapter(e.to_string())
    }
}

impl From<OperationPortError> for AgentInvocationError {
    fn from(e: OperationPortError) -> Self {
        Self::Operation(e)
    }
}

impl From<AgentStoreError> for AgentInvocationError {
    fn from(e: AgentStoreError) -> Self {
        match e {
            AgentStoreError::NotFound => Self::ExecutionNotFound,
            AgentStoreError::WorkspaceMismatch => Self::ExecutionNotFound,
            other => Self::Store(other),
        }
    }
}

/// The agent subject id for a registration (authorization model §2).
fn agent_subject(registration_id: &AgentRegistrationId) -> String {
    format!("agent:{}", registration_id.as_str())
}

/// The invocation pipeline coordinator (Application Coordination, ADR-017).
pub struct AgentInvocationCoordinator<'a, Ops, Regs, Exs, Apps, Auth, Ad>
where
    Ops: OperationService,
    Regs: AgentRegistrationRepository,
    Exs: AgentExecutionRepository,
    Apps: ApprovalRepository,
    Auth: Authorizer,
    Ad: AgentAdapter,
{
    operations: &'a Ops,
    registrations: &'a Regs,
    executions: &'a Exs,
    approvals: &'a Apps,
    /// The governed authorizer: fresh grant evaluation per side effect.
    authorizer: &'a Auth,
    /// The mediated adapter for the registration's transport. E5 wires the
    /// fake adapter; real transports wait for ADR-022.
    adapter: &'a Ad,
    /// The capability registry (metadata-only; ADR-026).
    registry: &'a CapabilityRegistry,
}

impl<'a, Ops, Regs, Exs, Apps, Auth, Ad>
    AgentInvocationCoordinator<'a, Ops, Regs, Exs, Apps, Auth, Ad>
where
    Ops: OperationService,
    Regs: AgentRegistrationRepository,
    Exs: AgentExecutionRepository,
    Apps: ApprovalRepository,
    Auth: Authorizer,
    Ad: AgentAdapter,
{
    pub fn new(
        operations: &'a Ops,
        registrations: &'a Regs,
        executions: &'a Exs,
        approvals: &'a Apps,
        authorizer: &'a Auth,
        adapter: &'a Ad,
        registry: &'a CapabilityRegistry,
    ) -> Self {
        Self {
            operations,
            registrations,
            executions,
            approvals,
            authorizer,
            adapter,
            registry,
        }
    }

    /// Step 1–7 of the pipeline: validate, authorize, canonicalize and park
    /// (or run). No external effect happens in this method when approval is
    /// required.
    pub fn propose(
        &self,
        proposal: &AgentProposal,
        now_ms: i64,
    ) -> Result<ProposalOutcome, AgentInvocationError> {
        // Parse/validate + resolve agent and workspace (steps 1–2).
        let registration = self
            .registrations
            .get_registration(&proposal.registration_id, &proposal.workspace_id)
            .map_err(|e| match e {
                AgentStoreError::NotFound | AgentStoreError::WorkspaceMismatch => {
                    AgentInvocationError::RegistrationNotFound
                }
                other => AgentInvocationError::Store(other),
            })?;
        registration.ensure_active().map_err(|e| match e {
            GovernanceError::RegistrationNotActive { status } => {
                AgentInvocationError::RegistrationNotActive { status }
            }
            other => AgentInvocationError::Governance(other),
        })?;

        // Resolve capability descriptor and contract version (step 3).
        let (major, minor, patch) = proposal.capability_contract_version;
        let requested = crate::ports::capability::ContractVersion::new(major, minor, patch);
        let descriptor = match self.registry.resolve(&proposal.capability_id, requested) {
            Resolution::Supported(descriptor) => *descriptor,
            Resolution::UnsupportedVersion { .. } | Resolution::UnknownCapability => {
                return Err(AgentInvocationError::CapabilityUnsupported);
            }
        };

        // Canonicalize the plan BEFORE authorization so the digest reflects
        // exactly what will be authorized and, when required, approved
        // (steps 4–6).
        let transport = self.adapter.capabilities().transport;
        let plan = CanonicalPlan {
            capability_id: proposal.capability_id.clone(),
            capability_contract_version: descriptor.contract_version.as_str(),
            actor_id: agent_subject(&registration.id),
            agent_registration_id: Some(registration.id.as_str().to_string()),
            data_workspace_id: proposal.workspace_id.as_str().to_string(),
            tool_adapter: transport.clone(),
            target_identity: format!("provider:{}", transport),
            parameters: proposal.parameters.clone(),
            side_effect_set: vec![proposal.capability_id.clone()],
        };
        let plan_digest = plan.digest();
        let material_params_digest = plan.material_params_digest();
        let parameters_json =
            serde_json_ok(&proposal.parameters).map_err(AgentInvocationError::Governance)?;

        // Evaluate current authorization — default deny (step 4 semantics;
        // ADR-029 §4.1). The invocation runs as the agent subject.
        let agent_ctx = ExecutionContext {
            actor_id: agent_subject(&registration.id),
            correlation_id: proposal.correlation_id.clone(),
        };
        self.authorizer
            .authorize(
                &agent_ctx,
                &ProtectedAction::CapabilityInvoke {
                    workspace_id: proposal.workspace_id.clone(),
                    capability_id: proposal.capability_id.clone(),
                },
            )
            .map_err(|_| AgentInvocationError::Denied)?;

        // Classify risk and approval requirement (step 5). Policy in E5:
        // `High` risk or `Required` hint forces approval; `Conditional`
        // defers to policy, which currently approves low/medium automatic
        // execution (spec §12.2).
        let approval_required = descriptor.risk_class == RiskClass::High
            || descriptor.approval_hint == ApprovalHint::Required;

        // Create the execution record and advance its lifecycle durably.
        let mut execution = AgentExecution::new(
            AgentExecutionId::new_v7(),
            registration.id.clone(),
            proposal.workspace_id.clone(),
            &proposal.capability_id,
            &descriptor.contract_version.as_str(),
            &plan_digest,
            &transport,
            &format!("provider:{}", transport),
            &material_params_digest,
            &parameters_json,
        )
        .map_err(AgentInvocationError::Governance)?;
        self.executions.insert_execution(&execution)?;
        execution
            .transition(ExecutionStatus::Validating)
            .map_err(AgentInvocationError::Governance)?;
        self.executions.update_execution(&execution)?;

        if approval_required {
            execution
                .transition(ExecutionStatus::AwaitingApproval)
                .map_err(AgentInvocationError::Governance)?;
            self.executions.update_execution(&execution)?;
            let display = ApprovalDisplay {
                execution_id: execution.id.as_str().to_string(),
                capability_id: execution.capability_id.clone(),
                capability_contract_version: execution.capability_contract_version.clone(),
                tool_adapter: execution.tool_adapter.clone(),
                target_identity: execution.target_identity.clone(),
                side_effect_set: vec![proposal.capability_id.clone()],
                risk_class: risk_class_str(descriptor.risk_class).to_string(),
                expires_at_ms: now_ms + proposal.approval_ttl_ms,
                attempt_policy: proposal.attempt_policy,
            };
            return Ok(ProposalOutcome::AwaitingApproval { execution, display });
        }

        // No approval required: run now.
        execution
            .transition(ExecutionStatus::Approved)
            .map_err(AgentInvocationError::Governance)?;
        self.executions.update_execution(&execution)?;
        let outcome = self.run(&execution, proposal, None, now_ms)?;
        Ok(match outcome {
            RunOutcome::Completed {
                execution,
                operation,
                summary,
            } => ProposalOutcome::Completed {
                execution,
                operation,
                summary,
            },
            RunOutcome::Failed {
                execution,
                operation,
                reason,
            } => ProposalOutcome::Completed {
                execution,
                operation,
                summary: format!("failed: {reason}"),
            },
            RunOutcome::Unresolved {
                execution,
                operation,
            } => ProposalOutcome::Completed {
                execution,
                operation,
                summary: "unresolved: reconciliation required".to_string(),
            },
        })
    }

    /// Step 7: validate the approval binding reconstructed **from the
    /// persisted execution** and decide. Approve records the approval and
    /// runs; deny cancels. The binding is never taken from caller input, so
    /// the displayed plan, the decided plan and the executed plan are the
    /// same object (ADR-029 §4.3).
    pub fn decide_approval(
        &self,
        workspace_id: &WorkspaceId,
        execution_id: &AgentExecutionId,
        approver: &ExecutionContext,
        decision: ApprovalDecision,
        now_ms: i64,
        proposal: &AgentProposal,
    ) -> Result<Option<RunOutcome>, AgentInvocationError> {
        let mut execution = self.executions.get_execution(execution_id, workspace_id)?;
        if execution.status != ExecutionStatus::AwaitingApproval {
            return Err(AgentInvocationError::ExecutionState {
                expected: "AwaitingApproval".to_string(),
                actual: execution.status.as_str().to_string(),
            });
        }
        match decision {
            ApprovalDecision::Deny => {
                execution
                    .transition(ExecutionStatus::Cancelled)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(&execution)?;
                Ok(None)
            }
            ApprovalDecision::Approve => {
                // Reconstruct the binding from persisted fields only.
                let binding = ApprovalBinding {
                    canonical_digest: execution.plan_digest.clone(),
                    actor_id: approver.actor_id.clone(),
                    agent_registration_id: Some(execution.registration_id.clone()),
                    workspace_id: execution.workspace_id.clone(),
                    capability_id: execution.capability_id.clone(),
                    capability_contract_version: execution.capability_contract_version.clone(),
                    tool_adapter: execution.tool_adapter.clone(),
                    target_identity: execution.target_identity.clone(),
                    material_params_digest: execution.material_params_digest.clone(),
                };
                let approval = Approval::new(
                    ApprovalId::new_v7(),
                    binding,
                    vec![execution.capability_id.clone()],
                    // Risk class string mirrors the descriptor at propose
                    // time; the display carries it to the decision surface.
                    "High",
                    now_ms + proposal.approval_ttl_ms,
                    proposal.attempt_policy,
                )
                .map_err(AgentInvocationError::Governance)?;
                self.approvals.insert_approval(&approval)?;
                execution
                    .transition(ExecutionStatus::Approved)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(&execution)?;
                Ok(Some(self.run(
                    &execution,
                    proposal,
                    Some(&approval.id),
                    now_ms,
                )?))
            }
        }
    }

    /// Steps 7b–13: open the durable envelope, re-authorize immediately
    /// before the side effect, submit via the mediated adapter and persist
    /// the outcome. Unknown outcomes hold for reconciliation.
    fn run(
        &self,
        execution: &AgentExecution,
        proposal: &AgentProposal,
        approval_id: Option<&ApprovalId>,
        now_ms: i64,
    ) -> Result<RunOutcome, AgentInvocationError> {
        // Open the durable Operation envelope (step 7b). Owner linkage:
        // ai_collaboration / agent_execution / execution id.
        let agent_ctx = ExecutionContext {
            actor_id: agent_subject(&execution.registration_id),
            correlation_id: proposal.correlation_id.clone(),
        };
        let operation = self.operations.open(&OpenOperation {
            kind: "agent.invocation".to_string(),
            owner_domain: "ai_collaboration".to_string(),
            owner_type: "agent_execution".to_string(),
            owner_id: execution.id.as_str().to_string(),
            workspace_id: Some(execution.workspace_id.as_str().to_string()),
            actor_id: agent_ctx.actor_id.clone(),
            correlation_id: proposal.correlation_id.clone(),
            causation_id: None,
        })?;

        // Per-side-effect re-authorization #1: immediately before submit
        // (ADR-029 §4.4). A denial after Operation creation leaves the
        // envelope queryable and failed — it does not roll back history.
        if let Err(_denied) = self.authorizer.authorize(
            &agent_ctx,
            &ProtectedAction::CapabilityInvoke {
                workspace_id: execution.workspace_id.clone(),
                capability_id: execution.capability_id.clone(),
            },
        ) {
            self.operations.record_attempt(
                &operation.id,
                &ExternalAttempt {
                    phase: AttemptPhase::Submit,
                    outcome: AttemptOutcome::Failed,
                    request_digest: Some(execution.plan_digest.clone()),
                    idempotency_key: Some(execution.id.as_str().to_string()),
                    external_reference: None,
                    retryable: false,
                    error_category: Some("authorization_revoked".to_string()),
                    error_code: None,
                },
            )?;
            let mut execution = execution.clone();
            execution
                .transition(ExecutionStatus::Failed)
                .map_err(AgentInvocationError::Governance)?;
            self.executions.update_execution(&execution)?;
            let operation = self.operations.transition(
                &operation.id,
                OperationStatus::Failed,
                Some("authorization_revoked"),
            )?;
            return Ok(RunOutcome::Failed {
                execution,
                operation,
                reason: "authorization revoked before side effect".to_string(),
            });
        }

        // Consume one attempt of the approval at the side effect boundary
        // (ADR-029 §4.5: idempotent exact retries reuse the approval only
        // within its attempt policy — consumption enforces the budget.)
        if let Some(approval_id) = approval_id {
            let mut approval = self
                .approvals
                .get_approval(approval_id, &execution.workspace_id)?;
            approval
                .consume(now_ms)
                .map_err(AgentInvocationError::Governance)?;
            self.approvals.update_approval(&approval)?;
        }

        // Link the envelope to the execution, advance to Running.
        let mut execution = execution.clone();
        execution
            .associate_operation(&operation.id)
            .map_err(AgentInvocationError::Governance)?;
        execution
            .transition(ExecutionStatus::Submitted)
            .map_err(AgentInvocationError::Governance)?;
        self.executions.update_execution(&execution)?;
        let operation =
            self.operations
                .transition(&operation.id, OperationStatus::Running, None)?;
        execution
            .transition(ExecutionStatus::Running)
            .map_err(AgentInvocationError::Governance)?;
        self.executions.update_execution(&execution)?;

        // The submit attempt: started → provider outcome.
        let parameters: BTreeMap<String, String> = serde_json_params(&execution.parameters_json)
            .map_err(AgentInvocationError::Governance)?;
        let request = crate::ports::agent_adapter::AdapterSubmitRequest {
            execution_id: execution.id.as_str().to_string(),
            plan_digest: execution.plan_digest.clone(),
            correlation_id: proposal.correlation_id.clone(),
            capability_id: execution.capability_id.clone(),
            parameters,
        };
        self.operations.record_attempt(
            &operation.id,
            &ExternalAttempt {
                phase: AttemptPhase::Submit,
                outcome: AttemptOutcome::Started,
                request_digest: Some(execution.plan_digest.clone()),
                idempotency_key: Some(execution.id.as_str().to_string()),
                external_reference: None,
                retryable: false,
                error_category: None,
                error_code: None,
            },
        )?;
        match self.adapter.submit(&request) {
            Ok(AdapterSubmitOutcome::Accepted { external_id }) => {
                self.operations.record_attempt(
                    &operation.id,
                    &ExternalAttempt {
                        phase: AttemptPhase::Submit,
                        outcome: AttemptOutcome::Accepted,
                        request_digest: Some(execution.plan_digest.clone()),
                        idempotency_key: Some(execution.id.as_str().to_string()),
                        external_reference: Some(external_id.clone()),
                        retryable: false,
                        error_category: None,
                        error_code: None,
                    },
                )?;
                self.poll_to_outcome(&mut execution, &operation, &external_id)
            }
            Ok(AdapterSubmitOutcome::Rejected { reason }) => {
                self.operations.record_attempt(
                    &operation.id,
                    &ExternalAttempt {
                        phase: AttemptPhase::Submit,
                        outcome: AttemptOutcome::Rejected,
                        request_digest: Some(execution.plan_digest.clone()),
                        idempotency_key: Some(execution.id.as_str().to_string()),
                        external_reference: None,
                        retryable: false,
                        error_category: Some("provider_rejected".to_string()),
                        error_code: Some(reason.clone()),
                    },
                )?;
                execution
                    .transition(ExecutionStatus::Failed)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(&execution)?;
                let operation = self.operations.transition(
                    &operation.id,
                    OperationStatus::Failed,
                    Some("provider_rejected"),
                )?;
                Ok(RunOutcome::Failed {
                    execution,
                    operation,
                    reason,
                })
            }
            Ok(AdapterSubmitOutcome::Indeterminate { reason }) => {
                // The provider could not confirm acceptance: hold at
                // OutcomeUnknown; a non-idempotent effect is never blind-
                // retried (ADR-019/ADR-029 §4.5).
                self.operations.record_attempt(
                    &operation.id,
                    &ExternalAttempt {
                        phase: AttemptPhase::Submit,
                        outcome: AttemptOutcome::Indeterminate,
                        request_digest: Some(execution.plan_digest.clone()),
                        idempotency_key: Some(execution.id.as_str().to_string()),
                        external_reference: None,
                        retryable: false,
                        error_category: Some("submit_indeterminate".to_string()),
                        error_code: Some(reason.clone()),
                    },
                )?;
                execution
                    .transition(ExecutionStatus::OutcomeUnknown)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(&execution)?;
                let operation = self.operations.transition(
                    &operation.id,
                    OperationStatus::OutcomeUnknown,
                    Some("submit_indeterminate"),
                )?;
                Ok(RunOutcome::Unresolved {
                    execution,
                    operation,
                })
            }
            Err(transport) => {
                self.operations.record_attempt(
                    &operation.id,
                    &ExternalAttempt {
                        phase: AttemptPhase::Submit,
                        outcome: AttemptOutcome::Failed,
                        request_digest: Some(execution.plan_digest.clone()),
                        idempotency_key: Some(execution.id.as_str().to_string()),
                        external_reference: None,
                        retryable: false,
                        error_category: Some("adapter_transport".to_string()),
                        error_code: Some(transport.to_string()),
                    },
                )?;
                execution
                    .transition(ExecutionStatus::Failed)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(&execution)?;
                let operation = self.operations.transition(
                    &operation.id,
                    OperationStatus::Failed,
                    Some("adapter_transport"),
                )?;
                Ok(RunOutcome::Failed {
                    execution,
                    operation,
                    reason: transport.to_string(),
                })
            }
        }
    }

    /// Poll the adapter result and persist the terminal (or unresolved)
    /// outcome. Per-side-effect re-authorization #2 guards the poll: a
    /// grant revoked while running stops the reconciliation side effect.
    fn poll_to_outcome(
        &self,
        execution: &mut AgentExecution,
        operation: &OperationEnvelope,
        external_id: &str,
    ) -> Result<RunOutcome, AgentInvocationError> {
        let agent_ctx = ExecutionContext {
            actor_id: agent_subject(&execution.registration_id),
            correlation_id: operation.correlation_id.clone(),
        };
        if let Err(_denied) = self.authorizer.authorize(
            &agent_ctx,
            &ProtectedAction::CapabilityInvoke {
                workspace_id: execution.workspace_id.clone(),
                capability_id: execution.capability_id.clone(),
            },
        ) {
            self.operations.record_attempt(
                &operation.id,
                &ExternalAttempt {
                    phase: AttemptPhase::Poll,
                    outcome: AttemptOutcome::Failed,
                    request_digest: Some(execution.plan_digest.clone()),
                    idempotency_key: Some(execution.id.as_str().to_string()),
                    external_reference: Some(external_id.to_string()),
                    retryable: false,
                    error_category: Some("authorization_revoked".to_string()),
                    error_code: None,
                },
            )?;
            execution
                .transition(ExecutionStatus::Failed)
                .map_err(AgentInvocationError::Governance)?;
            self.executions.update_execution(execution)?;
            let operation = self.operations.transition(
                &operation.id,
                OperationStatus::Failed,
                Some("authorization_revoked"),
            )?;
            return Ok(RunOutcome::Failed {
                execution: execution.clone(),
                operation,
                reason: "authorization revoked before result".to_string(),
            });
        }

        let result = self.adapter.result(external_id)?;
        self.operations.record_attempt(
            &operation.id,
            &ExternalAttempt {
                phase: AttemptPhase::Poll,
                outcome: match result.status {
                    crate::ports::agent_adapter::AdapterExecutionStatus::Completed => {
                        AttemptOutcome::Completed
                    }
                    crate::ports::agent_adapter::AdapterExecutionStatus::Unknown { .. } => {
                        AttemptOutcome::Indeterminate
                    }
                    _ => AttemptOutcome::Failed,
                },
                request_digest: Some(execution.plan_digest.clone()),
                idempotency_key: Some(execution.id.as_str().to_string()),
                external_reference: Some(external_id.to_string()),
                retryable: false,
                error_category: None,
                error_code: None,
            },
        )?;
        use crate::ports::agent_adapter::AdapterExecutionStatus as Status;
        match result.status {
            Status::Completed => {
                execution
                    .transition(ExecutionStatus::Completed)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(execution)?;
                let operation =
                    self.operations
                        .transition(&operation.id, OperationStatus::Succeeded, None)?;
                Ok(RunOutcome::Completed {
                    execution: execution.clone(),
                    operation,
                    summary: result.summary,
                })
            }
            Status::Failed => {
                execution
                    .transition(ExecutionStatus::Failed)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(execution)?;
                let operation =
                    self.operations
                        .transition(&operation.id, OperationStatus::Failed, None)?;
                Ok(RunOutcome::Failed {
                    execution: execution.clone(),
                    operation,
                    reason: result.summary,
                })
            }
            Status::Cancelled => {
                execution
                    .transition(ExecutionStatus::Cancelled)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(execution)?;
                let operation =
                    self.operations
                        .transition(&operation.id, OperationStatus::Cancelled, None)?;
                Ok(RunOutcome::Failed {
                    execution: execution.clone(),
                    operation,
                    reason: "cancelled".to_string(),
                })
            }
            Status::TimedOut => {
                execution
                    .transition(ExecutionStatus::TimedOut)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(execution)?;
                let operation = self.operations.transition(
                    &operation.id,
                    OperationStatus::OutcomeUnknown,
                    Some("provider_timed_out"),
                )?;
                Ok(RunOutcome::Unresolved {
                    execution: execution.clone(),
                    operation,
                })
            }
            Status::Unknown { reason } => {
                // The provider could not establish an outcome; hold for
                // reconciliation, never infer.
                self.operations.record_attempt(
                    &operation.id,
                    &ExternalAttempt {
                        phase: AttemptPhase::Poll,
                        outcome: AttemptOutcome::Indeterminate,
                        request_digest: Some(execution.plan_digest.clone()),
                        idempotency_key: Some(execution.id.as_str().to_string()),
                        external_reference: Some(external_id.to_string()),
                        retryable: false,
                        error_category: Some("outcome_unproven".to_string()),
                        error_code: Some(reason.clone()),
                    },
                )?;
                execution
                    .transition(ExecutionStatus::OutcomeUnknown)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(execution)?;
                let operation = self.operations.transition(
                    &operation.id,
                    OperationStatus::OutcomeUnknown,
                    Some("outcome_unproven"),
                )?;
                Ok(RunOutcome::Unresolved {
                    execution: execution.clone(),
                    operation,
                })
            }
            Status::Running => {
                // Running at poll time is treated as unresolved: E5's
                // synchronous pipeline does not await; the owner reconciles.
                execution
                    .transition(ExecutionStatus::OutcomeUnknown)
                    .map_err(AgentInvocationError::Governance)?;
                self.executions.update_execution(execution)?;
                let operation = self.operations.transition(
                    &operation.id,
                    OperationStatus::OutcomeUnknown,
                    Some("outcome_unproven"),
                )?;
                Ok(RunOutcome::Unresolved {
                    execution: execution.clone(),
                    operation,
                })
            }
        }
    }

    /// Owner reconciliation of an unresolved execution (ADR-019; the only
    /// path out of TimedOut/OutcomeUnknown). The resolution is an explicit,
    /// gated human decision; the operation envelope follows via its own
    /// machine transitions.
    pub fn reconcile(
        &self,
        workspace_id: &WorkspaceId,
        execution_id: &AgentExecutionId,
        resolution: ReconcileResolution,
        _reconciler: &ExecutionContext,
    ) -> Result<AgentExecution, AgentInvocationError> {
        let mut execution = self.executions.get_execution(execution_id, workspace_id)?;
        if !matches!(
            execution.status,
            ExecutionStatus::TimedOut | ExecutionStatus::OutcomeUnknown
        ) {
            return Err(AgentInvocationError::ExecutionState {
                expected: "TimedOut|OutcomeUnknown".to_string(),
                actual: execution.status.as_str().to_string(),
            });
        }
        let operation_id =
            execution
                .operation_id
                .clone()
                .ok_or_else(|| AgentInvocationError::ExecutionState {
                    expected: "linked operation".to_string(),
                    actual: "no operation".to_string(),
                })?;
        let (execution_status, operation_status, category, outcome) = match resolution {
            ReconcileResolution::Complete => (
                ExecutionStatus::Completed,
                OperationStatus::Succeeded,
                None,
                AttemptOutcome::Completed,
            ),
            ReconcileResolution::Fail => (
                ExecutionStatus::Failed,
                OperationStatus::Failed,
                Some("reconciled_failed".to_string()),
                AttemptOutcome::Failed,
            ),
        };
        self.operations.record_attempt(
            &operation_id,
            &ExternalAttempt {
                phase: AttemptPhase::Reconcile,
                outcome,
                request_digest: Some(execution.plan_digest.clone()),
                idempotency_key: Some(execution.id.as_str().to_string()),
                external_reference: None,
                retryable: false,
                error_category: category.clone(),
                error_code: None,
            },
        )?;
        execution
            .transition(execution_status)
            .map_err(AgentInvocationError::Governance)?;
        self.executions.update_execution(&execution)?;
        self.operations
            .transition(&operation_id, operation_status, category.as_deref())?;
        Ok(execution)
    }
}

fn risk_class_str(class: RiskClass) -> &'static str {
    match class {
        RiskClass::Low => "Low",
        RiskClass::Medium => "Medium",
        RiskClass::High => "High",
    }
}

/// Serialize the plan parameters to canonical JSON text.
fn serde_json_ok(parameters: &BTreeMap<String, PlanValue>) -> Result<String, GovernanceError> {
    serde_json::to_string(parameters).map_err(|_| GovernanceError::InvalidCapabilityId)
}

/// Deserialize the persisted parameter JSON into adapter-string parameters.
fn serde_json_params(parameters_json: &str) -> Result<BTreeMap<String, String>, GovernanceError> {
    let raw: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(parameters_json).map_err(|_| GovernanceError::InvalidCapabilityId)?;
    let mut out = BTreeMap::new();
    for (key, value) in raw {
        let text = match value {
            serde_json::Value::String(s) => s,
            other => other.to_string(),
        };
        out.insert(key, text);
    }
    Ok(out)
}
