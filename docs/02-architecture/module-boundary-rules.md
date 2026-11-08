# Module Boundary Rules

**Architecture Baseline:** E0.2 + Learning & Growth extension
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Ownership

Each domain module owns its entities, invariants, application commands, repositories, and authoritative writes. Other modules consume published application contracts or domain events. No module reads or writes another module's private repository or table directly.

## 2. Dependency direction

Presentation → Application → Domain. Infrastructure implements ports owned inward. Modules may depend on shared kernel types and published contracts, not each other's internals. Cycles are prohibited. Cross-module orchestration belongs in an application use case or Workflow Engine, not in a domain entity.

## 3. Read and write rules

- Commands go through the owning module's application service.
- Cross-domain reads use query contracts/read models.
- Dashboard is a composed read model; it is not authoritative.
- FTS5 and caches are rebuildable projections and must apply the same authorization scope as source data.
- No frontend-only workspace filtering.
- Direct cross-domain SQL writes are prohibited.
- Learning may request a Task from Work Management; it cannot insert or update Task records directly.
- Project may publish a learning-gap proposal; it cannot create or modify Learning Plans directly.
- Learning may reference a KnowledgeRef or Artifact; it cannot mutate the source asset.

## 4. Transaction ownership

A use case owns its local SQLite transaction. A coordinated cross-domain transaction is permitted only when one application orchestrator documents all participants, invariants, and rollback behavior. Never hold a database transaction open across a network/provider call. Use outbox-style persistence for reliable asynchronous follow-up; assume at-least-once delivery and require idempotent consumers.

## 5. Execution and learning object separation

- Task: user-facing business work owned by Work Management.
- Learning Item: learning objective/practice/checkpoint owned by Learning & Growth.
- Learning Session: actual learning activity record owned by Learning & Growth.
- Skill Assessment: user-confirmed competency judgement owned by Learning & Growth.
- Workflow Run: orchestration instance owned by Automation.
- Agent Execution: provider invocation/attempt owned by AI Collaboration.
- Artifact: engineering/content output metadata owned by its designated source domain.
- Audit Event: decision/operation evidence owned by System & Security.

Use IDs and correlation IDs to relate them. Do not reuse a single state machine or let one module mutate another module's internals.

## 6. Events

A module emits versioned domain events only after its invariant-preserving state change. Consumers validate schema/version, are idempotent, and record failures for retry/dead-letter handling. Event payloads minimize personal and secret data. Project learning-gap events create proposals, not automatically approved plans.

## 7. Enforcement

Enforce boundaries through crate/module visibility, application interfaces, repository ownership, architecture tests, and code review. A convention without tests is insufficient.
