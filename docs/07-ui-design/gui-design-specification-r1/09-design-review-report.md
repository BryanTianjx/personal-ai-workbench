# 09 — GUI Design R1 Review Report

**Review type:** Documentation and architecture consistency review  
**Status:** Conditional pass; prototype and contract review outstanding  
**Review baseline:** repository main inspected before this branch was created

## 1. Scope reviewed

- Alpha 0.1 release/platform boundary.
- Existing frontend command usage in src/App.tsx.
- Domain/Agent adapter and authorization boundaries.
- Offline acceptance baseline.
- Proposed navigation, shell, component and IPC design.

This review did not run the application, inspect every Rust handler/DTO, or execute visual tests. Findings must not be interpreted as runtime verification.

## 2. Findings

| ID | Priority | Finding | Required disposition |
|---|---|---|---|
| DR-001 | P1 | App.tsx is a large mixed-responsibility entry (approximately 1,221 lines observed) | Migrate incrementally into feature modules; no big-bang rewrite |
| DR-002 | P1 | Current UI surfaces include Learning/Knowledge/Agent-related commands beyond Alpha 0.1 core scope | Add release availability gate; verify feature matrix and remove/defer non-baseline routes |
| DR-003 | P1 | Dashboard and aggregate surfaces may include future-domain metrics | Confirm Dashboard DTO and Alpha 0.1 read model; hide unapproved metrics |
| DR-004 | P1 | IPC calls are distributed through the current UI | Centralize typed wrappers and normalized error mapping |
| DR-005 | P1 | Common interaction states need a shared specification | Apply state contract to all core pages |
| DR-006 | P1 | Archive/delete/purge semantics must be checked against Rust lifecycle contracts | Do not relabel operations; verify handlers and domain transitions |
| DR-007 | P1 | ADR-022 is Proposed and Agent authorization is not a UI-only concern | Keep executable AI/Agent interactions disabled until ADR and backend gate are accepted |
| DR-008 | P2 | Window and compact desktop behavior needs real viewport testing | Validate at 1440×900, 1280×720 and compact width |
| DR-009 | P2 | Keyboard/focus/accessibility rules are not yet evidenced by tests | Add keyboard and accessibility acceptance cases |

## 3. Decisions

1. Alpha 0.1 remains Windows delivery; macOS is not claimed as accepted.
2. Work Management is the primary navigation scope.
3. Learning & Growth, Agent execution and workflow authoring remain deferred unless separately ratified.
4. React must use IPC; it must not access SQLite directly.
5. UI feature flags are not authorization.
6. AI preview is not execution; approval click does not widen grants.
7. Prototypes are isolated mock-only artifacts.

## 4. Review outcome

**Conditional pass for design package preparation.** The visual direction, page composition and target frontend boundaries are suitable for prototype validation. This is not a final GUI baseline freeze.

## 5. Exit criteria before implementation

- Four prototypes reviewed at target desktop viewports.
- DR-002, DR-006 and DR-007 closed or explicitly accepted by product/architecture owner.
- Rust command signatures, DTOs, errors and workspace checks verified.
- Dashboard read-model scope verified.
- Design tokens and component behavior ratified.
- Acceptance tests mapped to approved requirements.

## 6. Deferred work

No Hermes implementation task package is authorized by this review. Generate it only after the GUI baseline is approved. The eventual task package must contain file scope, dependency order, acceptance matrix, tests, rollback and human approval gates.
