# System Architecture

**Architecture Baseline:** E0.2 + Learning & Growth extension  
**Decision Status:** Accepted design baseline — see ADR-002 through ADR-006 and ADR-013 through ADR-016  
**Implementation Status:** Not Implemented  
**Verification Status:** Not Verified  
**Applies to:** Alpha 0.1 through V1.0

## 1. Architectural objectives

The system is a local-first personal work and growth platform. Its architecture must preserve offline operation for core workflows, local ownership and export of user data, human governance over consequential Agent actions, evidence-based learning assessment, replaceable integrations, and a Windows-first delivery strategy with cross-platform boundaries reserved. Use a modular monolith until independent deployment or scaling is justified.

## 2. Logical architecture

```text
Presentation: Tauri 2 · React · TypeScript
        ↓ commands/events
Application: use cases · validation · authorization · coordination
        ↓ ports
Domain:
  Work Management · Learning & Growth · Knowledge & Context
  AI Collaboration · Engineering · Creator · Automation
  System & Security
        ↑ implemented by
Infrastructure: SQLite/FTS5 · files · backup · logging · OS credential store
Integration: Agent adapters · knowledge adapters · Git · optional remote services
```

UI code does not access SQLite or invoke external tools directly. Domain rules do not depend on Tauri, SQL drivers, or vendor SDKs. Infrastructure and adapters implement inward-facing ports.

## 3. Runtime topology

Alpha is a single desktop application with a local Rust backend and a React UI hosted by Tauri. Core operations require no daemon, Docker, PostgreSQL, or network service. Optional integrations communicate with configured local CLI processes or remote endpoints.

A future Web entry point is a separate deployment concern. It must reuse application/domain contracts where practical, but never expose the desktop database or filesystem directly.

## 4. Domain interaction model

- Work Management owns Goal, Area, Project, and Task.
- Learning & Growth owns Roadmap, Plan, Item, Session, Skill, Evidence, and Assessment.
- Knowledge & Context owns local notes, KnowledgeRef metadata, and permitted indexing state.
- Engineering owns engineering activity metadata and Artifact references as specified by its contract.
- AI Collaboration owns Agent registrations, capabilities, and Agent Executions.
- Automation owns Workflow definitions and Workflow Runs.
- System & Security owns authorization, audit, backup metadata, and security policy.

Cross-domain interaction uses published application commands, query DTOs, stable opaque identifiers, and versioned domain events. Direct cross-domain SQL writes are prohibited. See `cross-domain-data-contracts.md`.

## 5. Cross-cutting constraints

- Workbench-owned records are authoritative in local databases and managed files.
- Hindsight and Lingshu remain authoritative for their own knowledge content; the workbench stores references, metadata, permitted indexes, and sync state.
- Git repositories remain authoritative for source code and commit history.
- Learning completion does not automatically change a Skill assessment.
- Project-generated learning gaps are proposals until user acceptance.
- Agent execution and filesystem access pass through authorization and policy enforcement.
- Operations carry correlation IDs; structured logs redact secrets.
- Remote failures retain local intent and diagnostic state; no silent success.
- Schema changes are versioned migrations; adapters are independently testable.

## 6. Quality attributes and evidence

| Attribute | Requirement | Acceptance evidence |
|---|---|---|
| Offline | Core work and delivered local learning workflows work disconnected | Network-disabled test |
| Portability | Windows-first native build; cross-platform targets verified individually | Install and smoke-test on each supported OS |
| Security | Least privilege and explicit approval | Permission/path tests |
| Reliability | Transactional writes and recoverable backup | Restore drill |
| Maintainability | Explicit module ownership | Architecture review and CI checks |
| Interoperability | External tools behind adapters | Contract tests with fake providers |
| Learning integrity | Evidence-based, user-confirmed assessments | Domain and provenance tests |

## 7. V1 non-goals

No microservices, mandatory cloud account, mandatory PostgreSQL, shared writable SQLite over a network share, autonomous high-risk execution, automatic skill scoring, or dependency on undocumented vendor internals.

## 8. Accepted architecture decisions and remaining open items

The following decisions are part of the current baseline:
- Modular Monolith for Alpha 0.1 through V1.0; see ADR-002.
- Workbench-owned desktop records remain local-first; external knowledge systems retain authority over their own content; see ADR-003.
- Provider-specific capabilities remain behind Agent Adapters and central authorization; see ADR-004.
- Core workflows operate locally without mandatory network services; see ADR-005.
- One authoritative local `app.sqlite` for Alpha 0.1 through V1.0, with logical Workspace isolation enforced by backend policy; see ADR-006.
- Learning & Growth is a first-class domain; see ADR-013.
- Learning-to-Project integration uses explicit contracts and events; see ADR-014.
- Skill assessment is evidence-linked and user-confirmed; see ADR-015.
- Learning and Knowledge retain separate ownership; see ADR-016.

Remaining items requiring implementation design or prototype evidence include attachment content-addressing, plugin packaging/distribution, representative SQLite performance, and the future multi-device synchronization protocol. None of the above decisions implies implementation or verification is complete.