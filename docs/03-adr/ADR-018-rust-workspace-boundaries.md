# ADR-018: Rust Workspace and Enforceable Module Boundaries

- **Status:** Proposed
- **Date:** 2026-11-08
- **Decision scope:** Rust code organization and dependency direction
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified
- **Related:** ADR-002, ADR-017

## Context

The product uses Tauri 2, Rust, React, and TypeScript. Eight business domains need clear ownership, but creating a crate for every domain before implementation increases build and maintenance overhead. Directory naming alone does not enforce dependency direction.

## Decision proposal

Use a Cargo workspace with a small initial set:

- paw-domain: domain entities, value objects, invariants, and domain contracts; no infrastructure dependencies.
- paw-application: use cases, published ports, DTOs, authorization orchestration, and coordination.
- paw-infrastructure: SQLite repositories, filesystem, OS credential-store, logging, and external adapters.
- paw-desktop: Tauri commands/events, UI bridge, and composition root.

Business domains initially remain explicit modules within the appropriate crate. Extract a domain into its own crate only when independent ownership, dependency enforcement, test isolation, or parallel development demonstrates a concrete benefit.

Rules:

1. Dependencies point inward. Domain never depends on Tauri, SQL drivers, OS APIs, or vendor SDKs.
2. UI does not access SQLite, repositories, filesystem internals, or external tools directly.
3. A domain cannot import another domain's private entities, repositories, ORM models, or tables.
4. Cross-domain interaction uses published application contracts and stable IDs.
5. The composition root wires implementations; business modules do not dynamically locate global services.
6. Platform-specific behavior stays behind infrastructure ports.

## Alternatives

- One crate per business domain from day one: rejected as premature overhead.
- One undifferentiated application crate: rejected because ownership becomes difficult to enforce.
- Small layered workspace with domain modules: proposed.

## Consequences

**Positive:** explicit dependency direction, manageable crate count, testable boundaries, future extraction options.

**Negative:** module privacy and architecture tests require discipline; extraction may require visibility changes.

## Risks and mitigations

- Crate sprawl: require written extraction rationale and architecture review.
- Leaky abstractions: prohibit SQL/provider types in domain and test dependency graph.
- Composition-root coupling: keep Tauri types at desktop boundary and map to application DTOs.

## Validation required

Cargo workspace builds on Windows; domain tests run without Tauri/database dependencies; application tests use fake ports; repository tests use temporary SQLite; architecture checks reject forbidden imports; Tauri boundary tests validate authorization and DTOs.

## Revisit when

A domain needs independent distribution/deployment or a stable independent build boundary that outweighs crate overhead.

## E0.3.1 review outcome

**Disposition:** Recommended for acceptance with clarification. **Status remains Proposed pending repository-owner ratification.**

Clarifications:

- The four initial crates are paw-domain, paw-application, paw-infrastructure, and paw-desktop. No one-crate-per-domain split is required for Alpha.
- Domain events, entities and invariants belong to owning domain modules in paw-domain. Published application DTOs, commands/queries and ports belong to paw-application; paw-domain must not depend on paw-application.
- paw-infrastructure implements inward-owned ports. paw-desktop is the composition root and Tauri boundary.
- The dependency graph must be enforced by automated checks; a directory layout or naming convention is not sufficient.
- Any additional crate requires a written extraction rationale and testable boundary.
