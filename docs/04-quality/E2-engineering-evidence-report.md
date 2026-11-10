# E2 — Engineering Evidence Report (Work Packages E2-01 … E2-06)

**Branch:** feat/e2-work-management
**Status:** Work packages E2-01–E2-05 implemented with automated evidence; E2-06 review recorded here. E2 is NOT complete until the windowed Gate E run and milestone acceptance are recorded as explicit human steps.
**Implementation Status:** Implemented (automated evidence; windowed run pending)
**Verification Status:** Partially Verified — see the gate table

## 1. Work package evidence

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E2-01 | Areas/Goals/Tasks data layer; project links; migration 0004 | CRUD + scope-denial + cross-workspace link refusal at composite-FK level; RESTRICT protections (project with tasks, area with goals); 0003→0004 upgrade fixture (projects rebuilt, data preserved); cascade deletion order; TaskStatus domain state machine | Done |
| E2-02 | Use cases + E2 authorization/audit + workspace picker | Per-entity use-case units with fakes (denial audited, nothing written); full-flow desktop integration test incl. restart persistence; `list_workspaces` picker port + UI selection persistence; typed ProjectNotFound | Done |
| E2-03 | Scoped FTS5 search (migration 0005) | Index follows CRUD in-step (same transaction); cross-workspace invisibility; rename/delete propagation; rebuild-from-corruption; hostile-query and empty-query typed rejections; CJK character segmentation on index and query sides | Done |
| E2-04 | Attachments (migration 0006) | Managed store round-trip/traversal/oversize/scope units; real-DB integration: sha256 metadata, cross-workspace denial, owner verification incl. foreign-workspace ids, deletion guards for project/task/workspace, guarded-then-clean order; compensating byte removal on metadata failure | Done |
| E2-05 | Dashboard read model + UI | Real-count aggregation with zero-filled canonical status rows asserted in the restart flow; UI builds (tsc + vite) with picker/dashboard/CRUD/transitions/search/attachments | Done |
| E2-06 | Release-readiness review | This report; README status alignment; Gate E runbook §5 | Review recorded |

Command-level evidence baseline at review time: 62 tests passing with `--no-fail-fast`, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --check` clean, `pnpm build` clean (Windows, toolchain 1.98.1).

## 2. Verification-process errata (recorded per E2-01/E2-02)

1. **E1 outbox blind spot (from E2-01):** the E1 operations_outbox tests never executed — a compile failure in that target silently truncated the earlier `grep "test result:" | awk` summaries. Rewritten with a deterministic far-future dispatcher clock (enqueue stamps real-epoch `available_at`; the old `now=100` claim condition was unsatisfiable) and connection drops before Windows directory cleanup. The E1-07/08 evidence claim in the E1 report therefore carried an unexercised suite; the suite now runs green as part of the standard workspace run.
2. **Summary formula bug (from E2-02):** the aggregation used awk `$6`, which under `-F'[ ;]'` is the empty field created by the `"; "` double separator — **failure counts were always zero, on every milestone summary so far**. Corrected to `$7` and all verification runs now use `cargo test --no-fail-fast`. The correction surfaced a real rusqlite defect (list_goals/list_tasks bound an unused parameter when optional filters were absent) that the masked runs had hidden.

## 3. Gate status

| Gate | Item | Status |
|---|---|---|
| A — Architecture | Dependency graph matches ADR-018 (new adapters stay inside paw-infrastructure; serde/uuid joins paw-application's published-contract allowlist by decision); no forbidden imports | Pass (automated) |
| B — Persistence | Migrations 0004 (table rebuild) / 0005 (FTS5) / 0006 (attachments) checksum-verified; upgrade fixtures from 0003 preserve data; FK on every connection | Pass (automated; interrupted-migration live drill remains manual) |
| C — Security | Workspace scoping on every new read/write path incl. search and attachments; authorization recheck per side effect; audit for allowed/denied/error; filename and path-component validation | Pass (automated) |
| D — Reliability | Unchanged from E1 (Tier 1 Outbox/Operation); attachment compensating removal follows the no-orphan rule | Pass (Tier 1 scope) |
| E — Product | Windowed run: picker, CRUD, transitions, dashboard, search, attachments offline | **Pending — manual run (§5)** |
| F — Evidence | CI green on candidate push; traceability notes below; human acceptance record outstanding | Partial |

## 4. Requirement/ADR traceability notes

- PAW-F-001～008 (Alpha 0.1 P0): Workspace/Project/Task management and Goal/Area model implemented; real-data Dashboard implemented (PAW-F-004); scoped search (PAW-F-005) and attachment metadata + controlled access (PAW-F-006) implemented; backup/restore and audit were closed in E1 and are regression-covered here.
- PAW-INF-001 (ADR-017, E2/E3): the Work Management surface is now a named, authorized, audited use-case family; cross-domain orchestration beyond Work Management remains E3+.
- PAW-INF-004 (ADR-020): outbox events unchanged; attachment metadata records follow the no-silent-cascade lifecycle rule.
- ADR-021: Windows-only delivery honored; all evidence from Windows.
- ADR-022: untouched (Proposed; Alpha 0.2 gate).

## 5. Gate E runbook (manual, owner)

1. `pnpm tauri dev` → window opens; the workspace picker lists persisted workspaces (selection persists across relaunches).
2. Create/enter a workspace → create Area, Goal (with area), Project (link both), Tasks (linked and standalone) → verify the Dashboard counts update with real data.
3. Move a task Todo → In progress → Done; verify the rework path and the terminal freeze (Done offers no transitions).
4. Search a project/task by title fragment → verify hits are workspace-scoped.
5. Attach a file to a task → open (download) → remove; try deleting the task with an attachment → typed refusal; remove the attachment → deletion succeeds.
6. Disconnect the network → repeat core operations offline → reconnect, no false success.
7. Close and relaunch → everything restored (records, statuses, links, attachments metadata).
8. Record OS/toolchain/commit + pass-fail per item as the human acceptance record (Gate F).

## 6. Explicitly deferred

Project/task editing beyond rename-through-update commands surfaced in the bridge but not yet wired into dedicated UI edit forms (commands exist; E3 UI increments); interrupted-migration live drill and OFF-matrix manual cases (Gate E/F); retry/cancellation/quarantine UX and provider reconciliation (E3/E5 per the two-tier strategy); audit retention policy (E3).
