# R2-EXEC-06 — Business Function & Requirements Audit

- Parent: [Engineering Audit R2.0](./README.md)
- Dependency: R2-EXEC-01 baseline frozen
- Scope: E0–E7 and end-to-end product workflows

## Objective

Determine whether product requirements are implemented, integrated, verified, and accepted. Prevent design completion, backend-only implementation, or historical reports from being mistaken for user-facing functionality.

## Scope

Product baseline, roadmap, requirements, ADRs, E0–E7 specifications/evidence, production code, tests, GUI, cross-domain workflows.

## Audit tasks

### A. Build requirements inventory

For every in-scope requirement, capture:
- requirement ID and exact statement;
- source document and section;
- related ADR;
- expected user-visible behavior;
- production implementation path/symbol;
- related tests and runtime evidence;
- GUI route/page;
- implementation/integration/verification/acceptance status;
- gaps, assumptions, and dependencies.

### B. E0–E7 stage verification

For each stage:
1. Extract original scope and acceptance criteria.
2. Locate production implementation.
3. Locate integration points and UI entry points.
4. Locate tests and identify whether they use fakes.
5. Verify critical behavior using safe isolated tests where applicable.
6. Identify deferred, missing, partially implemented, or simulated behavior.
7. Compare top-level report status with detailed evidence and later correction notes.
8. Do not assume a stage is complete because a later stage depends on it.

### C. Product domains

Assess Work Management, Learning & Growth, Knowledge & Context, AI Collaboration, Creator, Plugin/Profile/Composition, and cross-domain workflows.

For each domain, verify CRUD lifecycle, validation, permissions, persistence, search/retrieval, error states, GUI integration, and restart persistence where relevant.

### D. End-to-end journeys

Trace at least:
- Workspace creation and switching;
- project/task lifecycle;
- learning item to project linkage and gap workflow;
- knowledge ingestion and retrieval;
- Agent proposal → risk evaluation → approval/deny → execution → reconciliation;
- plugin enable/disable and composition/profile linkage;
- creator draft/version/review/publish lifecycle;
- backup and restore;
- restart and persistence.

Use synthetic data and isolated test profile only.

### E. Scope and governance

Identify missing requirements, undocumented behavior, scope expansion, deferred features, contradictory ADRs, acceptance criteria without tests, and implementation without product authorization.

## Required outputs

- `R2-EXEC-06-requirements-traceability-matrix.md`
- `R2-EXEC-06-e0-e7-functionality-report.md`
- `R2-EXEC-06-end-to-end-workflow-report.md`
- `R2-EXEC-06-functional-gap-list.md`

## Acceptance criteria

Every in-scope requirement has an explicit state. IMPLEMENTED has source evidence; INTEGRATED has end-to-end wiring evidence; VERIFIED has execution/runtime evidence; ACCEPTED has an identifiable acceptance record. Gaps and deferred scope are explicit.

## Stop conditions

Stop a workflow if it would send real messages, publish content, invoke real external side effects, or access user data. Replace with a fake/test provider only if the test boundary is explicit; otherwise mark BLOCKED.
