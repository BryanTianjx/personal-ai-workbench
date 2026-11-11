# Task Breakdown

## T0 — Baseline and source custody (READ ONLY)

**Goal:** Establish exact repository state before analysis.

1. Record repository URL, branch, HEAD SHA, working-tree status and retrieval timestamp.
2. Confirm all seven ADR files and cross-review report exist at canonical paths.
3. Read ADR-001～006 and ADR-013～022; do not infer missing ADR-007～012.
4. Read current PRD, roadmap, domain architecture, cross-domain contracts, module boundaries, Agent Adapter Architecture, Agent Authorization Model, and Operation lifecycle documents.
5. Record source path, commit SHA and relevant section for every cited constraint.

**Acceptance:** source inventory is complete enough to establish traceability; missing sources are listed as gaps, not guessed.

## T1 — ADR-023～025 boundary review (READ ONLY)

Check Domain versus Plugin ownership; four independent plugin state dimensions; static registration; Role Profile contains no grants/secrets/approval; WorkspaceComposition is scoped to one Data Workspace and is not a security boundary; invalid profile/composition safe fallback.

**Output:** evidence-backed findings for each check.

## T2 — ADR-026～027 capability and domain review (READ ONLY)

Check Capability Registry is metadata-only and not Service Locator; invocation passes through owning Domain use case and Application Coordination; AI Adapter and external Agent Runtime boundaries remain intact; plugin adoption does not move ownership into Core; Learning & Growth remains first-class and does not expand Alpha scope.

**Output:** evidence-backed findings for each check.

## T3 — ADR-028～029 compatibility and security review (READ ONLY)

Check dependency DAG validation, cycles, required/optional semantics and deterministic resolution; independent version dimensions; fail-closed breaking/security-critical changes; domain-owned migrations and Operation drain/reconcile; Agent output is untrusted; approval binds canonical plan digest, actor, workspace, scope, target, capability, tool, expiry and attempt policy; authorization rechecked before every side effect; OutcomeUnknown is reconciled without blind retry.

**Output:** threat scenarios, findings and evidence.

## T4 — Cross-ADR review (READ ONLY)

Construct a matrix across all seven ADRs for domain/data ownership, Data Workspace security, plugin/profile/composition lifecycle, capability and Agent invocation, contract/version compatibility, Operation reliability/audit, offline behavior and product-stage scope. Identify contradiction, duplicated policy, missing owner, untestable MUST and inconsistent terminology.

## T5 — Findings and approval packet (DOCUMENTATION ONLY)

Produce a severity-ranked findings register; proposed corrections as patch suggestions (not applied edits unless separately authorized); ADR-by-ADR owner decision sheet; deferred implementation-spec backlog with dependencies and risks; explicit go/no-go recommendation.

## H0 — Human approval gate

Stop and request an owner decision for each ADR. No ADR status changes and no implementation tasks start until approval is explicit. Approval of the ADRs does not itself authorize code changes; implementation specifications and implementation authorization are separate gates.
