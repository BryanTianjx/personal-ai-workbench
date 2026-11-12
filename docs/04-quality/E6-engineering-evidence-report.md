# E6 — Engineering Evidence Report (Beta 0.5 第一批：技能/评估/学习缺口/Creator)

**Branch:** feat/e6-growth-creator (merged into main; branch deleted)
**Status:** Work packages E6-01–E6-04 implemented with automated evidence per the roadmap v0.5.1 Beta 0.5 scope (learning-growth-PRD §3.5–3.8, PAW-LG-007/008/009, ADR-015) and merged. The windowed Gate E run and milestone acceptance remain explicit human steps. **Beta 0.5 后续增量**（项目学习产物回流复盘深化、Creator 发布辅助）与 E7（平台组合，V1.0 前置）另行推进。
**Implementation Status:** Implemented (automated evidence; windowed run pending)
**Verification Status:** Partially Verified — see the gate table

## 1. Work package evidence

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E6-01 | Skill / SkillEvidence / Assessment domain + persistence (PAW-LG-008/009, ADR-015) | `paw_domain::skills`: Skill with versioned assessment scale (bump starts a new series, history keeps per-assessment snapshots — never reinterpreted), SkillEvidence pointing at sources by opaque id without copying content (8 kinds × 5 source kinds), Assessment as user-confirmed evidence-linked fact (level 0..=5, link table enforces ≥1 evidence, append-only — no update/delete path); migration `0010`; integration tests incl. scale-bump history snapshots and cross-workspace invisibility | Done |
| E6-02 | Project learning-gap proposal (PAW-LG-007, PRD §3.8/§5.3) | `LearningGapProposal` (Learning-owned; project referenced by id without ownership transfer, ADR-014): Proposed → Confirmed(binds item once) \| Dismissed, both terminal; `insert/update` emit versioned outbox events atomically — `LearningGapProposed.v1` / `LearningGapConfirmed.v1` / `LearningGapDismissed.v1` with minimal payloads (ADR-020 §4); integration test asserts exactly one event per transition and the confirmed-item binding in the payload | Done |
| E6-03 | Creator content pipeline (选题/草稿/版本/审核) | `paw_domain::creator`: Topic (Idea/Selected/Parked), Draft (Writing/InReview/Approved) with immutable version snapshots and reviews bound to exact version numbers; migration `0011`; `append_version` bumps the draft head atomically; publishing automation out of scope until official API verification | Done |
| E6-04 | Desktop bridge + UI + e2e | 19 gated+audited commands; CommandError gains skill_not_found/creator_not_found/creator_state_invalid/item_not_found; local_policy extends to the human-only E6 mutations; UI 技能评估/学习缺口/内容创作 sections (gap confirm binds an item selected from the existing learning list); e2e flow: skill+evidence+assessment (incl. no-evidence refusal), gap propose→confirm with exactly-one versioned event each and terminal re-confirmation refusal, creator topic→draft→v1→review→approved, cross-workspace invisibility | Done |

## 2. Policy decisions recorded (E6 scope)

1. **Assessment discipline (ADR-015 encoded):** levels are user-confirmed facts — the schema link table enforces ≥1 evidence per assessment, the API validates each evidence exists in-workspace and belongs to the same skill, and there is no update/delete path (corrections are new assessments). Learning completion never raises levels.
2. **Scale versioning:** each assessment snapshots `scale_version` at creation; bumping a skill's scale starts a new assessment series while history keeps its snapshots.
3. **Gap confirmation:** confirming binds an existing learning item (item creation remains a separate user decision through the learning commands); dismissal keeps the proposal as decision history.
4. **Creator scope:** 选题/草稿/版本/审核 only — no publish tables until 微信公众号 official API verification (feature-matrix note 4).

## 3. Command-level evidence baseline at merge time

`cargo check/test --workspace`: **154 tests passing** with `--no-fail-fast`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; `pnpm build` clean (tsc + vite). Windows, toolchain 1.98.1. Migration ledger fixtures updated 9→11 for migrations 0010/0011.

## 4. Gate status

| Gate | Item | Status |
|---|---|---|
| A — Architecture | Four-crate boundaries unchanged (dep-graph green); skills/gap/creator repos behind ports; no cross-domain table writes (gap proposals reference projects by composite FK, RESTRICT) | Pass (automated) |
| B — Persistence | Migrations 0010/0011 checksum-verified; workspace-consistent composite keys; evidence link table enforces the ADR-015 invariant at the schema level; 0009→0011 upgrade covered by ledger fixtures | Pass (automated) |
| C — Security | All new mutations gated + audited (denial audited); assessment evidence validated in-workspace; cross-workspace invisibility asserted | Pass (automated) |
| D — Reliability | Gap confirm/dismiss emit exactly one versioned event per transition, atomically with state (outbox); assessment inserts transactional | Pass (automated) |
| E — Product | Windowed run: 技能评估/学习缺口/内容创作 sections end to end | **Pending — manual run (§6)** |
| F — Evidence | This report; README status alignment; human acceptance record outstanding | Partial |

## 5. Requirement/ADR traceability notes

- PAW-LG-008 (Skill/SkillEvidence provenance) → E6-01; PAW-LG-009 (user-confirmed assessment history + scale versioning) → E6-01; PAW-LG-007 (project gap proposal, user confirmation, idempotent event consumption) → E6-02.
- ADR-013/014/015/016: Learning stays a first-class domain owning skills/evidence/assessments; project references stay contractual (ADR-014); knowledge untouched.
- ADR-020: three new versioned outbox event types produced by the Learning domain.
- PRD §6: AI-generated suggestions remain suggestions — gap proposals enter as `Proposed` and never auto-confirm; assessment levels are never inferred.
- ADR-027: Creator is composed of its own domain module — no catch-all Core absorption.

## 6. Gate E runbook (manual, owner)

1. `pnpm tauri dev` → migrations through 0011 applied.
2. 技能评估: add a skill; 录入证据与评估 via the command surface; verify the matrix shows scale version and assessments persist across restart.
3. 学习缺口: pick a project, 提出缺口 → 确认（绑定一个学习条目）→ verify the gap shows 已绑定 and the 操作队列/审计记录 the versioned events; 搁置 a second gap and verify history.
4. 内容创作: 添加选题 → 推进状态 → 建草稿 → 保存新版本（head bump）→ 提交审核 → 通过/驳回（绑定精确版本）→ verify review history order.
5. Offline + restart: skills/assessments/gaps/creator records persist.
6. Record OS/toolchain/commit + pass-fail per item (Gate F).

### 6.1 Windowed run record

_Pending — owner to record._

## 7. Second batch (Beta 0.5 第二批, same stage branch family)

Work packages E6-05/06 implemented on feat/e6b-beta05-second and merged:

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E6-05 | Creator publish records (发布辅助) | `CreatorPublishRecord` bound to exact draft versions; DraftStatus gains terminal `Published` (0011 CHECK extended); `record_publish` validates the state-machine transition BEFORE inserting — a republish of a Published draft never inserts a second record; publishing automation stays gated (feature-matrix note 4); e2e asserts terminal republish refusal + manual-fact-only discipline | Done |
| E6-06 | Project retrospectives with learning backflow (PAW-LG-007 closure, PRD §3.8/§5.3) | `ProjectRetrospective` (Work Management domain, append-only); backflow is REAL — every referenced learning session must belong to an item linked to the same project (RESTRICT FKs + application validation through the item→project chain); `ProjectRetrospectiveRecorded.v1` emitted atomically with ids-only payload (ADR-020); migration 0012; e2e asserts unlinked-session refusal, exactly-one event, cross-workspace invisibility | Done |

Second-batch baseline: **156 tests** passing --no-fail-fast, clippy/fmt/pnpm clean. Migration ledger fixtures 11→12.
