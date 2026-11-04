# Agent Adapter Architecture

**Status:** Proposed — E0.2

## 1. Purpose and boundary

The Agent subsystem coordinates workbench-owned executions. Provider adapters translate a stable contract into supported, documented Hermes, Codex, Cursor, or other interfaces. Core logic does not depend on private provider databases, undocumented APIs, or assumptions about provider persistence.

Agent Adapter and Workflow Engine are separate:
- Adapter: connectivity, capability discovery, submission, status, cancellation, result normalization.
- Workflow Engine: workbench-owned sequencing, dependencies, approvals, retries, and run history.

## 2. Adapter contract

Conceptual operations:
- `capabilities()`: supported operations and constraints.
- `health()`: readiness without exposing credentials.
- `submit(request)`: submit and return provider correlation data.
- `status(external_id)`: query status where supported.
- `cancel(external_id)`: request cancellation where supported.
- `result(external_id)`: retrieve normalized result/artifacts where supported.

Capabilities declare limitations, supported transports (CLI/API/MCP), cancellation/streaming support, and permissions. Unsupported operations return explicit not-supported results.

## 3. Execution lifecycle

```text
Created → Validating → AwaitingApproval (if required)
→ Approved → Submitted → Running
→ Completed | Failed | Cancelled | TimedOut | OutcomeUnknown
```

Transitions are validated and persisted. OutcomeUnknown is required when a submit call times out or disconnects after the request may have reached the provider. Reconcile provider state or request human resolution; blind resubmission risks duplicate work.

## 4. Authorization

Permission levels are Read, Write, Execute, and Admin, scoped by workspace/project, filesystem roots, tools, and operation. Read-only inspection may be auto-allowed by policy. File writes/code changes require explicit scope. Shell execution, publication, destructive operations, credential access, privilege escalation, and broad network access require explicit approval. Approval binds to a concrete plan, scope, and expiry; material changes invalidate it.

Provider-side permissions are additional defense, not a replacement. An adapter must never widen approved scope.

## 5. Request and result

Requests include execution ID, provider, task/prompt reference, working-directory scope, allowed capabilities, timeout, approval reference, and correlation ID. Results include normalized status, provider execution ID, timestamps, exit/error category, summary, and artifact references. Do not copy sensitive transcripts by default; disclose retention controls.

## 6. Reliability

Use a workbench execution ID as idempotency key where supported. Otherwise record attempts and external IDs; do not imply exactly-once execution. Retry only after classifying the previous attempt as rejected, accepted, or indeterminate.

## 7. Conformance

Test capabilities, malformed input, authorization refusal, timeout, cancellation, partial result, provider outage, unknown outcome, and secret redaction using fake adapters.

## 8. Initial integrations

Hermes, Codex, and Cursor are candidates. Use stable public CLI/API/MCP/file interfaces verified against target versions. No integration is supported until a versioned compatibility test and operational guide exist.
