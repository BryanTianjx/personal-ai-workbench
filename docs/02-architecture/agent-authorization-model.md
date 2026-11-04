# Agent Authorization Model

**Architecture Baseline:** E0.2 Candidate  
**Policy:** Default deny; least privilege; explicit scope  
**Implementation Status:** Not Implemented  
**Verification Status:** Not Verified

## 1. Purpose
This document defines workbench-side authorization for Agent and automation side effects. Provider-native permissions are defense in depth and never replace this policy.

## 2. Permission dimensions
Permission levels:
- Read: inspect explicitly scoped resources.
- Write: create or modify explicitly scoped resources.
- Execute: invoke an approved tool/command or external action.
- Admin: change workbench policy, identities, or security configuration. Admin is user-controlled and is not granted to ordinary Agents.

Every grant binds a subject (user/Agent identity), workspace, optional project, resource roots, operation set, tool/capability set, and validity interval. Missing or ambiguous scope is denied. No implicit elevation or Agent-to-Agent delegation.

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

## 4. Approval requirements
Human approval is required for shell execution, destructive operations, broad or out-of-scope writes, credential use, privilege escalation, external publication, and broad network access. Approval displays the exact action, target, resource scope, data transfer, risk, and expiry. It binds to a canonical plan digest, exact capabilities, resource scope, and one execution attempt or explicitly bounded set of attempts. Any material plan/scope/tool/target change invalidates it. Expired or revoked approval is denied.

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
