# E3 — Engineering Evidence Report (Work Packages E3-01 … E3-05)

**Branch:** feat/e3-reliability
**Status:** Work packages E3-01–E3-04 implemented with automated evidence; E3-05 review recorded here. E3 is NOT complete until the windowed Gate E run and milestone acceptance are recorded as explicit human steps. With E3 closed, **Alpha 0.1's engineering stages (E1–E3 per ADR-021) are code-complete** pending the acceptance runs.
**Implementation Status:** Implemented (automated evidence; windowed run pending)
**Verification Status:** Partially Verified — see the gate table

## 1. Work package evidence

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E3-01 | Operation Tier 2: retry classification, due-claim scheduling, cancellation semantics | Rejected → RetryScheduled with capped exponential backoff (next_attempt_at) → due-claim rerun; Accepted stays Running (no resubmit); Indeterminate → OutcomeUnknown blocks until reconciliation; cancellation request stamped / unsupported typed refusal / terminal refusal; list_operations workspace-scoped (NULL workspace never leaks) | Done |
| E3-02 | Outbox re-drive + retention/export/purge | Re-drive preserves event id, increments redrive_count, records operator (migration 0007 columns); export carries operations + attempts (storage-class-faithful JSON, numeric timestamps); purge boundaries (91-day terminal purged, unreconciled OutcomeUnknown never auto-expires, quarantined messages outside purge scope) | Done |
| E3-03 | Audit retention policy; workspace-deletion closure (migration 0007) | audit_events rebuilt with workspace_id ON DELETE SET NULL — deletion succeeds with audit history and rows survive detached as system-scope evidence; outbox re-drive columns live; `purge_expired_audit_events` under the configurable audit_days policy | Done |
| E3-04 | Desktop bridge + operation queue page | 8 new commands (list/cancel/resume/quarantine/redrive/export/purge) gated via the desktop-side check helper and audited allowed/denied; CommandError gains operation_not_found; UI 操作队列 section with status rows, next-attempt display, cancel/reconcile/redrive/export/purge buttons; integration test through the Api layer | Done |
| E3-05 | Release-readiness review | This report; README status alignment; Gate E runbook §5 | Review recorded |

Command-level evidence baseline at review time: 72 tests passing with `--no-fail-fast`, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --check` clean, `pnpm build` clean (Windows, toolchain 1.98.1).

## 2. Policy decisions recorded (E3 scope)

1. **Audit retention vs workspace deletion (owner decision: FK SET NULL).** Migration 0007 rebuilds audit_events with `workspace_id ON DELETE SET NULL`. Deleting a workspace detaches its audit rows into system scope as workspace-less evidence — deletion is no longer blocked by audit history, and the evidence chain is preserved rather than destroyed. The rows fall under the configurable `audit_days` policy (default 365) with an explicit purge command.
2. **Retention baseline (E0.3.2 §7 encoded).** Terminal operations expire 90 days after `finished_at`; reconciled `OutcomeUnknown` operations expire 90 days after their resolution update; **unreconciled OutcomeUnknown never expires automatically**; quarantined messages are outside purge scope entirely (re-drive or explicit resolution first); export is user-initiated; there is no background purge — all purges are explicit gated commands returning counts.
3. **Cancellation semantics (ADR-019 encoded).** Capabilities are declared at creation (`supported`/`unsupported`/`unknown`); requests against `unsupported` are explicit typed errors; a stamped request is not proof of remote stop; the actual `Cancelled` transition always goes through the state machine.
4. **Queue visibility.** Workspace-scoped reads expose the queue; system-scope rows (NULL workspace) never appear in workspace queries.

## 3. Gate status

| Gate | Item | Status |
|---|---|---|
| A — Architecture | New services stay in paw-infrastructure; desktop coordination connection justified (rusqlite joins the desktop allowlist, enforced); ports/edges unchanged | Pass (automated) |
| B — Persistence | Migration 0007 (audit rebuild + outbox columns) checksum-verified; fixtures updated; FK semantics tested | Pass (automated) |
| C — Security | All queue mutations gated + audited; denial audited; system-scope rows never leak into workspace queries; export carries no secrets by schema construction | Pass (automated) |
| D — Reliability | **Tier 2 complete**: classification, backoff, due-claim, cancellation, re-drive, retention — the two-tier strategy from the E1 delivery plan is now fully implemented | Pass |
| E — Product | Windowed run: queue page, cancellation, reconcile, re-drive, export, purge | **Pending — manual run (§5)** |
| F — Evidence | CI on candidate push; traceability notes; human acceptance record outstanding | Partial |

## 4. Requirement/ADR traceability notes

- PAW-INF-003 (ADR-019, E1/E3): **closed at the design level** — classification, backoff, due scheduling, cancellation and retention now implemented; provider-side reconciliation (real external outcomes) remains an E5 concern by nature (no providers exist in Alpha 0.1).
- PAW-INF-004 (ADR-020): re-drive is an explicit audited action preserving the event id (spec §4 rule implemented).
- PAW-INF-001 (ADR-017): the queue commands extend the named-operation surface; no new cross-domain orchestration was required.
- E0.3.2 §7: retention/export baseline implemented as specified; retention is configurable, no automatic purge.
- ADR-021: Alpha 0.1 engineering stages E1–E3 code-complete; Windows-only evidence maintained.

## 5. Gate E runbook (manual, owner)

1. `pnpm tauri dev` → window opens; prior data restored.
2. 操作队列 section: verify the list shows operations (may be empty); create activity through the normal UI to see operations/audit grow.
3. Export: 导出操作历史 → notice shows the temp JSON path → open the file and verify records + attempts.
4. 清除过期数据 → notice shows purge counts; verify nothing recent was removed (90-day boundary).
5. Quarantined events only appear when a consumer fails — if any exist, verify 重驱 increments the count and records the operator.
6. Cancellation/reconcile buttons appear contextually (running / outcome_unknown rows).
7. Offline + restart: same as E2 runbook items 6–7 (offline remains the owner's deferred item from E1/E2 if still open).
8. Record OS/toolchain/commit + pass-fail per item as the human acceptance record (Gate F).

### 5.1 Windowed run record (2026-11-10, Windows, toolchain 1.98.1, branch feat/e3-reliability)

Executed via `pnpm exec tauri dev`. One startup defect was found and fixed during the run:
migration checksum line-ending sensitivity (git autocrlf rewrote working-tree file endings
between checkouts; raw-bytes hashing failed closed) — fixed with line-ending-invariant
checksums plus legacy ledger reconciliation (`574a9cd`, E2 report §5.1 records the same fix
from its side of the run). The E2-era windowed run had already exercised the Work Management
UI end to end; this run focused on the E3 additions.

| Runbook item | Result | Notes |
|---|---|---|
| 1. Window opens; prior data restored | **Pass** | Startup clean after `574a9cd`; ledger reconciled, migrations 0004–0007 applied |
| 2. Queue view visible; workspace-scoped | **Pass** | 操作队列 section rendered; initially empty — by design: the queue lists durable operation envelopes, and no Alpha 0.1 flow opened one yet |
| 3–4. Export / purge envelopes | **Pass** after `c210adb` | The owner reported an empty queue with workspace content present; export/purge now run under their own durable envelopes (`data.export` workspace-scoped / `data.purge` system-scope, Succeeded/Failed transitions + attempt rows), making the queue verifiable; automated test asserts the succeeded envelope after export |
| 5. Quarantined events + re-drive | **Verified at test level** | No quarantined messages existed in the windowed session (no failing consumers); re-drive semantics covered by the integration suite |
| 6. Cancellation/reconcile buttons | **Verified at test level** | Contextual buttons land with running/outcome_unknown rows; semantics covered by the integration suite |
| 7. Offline + restart | **Deferred** | Owner's standing deferred item (E1/E2/E3); to be closed together at the Alpha 0.1 release gate |
| 8. Acceptance record | This section | Items 1–4 pass (3–4 with the envelope fix); 5–6 test-level; 7 deferred; owner accepted and directed the milestone merge |

**Merge decision:** the owner accepted the run and directed the E2 and E3 milestone merges
(E2 pushed as `9eec0af` after a proxy outage; E3 merged with this record). Outstanding
follow-ups carried forward: offline acceptance items (E1/E2/E3), quarantined-event re-drive
observed live when a consumer failure first occurs.

## 6. Explicitly deferred

Provider-side reconciliation and idempotency keys (E5 — Alpha 0.2 gate per ADR-022); interrupted-migration live drill and OFF-matrix manual cases; plugin packaging/distribution and attachment content-addressing decisions (P1 gates, evidence-driven); background purge (explicitly excluded by policy).
