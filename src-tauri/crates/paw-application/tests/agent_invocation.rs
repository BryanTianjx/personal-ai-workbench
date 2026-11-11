//! E5-05 pipeline conformance tests (in-memory port fakes).
//!
//! Drives the governed invocation coordinator end to end against fake
//! ports, covering the conformance matrix from
//! `E5-implementation-specification` §11 that belongs to the coordination
//! layer: default-deny, the two-phase approval flow, per-side-effect
//! re-authorization (revocation between side effects), unresolved outcomes
//! without blind retry, and reconciliation. SQLite-level persistence is
//! covered by the infrastructure suite; the domain state machines by their
//! unit tests.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use paw_application::ports::agent_adapter::{
    AdapterCapabilities, AdapterError, AdapterExecutionStatus, AdapterHealth, AdapterResult,
    AdapterSubmitOutcome, AdapterSubmitRequest, AgentAdapter,
};
use paw_application::ports::agent_repository::{
    AgentExecutionRepository, AgentRegistrationRepository, AgentStoreError, ApprovalRepository,
};
use paw_application::ports::authorization::{
    AgentGrant, AuthorizationError, Authorizer, ExecutionContext, GrantStore, ProtectedAction,
};
use paw_application::ports::capability::{
    ApprovalHint, CapabilityDescriptor, CapabilityDomain, CapabilityRegistry, ContractVersion,
    Idempotency, OfflinePolicy, RiskClass, ScopeRequirement,
};
use paw_application::ports::operation_service::{
    ExternalAttempt, OpenOperation, OperationEnvelope, OperationPortError, OperationService,
};
use paw_application::use_cases::agent_invocation::{
    AgentInvocationCoordinator, AgentInvocationError, AgentProposal, ApprovalDecision,
    ProposalOutcome, ReconcileResolution, RunOutcome,
};
use paw_domain::ai_collaboration::{
    AdapterKind, AgentExecution, AgentExecutionId, AgentRegistration, AgentRegistrationId,
    Approval, ApprovalId, AttemptPolicy, ExecutionStatus, RegistrationStatus,
};
use paw_domain::operation_machine::OperationStatus;
use paw_domain::work_management::WorkspaceId;

// ---------------------------------------------------------------------------
// In-memory port fakes (interior mutability; the coordinator takes &refs)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct MemOps {
    envelopes: RefCell<std::collections::HashMap<String, (OperationStatus, String)>>,
    attempts: RefCell<std::collections::HashMap<String, Vec<(String, bool)>>>,
    sequence: RefCell<u32>,
}

impl MemOps {
    fn attempts_for(&self, id: &str) -> Vec<(String, bool)> {
        self.attempts.borrow().get(id).cloned().unwrap_or_default()
    }
}

impl OperationService for MemOps {
    fn open(&self, open: &OpenOperation) -> Result<OperationEnvelope, OperationPortError> {
        let mut sequence = self.sequence.borrow_mut();
        *sequence += 1;
        let id = format!("op-{sequence}");
        self.envelopes.borrow_mut().insert(
            id.clone(),
            (OperationStatus::Pending, open.correlation_id.clone()),
        );
        Ok(OperationEnvelope {
            id,
            status: OperationStatus::Pending,
            correlation_id: open.correlation_id.clone(),
        })
    }

    fn get(&self, id: &str) -> Result<OperationEnvelope, OperationPortError> {
        self.envelopes
            .borrow()
            .get(id)
            .map(|(status, correlation)| OperationEnvelope {
                id: id.to_string(),
                status: *status,
                correlation_id: correlation.clone(),
            })
            .ok_or_else(|| OperationPortError::NotFound(id.to_string()))
    }

    fn transition(
        &self,
        id: &str,
        to: OperationStatus,
        _error_category: Option<&str>,
    ) -> Result<OperationEnvelope, OperationPortError> {
        let mut envelopes = self.envelopes.borrow_mut();
        let entry = envelopes
            .get_mut(id)
            .ok_or_else(|| OperationPortError::NotFound(id.to_string()))?;
        // The ADR-019 machine is the validator (E4 refactor).
        if !paw_domain::operation_machine::is_valid_transition(entry.0, to) {
            return Err(OperationPortError::InvalidTransition {
                from: entry.0.as_str().to_string(),
                to: to.as_str().to_string(),
            });
        }
        entry.0 = to;
        Ok(OperationEnvelope {
            id: id.to_string(),
            status: to,
            correlation_id: entry.1.clone(),
        })
    }

    fn record_attempt(
        &self,
        operation_id: &str,
        attempt: &ExternalAttempt,
    ) -> Result<(), OperationPortError> {
        if !self.envelopes.borrow().contains_key(operation_id) {
            return Err(OperationPortError::NotFound(operation_id.to_string()));
        }
        self.attempts
            .borrow_mut()
            .entry(operation_id.to_string())
            .or_default()
            .push((format!("{:?}", attempt.outcome), attempt.retryable));
        Ok(())
    }
}

#[derive(Default)]
struct MemRegs {
    rows: RefCell<Vec<AgentRegistration>>,
}

impl AgentRegistrationRepository for MemRegs {
    fn insert_registration(&self, registration: &AgentRegistration) -> Result<(), AgentStoreError> {
        self.rows.borrow_mut().push(registration.clone());
        Ok(())
    }

    fn get_registration(
        &self,
        id: &AgentRegistrationId,
        workspace_id: &WorkspaceId,
    ) -> Result<AgentRegistration, AgentStoreError> {
        match self.rows.borrow().iter().find(|r| r.id == *id) {
            None => Err(AgentStoreError::NotFound),
            Some(found) if found.workspace_id != *workspace_id => {
                Err(AgentStoreError::WorkspaceMismatch)
            }
            Some(found) => Ok(found.clone()),
        }
    }

    fn list_registrations(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<AgentRegistration>, AgentStoreError> {
        Ok(self
            .rows
            .borrow()
            .iter()
            .filter(|r| r.workspace_id == *workspace_id)
            .cloned()
            .collect())
    }

    fn update_registration_status(
        &self,
        registration: &AgentRegistration,
    ) -> Result<(), AgentStoreError> {
        let mut rows = self.rows.borrow_mut();
        let found = rows
            .iter_mut()
            .find(|r| r.id == registration.id)
            .ok_or(AgentStoreError::NotFound)?;
        found.status = registration.status;
        Ok(())
    }
}

#[derive(Default)]
struct MemExs {
    rows: RefCell<Vec<AgentExecution>>,
}

impl AgentExecutionRepository for MemExs {
    fn insert_execution(&self, execution: &AgentExecution) -> Result<(), AgentStoreError> {
        self.rows.borrow_mut().push(execution.clone());
        Ok(())
    }

    fn get_execution(
        &self,
        id: &AgentExecutionId,
        workspace_id: &WorkspaceId,
    ) -> Result<AgentExecution, AgentStoreError> {
        match self.rows.borrow().iter().find(|e| e.id == *id) {
            None => Err(AgentStoreError::NotFound),
            Some(found) if found.workspace_id != *workspace_id => {
                Err(AgentStoreError::WorkspaceMismatch)
            }
            Some(found) => Ok(found.clone()),
        }
    }

    fn list_executions(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<AgentExecution>, AgentStoreError> {
        Ok(self
            .rows
            .borrow()
            .iter()
            .filter(|e| e.workspace_id == *workspace_id)
            .take(limit)
            .cloned()
            .collect())
    }

    fn update_execution(&self, execution: &AgentExecution) -> Result<(), AgentStoreError> {
        let mut rows = self.rows.borrow_mut();
        let found = rows
            .iter_mut()
            .find(|e| e.id == execution.id)
            .ok_or(AgentStoreError::NotFound)?;
        *found = execution.clone();
        Ok(())
    }

    fn list_unresolved_executions(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<AgentExecution>, AgentStoreError> {
        Ok(self
            .rows
            .borrow()
            .iter()
            .filter(|e| {
                e.workspace_id == *workspace_id
                    && matches!(
                        e.status,
                        ExecutionStatus::TimedOut | ExecutionStatus::OutcomeUnknown
                    )
            })
            .cloned()
            .collect())
    }

    fn count_executions_by_status(
        &self,
        workspace_id: &WorkspaceId,
        status: ExecutionStatus,
    ) -> Result<usize, AgentStoreError> {
        Ok(self
            .rows
            .borrow()
            .iter()
            .filter(|e| e.workspace_id == *workspace_id && e.status == status)
            .count())
    }
}

#[derive(Default)]
struct MemApprovals {
    rows: RefCell<Vec<Approval>>,
}

impl ApprovalRepository for MemApprovals {
    fn insert_approval(&self, approval: &Approval) -> Result<(), AgentStoreError> {
        self.rows.borrow_mut().push(approval.clone());
        Ok(())
    }

    fn get_approval(
        &self,
        id: &ApprovalId,
        workspace_id: &WorkspaceId,
    ) -> Result<Approval, AgentStoreError> {
        match self.rows.borrow().iter().find(|a| a.id == *id) {
            None => Err(AgentStoreError::NotFound),
            Some(found) if found.workspace_id != *workspace_id => {
                Err(AgentStoreError::WorkspaceMismatch)
            }
            Some(found) => Ok(found.clone()),
        }
    }

    fn list_pending_approvals(
        &self,
        workspace_id: &WorkspaceId,
        now_ms: i64,
    ) -> Result<Vec<Approval>, AgentStoreError> {
        Ok(self
            .rows
            .borrow()
            .iter()
            .filter(|a| {
                a.workspace_id == *workspace_id
                    && a.effective_status(now_ms)
                        == paw_domain::ai_collaboration::ApprovalStatus::Active
            })
            .cloned()
            .collect())
    }

    fn update_approval(&self, approval: &Approval) -> Result<(), AgentStoreError> {
        let mut rows = self.rows.borrow_mut();
        let found = rows
            .iter_mut()
            .find(|a| a.id == approval.id)
            .ok_or(AgentStoreError::NotFound)?;
        *found = approval.clone();
        Ok(())
    }

    fn find_active_by_digest(
        &self,
        workspace_id: &WorkspaceId,
        canonical_digest: &str,
    ) -> Result<Option<Approval>, AgentStoreError> {
        match self.rows.borrow().iter().find(|a| {
            a.workspace_id == *workspace_id
                && a.canonical_digest == canonical_digest
                && a.effective_status(NOW) == paw_domain::ai_collaboration::ApprovalStatus::Active
        }) {
            Some(found) => Ok(Some(found.clone())),
            None => Ok(None),
        }
    }
}

/// Scripted adapter: queued submit outcomes and queued result statuses.
struct ScriptedAdapter {
    transport: String,
    submits: RefCell<Vec<AdapterSubmitOutcome>>,
    result_status: RefCell<Vec<AdapterExecutionStatus>>,
}

impl ScriptedAdapter {
    fn scripted(submits: Vec<AdapterSubmitOutcome>, results: Vec<AdapterExecutionStatus>) -> Self {
        Self {
            transport: "fake".to_string(),
            submits: RefCell::new(submits),
            result_status: RefCell::new(results),
        }
    }
}

impl AgentAdapter for ScriptedAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            transport: self.transport.clone(),
            supports_cancellation: true,
            supports_streaming: false,
            supports_idempotency_key: true,
        }
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Available
    }

    fn submit(
        &self,
        _request: &AdapterSubmitRequest,
    ) -> Result<AdapterSubmitOutcome, AdapterError> {
        let mut queue = self.submits.borrow_mut();
        if queue.is_empty() {
            return Ok(AdapterSubmitOutcome::Accepted {
                external_id: "external-1".to_string(),
            });
        }
        Ok(queue.remove(0))
    }

    fn status(&self, _external_id: &str) -> Result<AdapterExecutionStatus, AdapterError> {
        let mut queue = self.result_status.borrow_mut();
        if queue.is_empty() {
            return Ok(AdapterExecutionStatus::Completed);
        }
        Ok(queue.remove(0))
    }

    fn cancel(&self, _external_id: &str) -> Result<AdapterExecutionStatus, AdapterError> {
        Ok(AdapterExecutionStatus::Cancelled)
    }

    fn result(&self, external_id: &str) -> Result<AdapterResult, AdapterError> {
        Ok(AdapterResult {
            status: self.status(external_id)?,
            summary: "scripted result".to_string(),
            artifact_references: Vec::new(),
        })
    }
}

/// Authorization stub with a decision queue for `CapabilityInvoke` checks.
/// Each check pops the next decision; an empty queue denies (the
/// default-deny surface under test).
struct StubAuthorizer {
    decisions: RefCell<Vec<bool>>,
}

impl StubAuthorizer {
    fn allow_next(n: usize) -> Self {
        Self {
            decisions: RefCell::new(vec![true; n]),
        }
    }
}

impl Authorizer for StubAuthorizer {
    fn authorize(
        &self,
        _ctx: &ExecutionContext,
        action: &ProtectedAction,
    ) -> Result<(), AuthorizationError> {
        match action {
            ProtectedAction::CapabilityInvoke { .. } => {
                let mut queue = self.decisions.borrow_mut();
                if queue.is_empty() {
                    Err(AuthorizationError::Denied)
                } else if queue.remove(0) {
                    Ok(())
                } else {
                    Err(AuthorizationError::Denied)
                }
            }
            _ => Err(AuthorizationError::Denied),
        }
    }
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

const NOW: i64 = 1_000_000;
const CORR: &str = "corr-e5";

fn ctx(actor: &str) -> ExecutionContext {
    ExecutionContext {
        actor_id: actor.to_string(),
        correlation_id: CORR.to_string(),
    }
}

fn registry_with() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(CapabilityDescriptor {
            id: "work.task.create".to_string(),
            owner: CapabilityDomain::WorkManagement,
            contract_version: ContractVersion::new(1, 0, 0),
            required_scope: ScopeRequirement::none(),
            risk_class: RiskClass::Low,
            idempotency: Idempotency::Idempotent,
            offline_behavior: OfflinePolicy::LocalOnly,
            approval_hint: ApprovalHint::None,
            summary: "create a task".to_string(),
        })
        .expect("low-risk capability registers");
    registry
        .register(CapabilityDescriptor {
            id: "work.shell.run".to_string(),
            owner: CapabilityDomain::WorkManagement,
            contract_version: ContractVersion::new(1, 0, 0),
            required_scope: ScopeRequirement::none(),
            risk_class: RiskClass::High,
            idempotency: Idempotency::NonIdempotent,
            offline_behavior: OfflinePolicy::Unavailable,
            approval_hint: ApprovalHint::Required,
            summary: "run a shell command".to_string(),
        })
        .expect("high-risk capability registers");
    registry
}

fn proposal(
    workspace: &WorkspaceId,
    registration: &AgentRegistrationId,
    capability: &str,
) -> AgentProposal {
    AgentProposal {
        workspace_id: workspace.clone(),
        registration_id: registration.clone(),
        capability_id: capability.to_string(),
        capability_contract_version: (1, 0, 0),
        parameters: BTreeMap::new(),
        correlation_id: CORR.to_string(),
        approval_ttl_ms: 60_000,
        attempt_policy: AttemptPolicy::Single,
    }
}

/// Test harness: in-memory stores, one active registration, and a
/// coordinator constructor that borrows the stores (fresh per call so each
/// phase of a test can use different adapter scripts / authorizer queues).
struct Harness {
    workspace: WorkspaceId,
    registration: AgentRegistration,
    regs: Rc<MemRegs>,
    exs: Rc<MemExs>,
    apps: Rc<MemApprovals>,
    ops: Rc<MemOps>,
}

impl Harness {
    fn new(capabilities: &[&str]) -> Self {
        let workspace = WorkspaceId::new_v7();
        let registration = AgentRegistration::new(
            AgentRegistrationId::new_v7(),
            workspace.clone(),
            "worker",
            AdapterKind::Fake,
            capabilities.iter().map(|c| c.to_string()).collect(),
        )
        .expect("valid registration");
        let regs = Rc::new(MemRegs::default());
        regs.insert_registration(&registration).expect("stored");
        Self {
            workspace,
            registration,
            regs,
            exs: Rc::new(MemExs::default()),
            apps: Rc::new(MemApprovals::default()),
            ops: Rc::new(MemOps::default()),
        }
    }

    fn coordinator<'a>(
        &'a self,
        adapter: &'a ScriptedAdapter,
        authorizer: &'a StubAuthorizer,
    ) -> AgentInvocationCoordinator<
        'a,
        MemOps,
        MemRegs,
        MemExs,
        MemApprovals,
        StubAuthorizer,
        ScriptedAdapter,
    > {
        AgentInvocationCoordinator::new(
            &self.ops,
            &self.regs,
            &self.exs,
            &self.apps,
            authorizer,
            adapter,
            &*Box::leak(Box::new(registry_with())),
        )
    }

    fn proposal(&self, capability: &str) -> AgentProposal {
        proposal(&self.workspace, &self.registration.id, capability)
    }
}

// ---------------------------------------------------------------------------
// Conformance tests
// ---------------------------------------------------------------------------

#[test]
fn low_risk_granted_invocation_runs_to_completion() {
    let harness = Harness::new(&["work.task.create"]);
    let adapter = ScriptedAdapter::scripted(
        vec![AdapterSubmitOutcome::Accepted {
            external_id: "ext-1".to_string(),
        }],
        vec![AdapterExecutionStatus::Completed],
    );
    let authorizer = StubAuthorizer::allow_next(3); // pre-submit + pre-poll rechecks
    let outcome = harness
        .coordinator(&adapter, &authorizer)
        .propose(&harness.proposal("work.task.create"), "local-user", NOW)
        .expect("completed");
    match outcome {
        ProposalOutcome::Completed {
            execution,
            operation,
            summary,
        } => {
            assert_eq!(execution.status, ExecutionStatus::Completed);
            assert_eq!(operation.status, OperationStatus::Succeeded);
            assert_eq!(summary, "scripted result");
            // Attempt trail: started → accepted → completed.
            assert_eq!(harness.ops.attempts_for(&operation.id).len(), 3);
        }
        other => panic!("expected completed outcome, got {other:?}"),
    }
    let stored = &harness.exs.rows.borrow()[0];
    assert_eq!(stored.status, ExecutionStatus::Completed);
    assert!(stored.operation_id.is_some());
}

#[test]
fn default_deny_blocks_before_any_state_is_created() {
    let harness = Harness::new(&["work.task.create"]);
    let adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let authorizer = StubAuthorizer::allow_next(0);
    let outcome = harness.coordinator(&adapter, &authorizer).propose(
        &harness.proposal("work.task.create"),
        "local-user",
        NOW,
    );
    assert!(matches!(outcome, Err(AgentInvocationError::Denied)));
    assert!(harness.exs.rows.borrow().is_empty());
    assert!(harness.ops.envelopes.borrow().is_empty());
}

#[test]
fn unknown_capability_fails_closed() {
    let harness = Harness::new(&["work.task.create"]);
    let adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let authorizer = StubAuthorizer::allow_next(2);
    let outcome = harness.coordinator(&adapter, &authorizer).propose(
        &harness.proposal("no.such.capability"),
        "local-user",
        NOW,
    );
    assert!(matches!(
        outcome,
        Err(AgentInvocationError::CapabilityUnsupported)
    ));
    assert!(harness.exs.rows.borrow().is_empty());
}

#[test]
fn high_risk_requires_human_approval_two_phase() {
    let harness = Harness::new(&["work.shell.run"]);
    let propose_adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let propose_authorizer = StubAuthorizer::allow_next(1); // initial grant check only

    // Phase 1: parked for approval, nothing executed.
    let execution_id = match harness
        .coordinator(&propose_adapter, &propose_authorizer)
        .propose(&harness.proposal("work.shell.run"), "local-user", NOW)
        .expect("awaiting approval")
    {
        ProposalOutcome::AwaitingApproval { execution, display } => {
            assert_eq!(execution.status, ExecutionStatus::AwaitingApproval);
            assert_eq!(display.risk_class, "High");
            assert_eq!(display.tool_adapter, "fake");
            execution.id
        }
        other => panic!("expected awaiting approval, got {other:?}"),
    };
    // The propose-time approval record exists (binding reconstructed from
    // the persisted execution); the decision only consumes or revokes it.
    assert_eq!(harness.apps.rows.borrow().len(), 1);
    assert!(harness.ops.envelopes.borrow().is_empty());

    // Phase 2: the owner approves; the binding is reconstructed from the
    // persisted execution and the run proceeds.
    let run_adapter = ScriptedAdapter::scripted(
        vec![AdapterSubmitOutcome::Accepted {
            external_id: "ext-2".to_string(),
        }],
        vec![AdapterExecutionStatus::Completed],
    );
    let run_authorizer = StubAuthorizer::allow_next(3); // pre-submit + pre-poll rechecks
    let run = harness
        .coordinator(&run_adapter, &run_authorizer)
        .decide_approval(
            &harness.workspace,
            &execution_id,
            ApprovalDecision::Approve,
            NOW,
        )
        .expect("approved and run")
        .expect("run outcome");
    match run {
        RunOutcome::Completed {
            execution,
            operation,
            ..
        } => {
            assert_eq!(execution.status, ExecutionStatus::Completed);
            assert_eq!(operation.status, OperationStatus::Succeeded);
        }
        other => panic!("expected completed run, got {other:?}"),
    }
    // The approval record binds the plan digest of the persisted execution.
    let approval = &harness.apps.rows.borrow()[0];
    assert_eq!(
        approval.canonical_digest,
        harness.exs.rows.borrow()[0].plan_digest
    );
    assert_eq!(
        approval.status,
        paw_domain::ai_collaboration::ApprovalStatus::Consumed
    );
}

#[test]
fn denied_approval_cancels_without_side_effect() {
    let harness = Harness::new(&["work.shell.run"]);
    let adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let authorizer = StubAuthorizer::allow_next(2);
    let execution_id = match harness
        .coordinator(&adapter, &authorizer)
        .propose(&harness.proposal("work.shell.run"), "local-user", NOW)
        .expect("awaiting approval")
    {
        ProposalOutcome::AwaitingApproval { execution, .. } => execution.id,
        other => panic!("expected awaiting approval, got {other:?}"),
    };
    let run_adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let run_authorizer = StubAuthorizer::allow_next(0);
    let outcome = harness
        .coordinator(&run_adapter, &run_authorizer)
        .decide_approval(
            &harness.workspace,
            &execution_id,
            ApprovalDecision::Deny,
            NOW,
        )
        .expect("decision recorded");
    assert!(outcome.is_none(), "deny runs nothing");
    let stored = &harness.exs.rows.borrow()[0];
    assert_eq!(stored.status, ExecutionStatus::Cancelled);
    assert!(stored.operation_id.is_none());
    assert!(harness.ops.envelopes.borrow().is_empty());
}

#[test]
fn revocation_between_side_effects_blocks_the_next_effect() {
    // The queue allows the initial authorization; the pre-submit recheck is
    // denied — the side effect must be blocked even though the Operation
    // envelope was already opened (it stays queryable, failed).
    let harness = Harness::new(&["work.task.create"]);
    let adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let authorizer = StubAuthorizer::allow_next(1); // propose ok; recheck denied
    let outcome = harness
        .coordinator(&adapter, &authorizer)
        .propose(&harness.proposal("work.task.create"), "local-user", NOW)
        .expect("propose completes with a failed run");
    match outcome {
        ProposalOutcome::Completed {
            execution,
            operation,
            summary,
        } => {
            assert_eq!(execution.status, ExecutionStatus::Failed);
            assert_eq!(operation.status, OperationStatus::Failed);
            assert!(summary.contains("authorization revoked"));
            // The attempt trail records the blocked submit.
            assert_eq!(harness.ops.attempts_for(&operation.id).len(), 1);
        }
        other => panic!("expected failed run, got {other:?}"),
    }
}

#[test]
fn indeterminate_submit_holds_unresolved_and_reconcile_resolves() {
    let harness = Harness::new(&["work.task.create"]);
    let adapter = ScriptedAdapter::scripted(
        vec![AdapterSubmitOutcome::Indeterminate {
            reason: "disconnect after send".to_string(),
        }],
        vec![],
    );
    let authorizer = StubAuthorizer::allow_next(2);
    let outcome = harness
        .coordinator(&adapter, &authorizer)
        .propose(&harness.proposal("work.task.create"), "local-user", NOW)
        .expect("propose completes with unresolved run");
    match outcome {
        ProposalOutcome::Completed {
            execution,
            operation,
            ..
        } => {
            assert_eq!(execution.status, ExecutionStatus::OutcomeUnknown);
            assert_eq!(operation.status, OperationStatus::OutcomeUnknown);
            // Indeterminate attempt recorded, non-retryable.
            let attempts = harness.ops.attempts_for(&operation.id);
            assert!(attempts.iter().any(|(_, retryable)| !retryable));
        }
        other => panic!("expected unresolved run, got {other:?}"),
    }

    // Reconciliation: the only path out of OutcomeUnknown.
    let execution_id = harness.exs.rows.borrow()[0].id.clone();
    let reconcile_adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let reconcile_authorizer = StubAuthorizer::allow_next(0);
    let resolved = harness
        .coordinator(&reconcile_adapter, &reconcile_authorizer)
        .reconcile(
            &harness.workspace,
            &execution_id,
            ReconcileResolution::Complete,
            &ctx("local-user"),
        )
        .expect("reconciled");
    assert_eq!(resolved.status, ExecutionStatus::Completed);
    let op_id = resolved.operation_id.clone().expect("linked");
    let op = harness.ops.get(&op_id).expect("envelope");
    assert_eq!(op.status, OperationStatus::Succeeded);
}

#[test]
fn timed_out_provider_result_becomes_reconcilable_not_inferred() {
    let harness = Harness::new(&["work.task.create"]);
    let adapter = ScriptedAdapter::scripted(
        vec![AdapterSubmitOutcome::Accepted {
            external_id: "ext-3".to_string(),
        }],
        vec![AdapterExecutionStatus::TimedOut],
    );
    let authorizer = StubAuthorizer::allow_next(3);
    let outcome = harness
        .coordinator(&adapter, &authorizer)
        .propose(&harness.proposal("work.task.create"), "local-user", NOW)
        .expect("run with timeout");
    match outcome {
        ProposalOutcome::Completed {
            execution,
            operation,
            ..
        } => {
            assert_eq!(execution.status, ExecutionStatus::TimedOut);
            assert_eq!(operation.status, OperationStatus::OutcomeUnknown);
        }
        other => panic!("expected unresolved timeout run, got {other:?}"),
    }
}

#[test]
fn suspended_registration_blocks_proposal() {
    let harness = Harness::new(&["work.task.create"]);
    // Suspend the registration directly in the store.
    let mut suspended = harness.registration.clone();
    suspended.status = RegistrationStatus::Suspended;
    harness
        .regs
        .update_registration_status(&suspended)
        .expect("updated");

    let adapter = ScriptedAdapter::scripted(vec![], vec![]);
    let authorizer = StubAuthorizer::allow_next(2);
    let outcome = harness.coordinator(&adapter, &authorizer).propose(
        &harness.proposal("work.task.create"),
        "local-user",
        NOW,
    );
    assert!(matches!(
        outcome,
        Err(AgentInvocationError::RegistrationNotActive { .. })
    ));
    assert!(harness.exs.rows.borrow().is_empty());
}

#[test]
fn grant_driven_authorization_scope_intersection() {
    // The GrantEvaluatedAuthorizer (application-level policy): a grant
    // covering capability + workspace allows; a different workspace,
    // different capability, or administrative action denies. The SQLite
    // grant store is exercised by the infrastructure suite.
    let ws = WorkspaceId::new_v7();
    let other_ws = WorkspaceId::new_v7();
    let grant = AgentGrant {
        id: "grant-1".to_string(),
        subject_id: "agent:reg-1".to_string(),
        capabilities: vec!["work.task.create".to_string()],
        workspace_id: ws.clone(),
        tools: vec![],
        valid_from_ms: NOW - 1_000,
        valid_until_ms: None,
    };
    let authorizer = paw_application::ports::authorization::GrantEvaluatedAuthorizer::new(
        StubGrants {
            grants: vec![grant],
        },
        || NOW,
    );
    let agent_ctx = ctx("agent:reg-1");

    assert!(
        authorizer
            .authorize(
                &agent_ctx,
                &ProtectedAction::CapabilityInvoke {
                    workspace_id: ws.clone(),
                    capability_id: "work.task.create".to_string(),
                },
            )
            .is_ok()
    );
    assert!(
        authorizer
            .authorize(
                &agent_ctx,
                &ProtectedAction::CapabilityInvoke {
                    workspace_id: other_ws,
                    capability_id: "work.task.create".to_string(),
                },
            )
            .is_err()
    );
    assert!(
        authorizer
            .authorize(
                &agent_ctx,
                &ProtectedAction::CapabilityInvoke {
                    workspace_id: ws.clone(),
                    capability_id: "work.shell.run".to_string(),
                },
            )
            .is_err()
    );
    // Agent subjects hold no administrative authority.
    assert!(
        authorizer
            .authorize(
                &agent_ctx,
                &ProtectedAction::CreateAgentGrant {
                    workspace_id: WorkspaceId::new_v7(),
                },
            )
            .is_err()
    );
}

/// Grant-store stub for the grant-driven path.
struct StubGrants {
    grants: Vec<AgentGrant>,
}

impl GrantStore for StubGrants {
    fn grants_for_subject(&self, subject_id: &str) -> Result<Vec<AgentGrant>, AuthorizationError> {
        Ok(self
            .grants
            .iter()
            .filter(|g| g.subject_id == subject_id)
            .cloned()
            .collect())
    }
}
