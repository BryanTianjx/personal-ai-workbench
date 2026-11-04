# System Architecture

**Architecture Baseline:** E0.2 Candidate
**Decision Status:** Accepted design baseline — see ADR-002 through ADR-006
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified
**Applies to:** Alpha 0.1 through V1.0

## 1. Architectural objectives

The system is a local-first personal AI workbench. Its architecture must preserve offline operation for core workflows, local ownership and export of user data, human governance over consequential Agent actions, replaceable integrations, and Windows/macOS as first-class desktop targets. Use a modular monolith until independent deployment or scaling is justified.

## 2. Logical architecture

```text
Presentation: Tauri 2 · React · TypeScript
        ↓ commands/events
Application: use cases · validation · authorization · coordination
        ↓ ports
Domain: Workspace · Project · Task · Agent · Knowledge · Creator · Automation · Audit
        ↑ implemented by
Infrastructure: SQLite/FTS5 · files · backup · logging · OS credential store
Integration: Agent adapters · knowledge adapters · Git · optional remote services
```

UI code does not access SQLite or invoke external tools directly. Domain rules do not depend on Tauri, SQL drivers, or vendor SDKs. Infrastructure and adapters implement inward-facing ports.

## 3. Runtime topology

Alpha is a single desktop application with a local Rust backend and a React UI hosted by Tauri. Core operations require no daemon, Docker, PostgreSQL, or network service. Optional integrations communicate with configured local CLI processes or remote endpoints.

A future Web entry point is a separate deployment concern. It must reuse application/domain contracts where practical, but never expose the desktop database or filesystem directly.

## 4. Cross-cutting constraints

- Workbench-owned records are authoritative in local databases and managed files.
- Hindsight and Lingshu remain authoritative for their own knowledge content; the workbench stores references, metadata, permitted indexes, and sync state.
- Agent execution and filesystem access pass through authorization and policy enforcement.
- Operations carry correlation IDs; structured logs redact secrets.
- Remote failures retain local intent and diagnostic state; no silent success.
- Schema changes are versioned migrations; adapters are independently testable.

## 5. Quality attributes and evidence

| Attribute | Requirement | Acceptance evidence |
|---|---|---|
| Offline | Core project/task/search/file workflows work disconnected | Network-disabled test |
| Portability | Native Windows and macOS builds | Install and smoke-test on each OS |
| Security | Least privilege and explicit approval | Permission/path tests |
| Reliability | Transactional writes and recoverable backup | Restore drill |
| Maintainability | Explicit module ownership | Architecture review and CI checks |
| Interoperability | External tools behind adapters | Contract tests with fake providers |

## 6. V1 non-goals

No microservices, mandatory cloud account, mandatory PostgreSQL, shared writable SQLite over a network share, autonomous high-risk execution, or dependency on undocumented vendor internals.

## 7. Accepted architecture decisions and remaining open items

The following decisions are part of the current E0.2 architecture candidate and are not open alternatives:

- **Module/deployment topology:** Modular Monolith for Alpha 0.1 through V1.0; see ADR-002.
- **Data ownership:** workbench-owned desktop records remain local-first; external knowledge systems retain authority over their own content; see ADR-003.
- **Agent integration boundary:** provider-specific capabilities remain behind Agent Adapters and central authorization; see ADR-004.
- **Offline runtime:** core workflows operate locally without mandatory network services; see ADR-005.
- **Database topology:** one authoritative local `app.sqlite` for Alpha 0.1 through V1.0, with logical Workspace isolation enforced by backend policy; see ADR-006. Database-per-Workspace is not the selected baseline.

Remaining items requiring implementation design or prototype evidence include attachment content-addressing, plugin packaging/distribution, representative SQLite performance, and the future multi-device synchronization protocol. These are not unresolved choices about the current database topology. None of the above decisions implies that implementation or verification is complete.
