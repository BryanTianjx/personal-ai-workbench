# R2-EXEC-03 — Frontend & GUI Implementation Audit

- Parent: [Engineering Audit R2.0](./README.md)
- Dependency: R2-EXEC-01 baseline frozen
- Priority focus: GUI Design Specification R1 conformance

## Objective

Determine the actual production GUI implementation status and compare it with GUI Design Specification R1. The accepted starting fact is that R1 has **not yet been implemented as the production GUI**. Verify the extent of existing feature-oriented GUI code without overstating it as R1 completion.

## Scope

Frontend entry points, React components, routing/navigation, styling/tokens, state management, Tauri IPC, package scripts, prototypes, GUI R1 specifications, frontend tests, runtime screenshots.

## Audit tasks

### A. Production GUI inventory

1. Identify production entry point and root component.
2. Map pages, navigation, routes, shared components, state containers, and IPC services.
3. Record component size and responsibility concentration, including large monolithic components.
4. Identify mock data, fake services, hardcoded data, placeholders, and incomplete flows.
5. Map each screen to actual backend commands/use cases.
6. Identify loading, empty, error, disabled, permission-denied, and retry states.

### B. R1 conformance matrix

For every requirement in the R1 design package, record:
- requirement/document and section;
- production implementation path and symbol;
- prototype path if applicable;
- state: DESIGNED / SPECIFIED / IMPLEMENTED / INTEGRATED / VERIFIED / ACCEPTED / NOT_STARTED;
- evidence and gap.

Assess at minimum:
1. Application Shell.
2. Navigation and routing.
3. Design System and design tokens.
4. Four core pages and their information architecture.
5. Component architecture.
6. IPC integration contract.
7. Interaction and loading/error states.
8. Accessibility, keyboard behavior, and focus.
9. Responsive behavior and window resizing.
10. Visual consistency and typography/spacing.

A prototype is not production implementation. A static screenshot is not interaction evidence.

### C. Runtime GUI verification

Use a dedicated test profile and synthetic data only:
1. Launch the desktop application.
2. Verify startup, navigation, primary workflows, and window resizing.
3. Exercise loading, empty, error, and permission-denied states where possible.
4. Verify IPC failures are surfaced and recoverable.
5. Capture screenshots with route/page and environment metadata.
6. Record console, WebView, and Rust logs.

### D. Workspace switching race — mandatory

Investigate candidate `R2-SEC-UI-001`:
1. Start multiple asynchronous data loads for workspace A.
2. Delay selected A responses using a safe test seam or controlled test harness; do not modify production source.
3. Switch to workspace B before A requests finish.
4. Allow B requests to complete, then A's older responses.
5. Observe whether A data appears while B is selected.
6. Repeat across all relevant domains and record request/response ordering, workspace IDs, visible UI state, and reproducibility.

If no safe delay seam exists, do not patch production code; use existing tests or report NOT_TESTED with the reason. Distinguish stale frontend display from backend authorization bypass.

### E. Frontend quality

Run only configured, non-mutating checks discovered from package manifests, such as typecheck, lint check, test, and production build. Record absent frontend test infrastructure as NOT_CONFIGURED, not PASS.

## Required outputs

- `R2-EXEC-03-gui-implementation-report.md`
- `R2-EXEC-03-r1-conformance-matrix.md`
- `R2-EXEC-03-frontend-quality-report.md`
- `R2-EXEC-03-workspace-switching-test.md`
- `R2-EXEC-03-screenshots/`

## Acceptance criteria

Production code, prototype, design spec, integration, and acceptance are reported separately. Every R1 requirement is mapped. Workspace-switching behavior is reproduced or explicitly unverified. Screenshots and runtime logs are tied to the audited SHA and isolated profile.

## Stop conditions

Stop if GUI launch would use a real user profile/database, if a test requires source modification, or if the test may affect existing user data. Report BLOCKED.
