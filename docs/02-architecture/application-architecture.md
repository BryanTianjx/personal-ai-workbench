# Application Architecture

**Architecture Baseline:** E0.2 Candidate  
**Decision Status:** Accepted design rules; individual ADR status is authoritative  
**Implementation Status:** Not Implemented  
**Verification Status:** Not Verified

## 1. Purpose
The application layer translates user intent into validated, authorized, transactional behavior. It is the boundary between presentation and domain/infrastructure.

## 2. Layers and dependency rules
1. Presentation: React views and Tauri commands/events; rendering and input collection only.
2. Application: use cases, DTOs, validation, context resolution, authorization, transaction coordination, queries, and result mapping.
3. Domain: entities, value objects, invariants, domain services, and events; no UI, SQL, OS, or vendor dependencies.
4. Ports: repository, clock, filesystem, credential-store, Agent-provider, and external-knowledge interfaces.
5. Infrastructure/adapters: SQLite repositories, filesystem and OS implementations, provider-specific clients.

Dependencies point inward. Domain must not import infrastructure. Modules do not import another module's internal implementation or repository.

## 3. Use-case flow
```text
UI input → Tauri boundary → DTO validation → context resolution
→ authorization → use case → domain invariants
→ repository transaction / adapter call → audit outcome → typed response
```
All protected side effects pass through application authorization. Remote calls must not be held inside long-running SQLite transactions.

## 4. Commands, queries, and Dashboard
Commands mutate authoritative state through the owning module's application service. Queries return read models and do not mutate authoritative business records. Search may update disposable indexes/cache only.

Dashboard is a composition Read Model, not a domain module or source of truth. It obtains data through published application query contracts from Workspace, Project, Task, Agent, Automation, and System modules. It must not query their private tables directly. Each remote status projection includes provider, observed-at time, and freshness/availability; stale or unavailable values are not presented as current. Dashboard caches are disposable and rebuildable.

## 5. Module ownership and cross-module transactions
Each module owns writes to its authoritative records. A cross-module use case is coordinated by an application-level orchestrator and documents participating module contracts and invariants. No module writes another module's tables. For the single-database topology, a short local transaction may span repositories only when coordinated by that use case; external network calls are outside the transaction. Failure rolls back local state or records an explicit recoverable state. Reliable downstream work uses an outbox-style record committed with the local change; delivery is at-least-once and consumers are idempotent.

## 6. Distinct execution concepts
- Task: business work item and its user-facing status.
- Workflow Run: one workbench-defined orchestration instance, owned by Automation.
- Agent Execution: one provider execution attempt, owned by AI/Agent.
- Audit Event: immutable-by-module record of a decision or operation, owned by System & Security.
These are related by IDs/correlation IDs, not collapsed into one state machine. Workflow Run may create Agent Executions; Task status changes only through Task use cases.

## 7. Errors and long-running work
Stable errors: validation, not-found, conflict, permission-denied, unavailable, timeout, cancelled, integrity, internal. External operations distinguish rejected-before-submit, submitted-with-unknown-outcome, accepted/running, completed, failed, and cancelled. Unknown remote outcomes are neither success nor definite failure.

Agent runs, imports, indexing, backup, and publishing preparation are durable records with status, timestamps, correlation ID, and cancellation capability where supported. UI progress is a projection of persisted state.

## 8. Testing
Domain unit tests; application tests with fake ports; repository tests with temporary SQLite; adapter contract tests; Tauri boundary authorization tests; Dashboard query-to-database reconciliation tests.
