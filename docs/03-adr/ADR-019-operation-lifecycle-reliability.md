# ADR-019: Durable Operation Lifecycle and Reliability Semantics

- **Status:** Proposed
- **Date:** 2026-11-08
- **Decision scope:** Long-running operation state, retries, cancellation, and recovery
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified
- **Related:** ADR-004, ADR-005, ADR-006, ADR-017

## Context

Agent execution, workflow runs, imports, indexing, backup, and publication preparation may outlive a UI request or process lifetime. They need consistent recovery and diagnostics, but Task, Workflow Run, and provider Agent Execution must not collapse into one state machine.

## Decision proposal

Introduce a cross-cutting Operation record for durable execution bookkeeping. Operation is not a replacement for Task, Workflow Run, or Agent Execution; it records the execution envelope and recovery mechanics for a bounded unit of work.

Minimum metadata: operation ID, kind, owner/domain and owning record ID, workspace ID when scoped, status, timestamps, correlation/causation IDs, attempt and retry classification, cancellation state, safe diagnostic category, and references to approvals, Outbox messages, external execution IDs, and produced artifacts.

Required semantics:
- Persist state transitions before presenting durable progress.
- Use explicit states such as Pending, Running, AwaitingApproval, RetryScheduled, Succeeded, Failed, Cancelled, and OutcomeUnknown.
- Do not reuse one enum across Task, Workflow Run, Agent Execution, and Operation.
- Never hold SQLite transactions open across provider/network calls.
- Retry only after classifying the previous attempt as rejected, accepted, or indeterminate.
- Treat uncertain submission outcomes as OutcomeUnknown; reconcile before non-idempotent resubmission.
- Use provider idempotency keys where supported; otherwise record attempts and external IDs. Do not claim exactly-once semantics.
- Cancellation is cooperative and capability-dependent; unsupported cancellation is explicit.
- Commit Outbox entries atomically with local state when downstream delivery is required.

## Alternatives

1. UI-local progress only: rejected because it is lost on restart.
2. One universal state machine: rejected because business/provider semantics differ.
3. Ad hoc reliability in each feature: rejected due to divergent behavior.
4. Cross-cutting Operation envelope with owner-specific lifecycles: proposed.

## Consequences

**Positive:** restart recovery, consistent observability, safe retry decisions, clear distinction between business work and execution attempts.

**Negative:** persistence, retention, and transition complexity increase.

## Risks and mitigations

- Duplicate side effects: reconcile indeterminate submissions; require idempotency or human resolution.
- State duplication: store references/orchestration state, not shadow domain state.
- Sensitive diagnostics: redact credentials, prompts, and payloads by default.
- Unbounded history: define retention/export policy.

## Validation required

Process restart recovery; timeout-after-submit and duplicate-submit tests; supported/unsupported cancellation; retry classification/backoff; workspace-scoped operation queries; audit redaction and owner reconciliation.

## Revisit when

A proven durable execution engine is adopted or operation complexity exceeds the local implementation. Authorization, audit, and ownership semantics must be preserved.
