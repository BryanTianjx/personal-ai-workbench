# Owner Review and Approval Checklist

## Before reviewing

- [ ] Confirm repository and branch are the intended review target.
- [ ] Confirm the source HEAD SHA and no unexpected concurrent changes.
- [ ] Confirm all seven ADRs are still Proposed.
- [ ] Read the cross-ADR review and all Critical/High findings.

## Decision record — complete independently for each ADR

| ADR | Decision | Conditions / required revisions | Approver | Date |
|---|---|---|---|---|
| ADR-023 | Proposed / Accepted / Rejected / Revise |  |  |  |
| ADR-024 | Proposed / Accepted / Rejected / Revise |  |  |  |
| ADR-025 | Proposed / Accepted / Rejected / Revise |  |  |  |
| ADR-026 | Proposed / Accepted / Rejected / Revise |  |  |  |
| ADR-027 | Proposed / Accepted / Rejected / Revise |  |  |  |
| ADR-028 | Proposed / Accepted / Rejected / Revise |  |  |  |
| ADR-029 | Proposed / Accepted / Rejected / Revise |  |  |  |

## Required invariant confirmation

- [ ] Domain owns business entities, invariants, persistence and events.
- [ ] Plugin is a static registration/composition unit, not a replacement domain.
- [ ] Plugin registration, availability, composition preference and UI visibility remain separate.
- [ ] Role Profile and WorkspaceComposition do not grant authority.
- [ ] Data Workspace remains the backend-enforced data/security boundary.
- [ ] Capability Registry is metadata-only; invocation is mediated.
- [ ] Learning & Growth remains a first-class domain; no implicit Alpha scope expansion.
- [ ] Dependency/version/migration governance is fail-closed and domain-owned.
- [ ] Agent output is untrusted; approval binds exact plan/target/scope/expiry.
- [ ] Authorization is rechecked before each side effect; OutcomeUnknown is reconciled.

## Gate decisions

- [ ] ADR decision gate complete.
- [ ] Any required R1 revision has been made and re-reviewed.
- [ ] Implementation specification gate explicitly authorized (separate from ADR acceptance).
- [ ] Implementation work gate explicitly authorized (separate from specification approval).
- [ ] Git commit/push/PR/merge/release permissions separately specified.

**Default:** blank decisions mean no approval. Silence, task completion, or automated checks do not constitute acceptance.
