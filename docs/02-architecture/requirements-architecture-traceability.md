# Requirements–Architecture Traceability

**Baseline:** E0.2 + Learning & Growth extension
**Status:** Accepted design traceability; implementation evidence pending

| Requirement ID | Capability | Owning domain | Primary design | ADR | Target |
|---|---|---|---|---|---|
| PAW-LG-001 | Learning Roadmap lifecycle | Learning & Growth | learning-growth-PRD.md | ADR-013 | Alpha 0.2 |
| PAW-LG-002 | Learning Plan and Roadmap relationship | Learning & Growth | learning-growth-PRD.md | ADR-013 | Alpha 0.2 |
| PAW-LG-003 | Learning Item lifecycle | Learning & Growth | learning-growth-PRD.md | ADR-013 | Alpha 0.2 |
| PAW-LG-004 | Learning Session records | Learning & Growth | learning-growth-PRD.md | ADR-013 | Alpha 0.2 |
| PAW-LG-005 | Learning and Project association | Learning / Work Management | cross-domain-data-contracts.md | ADR-014 | Alpha 0.2 |
| PAW-LG-006 | Learning Item requests Task creation | Learning / Work Management | cross-domain-data-contracts.md | ADR-014 | Alpha 0.2 |
| PAW-LG-007 | Project learning-gap proposal | Work Management / Learning | cross-domain-data-contracts.md | ADR-014 | Beta 0.5 |
| PAW-LG-008 | Skill and SkillEvidence provenance | Learning & Growth | learning-growth-PRD.md | ADR-015 | Beta 0.5 |
| PAW-LG-009 | User-confirmed LearningAssessment history | Learning & Growth | learning-growth-PRD.md | ADR-015 | Beta 0.5 |
| PAW-LG-010 | Explainable learning and project growth reports | Learning & Growth | learning-growth-PRD.md | ADR-013, ADR-015 | V1.0 |
| PAW-S-004 | Agent authorization, approval, and audit gate | AI Collaboration / System | agent-authorization-model.md | ADR-004 | Alpha 0.2 gate |
| PAW-INF-001 | Cross-domain coordination boundary (named use cases, no private repositories, re-authorization per side effect) | Cross-cutting (paw-application) | application-architecture.md; system-architecture.md | ADR-017 | E2/E3 |
| PAW-INF-002 | Rust workspace dependency direction and enforced forbidden edges | Cross-cutting (workspace) | module-boundary-rules.md; E1-rust-workspace-layout.md | ADR-018 | E1 |
| PAW-INF-003 | Durable Operation/OperationAttempt persistence, restart recovery, unsafe-retry blocking | Cross-cutting (paw-application/infrastructure) | E0.3.2-engineering-specification.md; data-architecture.md | ADR-019 | E1/E3 |
| PAW-INF-004 | Contract/event versioning, idempotent consumption, poison-message quarantine | Cross-cutting (published contracts) | cross-domain-data-contracts.md; E0.3.2-engineering-specification.md | ADR-020 | E3/E4 |
| PAW-INF-005 | Capability registry metadata-only resolution; governed invocation through owning-domain use cases and coordination | Cross-cutting (capability governance) | ADR-026-capability-registry-and-ai-invocation.md | ADR-026 | E5 |
| PAW-INF-006 | Agent invocation security: untrusted input default-deny, exact approval binding, per-side-effect re-authorization, OutcomeUnknown reconciliation, redacted audit | Cross-cutting (AI/coordination boundary) | ADR-029-agent-invocation-security-and-approval.md; agent-authorization-model.md | ADR-029 | E5 |
| PAW-INF-007 | First-party plugin static registration, dependency DAG governance, four independent lifecycle dimensions, no data loss on disable, Learning boundary retained | Cross-cutting (platform composition) | ADR-023-domain-plugin-model.md; ADR-027-first-party-domain-plugin-policy.md; ADR-028-plugin-dependency-compatibility-governance.md | ADR-023/027/028 | E7 |
| PAW-INF-008 | RoleProfile as experience template only: no grants/secrets/approval, safe fallback, no authorization effect | Cross-cutting (presentation) | ADR-024-role-profile-model.md | ADR-024 | E7 |
| PAW-INF-009 | WorkspaceComposition scoped to one Data Workspace, references/configuration only, backend authorization at use time, safe fallback | Cross-cutting (presentation) | ADR-025-workspace-composition.md | ADR-025 | E7 |

PAW-INF-005～009 trace the ADR-023～029 R1 direction (owner-directed planning adjustment, 2026-11-12). The governing ADRs were **Accepted on 2026-11-12** by the repository owner (R1 conditions normative; implementation and verification remain Not Implemented / Not Verified). The Target column follows the roadmap owner's stage assignment (E5 absorbs ADR-026/029; E7 = platform composition, V1.0 前置). The E5 implementation specification is finalized; E7 specification work is not yet authorized.

## Verification rules

A design link is not implementation evidence. Each requirement must later identify its migration, application command/query, UI workflow, automated test, and manual acceptance evidence. `verified` may be assigned only after reproducible evidence exists.

## E0.3 architecture decision traceability

As of the E0.3 post-ratification governance pass (2026-11-08), the four E0.3 architecture concerns are additionally tracked as infrastructure requirements PAW-INF-001～004 in `requirements-baseline.md` §4. This supersedes the earlier position that these concerns carried no formal requirement IDs. The ADRs remain the governing source: each PAW-INF requirement's acceptance basis is the Validation required section of its ADR, and the one-to-one ID-set validation between the requirements baseline and this traceability table applies to PAW-INF as to every other ID.

| Architecture concern | Decision record | Design location | Required future evidence |
|---|---|---|---|
| Cross-domain orchestration boundary | ADR-017 | application-architecture.md; system-architecture.md | dependency checks; Outbox atomicity; cross-domain use-case tests |
| Rust crate/module dependency direction | ADR-018 | module-boundary-rules.md; system-architecture.md | Cargo workspace build; forbidden-edge tests |
| Durable operation and recovery | ADR-019 | application-architecture.md; data-architecture.md; agent-adapter-architecture.md | restart recovery; indeterminate-submit reconciliation; cancellation tests |
| Contract/event evolution | ADR-020 | cross-domain-data-contracts.md; data-architecture.md | version fixtures; duplicate delivery; quarantine and migration tests |

All four ADRs were ratified as Accepted on 2026-11-08 (E0.3.2). Accepted is a design-decision status; implementation remains Not Implemented and verification Not Verified, and this traceability must not be represented as implemented or verified.
