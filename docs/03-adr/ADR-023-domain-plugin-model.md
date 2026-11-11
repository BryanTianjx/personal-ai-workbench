# ADR-023: Domain Plugin Model and Static Registration First

- **Status:** Accepted (R1)
- **Decision status:** Accepted (repository owner decision, 2026-11-12; R1 clarifications are normative; implementation and verification remain separate gates)
- **Implementation status:** Not Implemented
- **Verification status:** Not Verified
- **Date:** 2026-11-11 (accepted 2026-11-12)
- **Scope:** Domain extension model, registration, lifecycle
- **Related:** ADR-002, ADR-003, ADR-006, ADR-017, ADR-018, ADR-020

## Context

PAW has multiple business domains that should be independently owned and selectively composed. The existing modular monolith already provides domain modules, but it does not yet define a platform-level plugin descriptor, capability catalog, UI extension registration, or enable/disable lifecycle.

Treating every page as a plugin would fragment the domain model. Introducing arbitrary dynamic code loading would add security, compatibility, and operational costs before the first-party boundaries are proven.

## Decision

1. A **Domain** remains the business bounded context and owns entities, invariants, state machines, authoritative records, and domain events.
2. A **Domain Plugin** is the platform registration/composition unit for one or more cohesive domain capabilities. It declares identity, version, dependencies, capabilities, UI extensions, migration namespace, and lifecycle policy.
3. Initial plugins are first-party, statically linked modules registered at application startup.
4. Plugin registration and lifecycle orchestration live in the existing four-Crate architecture; no fifth crate or separate process is introduced.
5. Plugin disable hides its extensions and blocks new capability invocation while preserving all domain data and history.
6. Dynamic libraries, downloaded code, remote JavaScript, third-party marketplace, hot-unload, and out-of-process plugin sandbox are deferred to separate ADRs.

## Consequences

- Domain ownership remains independent of the plugin catalog.
- Plugin boundaries can be tested without introducing microservices.
- Static registration is less flexible than dynamic loading but substantially easier to secure and verify.
- Existing domains can be adopted incrementally; no big-bang rewrite is required.

## Required validation

Manifest validation, dependency DAG and cycle rejection, API compatibility, disable/data-retention behavior, architecture boundary tests, and startup failure isolation.

## Revisit when

A real third-party extension requirement or independent plugin distribution requirement exists and can justify signing, sandboxing, permission review, compatibility policy, and rollback design.

## R1 clarification: lifecycle dimensions and ownership

The platform MUST keep these dimensions independent; no single `enabled` boolean may represent all of them:

1. **Registration:** whether the statically linked descriptor is known to the registry.
2. **Availability:** whether dependencies, compatibility, policy and startup checks permit invocation.
3. **Composition preference:** whether a plugin is selected in a particular `WorkspaceComposition`.
4. **Presentation visibility:** whether a contribution is currently shown in a UI surface.

A registered plugin may be blocked; an available plugin may be omitted from one composition; a hidden contribution may still own valid domain data. None of these states changes domain ownership or backend authorization.

Disabling a plugin blocks new invocations and hides its contributions according to policy. It MUST NOT delete data, rewrite historical records, revoke unrelated permissions, or silently cancel an in-flight durable Operation. The owning domain and Application Coordination layer MUST apply ADR-019 reconciliation/drain semantics.

## R1 implementation boundary

The initial implementation is limited to first-party, statically registered plugins in the existing four-crate modular monolith. No fifth crate, runtime code download, hot unload, marketplace, remote JavaScript, or plugin process is authorized by this ADR. Descriptor serialization, concrete Rust types, contribution APIs and SQL persistence remain implementation-specification work.

## R1 risk controls

| Risk | Control |
|---|---|
| Plugin flag conflates registration and preference | Four independent state dimensions and state-transition tests |
| Plugin boundary replaces domain boundary | Domain remains owner of entities, invariants, persistence and events |
| Disable strands Operations | ADR-019 drain/reconcile; preserve Operation and audit history |
| Registry becomes execution container | Registry stores descriptors and resolves metadata only |

## R1 validation additions

Test each state dimension independently; verify hidden/disabled plugins do not delete data; verify disabled capabilities reject new calls; verify existing Operations remain queryable and reconcile under ADR-019; verify no plugin can write another domain's tables.
