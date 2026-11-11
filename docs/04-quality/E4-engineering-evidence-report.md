# E4 — Engineering Evidence Report (Work Packages E4-01 … E4-03)

**Branch:** feat/e4-learning-knowledge (merged into main; branch deleted)
**Status:** Work packages E4-01–E4-03 implemented with automated evidence and merged (merge commit `05e1470`). E4 is NOT complete until the windowed Gate E run and milestone acceptance are recorded as explicit human steps. With E4 code-complete, **Alpha 0.2 (E4 + E5 per ADR-021) has E4 delivered; E5 (Agent adapter 与受控授权) remains outstanding**.
**Implementation Status:** Implemented (automated evidence; windowed run pending)
**Verification Status:** Partially Verified — see the gate table

## 1. Work package evidence

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E4-01 | Learning lifecycle domain (Roadmap/Plan/Item/Session) | `paw_domain::learning` per PRD §3.1–3.4: PlanStatus free transitions, ItemStatus state machine mirroring Task, Sessions carry optional task/project association snapshots; migration `0008_learning_knowledge.sql` (learning_roadmaps/plans/items/sessions, all workspace-consistent composite keys; sessions RESTRICT item deletion — historical records never silently cascade) | Done |
| E4-02 | KnowledgeRef metadata + cross-domain contracts | `paw_domain::knowledge`: source lingshu/hindsight/web/local, external URI separate from opaque ids, availability available/stale/unavailable; ports + adapters with workspace predicate on every query; `set_availability` atomic with a versioned `KnowledgeReferenceAvailabilityChanged.v1` outbox event — the first real producer on the E3 transactional outbox (no-op when unchanged); integration tests: lifecycle CRUD + scope denial, cross-workspace link refusal via composite FK, sessions never overwrite, availability → exactly one versioned event with minimal payload, source deletion never touches learning data, 0007→0008 upgrade fixture | Done |
| E4-03 | Learning/knowledge bridge + dashboard extension + Work Management UI | 18 new `#[tauri::command]`s: roadmap/plan/item/session lifecycle, item↔project linking, `CreateTaskForLearningItem`, knowledge CRUD + availability; Api methods run on the five new adapters with gate_and_audit (denial audited + Allowed recorded; post-gate constraint failures surface as typed command errors); CommandError gains learning_record_not_found / knowledge_ref_not_found; paw-application re-exports contract types for the desktop boundary (desktop still never touches paw-domain directly); dashboard renders roadmap/plan/item/session counts as distinct real-data rows (PRD §4.1 — plan progress and session activity never merged into a fabricated score); UI 学习 section (hierarchy, kind select, status advance, project link, 生成任务, append-only session records) and 知识引用 section (CRUD + availability cycle) | Done |

E4-03 重放注记：commands.rs 曾因 awk 循环内重写文件而截断损毁，`git checkout` 恢复到提交版后全部重放并验证。教训已记录：大文件禁止在 awk/循环管道中做就地重写，一律用 Edit 工具。

### 1.1 Pre-merge follow-up commits (on the E4 branch)

1. **ADR-019 refactor:** the Operation state machine moved to a pure `paw_domain::operation_machine` module (storage-agnostic `OperationStatus`, `allowed_transitions`/`is_terminal`/`is_valid_transition` + unit tests). `paw-infrastructure::operations` persists transitions and delegates validation to the domain machine; the storage layer no longer owns state-machine policy, and the SQLite CHECK values always agree with the published transition rules. The public `paw_infrastructure::operations::OperationStatus` path is preserved via re-export. During reconciliation a leaked working-tree edit had removed the `paw_domain::learning` module export and part of the machine delegation — both were completed/repaired on the branch (the learning export restored; machine delegation finished with the domain free functions) before merge.
2. **Windows bundling:** `bundle.active=true` with targets nsis/msi, `icons/icon.ico`, wix language en-US. Fields rejected by the pinned tauri-build (`bundle.update`, `windows.language`) were dropped so the build script passes — schema-valid Tauri 2 config only.

## 2. Command-level evidence baseline at merge time

`cargo check/test --workspace`: **79 tests passing** with `--no-fail-fast`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; `pnpm build` clean (tsc + vite). All on the merged `main` tip (Windows, toolchain 1.98.1).

## 3. Gate status

| Gate | Item | Status |
|---|---|---|
| A — Architecture | Domain crate remains stdlib + serde/uuid (allowlist test enforces); the operation machine now lives in paw-domain per ADR-017/018/019 (pure machine, side effects in the adapter); desktop still reaches learning/knowledge contracts only through paw-application re-exports | Pass (automated) |
| B — Persistence | Migration 0008 checksum-verified; workspace-consistent composite keys; sessions RESTRICT item deletion; 0007→0008 upgrade fixture tested | Pass (automated) |
| C — Security | All 18 new commands gated via gate_and_audit (denial audited, Allowed recorded); workspace predicate on every repository query; cross-workspace links refused at the FK layer | Pass (automated) |
| D — Contracts | `KnowledgeReferenceAvailabilityChanged.v1` — the first versioned cross-domain event produced on the E3 transactional outbox (ADR-020); exactly-once per change, minimal payload | Pass (automated) |
| E — Product | Windowed run: 学习 hierarchy + status advance + 生成任务, 知识引用 CRUD + availability, dashboard learning rows | **Pending — manual run (§5)** |
| F — Evidence | This report; README status alignment; human acceptance record outstanding | Partial |

## 4. Requirement/ADR traceability notes

- PRD §3.1–3.4 / [Learning & Growth PRD](../01-product/learning-growth-PRD.md): learning lifecycle entities and UI implemented; assessment/skill evidence remains Beta (E6.x) scope.
- ADR-013/ADR-014 (learning domain, project integration): item↔project linking + `CreateTaskForLearningItem` realize the learning→work bridge; the reverse direction (项目提出学习缺口) is Beta scope.
- ADR-016 (learning-knowledge boundary): knowledge stays metadata-only (`KnowledgeRef`); source deletion never touches learning data (tested).
- ADR-020 (cross-domain contract versioning): first real versioned event producer on the transactional outbox.
- ADR-019: machine interaction made literal — transition rules now live in the pure domain module; storage delegates.
- ADR-021: Alpha 0.2 = E4 + E5. **E4 code-complete; E5 (Agent adapter 与受控授权) outstanding**; Gate Alpha 0.2 closes only when E5 delivers and the gate passes.

## 5. Gate E runbook (manual, owner)

1. `pnpm tauri dev` → window opens; prior data restored (migrations through 0008 applied).
2. 学习 section: create a roadmap → plan → item; advance item status; link the item to a project; 生成任务 (CreateTaskForLearningItem) and verify the task appears in Work Management.
3. Record an append-only learning session; verify it cannot be overwritten and blocks item deletion (typed refusal).
4. 知识引用 section: CRUD a reference; cycle availability available → stale → unavailable; verify each change persists across restart.
5. Dashboard: verify roadmap/plan/item/session counts render as distinct rows with real data (no fabricated aggregate).
6. Offline + restart: same as E2/E3 runbook items (offline remains the owner's standing deferred item).
7. Record OS/toolchain/commit + pass-fail per item as the human acceptance record (Gate F).

### 5.1 Windowed run record

_Pending — owner to record._
