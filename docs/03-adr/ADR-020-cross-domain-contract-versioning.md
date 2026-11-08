# ADR-020: Cross-Domain Contract and Event Versioning

- **Status:** Proposed
- **Date:** 2026-11-08
- **Decision scope:** Application DTOs, domain events, compatibility and evolution
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified
- **Related:** ADR-002, ADR-006, ADR-013, ADR-014, ADR-016, ADR-017

## Context

Cross-domain contracts define stable opaque references, published commands/queries, and versioned event envelopes. They need implementable rules for schema compatibility, idempotency, deprecation, and migration so modules and adapters can evolve safely.

## Decision proposal

### Identifiers and scope

- IDs are opaque, stable, typed in application code, and never derived from mutable titles, paths, or URLs.
- Workspace-scoped commands, events, and records carry explicit workspace context.
- Authorization is re-evaluated when resolving a reference; possession of an ID is not authorization.
- Cross-workspace relationships are rejected by default.

### Commands and queries

- Commands represent intent and return typed outcomes; the owning domain handles them.
- Queries return read DTOs and do not mutate authoritative business records.
- DTOs have explicit schema/version ownership and validation rules.
- Breaking changes require a new version or documented migration window; silent semantic changes are prohibited.

### Events

- Event envelopes include event ID, event type, schema version, workspace ID when scoped, aggregate ID, occurred-at timestamp, correlation ID, optional causation ID, and minimal payload.
- Delivery is at-least-once. Consumers deduplicate by event ID and make handlers idempotent.
- Unknown event types are safely ignored or quarantined with diagnostics; unsupported schema versions are not interpreted as a known version.
- Additive optional fields may be introduced compatibly. Removing/renaming fields or changing meaning requires a new version and migration/deprecation plan.
- Events must not carry secrets, credentials, full note/document bodies, or raw source files.

### Idempotency and Outbox

- Mutating commands define idempotency requirements and key scope.
- Outbox records are committed atomically with the owning local state change.
- Dispatch retries are bounded/backed off and observable; poison messages enter an explicit quarantined state.
- Consumers record processed event IDs in the same transaction as their local effect where feasible.

### Lifecycle and deletion

- Archive is distinct from deletion.
- Detaching a reference does not delete either endpoint.
- Deletion is performed by the owning domain and emits a tombstone/availability event where needed.
- Historical Learning Sessions, Assessments, and Audit Events are not silently cascade-deleted.

## Alternatives

1. Unversioned internal DTOs/events: rejected due to semantic drift.
2. Strict lockstep upgrades: rejected due to unnecessary coupling.
3. Versioned envelopes with compatibility rules: proposed.

## Consequences

**Positive:** safer module evolution, idempotent delivery, explicit compatibility, improved testability.

**Negative:** schema governance and migration tests add upfront work; versioning must remain proportional to a single-process application.

## Risks and mitigations

- Over-versioning: version public cross-domain contracts, not every private function.
- Compatibility ambiguity: document producer/consumer support and test old/new fixtures.
- Poison event loops: bound retries and quarantine with safe diagnostics.
- Data leakage: minimize payloads and test redaction.

## Validation required

Contract fixture compatibility; duplicate delivery/idempotent consumer; unknown event/schema handling; Outbox atomicity/retry/quarantine; Workspace scope and tombstone integration; documentation validation and traceability.

## Revisit when

A separately deployed service or external public API requires stronger compatibility guarantees. Preserve these principles while adding an API lifecycle policy.

## E0.3.1 review outcome

**Disposition:** Recommended for acceptance with clarification. **Status remains Proposed pending repository-owner ratification.**

Clarifications:
- Apply explicit versions to persisted/event envelopes and published cross-domain DTOs; do not version every private function or internal Rust type.
- Additive optional fields are compatible only if old consumers can safely ignore them and the existing field semantics remain unchanged.
- Field removal, requiredness changes, semantic changes, and incompatible enum evolution require a new schema version and a documented migration/deprecation policy.
- Unknown event types may be ignored only where no required invariant depends on them. Unsupported schema versions must be quarantined/rejected, not guessed or parsed as a known version.
- Idempotency keys are scoped to the command/consumer and intent; same key with a different canonical request digest is a conflict.
- Outbox dispatch is at-least-once; consumer deduplication is committed atomically with local effects where feasible.
