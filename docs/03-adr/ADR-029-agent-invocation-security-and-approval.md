# ADR-029: Agent Invocation Security and Human Approval Binding

- **Status:** Proposed (R1)
- **Decision status:** Proposed / Not Accepted
- **Implementation status:** Not Implemented
- **Verification status:** Not Verified
- **Date:** 2026-11-11
- **Scope:** Agent-to-capability invocation, authorization, approval, side-effect execution and uncertain outcomes
- **Related:** ADR-004, ADR-017, ADR-019, ADR-020, ADR-022, ADR-023, ADR-024, ADR-025, ADR-026, ADR-028; `agent-authorization-model.md`

## 1. Context

PAW allows external Agent runtimes to assist with domain workflows. Agent output, tool arguments, provider metadata, and returned content are untrusted input. A Role Profile, WorkspaceComposition, plugin descriptor, capability declaration, Agent prompt, or provider-side permission is not an authorization grant.

The existing Agent Authorization Model establishes default deny, least privilege, explicit workspace/resource scope, backend enforcement, approval for high-risk operations, and audit. ADR-019 establishes durable Operation semantics, including uncertainty and reconciliation. This ADR makes those rules precise at the invocation boundary so that approval cannot be replayed against a changed action and retries cannot duplicate non-idempotent effects.

## 2. Decision drivers

1. Human intent must be bound to the exact action being approved.
2. Authorization must be evaluated by the Workbench backend, not delegated to the Agent or UI.
3. Every protected side effect must be re-authorized immediately before execution.
4. Durable Operation state and Agent execution state remain distinct but correlated.
5. Unknown external outcomes must be reconciled rather than blindly retried.
6. Secret material and sensitive payloads must not leak into prompts, approval displays, or ordinary logs.

## 3. Options considered

- **A. Trust Agent tool selection and provider permissions:** rejected; prompt injection and provider behavior cannot establish Workbench authority.
- **B. Approve a broad task/session once:** rejected; scope and plan can drift after approval.
- **C. Per-invocation backend authorization with immutable approval binding:** selected; auditable and compatible with ADR-019/022.

## 4. Decision

### 4.1 Trust boundary and default deny

Treat Agent-generated plans, tool calls, arguments, files, URLs, provider messages, and tool results as untrusted. The backend MUST validate identity, capability contract, schema, data workspace, resource scope, policy, and risk before dispatch.

Missing, malformed, stale, ambiguous, or unavailable authorization context results in deny. No Role Profile, plugin state, Agent identity, provider credential, or UI visibility setting can widen effective authority.

### 4.2 Invocation pipeline

```text
Agent proposal (untrusted)
  -> parse and schema-validate
  -> resolve actor / Agent / Data Workspace / target
  -> resolve capability descriptor and contract version
  -> evaluate current authorization (default deny)
  -> classify risk and approval requirement
  -> canonicalize exact invocation plan
  -> validate approval binding (if required)
  -> create/associate durable Operation and attempt
  -> re-authorize immediately before each side effect
  -> execute via mediated capability adapter
  -> persist outcome / OutcomeUnknown
  -> reconcile, audit and return bounded result
```

The Application Coordination layer owns Operation lifecycle, correlation/causation, outbox, retry and recovery. The domain owns business invariants and capability execution. The Agent Runtime owns planning and tool orchestration only. AI Adapter owns provider protocol translation only.

### 4.3 Approval binding

An approval record MUST bind to:

- the canonical digest of the immutable invocation plan;
- actor and Agent identity;
- Data Workspace and exact resource scope;
- capability ID and contract version;
- tool/adapter and target/provider identity where relevant;
- normalized operation and material parameters;
- risk classification and requested side-effect set;
- expiry and one-time or explicitly bounded attempt policy.

The approval display MUST make the target, action, affected resources, material data transfer, risk, and expiry understandable to the human approver. Secret values MUST be redacted; opaque credential references may be shown when needed to identify the credential scope.

Any material change to target, capability, tool, resource scope, payload semantics, or side-effect set invalidates the approval and requires a new decision. An approval is not a grant and cannot override a denied permission. Expired, revoked, mismatched, or unverifiable approval is denied.

### 4.4 Authorization recheck

The backend MUST re-evaluate current policy immediately before each protected side effect. It MUST NOT rely solely on authorization performed when the Operation was created or when a human approved it. Revoked/expired grants, changed workspace scope, disabled capability, or newly blocked plugin prevent the next side effect.

### 4.5 Retry and uncertain outcomes

- Idempotent retries may reuse an approval only when the canonical action, target, scope, and attempt policy are unchanged and the approval remains valid.
- A changed action or target requires fresh authorization and approval.
- Non-idempotent actions MUST NOT be retried when the prior outcome is uncertain until reconciliation establishes whether the effect occurred.
- If the external system cannot prove the outcome, retain `OutcomeUnknown`, surface it for human or domain-specific reconciliation, and do not report success or failure by inference.
- Operation completion and Agent execution completion are separate facts; one MUST NOT be inferred from the other.

### 4.6 Result and data handling

Agent/provider/tool output is untrusted. Validate output schema and resource references before persistence or downstream invocation. Apply workspace scope and secret redaction to returned data. Do not allow a tool result to inject executable instructions into the policy or approval system.

### 4.7 Audit evidence

Record invocation/Operation/attempt IDs, actor and Agent IDs, workspace, capability and version, target identity, canonical digest, policy decision and reason, approval reference, timestamps, recheck decisions, adapter/provider, idempotency key where applicable, and outcome/reconciliation state. Avoid storing secrets or unnecessary full payloads. Audit records must preserve enough evidence to explain who authorized what exact action and what happened.

## 5. Consequences

- Agent autonomy is constrained by explicit capability and policy boundaries.
- High-risk actions require precise, reviewable human intent.
- The approval lifecycle is more complex but avoids stale-plan approval reuse.
- Operation reconciliation is required for external effects whose outcomes cannot be proven synchronously.

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Canonicalization omits a material parameter | Define deterministic canonical form per capability; contract tests mutate each material field |
| Approval is replayed | Bind to identity, scope, target, digest, expiry and attempt policy; enforce single-use where possible |
| Authorization changes after approval | Recheck before each side effect |
| Provider reports ambiguous outcome | Persist OutcomeUnknown and reconcile; no blind retry |
| Sensitive payload appears in approval/audit | Redaction and data-minimization tests |
| Agent result causes confused-deputy behavior | Re-resolve actor, workspace and resource scope for every follow-on invocation |

## 7. Required validation

- Default-deny for missing or invalid context.
- Cross-workspace and out-of-scope invocation denied through direct backend calls.
- Role Profile and WorkspaceComposition changes do not alter authorization.
- Plan digest mismatch, changed target, changed tool, expired/revoked approval all denied.
- Authorization revocation between two side effects blocks the second side effect.
- Approval cannot elevate grants or override system policy.
- Idempotent exact retry behavior is bounded; non-idempotent uncertain outcome is not retried.
- OutcomeUnknown remains visible until reconciled.
- Agent execution status cannot overwrite Operation state.
- Secret redaction and audit completeness are verified.

## 8. Implementation and verification boundary

This ADR is a policy decision proposal, not an implementation authorization. It does not specify Rust structs, canonical serialization library, digest algorithm, approval SQL schema, migration, UI implementation, or adapter code. Those require separate implementation specifications and human approval. Current state remains **Not Implemented / Not Verified**.

## 9. Revisit when

PAW introduces multi-user delegated approvals, organization-wide policy administration, third-party Agent marketplace, remote execution workers, or provider-native delegated credentials. Revisit without weakening backend default-deny and per-side-effect authorization.
