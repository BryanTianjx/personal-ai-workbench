# Acceptance Matrix

| ID | Requirement | Evidence / test | Pass condition | Gate |
|---|---|---|---|---|
| AM-001 | Source baseline captured | Branch, HEAD SHA, status, timestamp | Exact reproducible baseline recorded | T0 |
| AM-002 | ADR source set complete | Seven ADR paths and cross-review | All expected files found; gaps explicit | T0 |
| AM-003 | Existing decision constraints traced | ADR/PRD/architecture source map | Every material MUST has source evidence | T0 |
| AM-004 | Domain owns business facts | Ownership matrix and examples | No Plugin/Registry/Core ownership transfer | T1/T2 |
| AM-005 | Plugin lifecycle dimensions separate | State transition analysis | Registration, availability, preference, visibility are distinct | T1 |
| AM-006 | Static-first scope preserved | Dependency and loading review | No dynamic loading/marketplace implied | T1 |
| AM-007 | Profile is not authorization | Grant/secret/approval field audit | Profile cannot grant or widen authority | T1 |
| AM-008 | Composition is not Data Workspace | Scope and persistence review | Composition references exactly one Data Workspace; backend enforces access | T1 |
| AM-009 | Capability Registry metadata-only | Dispatch path review | No Service Locator or arbitrary callable resolution | T2 |
| AM-010 | Invocation governance centralized | Sequence/ownership review | Auth, approval, Operation and audit pass through governance/coordination | T2 |
| AM-011 | Learning boundary retained | ADR-013～016 traceability | Learning ownership/evidence/assessment and Alpha scope preserved | T2 |
| AM-012 | Dependency graph deterministic | DAG/cycle/optional dependency analysis | Required failures block; optional fallback explicit; cycles rejected | T3 |
| AM-013 | Version dimensions independent | Version table review | Plugin/descriptor/capability/app/data versions not conflated | T3 |
| AM-014 | Migration ownership preserved | Migration responsibility map | Domain owns own data migration; no cross-domain table writes | T3 |
| AM-015 | Approval exact-binding | Threat scenario and contract review | Digest, actor, workspace, scope, target, capability/tool, expiry and attempt bound | T3 |
| AM-016 | Per-side-effect authorization | Invocation sequence review | Policy rechecked before each protected effect | T3 |
| AM-017 | OutcomeUnknown safe | ADR-019 reconciliation trace | No blind retry for uncertain non-idempotent action | T3 |
| AM-018 | Cross-ADR no contradiction | Matrix and finding register | No unresolved Critical/High contradiction | T4 |
| AM-019 | Human approval recorded | Owner decision sheet | Seven ADRs have explicit decisions, approver and date | H0 |
| AM-020 | Implementation remains gated | Change log / git diff | No source/schema/config/runtime changes | All |

## Severity exit criteria

- **Critical:** security bypass, data loss, domain ownership violation, or irreversible side-effect ambiguity. Must be resolved before approval.
- **High:** material architectural contradiction or missing safety invariant. Must be resolved or explicitly accepted by owner with mitigation before approval.
- **Medium:** incomplete specification or testability gap. May be deferred only with owner, milestone and acceptance criteria.
- **Low:** editorial or non-blocking clarity issue. May be handled in a later documentation pass.

No automated result can replace H0 human approval.
