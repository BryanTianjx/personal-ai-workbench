# E5 — Engineering Evidence Report (Work Packages E5-01 … E5-06)

**Branch:** feat/e5-agent-governance (merged into main; branch deleted)
**Status:** Work packages E5-01–E5-06 implemented with automated evidence per the [E5 implementation specification](../05-architecture-evolution/E5-implementation-specification.md) (Finalized R1, ADR-026/029 Accepted 2026-11-12) and merged. E5 is NOT complete until the windowed Gate E run and milestone acceptance are recorded as explicit human steps. **With E5 delivered, Alpha 0.2 (E4 + E5) is code-complete**; Gate Alpha 0.2 also requires ADR-022 ratification before external Agent execution is enabled.
**Implementation Status:** Implemented (automated evidence; windowed run pending)
**Verification Status:** Partially Verified — see the gate table

## 1. Work package evidence

| WP | Deliverable | Automated evidence | Status |
|---|---|---|---|
| E5-01 | Capability descriptor model + metadata-only registry | `ports::capability`: 8 static first-party descriptors over the gated Api surface (`work_management.task.*`, `learning.item.create`, `knowledge.ref.*`, `data.export`, `data.purge` High→approval); exact-version resolution, duplicate rejection, unknown capability / unsupported version fail closed, High-risk-without-approval rejected; registry never returns a callable (ADR-026 R1) | Done |
| E5-02 | Authorization v2 + canonical plan digest | Grant model with scope intersection (`AgentGrant::covers`: subject ∧ capability ∧ workspace ∧ time window); `grant_allows` shared policy helper + `GrantEvaluatedAuthorizer` (fresh reads per check) + `GovernedAuthorizer` (local policy for the human subject, grants for `agent:<id>` subjects); `CanonicalPlan` deterministic JSON (BTreeMap-sorted keys, compact UTF-8, `canonicalization_version` embedded) + SHA-256 digest; mutation-sensitivity tests over every material field | Done |
| E5-03 | Agent registration + execution lifecycle + fake adapter | `paw_domain::ai_collaboration`: registration state machine (Active/Suspended/Retired, Retired terminal) with invoke guard; execution lifecycle per adapter-architecture §3 with ADR-019 reconciliation semantics (OutcomeUnknown blocks rerun into Running); `FakeAgentAdapter` with scripted Accepted/Rejected/Indeterminate/Transport outcomes; migration `0009_agent_governance` (4 tables, workspace-bound FKs, RESTRICT everywhere, state-string CHECKs mirroring the domain machines, open-approval unique index per digest) | Done |
| E5-04 | Approval binding | Approvals bind canonical digest + actor + agent + workspace + capability/contract version + tool + target + material-params digest; time-derived expiry; single/bounded attempt policy consumed at the side effect; propose-time record (binding reconstructed from persisted execution fields only — never caller input); deny revokes; revoke human-only | Done |
| E5-05 | Invocation pipeline + per-side-effect recheck + reconciliation | Application Coordination pipeline (spec §8): validate → resolve → authorize (default deny) → risk/approval classify → canonicalize → open durable Operation → **re-authorize immediately before each side effect** (pre-submit + pre-poll) → mediated adapter submit → attempt trail (started/accepted/rejected/indeterminate/failed per ADR-019 §3) → OutcomeUnknown holds without blind retry → owner `reconcile` as the only exit; 10 pipeline conformance tests (in-memory port fakes) covering spec §11 items 1–8 | Done |
| E5-06 | Desktop bridge + UI + e2e | 12 gated+audited commands; CommandError gains typed non-leaky variants; Agent 协作 UI (registration with revocable default grant, suspend/retire, proposal form, approval surface with capability/target/risk/expiry/side-effects, execution list with reconcile buttons); e2e flow test through the Api: grant-gated deny → low-risk run → high-risk two-phase approval → cross-workspace invisibility → grant revocation re-denies; suspended-registration + terminal-Retired guards | Done |

## 2. Policy decisions recorded (E5 scope)

1. **Approval timing:** the approval record is created at *propose* time (binding reconstructed from the persisted execution; the human actor is the proposer) and looked up by canonical digest at *decide* time — the displayed plan, the decided plan and the executed plan are the same persisted object; consumption happens at the side effect.
2. **Agent identity and grants:** invocations run as the agent subject (`agent:<registration_id>`); grants are workspace-bound by construction (no wildcard), read fresh per authorization check, and admin actions (register/grant/approve) are never grantable to agents.
3. **E5 dispatch target:** the pipeline executes through the mediated adapter (fake transport in E5). Domain-use-case dispatch for real provider effects arrives with real transports (ADR-022 gate) — the governed path, attempt trail and reconciliation semantics are already the production ones.
4. **Correlation:** a fresh correlation id is generated at approval decision time; the parked execution is the causation object.

## 3. Command-level evidence baseline at merge time

`cargo check/test --workspace`: **120 tests passing** with `--no-fail-fast`; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean; `pnpm build` clean (tsc + vite). All on the branch tip (Windows, toolchain 1.98.1). Dependency-graph allowlist updated: `serde_json` joins the desktop shell allowlist (command-bridge serialization, same class as `serde`).

## 4. Gate status

| Gate | Item | Status |
|---|---|---|
| A — Architecture | Four-crate boundaries unchanged (dep-graph tests green); registry metadata-only; coordination owns the pipeline; desktop reaches AI Collaboration contracts via paw-application re-exports | Pass (automated) |
| B — Persistence | Migration 0009 checksum-verified; workspace-consistent FKs with RESTRICT (history never cascades); state-string CHECKs mirror the domain machines; 0008→0009 upgrade covered by the ledger fixtures | Pass (automated) |
| C — Security | Default deny; cross-workspace invisibility asserted through direct backend calls; every mutation gated + audited (denial audited); approvals bind exact digests; secret-free records (opaque refs + digests only) | Pass (automated) |
| D — Reliability | Per-side-effect recheck (revocation between effects blocks the next); OutcomeUnknown + TimedOut non-terminal until explicit reconciliation; non-idempotent uncertain outcomes never retried (state machine + no retry path) | Pass (automated) |
| E — Product | Windowed run: Agent 协作 section end to end (register → grant → propose → approve → executions → reconcile) | **Pending — manual run (§6)** |
| F — Evidence | This report; README status alignment; human acceptance record outstanding; **ADR-022 ratification required before external Agent execution** | Partial |

## 5. Requirement/ADR traceability notes

- PAW-INF-005 (ADR-026): capability registry metadata-only, dispatch mediated, five-layer ownership realized (§4/§8 of the spec).
- PAW-INF-006 (ADR-029): untrusted-input default deny, exact approval binding, per-side-effect recheck, OutcomeUnknown reconciliation, redacted audit (§5/§6/§8/§11).
- PAW-S-004 (agent authorization gate): default deny / least privilege / scope intersection / approval binding / audit — now implemented and tested (the Alpha 0.2 enablement gate).
- ADR-022: still Proposed — **external Agent execution stays disabled until it is ratified**; the fake transport exercises the governed path without external effects.
- E0.3 §4 E5 exit evidence: fake-provider conformance ✓ (scripted adapter suites), approval binding ✓ (two-phase + digest tests), unknown-outcome reconciliation ✓ (OutcomeUnknown/reconcile tests).

## 6. Gate E runbook (manual, owner)

1. `pnpm tauri dev` → window opens; migrations through 0009 applied.
2. Agent 协作 section: register an agent (auto-seeds a revocable default grant for `work_management.task.create`).
3. 发起受控调用 with the low-risk capability → expect immediate completion notice + Succeeded operation in 操作队列.
4. Propose `data.purge` (high-risk) → the approval surface shows capability/契约版本/目标/风险/有效期/副作用集; 批准执行 → runs; create a second proposal and 拒绝 → execution Cancelled.
5. Execution list: verify attempt-linked operations in the 操作队列 page; statuses Completed/Failed/Cancelled render; no unresolved rows exist unless a scripted provider fault is introduced.
6. Offline + restart: registrations/executions/approvals/grants persist; restart recovery of in-flight operations follows the E3 runbook behavior.
7. Record OS/toolchain/commit + pass-fail per item as the human acceptance record (Gate F).

### 6.1 Windowed run record

_Pending — owner to record._
