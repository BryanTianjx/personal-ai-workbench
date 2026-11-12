# E7 — Engineering Evidence Report (Platform Composition: Plugins, Profiles, Compositions)

**Branch:** feat/e7-platform-composition (merged into main; branch deleted)
**Status:** Work packages E7-01–E7-04 implemented with automated evidence per the [E7 implementation specification](../05-architecture-evolution/E7-implementation-specification.md) (governance ADR-023/024/025/027/028 Accepted 2026-11-12) and merged. E7 is NOT complete until the windowed Gate E run and milestone acceptance are recorded as explicit human steps. **With E7 delivered, the V1.0 平台化前置 (E7) is code-complete**; the windowed Gate V1.0 run (E7 + 收口) and the release gate (install/upgrade/recovery/security checks) remain explicit human steps.
**Implementation Status:** Implemented (automated evidence; windowed run pending)
**Verification Status:** Partially Verified — see the gate table

## 1. Work package evidence

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E7-01 | Plugin descriptor model + static registry + dependency DAG | `paw_domain::platform` (RoleProfile/WorkspaceComposition — no grant/credential/approval fields by construction; malformed references rejected) + `ports::plugin_registry` (static first-party catalog `core.work_management`/`core.learning_growth`/`core.knowledge_context`/`core.ai_collaboration`/`core.system_security`, four independent lifecycle dimensions, deterministic topological order with stable tie-break by id, exact-cycle rejection reporting the path, fail-closed availability derivation with structured reasons incl. transitive DownstreamOfBlocked) | Done |
| E7-02 | Plugin lifecycle persistence + disable semantics | migration `0013` (plugin_states with composite-unique (plugin_id, workspace_id), role_profiles, workspace_compositions) + SQLite repos with fresh reads per check; disable gate integrated into `propose_agent_invocation` — disabled plugins refuse NEW invocations with audited typed denial (`PluginDisabled`) while data, audit history and in-flight Operations stay intact (ADR-019 drain/reconcile; e2e asserts prior execution records survive untouched) | Done |
| E7-03 | RoleProfile experience template | No grant/credential/approval fields exist by construction (type-level test); malformed/unknown references rejected; import-strip discipline documented; profile creation/listing gated+audited | Done |
| E7-04 | WorkspaceComposition + bridge + UI + e2e | Composition scoped to actor + one Data Workspace (composite FKs, cross-workspace invisibility); 8 gated+audited commands; UI 平台组合 section (plugin toggles, profile→composition flow); e2e: catalog completeness (5 plugins), disable→typed refusal→re-enable, grant-independent authorization, history preservation | Done |

## 2. Policy decisions recorded (E7 scope)

1. **Disable as preference:** the persisted per-workspace state covers only the composition/availability preference dimension — registration is static code, and the disable never touches authorization grants (agent grants remain valid data; only new invocations are gated).
2. **Static adoption:** five first-party plugins registered incrementally over existing use cases — no big-bang rewrite, no fifth crate, no dynamic loading (ADR-027 §R1; ownership invariants asserted in the catalog builder comments).
3. **Composition fallback:** malformed/unknown references in profiles or compositions surface typed errors at construction; the UI never widens access through a composition (authorization is evaluated at each command as before).
4. **Domain data untouched:** disabling blocks new invocations only — existing Operations reconcile per ADR-019 and all domain records stay queryable (e2e asserts).

## 3. Command-level evidence baseline at merge time

`cargo check/test --workspace`: **167 tests passing** with `--no-fail-fast`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; `pnpm build` clean (tsc + vite). Windows, toolchain 1.98.1. Migration ledger fixtures 12→13 for migration 0013.

## 4. Gate status

| Gate | Item | Status |
|---|---|---|
| A — Architecture | Four-crate boundaries unchanged (dep-graph green); registry metadata-only (no callables); E5 governance pipeline untouched; no fifth crate, no dynamic loading | Pass (automated) |
| B — Persistence | Migration 0013 checksum-verified; composite-unique (id, workspace_id) keys; disable preference fresh-read per capability resolution | Pass (automated) |
| C — Security | Profile/composition carry no authority (type-level + e2e); disable gate audited; cross-workspace isolation asserted; registry/Core never write domain tables (ADR-027) | Pass (automated) |
| D — Reliability | Disable preserves data/audit/Operations (drain semantics via the E5 pipeline); fail-closed availability with structured reasons; deterministic cycle rejection | Pass (automated) |
| E — Product | Windowed run: 平台组合 section end to end (toggles, profiles, compositions) | **Pending — manual run (§6)** |
| F — Evidence | This report; README status alignment; human acceptance record outstanding | Partial |

## 5. Requirement/ADR traceability notes

- PAW-INF-007 (ADR-023/027/028): static registration, dependency DAG, four lifecycle dimensions, fail-closed compatibility, Learning boundary retained (ADR-027; catalog comments assert ownership invariants), no Alpha/Beta scope expansion.
- PAW-INF-008 (ADR-024): RoleProfile = experience template only; switching never changes grants.
- PAW-INF-009 (ADR-025): WorkspaceComposition scoped to one Data Workspace; backend authorization at use time unchanged.
- ADR-019: disable semantics preserve Operations; reconciliation untouched.
- ADR-021/roadmap v0.5.1: E7 = V1.0 前置; Gate V1.0 (E7 + 收口) is the remaining release gate.

## 6. Gate E runbook (manual, owner)

1. `pnpm tauri dev` → migrations through 0013 applied.
2. 平台组合 section: verify five plugins listed as 可用; toggle one 停用 and back 启用.
3. Agent 协作: with core.ai_collaboration disabled, propose an invocation of `data.export` → typed refusal (plugin disabled, audited); re-enable and verify it runs.
4. 角色档案: create a profile → 生成组合 → verify the composition lists the profile reference and plugin count.
5. Offline + restart: plugin states/profiles/compositions persist.
6. Record OS/toolchain/commit + pass-fail per item (Gate F).

### 6.1 Windowed run record (2026-11-13, Windows, toolchain 1.98.1, main @ 1b74b64→7217b30)

Executed via `pnpm tauri dev`. 平台组合 end to end:

| Runbook item | Result | Evidence |
|---|---|---|
| 2 插件开关 | **Pass** | five core.* plugins listed 可用; disable/enable toggles exercised ×10 (all audited allowed) |
| 3 停用门禁联动 | **Pass** | core.ai_collaboration disabled → data.export proposal refused (typed PluginDisabled, audited denial); re-enabled → invocation runs |
| 4 角色档案→组合 | **Pass** | profile rtrtrytty created; composition generated 基于角色档案; compositions scoped to actor + workspace |
| 5 跨工作区隔离 | **Pass** | additional workspaces created (defect #1 — creation form hidden after first run — fixed `9a9d4fa`); plugin states/profiles/compositions scoped per workspace |
| Persistence (G19) | **Pass** | plugin states/profiles/compositions persist across restarts (live-DB probe) |

**Defect found and fixed during the run:** #1 workspace creation form (`9a9d4fa`, recorded in the E4 report).
