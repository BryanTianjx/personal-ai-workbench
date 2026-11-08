# Requirements–Architecture Traceability

**Baseline:** E0.2 + Learning & Growth extension  
**Status:** Accepted design traceability; implementation evidence pending

| Requirement / Capability | Owning domain | Primary design | ADR | Target |
|---|---|---|---|---|
| Goal / Area / Project / Task | Work Management | domain-architecture.md | ADR-002, ADR-006 | Alpha 0.1 |
| Local data and workspace isolation | System / all domains | data-architecture.md | ADR-003, ADR-006 | Alpha 0.1 |
| Agent registration and execution boundary | AI Collaboration | agent-adapter-architecture.md, agent-authorization-model.md | ADR-004 | Alpha 0.2 |
| Offline core workflows | Platform / all domains | offline-first-architecture.md | ADR-005 | Alpha 0.1 |
| Learning Roadmap / Plan / Item / Session | Learning & Growth | learning-growth-PRD.md | ADR-013 | Alpha 0.2 |
| Learning ↔ Project / Task integration | Learning & Work Management | cross-domain-data-contracts.md | ADR-014 | Alpha 0.2 |
| Skill, evidence, assessment history | Learning & Growth | learning-growth-PRD.md, cross-domain-data-contracts.md | ADR-015 | Beta 0.5 |
| Learning ↔ Knowledge ownership boundary | Learning & Knowledge | cross-domain-data-contracts.md, data-architecture.md | ADR-016 | Alpha 0.2 |
| External KnowledgeRef | Knowledge & Context | data-architecture.md | ADR-003, ADR-016 | Alpha 0.2 |
| Backup/restore of Learning records | System / Learning | data-architecture.md | ADR-006, ADR-013 | Alpha 0.2 |

## Verification rules

A design link is not implementation evidence. Each requirement must later identify its migration, application command/query, UI workflow, automated test, and manual acceptance evidence. `verified` may be assigned only after reproducible evidence exists.