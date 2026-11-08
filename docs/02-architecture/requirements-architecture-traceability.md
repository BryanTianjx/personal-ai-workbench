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

## Verification rules

A design link is not implementation evidence. Each requirement must later identify its migration, application command/query, UI workflow, automated test, and manual acceptance evidence. `verified` may be assigned only after reproducible evidence exists.

## E0.3 architecture decision traceability

The following are architecture decisions and implementation-readiness constraints, not new product requirement IDs. They therefore do not alter the formal requirements baseline or the one-to-one ID-set validation.

| Architecture concern | Decision record | Design location | Required future evidence |
|---|---|---|---|
| Cross-domain orchestration boundary | ADR-017 | application-architecture.md; system-architecture.md | dependency checks; Outbox atomicity; cross-domain use-case tests |
| Rust crate/module dependency direction | ADR-018 | module-boundary-rules.md; system-architecture.md | Cargo workspace build; forbidden-edge tests |
| Durable operation and recovery | ADR-019 | application-architecture.md; data-architecture.md; agent-adapter-architecture.md | restart recovery; indeterminate-submit reconciliation; cancellation tests |
| Contract/event evolution | ADR-020 | cross-domain-data-contracts.md; data-architecture.md | version fixtures; duplicate delivery; quarantine and migration tests |

All four ADRs remain Proposed pending repository-owner ratification. Their proposed design must not be represented as implemented or verified.
