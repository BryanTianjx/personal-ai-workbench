# Agent Authorization Model

**Architecture Baseline:** E0.2 Candidate
**Policy:** Default deny; least privilege; explicit scope
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Purpose
This document defines workbench-side authorization for Agent and automation side effects. Provider-native permissions are defense in depth and never replace this policy.

## 2. Permission dimensions and composition

Permissions are independent capabilities, not an automatic hierarchy:

- **Read:** inspect explicitly scoped resources.
- **Write:** create or modify explicitly scoped resources. Write does not imply Read outside the exact resource/operation needed to perform the granted write.
- **Execute:** invoke explicitly approved tools, commands, or external actions. Execute does not imply unrestricted filesystem Read/Write.
- **Admin:** change workbench policy, identities, or security configuration. Admin is reserved for the human owner and is never granted to ordinary Agents.

A grant is the explicit set of capabilities assigned to a subject; possession of one capability never silently implies another. Every grant binds a subject (user/Agent identity), workspace, optional project, resource roots, operation set, tool/capability set, and validity interval. Scope is the intersection of all applicable constraints (subject, workspace, project, resource root, operation, tool, and time). Missing, conflicting, or ambiguous scope is denied. There is no implicit inheritance, elevation, or Agent-to-Agent delegation.

## 3. Decision pipeline
```text
Request
 → Capability validation
 → Subject and workspace resolution
 → Resource/path scope validation
 → Policy evaluation (default deny)
 → Denied | Approval required | Allowed
 → Approval validation (when required)
 → Execute through mediated adapter
 → Result validation
 → Audit decision and outcome
```
Re-evaluate policy immediately before every protected side effect. Authorization is enforced in the backend application layer, not in UI or prompt text.

## 4. Authorization, denial and approval semantics

Evaluate each request in this order:

1. **Within the active grant and low risk:** allow only the explicitly granted capability and scope.
2. **Within the active grant but classified as high risk:** require human approval before execution. Approval confirms the exact action; it does not add capabilities or widen resource scope.
3. **Outside the active grant but potentially user-authorizable:** deny the current request. Present the missing capability/scope to the human owner; the owner must create or amend a separate grant. Re-evaluate the request from the beginning after the grant changes. Do not treat an approval click as a grant expansion.
4. **Prohibited by system policy or an invariant security boundary:** always deny. No Agent request or ordinary approval can override this class.

Human approval is required for shell execution, destructive operations, high-impact writes, credential use, privilege changes, external publication, and broad network access when the action is otherwise within an explicitly grantable policy boundary. Approval displays the exact action, target, resource scope, data transfer, risk, and expiry. It binds to a canonical plan digest, exact capabilities, resource scope, and one execution attempt or explicitly bounded set of attempts. Any material plan/scope/tool/target change invalidates it. Expired or revoked approval is denied.

## 5. Filesystem authorization
Resolve paths against configured allowlisted roots using platform-aware canonicalization. Reject traversal, unexpected absolute paths, UNC/network roots unless explicitly allowed, and symlink escapes. Recheck immediately before open/write to reduce time-of-check/time-of-use risk. Enforce workspace ownership for both source and destination. UI file picker selection is not a persistent grant.

## 6. Credential boundary
Agents do not receive unrestricted credential-store access. Credentials are referenced by opaque IDs and used through narrowly scoped broker operations; each use is policy checked and audited. Secret values never enter prompts, ordinary execution logs, or result summaries by default.

## 7. Audit contract
Record event ID, timestamp, actor/Agent, workspace, optional project, operation, resource identifier, requested capability, policy decision/reason code, approval ID and plan digest, execution ID, outcome, and correlation ID. Do not store secrets or full sensitive payloads. Audit writer is append-only to ordinary modules.

## 8. Failure semantics
Policy service unavailable, scope resolution failure, stale approval, unknown subject, or malformed request results in deny. On uncertain external outcome, persist OutcomeUnknown and reconcile; do not blindly retry non-idempotent actions.

## 9. Minimum conformance tests
Default-deny; cross-workspace read/write denial; path traversal and symlink escape; scope narrowing; stale/expired/revoked approval; changed plan digest; unauthorized tool; credential mediation; Agent delegation denial; audit redaction; policy-service failure.

## 10. E0.3 operation and approval binding

Operation and correlation identifiers support traceability but never grant authority. Each protected side effect is independently authorized against the current actor, Workspace, target resource, capability, tool, and time window.

An approval is bound to a canonical immutable plan digest, exact operation set, exact resource scope, target/provider identity, and expiry. The execution attempt must present the same digest and scope. A retry that changes target, payload semantics, tool, permissions, or resource scope requires a new authorization decision and, where applicable, a new approval. An idempotent transport retry of the exact approved request may reuse the same approval only within its defined validity and attempt policy.

The application records the approval reference and digest on the Operation/attempt for audit linkage. It does not copy secrets or sensitive full payloads into the approval record. Authorization is re-evaluated immediately before each side effect; an Operation created while a grant was valid does not preserve that grant after expiry or revocation.
