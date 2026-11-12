# R2-EXEC-05 — Security & Agent Governance Audit

- Parent: [Engineering Audit R2.0](./README.md)
- Dependency: R2-EXEC-01 baseline frozen
- Priority: trust boundaries, Workspace isolation, Agent side effects

## Objective

Assess the security boundaries that protect user data and control side effects, including Tauri command exposure, Workspace authorization, filesystem access, Agent approval, and audit logging.

## Scope

Tauri commands/capabilities/CSP, frontend trust boundary, application authorizers, Agent invocation, approvals, filesystem adapters, secret handling, audit trail, external process/provider integration.

## Audit tasks

### A. Tauri and WebView boundary

1. Inventory exposed commands and command argument types.
2. Map each command to validation, authorization, use case, and persistence.
3. Inspect capability files, window permissions, plugin permissions, shell/process access, external URL handling, and filesystem scopes.
4. Inspect CSP and content origins. A null CSP is a risk candidate requiring threat-model evidence, not automatically a confirmed exploit.
5. Inspect deep links, navigation, asset protocol, and untrusted content rendering.

### B. Workspace isolation

Create synthetic workspaces A and B and synthetic users/resources. Attempt controlled cross-workspace access via:
- direct resource IDs;
- repository reads/writes;
- project/task relationships;
- search and knowledge retrieval;
- attachments;
- Agent proposals and actions;
- export/import or backup surfaces.

Verify that scope is enforced server-side/application-side, not only by frontend filtering. Test read and write paths. Do not use real user data.

### C. Authorization and approval lifecycle

Inspect:
- actor identity and provenance;
- action and resource scope;
- Workspace binding;
- risk classification;
- approval token creation and consumption;
- plan-summary binding;
- expiry, revocation, replay and duplicate handling;
- reauthorization immediately before side effects;
- idempotency and OutcomeUnknown reconciliation;
- denied-action behavior.

Verify all side-effect entry points route through the intended central authorization boundary.

### D. Agent trust and Hermes integration

1. Trace production composition and identify whether FakeAgentAdapter is wired into production.
2. Locate any real Hermes adapter and determine its process/protocol contract.
3. Verify fake tests are clearly distinguished from real-provider conformance.
4. Inspect input validation, command allowlists, arguments, working-directory boundaries, environment inheritance, timeouts, output limits, cancellation, and process cleanup.
5. Inspect secret injection and redaction.
6. Verify untrusted Agent output cannot directly bypass approval or trigger unauthorized side effects.
7. Compare actual implementation with ADR-022; Accepted architecture status is not implementation proof.

### E. Filesystem security

Inspect canonical path enforcement, symlink races, traversal, path normalization, attachment ownership, Workspace boundaries, deletion, temporary files, and archive extraction. Test only in disposable directories.

### F. Audit evidence reliability

Inspect allowed/denied/failed event coverage, correlation IDs, tamper resistance, sensitive-data redaction, persistence failures, and whether audit failure is fail-open or fail-closed for protected operations.

### G. Safe security test execution

Run existing security tests with synthetic data. Do not probe external systems, real accounts, production files, or real credentials. Do not run destructive exploit scenarios.

## Required outputs

- `R2-EXEC-05-security-architecture-report.md`
- `R2-EXEC-05-workspace-isolation-report.md`
- `R2-EXEC-05-agent-governance-report.md`
- `R2-EXEC-05-file-access-report.md`
- `R2-EXEC-05-security-findings.md`

## Acceptance criteria

All major trust boundaries are diagrammed; read and write isolation are evaluated; production fake/real adapter wiring is explicit; ADR-022 is assessed against code and runtime evidence; every finding is evidence-backed.

## Stop conditions

Stop if a test could affect real resources, execute an uncontrolled command, disclose secrets, or bypass a real security boundary outside an isolated harness. Report BLOCKED and preserve evidence safely.
