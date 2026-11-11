# Hermes Task Package — PAW ADR-023～029 R1

**Package status:** Ready for controlled review; not authorized for implementation.

## Objective

让 Hermes 对 ADR-023～029 R1 文档进行只读核对、跨 ADR 一致性复核、缺陷登记和审批材料准备。只有人工逐份批准 ADR 并单独授权实施规格阶段后，才能开展下一阶段工作。

## Inputs

- `docs/03-adr/ADR-023-domain-plugin-model.md`
- `docs/03-adr/ADR-024-role-profile-model.md`
- `docs/03-adr/ADR-025-workspace-composition.md`
- `docs/03-adr/ADR-026-capability-registry-and-ai-invocation.md`
- `docs/03-adr/ADR-027-first-party-domain-plugin-policy.md`
- `docs/03-adr/ADR-028-plugin-dependency-compatibility-governance.md`
- `docs/03-adr/ADR-029-agent-invocation-security-and-approval.md`
- `docs/04-quality/ADR-023-029-R1-cross-adr-review.md`
- Existing ADRs 001～006 and 013～022; PRD; roadmap; domain architecture; cross-domain contracts; module boundaries; Agent Adapter Architecture; Agent Authorization Model; Operation reliability specifications.

## Execution order

1. T0 — Establish repository/branch/read-only baseline and enumerate authoritative inputs.
2. T1 — Review ADR-023～025: plugin, profile, composition boundaries.
3. T2 — Review ADR-026～027: capability governance, first-party domain adoption and Learning boundary.
4. T3 — Review ADR-028～029: dependency compatibility and Agent invocation security.
5. T4 — Cross-ADR consistency, threat/risk review, contradiction and traceability matrix.
6. T5 — Produce review findings, proposed text-only corrections and human approval packet.
7. H0 — Stop for explicit human approval. No implementation work starts automatically.

## Required outputs

- Read-only audit log and exact source refs/commit SHA.
- Findings register with severity, affected ADR/section, evidence, consequence and proposed resolution.
- Updated cross-ADR matrix (only if authorized to edit docs).
- Seven-item owner decision sheet with Proposed / Accepted / Rejected / Revise choices.
- A separate implementation-specification backlog, explicitly marked not authorized.

## Hard stop

The current package is a documentation and review artifact. Hermes MUST NOT edit source code, add Rust types, create migrations, change SQLite schema, alter runtime behavior, change product scope, or mark any ADR Accepted. Any document edit beyond a review report requires explicit owner authorization. No commit, push, merge, PR, tag, or release is authorized by this task package.

See `execution-contract.md`, `task-breakdown.md`, `acceptance-matrix.md`, and `owner-review-checklist.md`.
