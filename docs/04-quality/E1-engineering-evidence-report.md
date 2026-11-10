# E1 — Engineering Evidence Report (Work Packages E1-01 … E1-10)

**Branch:** feat/e1-engineering
**Status:** Work packages E1-01–E1-09 implemented with automated evidence; E1-10 review recorded here. E1 is NOT complete: the windowed Gate E run, the full offline matrix cases and milestone acceptance are explicit human steps.
**Implementation Status:** Implemented (automated evidence; runtime windowed run pending)
**Verification Status:** Partially Verified — see the gate table; manual evidence outstanding

## 1. Work package evidence

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E1-01 | Pinned toolchain, Cargo workspace, frontend scaffold, CI matrix | Windows clean build; `cargo fmt/clippy -D warnings/test`; `pnpm build`; CI workflow (windows-latest Rust, frontend pre-build for tauri-build) | Done |
| E1-02 | Four-crate boundaries, typed IDs/errors, architecture enforcement | Allowlist + forbidden-edge + Tauri-only-in-desktop checks (`architecture_checks.rs`, `dep_graph.rs`); paw-domain external deps limited to serde/uuid | Done |
| E1-03 | SQLite foundation: connection policy, migration runner, schema ledger | FK/WAL/busy-timeout/synchronous=FULL test; checksum ledger fail-closed test (outdated-version conflict); migration count tracks files | Done |
| E1-04 | Workspace isolation | Cross-workspace denial test (typed rejections, cascade delete, temporal isolation); scoping enforced in service + repository predicates + composite constraints | Done |
| E1-05 | Work Management vertical slice (headless) | UI-to-SQLite integration tests: three-reopen restart persistence; missing-workspace/validation typed denials through the use-case layer | Done |
| E1-06 | Backup/restore integrity | Round-trip restores data after active-db destruction; corrupt backup rejected untouched; foreign snapshot (no metadata) rejected; restore re-migrates idempotently; snapshot carries schema/app metadata | Done |
| E1-07 | Durable operations foundation (Tier 1) | E0.3.2 §2 state machine enforced (terminal freeze); OutcomeUnknown blocks unsafe retry until `resume_after_reconciliation`; restart recovery marks Running → Failed (`process_restarted`) while RetryScheduled survives; monotonic attempts | Done |
| E1-08 | Outbox and dedup (Tier 1) | Intent commits atomically with source state (commit + rollback cases); duplicate delivery harmless; expired-lease redelivery after simulated crash; poison message quarantines after bounded retries; no-consumer events quarantine immediately | Done |
| E1-09 | Security and audit gates | Deny-by-default authorization rechecked per side effect (denied → audited, nothing written); allowed/error outcomes audited with opaque ids only; leaky-string scan over real audit rows; `CommandError` never carries driver text (unit-tested); migration 0003 audit_events | Done |
| E1-10 | Release-readiness review | This report; traceability notes below; rollback procedure §4; Gate E runbook §5 | Review recorded |

Command-level evidence baseline at review time: 27 tests passing, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --check` clean, `pnpm build` clean (Windows, toolchain 1.98.1).

## 2. Gate status

| Gate | Item | Status |
|---|---|---|
| A — Architecture | Dependency graph matches ADR-018; no forbidden imports; public contracts versioned (outbox schema_version) | Pass (automated) |
| B — Persistence | Deterministic transactional checksum-verified migrations; FK on every connection; upgrade/interrupted/backup-restore tests | Pass (automated; interrupted-migration live drill = manual) |
| C — Security | Workspace isolation in service+predicates+constraints; authorization recheck per side effect; logs/events/errors redacted | Pass (automated) |
| D — Reliability (Tier 1) | Atomic outbox intent; harmless duplicates; OutcomeUnknown blocks retry; cancellation is a design assertion only (Tier 2, E3+) | Pass (Tier 1 scope) |
| E — Product | Windows native app launches; user creates Workspace/Project; restart restores; offline works | **Pending — manual windowed run (§5)** |
| F — Evidence | CI green on candidate commit; test report; traceability; human acceptance record | Partial — CI runs on push; human acceptance record outstanding |

## 3. Requirement/ADR traceability notes

- PAW-INF-002 (ADR-018, E1): implemented — allowlist/forbidden-edge/Tauri-isolation checks are automated tests, not conventions.
- PAW-INF-003 (ADR-019, E1/E3): E1 Tier 1 implemented (envelope, attempts, restart recovery, OutcomeUnknown block); retry classification/backoff, cancellation, quarantine UX and provider reconciliation remain E3/E5.
- PAW-INF-004 (ADR-020, E3/E4): partial — outbox events carry schema_version with quarantine rules; full contract-versioning fixtures land with the first real cross-domain event consumers (E4).
- PAW-INF-001 (ADR-017, E2/E3): coordination primitives exist (Operation/Outbox services in paw-infrastructure, invoked from application use cases); named cross-domain use-case orchestration expands in E2.
- ADR-021: Windows-only delivery honored — all evidence from Windows; no macOS claims.

## 4. Rollback procedure

- Code: milestone merges to main are `--no-ff` merge commits; `git revert -m 1 <merge-sha>` reverts the whole milestone while preserving history. Per-work-package revert uses the individual `feat(e1):` commits (E1-01…E1-09 boundaries).
- Schema: migrations are forward-only with checksum ledger (no down-migrations by policy). Rollback of schema = restore the pre-migration backup via `backup::restore_backup` (validates integrity + metadata before replacing) or ship a compensating forward migration.
- Data safety: every restore path re-runs `integrity_check`/`foreign_key_check` and refuses files without `paw_backup_metadata`.

## 5. Gate E runbook (manual, owner)

1. `pnpm tauri dev` (or `cargo run -p paw-desktop`) on Windows → main window opens.
2. Create Workspace → create Projects → verify list; close the app.
3. Relaunch → confirm Workspace/Projects restored from `%APPDATA%/com.bryantianjx.personal-ai-workbench/app.sqlite`.
4. Disconnect network → repeat create/list → reconnect → confirm no false remote success.
5. Record OS/toolchain/commit + pass-fail per item as the human acceptance record (Gate F).

### 5.1 Windowed run record (2026-11-10, Windows, toolchain 1.98.1, branch feat/e1-engineering)

Executed via `pnpm exec tauri dev`; all four crates compiled; the main window opened and the
per-user database initialized (ADR-006 path, WAL mode, migrations 0001–0003 applied).

| Runbook item | Result | Evidence |
|---|---|---|
| 1. Native window launches | **Pass** | dev console: `Running paw-desktop.exe`; window visible |
| 2. Create Workspace / Projects | **Pass** | DB after first session: 1 workspace, `audit_events` = 1× `workspace.create` (allowed) |
| 3. Restart restores data | **Pass** | After relaunch: 2 workspaces + 2 projects (`test`, `test3`, status `active`) readable; 4 audit rows (2× workspace.create, 2× project.create, all allowed) |
| 4. Offline check | **Deferred** | Recorded as a follow-up item by the owner |
| 5. Acceptance record | This section | Partial: items 1–3 pass, item 4 deferred |

**Observed gap (recorded, non-blocking):** on relaunch the UI starts with no workspace
selected — the E1 slice publishes no `list_workspaces` command, so the operator created a
second workspace in the second session. Data-level restore is proven by database inspection
(both workspaces and both projects present and bound correctly); the UI-level
"select the persisted workspace" flow is an E2 backlog item (workspace picker / session
restore). Acceptance criterion "Restart restores data" is met at the data layer and through
`list_projects` once a workspace is selected.

## 6. Explicitly deferred (Tier 2 / later gates)

Retry classification and backoff policy (E3), quarantine recovery UX (E3+), cooperative cancellation semantics (E3+), provider idempotency keys and OutcomeUnknown reconciliation against real providers (E5, Alpha 0.2 gate per ADR-022), operation retention/export policy (E3+), installer packaging/bundle (release gate), interrupted-migration live drill and OFF-matrix manual cases (Gate E/F).
