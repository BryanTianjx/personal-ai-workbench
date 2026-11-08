# ADR-017: Coordination Layer for Cross-Domain Operations

- **Status:** Proposed
- **Date:** 2026-11-08
- **Decision scope:** Application coordination and cross-domain execution
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified
- **Related:** ADR-002, ADR-004, ADR-013, ADR-014

## Context

Multiple independently owned business domains need consistent orchestration, authorization, transaction coordination, durable operation tracking, event delivery, and recovery without transferring business ownership to a central module. Duplicating these mechanics in every domain creates divergent semantics; a new business domain or distributed service adds unnecessary complexity.

## Decision proposal

Introduce a logical Coordination Layer inside the modular monolith. It is an application-level capability, not a ninth business domain or separately deployed service.

Responsibilities:
- Orchestrate named cross-domain use cases through published commands and query DTOs.
- Propagate operation, correlation, and causation identifiers.
- Coordinate short local transactions and transactional Outbox writes.
- Dispatch at-least-once events to idempotent consumers.
- Persist long-running operation state and recovery metadata.
- Invoke central authorization immediately before protected side effects.
- Apply retry classification, cancellation, reconciliation, and audit conventions.

It does not own Project, Task, Learning, Knowledge, Agent, Workflow, Creator, or Artifact records. It does not expose private repositories or make network calls while holding long-lived SQLite transactions.

## Alternatives

1. Each domain implements its own orchestration/reliability: rejected due to duplicated lifecycle and retry semantics.
2. A central coordination business domain: rejected because it absorbs business ownership.
3. Distributed workflow service/microservices: rejected for initial desktop deployment complexity.
4. Application-level coordination within the modular monolith: proposed.

## Consequences

**Positive:** consistent cross-domain behavior, recoverable long-running work, testable orchestration, no new deployment dependency.

**Negative:** coordination can become a god layer unless limited to named use cases and cross-cutting mechanics.

## Risks and mitigations

- God-layer risk: keep business invariants in owning domains.
- Hidden coupling: prohibit private repository access and test dependency direction.
- Partial external outcomes: persist indeterminate states and reconcile before retry.
- Event loss: atomically persist Outbox with local state; consumers deduplicate by event ID.

## Validation required

Cross-domain use-case tests; transaction rollback and Outbox atomicity; duplicate delivery/idempotent consumer tests; authorization denial/stale approval tests; process restart recovery; architecture tests prohibiting private repository imports.

## Revisit when

A separate service is justified by independent deployment, scaling, or operational ownership evidence. A superseding ADR and server/runtime design are required.
