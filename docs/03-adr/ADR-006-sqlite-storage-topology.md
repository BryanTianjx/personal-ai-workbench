# ADR-006: Single SQLite Storage Topology

- **Status:** Accepted
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified

## Context
The desktop workbench requires offline persistence, cross-domain queries, FTS5 search, reliable migrations, and consistent backup/restore. The earlier E0.2 data architecture recorded single-database and database-per-workspace as alternatives; this ADR resolves that open question for the current desktop baseline.

## Decision drivers
- Keep desktop runtime self-contained and offline-capable.
- Preserve atomic local use cases and straightforward Dashboard composition.
- Enforce workspace data boundaries consistently.
- Minimize migration and backup coordination complexity.
- Do not preclude a future server data topology.

## Options considered
1. **Single SQLite database:** one `app.sqlite`; workspace-scoped records carry `workspace_id`; backend policy enforces isolation. Simpler transactions, FTS5, migrations, and backup; requires rigorous scope enforcement.
2. **Database per Workspace:** stronger physical separation and easier workspace-level file portability; complicates cross-workspace queries, atomic operations, migration orchestration, and consistent backups.

## Decision
Use one local authoritative SQLite database, `app.sqlite`, for Alpha 0.1 through V1.0. Enforce workspace isolation in backend application/repository policy, not frontend filters. Workspace-scoped relationships use appropriate composite constraints. Attachments are stored separately under managed workspace-scoped directories. FTS5 is a rebuildable projection and every query/result lookup enforces the same workspace authorization. PostgreSQL remains an optional future server technology and is not a desktop prerequisite.

## Consequences
**Positive:** simpler local transactions, Dashboard queries, FTS5, migration, backup/restore, and lower operational burden.
**Negative:** logical isolation is security-critical and needs cross-workspace tests; a large single database may eventually require reassessment; independent per-workspace encryption is not provided by this decision.

## Risks and mitigations
- **Risk:** A missing Workspace predicate exposes another Workspace's data. **Mitigation:** enforce scope in backend repositories/use cases, use composite constraints where applicable, and test CRUD, search, jobs, imports, attachments and Dashboard paths.
- **Risk:** A single database becomes a performance or lifecycle bottleneck. **Mitigation:** benchmark representative data volumes and revisit only against the explicit conditions in this ADR; do not prematurely introduce per-Workspace databases.
- **Risk:** A live SQLite file is copied or shared unsafely. **Mitigation:** use SQLite-consistent backup facilities and prohibit network-share concurrent writes as a sync substitute.

## Validation required before implementation verification
Prototype and test: multiple Workspace isolation across CRUD/search/jobs/imports; composite relationship integrity; FTS5 rebuild; migration from clean and prior versions; backup/restore with attachments; interrupted migration recovery; representative data volume and query latency. Tests must run on Windows and macOS.

## Revisit conditions
Reconsider if independent workspace encryption/keys or lifecycle becomes a requirement, scale testing demonstrates material single-database limits, or a multi-user server architecture requires a different isolation boundary. Any change requires a superseding ADR.
