# Alpha 0.1 Release Closure — Hermes Execution Package

- **Package ID:** PAW-REL-CLOSURE-ALPHA-0.1
- **Baseline:** `main` at `f558c8c2bdee90b572f308324e5ea9ef29486001` (2026-11-11 audit snapshot)
- **Audit baseline:** [Engineering Audit Baseline v1.0](../../04-quality/engineering-audit-baseline-v1.0.md)
- **Execution mode:** staged; read-only first, then narrowly authorized implementation/fixes, then owner-run acceptance
- **Release decision at package creation:** NOT RELEASE-READY
- **Repository policy:** no automatic commit, push, merge, tag, publish or release

## 1. Mission

Close the outstanding Windows Alpha 0.1 release gates for the already implemented E1–E3 scope. Do not start E4, add new product domains, widen the platform target, or implement Alpha 0.2 provider reconciliation as part of this package.

The goal is to produce reproducible evidence that the exact candidate build can be installed/launched, preserve data, operate offline for the supported core workflows, recover safely from migration/backup scenarios, and pass owner acceptance.

## 2. Global Execution Contract

Hermes MUST:

1. Begin with a read-only repository and environment inspection. Record current branch, HEAD, working-tree status, remotes and toolchain before any change.
2. Use the current main-derived work branch; never base work on stale `feat/e1-engineering`, `feat/e2-work-management` or `feat/e3-reliability` branches.
3. Re-read the current versions of this package, the audit baseline, README, ADR-021/022 and E1/E2/E3 evidence reports before execution.
4. Verify every claim against source, tests or reproducible command output. Clearly label historical report evidence versus newly reproduced evidence.
5. Run tests with `cargo test --no-fail-fast`; preserve the real process exit code and full output. Do not use a pipeline/summary that can mask a failing test process.
6. Never delete or mutate the user's real database or attachments. Use isolated temporary directories and disposable test copies for destructive drills.
7. Never weaken authorization, workspace scoping, migration checksums, audit behavior, retry safety or data-retention rules to make a gate pass.
8. Keep changes minimal and scoped to Alpha 0.1 closure. Any architecture or product-scope change requires a separate ADR/owner decision.
9. Do not commit, push, merge, tag, publish or release automatically. Stop before each such operation and request explicit owner authorization.
10. Report blockers honestly. A test-level simulation is not equivalent to live desktop acceptance.

## 3. Work Package Dependency Graph

```text
REL-00 Read-only preflight
  ├── REL-01 Evidence and gate reconciliation
  ├── REL-02 Offline acceptance matrix
  ├── REL-03 Migration interruption and recovery drill
  ├── REL-04 Backup / restore end-to-end drill
  └── REL-05 Windows packaging / install / upgrade verification
         └── REL-06 Live reliability scenarios
                └── REL-07 Final release evidence and owner decision
```

REL-00 and REL-01 are prerequisites. REL-02 through REL-06 may be prepared in parallel only where their test environments are isolated. REL-07 depends on all required evidence.

## 4. Work Packages

### PAW-REL-00 — Read-only Preflight

**Priority:** P0 · **Mode:** Read-only · **No source modifications**

Tasks:

- Confirm repository, current main HEAD and clean/dirty status.
- Confirm Rust, Cargo, Node, pnpm, Tauri CLI, WebView2 and Windows versions.
- Inspect CI workflow definitions and latest checks for the target commit.
- Inspect scripts and documented commands for dev, test, bundle, install and upgrade.
- Identify the actual app data directory and attachment store location from source; do not assume paths from documentation without verification.
- Confirm whether an installer/bundle target exists and which formats are generated.
- Produce a command/evidence inventory and unresolved questions.

Acceptance:

- Environment and commit recorded.
- No files or user data modified.
- All later package steps have exact reproducible commands and safe test locations.

### PAW-REL-01 — Evidence and Gate Reconciliation

**Priority:** P0 · **Mode:** Read-only, then documentation-only proposal

Tasks:

- Reconcile README milestone status against E1/E2/E3 reports and current source.
- Confirm all deferred items and avoid marking test-level-only evidence as live acceptance.
- Check ADR-021 Windows-only scope and ADR-022 Alpha 0.2 boundary.
- Map each Alpha 0.1 P0 requirement and release gate to implementation, automated test, manual evidence and owner.
- Identify stale counts, branch references, commit references and contradictory status statements.

Acceptance:

- Traceability matrix has requirement/gate, source path, test name/command, evidence location, status and owner.
- Contradictions are documented; no unsupported “complete” claim remains.
- Any proposed documentation changes are presented for review before writing.

### PAW-REL-02 — Windows Offline Acceptance Matrix

**Priority:** P0 · **Mode:** Test preparation + owner-operated manual test

Test on a supported Windows environment using a disposable workspace and test data:

1. Launch the app while online.
2. Create/select a Workspace.
3. Create and update Area, Goal, Project and Task; exercise allowed status transitions.
4. Search known project/task terms, including Chinese text.
5. Add, open and remove an attachment; verify metadata and managed file consistency.
6. Disconnect network adapters or use a controlled network block.
7. Repeat all supported core operations offline.
8. Verify no operation reports remote success when no remote provider is available.
9. Restart while still offline; verify records, links, statuses and attachment metadata persist.
10. Reconnect and verify no duplicate/lost local records and no unintended queued side effects.

Record:

- Windows edition/build, app commit, toolchain, network isolation method.
- Exact steps, expected/actual results, screenshots or logs where useful.
- Database and attachment consistency checks on a copy.
- Any feature intentionally requiring network and its explicit degraded-state UX.

Acceptance:

- All supported core local workflows pass offline.
- No false success, data loss, cross-workspace leakage or unexpected remote dependency.
- Any exception is explicitly scoped and owner-approved.
- Owner records the manual result; Hermes must not claim owner acceptance on the owner's behalf.

### PAW-REL-03 — Interrupted Migration and Recovery Drill

**Priority:** P1 · **Mode:** Controlled test on disposable copies only

Tasks:

- Inspect migration runner, transaction boundaries, checksum ledger and startup recovery behavior.
- Determine safe fault-injection points from source; do not improvise against the real app database.
- Create a disposable database fixture from the previous schema version with representative records and audit rows.
- Simulate interruption/failure at supported migration boundaries in an isolated test process.
- Restart and verify deterministic recovery or fail-closed behavior.
- Verify schema version, checksum ledger, SQLite integrity and foreign-key integrity.
- Verify no partial destructive migration or silent ledger rewrite.

Acceptance:

- Each tested failure point has expected behavior and captured evidence.
- `PRAGMA integrity_check` and `PRAGMA foreign_key_check` pass after recovery where recovery is expected.
- Failure is explicit and non-destructive where automatic recovery is not supported.
- Test does not touch user production data.

### PAW-REL-04 — Backup / Restore End-to-End Drill

**Priority:** P1 · **Mode:** Controlled test on disposable copies

Tasks:

- Inspect backup format, metadata, restore replacement semantics and user-facing commands/UI.
- Create a disposable test workspace with linked projects/tasks, audit rows and attachments.
- Produce a backup through the supported application path.
- Validate backup metadata and integrity.
- Restore into an isolated target after simulating loss/corruption of the disposable target database.
- Verify records, relationships, schema migrations, audit history and attachment behavior.
- Test invalid/corrupt/foreign backup rejection without overwriting the target.
- Document whether attachment bytes are included, separately backed up or intentionally excluded; do not assume.

Acceptance:

- Successful restore reproduces all promised data.
- Invalid backup leaves target untouched.
- Integrity and foreign-key checks pass.
- Attachment guarantees are explicit and tested.
- No production database or attachment store is modified.

### PAW-REL-05 — Windows Packaging, Clean Install and Upgrade

**Priority:** P1 · **Mode:** Implementation only if required, then manual verification

Tasks:

- Identify configured Tauri bundle targets, installer format, signing assumptions and release scripts.
- Build from the exact candidate commit with pinned toolchain.
- Verify artifact provenance, version metadata and checksums.
- Install on a clean Windows test profile/VM where feasible.
- Launch and exercise first-run initialization.
- Upgrade from the previous test build with existing disposable user data.
- Verify app data is preserved and migrations run correctly.
- Verify uninstall behavior and explicitly document whether user data is retained.
- Test rollback/recovery strategy; do not claim installer rollback if only database restore is available.

Acceptance:

- Reproducible installer artifacts and checksums.
- Clean install launches.
- Upgrade preserves data and opens successfully.
- Uninstall/data-retention behavior is explicit.
- No secrets or development-only configuration are packaged.
- Any signing or distribution limitation is documented rather than hidden.

### PAW-REL-06 — Live Reliability Scenarios

**Priority:** P1 · **Mode:** Controlled test environment

Tasks:

- Create a controlled failing consumer / quarantined message scenario.
- Verify retry limits, quarantine transition and audit evidence.
- Re-drive the quarantined message and verify original event ID, incremented redrive count and operator attribution.
- Exercise cancellation against supported, unsupported and unknown capability declarations.
- Verify a cancellation request is not mistaken for proof of remote cancellation.
- Exercise OutcomeUnknown and verify unsafe automatic retry is blocked until reconciliation.
- Verify export/purge operations create their own durable operation envelopes and attempts.
- Verify retention boundaries without deleting current or unresolved records.

Acceptance:

- Runtime behavior matches ADR-019/020 and E3 policy.
- All side effects are authorized and audited.
- No unsafe duplicate execution or silent evidence deletion.
- Live evidence is distinguished from unit/integration test evidence.

### PAW-REL-07 — Final Release Evidence and Owner Gate

**Priority:** P0 · **Mode:** Read-only synthesis; owner decision required

Tasks:

- Re-run full verification on the exact candidate commit.
- Capture CI run identifiers and statuses.
- Summarize REL-02 through REL-06 evidence.
- Update E1/E2/E3 and README status only where facts justify it.
- Produce `Alpha-0.1-release-evidence-report.md`.
- Produce a final blocker list and explicit release recommendation.

Acceptance:

- P0 gates all pass.
- P1 items are closed or have explicit owner-approved risk acceptance.
- Exact candidate commit and artifact hashes are recorded.
- No unresolved critical data integrity, authorization, workspace isolation or unsafe retry issue.
- Release decision is marked one of: `GO`, `NO-GO`, or `GO-WITH-ACCEPTED-RISKS`.
- Only the owner can approve release and authorize tag/publish/deployment.

## 5. Required Evidence Format

Every work package report must include:

| Field | Required content |
|---|---|
| Work package | ID and title |
| Commit | Exact tested SHA |
| Environment | OS/build, toolchain and relevant runtime |
| Procedure | Exact commands and manual steps |
| Expected | Acceptance criteria |
| Actual | Observed result |
| Evidence | Test output, logs, screenshots, artifact hashes |
| Status | PASS / FAIL / BLOCKED / DEFERRED / NOT RUN |
| Data safety | Test database/path and confirmation no production data touched |
| Follow-up | Defect ID, owner and next action |

Never use “PASS” when the step was not run. Use “TEST-LEVEL VERIFIED” when only automated tests cover the behavior. Use “OWNER ACCEPTED” only after explicit owner confirmation.

## 6. Stop Conditions

Stop and report immediately if:

- The current branch is not the expected main-derived task branch or contains unexpected user changes.
- A test might modify the real user database or attachment store.
- Migration recovery could destroy or rewrite non-disposable data.
- A security boundary, workspace filter, audit record or retry invariant appears broken.
- Required credentials, signing keys or external services are unavailable.
- A requested fix requires an architecture decision or scope expansion.
- A commit, push, merge, tag, publish or release would be the next action.

## 7. Final Deliverables

1. `alpha-0.1-release-preflight.md`
2. `alpha-0.1-requirement-gate-traceability.md`
3. `alpha-0.1-offline-acceptance-report.md`
4. `alpha-0.1-migration-recovery-drill.md`
5. `alpha-0.1-backup-restore-report.md`
6. `alpha-0.1-windows-install-upgrade-report.md`
7. `alpha-0.1-reliability-live-scenarios.md`
8. `alpha-0.1-release-evidence-report.md`

Create each report only when the corresponding work has actually been performed. Do not create empty “passed” reports in advance.

## 8. Suggested Hermes Task Board

Create these cards with dependency links:

- `PAW-REL-00` Read-only preflight — Ready
- `PAW-REL-01` Evidence/gate reconciliation — Blocked by REL-00
- `PAW-REL-02` Offline acceptance — Blocked by REL-00/01; owner-operated
- `PAW-REL-03` Migration recovery drill — Blocked by REL-00/01
- `PAW-REL-04` Backup/restore drill — Blocked by REL-00/01
- `PAW-REL-05` Windows packaging/install/upgrade — Blocked by REL-00/01
- `PAW-REL-06` Live reliability scenarios — Blocked by REL-00/01
- `PAW-REL-07` Final evidence and owner gate — Blocked by REL-02…06

**Execution order:** REL-00 → REL-01 → REL-02…06 → REL-07.

**Do not start E4 until Alpha 0.1 release closure is decided or the owner explicitly approves parallel development.**
