# Application Architecture

**Status:** Proposed — E0.2

## 1. Purpose

The application layer translates user intent into validated, authorized, transactional behavior. It is the boundary between presentation and domain/infrastructure.

## 2. Layers and dependency rules

1. **Presentation:** React views and Tauri commands/events; rendering and input collection only.
2. **Application:** use cases, DTOs, validation orchestration, authorization, transaction coordination, and result mapping.
3. **Domain:** entities, value objects, domain services, invariants, and domain events; no UI, SQL, OS, or vendor dependencies.
4. **Ports:** repository, clock, filesystem, credential-store, Agent-provider, and external-knowledge interfaces.
5. **Infrastructure/adapters:** SQLite repositories, filesystem and OS implementations, provider-specific clients.

Dependencies point inward. Domain must not import infrastructure.

## 3. Use-case flow

```text
UI input → Tauri boundary → DTO validation → context resolution
→ authorization → use case → domain invariants
→ repository transaction / adapter call → audit outcome → typed response
```

Protected side effects require authorization first. Remote calls must not be held inside long-running SQLite transactions.

## 4. Commands and queries

Use command/query separation at the application API:
- Commands mutate state and return typed outcomes and identifiers.
- Queries return read models and do not mutate business state.
- Search may update disposable indexes/cache but not authoritative records.

A full CQRS infrastructure is not required.

## 5. Error model

Use stable categories: validation, not-found, conflict, permission-denied, unavailable, timeout, cancelled, integrity, and internal. UI messages are actionable without exposing secrets or stack traces. Structured logs retain redacted diagnostic context.

External operations distinguish rejected-before-submit, submitted-with-unknown-outcome, accepted/running, completed, failed, and cancelled. Unknown remote outcomes must not be represented as success or definite failure.

## 6. Transactions and events

A use case owns its local transaction boundary. If reliable downstream processing is needed, record domain events with state changes using an outbox-style table. Delivery is at-least-once unless stronger guarantees are proven; consumers must be idempotent. Do not claim exactly-once execution across external systems.

## 7. Long-running work

Agent runs, imports, indexing, backup, and publishing preparation are durable jobs/executions with status, timestamps, correlation ID, cancellation capability where supported, and diagnostic summary. UI progress is a projection of persisted state.

## 8. Test strategy

Domain unit tests; application tests with fake ports and deterministic clocks; repository tests with temporary SQLite databases; adapter contract tests; and Tauri boundary tests for validation and authorization.
