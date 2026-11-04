# Module Boundary Rules

**Architecture Baseline:** E0.2 Candidate  
**Implementation Status:** Not Implemented  
**Verification Status:** Not Verified

## 1. Ownership
Each domain module owns its entities, invariants, application commands, repositories, and authoritative writes. Other modules consume published application contracts or domain events. No module reads or writes another module's private repository or table directly.

## 2. Dependency direction
Presentation → Application → Domain. Infrastructure implements ports owned inward. Modules may depend on shared kernel types and published contracts, not each other's internals. Cycles are prohibited. Cross-module orchestration belongs in an application use case or Workflow Engine, not in a domain entity.

## 3. Read and write rules
- Commands go through the owning module's application service.
- Cross-module reads use query contracts/read models.
- Dashboard is a composed read model; it is not authoritative.
- FTS5 and caches are rebuildable projections and must apply the same authorization scope as source data.
- No frontend-only workspace filtering.
- Direct cross-module SQL writes are prohibited.

## 4. Transaction ownership
A use case owns its local SQLite transaction. A coordinated cross-module transaction is permitted only when one application orchestrator documents all participants, invariants, and rollback behavior. Never hold a database transaction open across a network/provider call. Use outbox-style persistence for reliable asynchronous follow-up; assume at-least-once delivery and require idempotent consumers.

## 5. Execution object separation
- Task: user-facing business work and lifecycle.
- Workflow Run: orchestration instance owned by Automation.
- Agent Execution: provider invocation/attempt owned by AI/Agent.
- Audit Event: decision/operation evidence owned by System & Security.
Use IDs and correlation IDs to relate them. Do not reuse a single state machine or let Automation mutate Agent Execution internals.

## 6. Failure and events
A module emits versioned domain events only after its invariant-preserving state change. Consumers validate schema/version, are idempotent, and record failures for retry/dead-letter handling. Event payloads minimize personal and secret data.

## 7. Enforcement
Enforce boundaries through crate/module visibility, application interfaces, repository ownership, architecture tests, and code review. A convention without tests is insufficient.
