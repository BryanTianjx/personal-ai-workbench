# Agent Adapter Architecture

**Architecture Baseline:** E0.2 Candidate
**Decision Status:** Accepted boundary; authorization rules in agent-authorization-model.md
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Purpose and boundary
The Agent subsystem coordinates workbench-owned Agent Executions. Provider adapters translate a stable contract into supported, documented Hermes, Codex, Cursor, or other interfaces. Core logic does not depend on private provider databases, undocumented APIs, or assumptions about provider persistence.

Agent Adapter and Workflow Engine are separate:

- Adapter: connectivity, capability discovery, submission, status, cancellation, result normalization.
- Workflow Engine: workbench-owned sequencing, dependencies, approvals orchestration, retries, and Workflow Run history.
- Authorization service: centralized policy decision and approval validation; neither Adapter nor Workflow Engine may bypass it.

## 2. Adapter contract
Conceptual operations: `capabilities()`, `health()`, `submit(request)`, `status(external_id)`, `cancel(external_id)`, and `result(external_id)`. Capabilities declare limitations, transports (CLI/API/MCP), cancellation/streaming support, and required permissions. Unsupported operations return explicit not-supported results.

## 3. Agent Execution lifecycle
```text
Created → Validating → AwaitingApproval (if required)
→ Approved → Submitted → Running
→ Completed | Failed | Cancelled | TimedOut | OutcomeUnknown
```
Transitions are validated and persisted by AI/Agent. `OutcomeUnknown` is required when a submit call times out or disconnects after the request may have reached the provider. Reconcile provider state or request human resolution; blind resubmission risks duplicate work. Workflow Run has its own lifecycle and references executions; Task has a separate business lifecycle.

## 4. Authorization
Every side effect is checked by the central authorization service immediately before execution. Permission levels are Read, Write, Execute, and Admin, scoped by workspace/project, filesystem roots, tools, and operation. Default is deny. Read-only inspection may be auto-allowed only by an explicit policy. File writes require explicit scope. Shell execution, publication, destructive operations, credential use, privilege escalation, and broad network access require human approval. Approval binds to an immutable plan digest, exact scope, operation set, and expiry; any material change invalidates it. See `agent-authorization-model.md`.

Provider-side permissions are defense in depth, not a replacement. An adapter must never widen approved scope.

## 5. Request and result
Requests include execution ID, provider, task/prompt reference, working-directory scope, allowed capabilities, timeout, approval reference, and correlation ID. Results include normalized status, provider execution ID, timestamps, exit/error category, summary, and artifact references. Do not copy sensitive transcripts by default; disclose retention controls.

## 6. Reliability
Use a workbench execution ID as idempotency key where supported. Otherwise record attempts and external IDs; do not imply exactly-once execution. Retry only after classifying the previous attempt as rejected, accepted, or indeterminate.

## 7. Conformance
Test capabilities, malformed input, authorization refusal, scope escape, stale approval, timeout, cancellation, partial result, provider outage, unknown outcome, duplicate-submit prevention, and secret redaction using fake adapters.

## 8. Initial integrations
Hermes, Codex, and Cursor are candidates. Use stable public CLI/API/MCP/file interfaces verified against target versions. No integration is supported until a versioned compatibility test and operational guide exist.

## 9. E0.3 Operation relationship and reconciliation

Agent Execution remains AI Collaboration-owned and represents a provider-facing execution lifecycle. Operation is the cross-cutting workbench execution envelope and may reference one or more Agent Execution IDs. Workflow Run remains Automation-owned and may coordinate multiple Operations. Task remains a Work Management business object. None inherits another object's status or transition authority.

Each submission attempt records the workbench execution ID, provider, request/plan digest, idempotency key when supported, submitted-at time, provider execution ID when known, and outcome classification. A timeout after possible provider acceptance becomes OutcomeUnknown. The adapter first attempts provider status reconciliation using a known external ID or provider-supported idempotency key. If reconciliation cannot establish outcome, the system presents an explicit unresolved state; it does not silently resubmit a non-idempotent request.

The adapter reports provider capability limitations (including cancellation and idempotency support) as data. It cannot decide workbench authorization, broaden scope, or convert an indeterminate result into success.
