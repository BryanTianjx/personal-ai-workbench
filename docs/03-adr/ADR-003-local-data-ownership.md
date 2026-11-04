# ADR-003: Local-First Data Ownership

- **Status:** Accepted
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified

## Context
The workbench must function offline and preserve user control. It integrates with systems such as Hindsight and Lingshu that own their own knowledge stores.

## Decision
SQLite and managed local files are authoritative for workbench-owned desktop records. The database topology is one local SQLite database, `app.sqlite`, as decided by ADR-006. PostgreSQL is optional for a future server deployment, not a desktop prerequisite. External knowledge systems remain authoritative for their content; the workbench stores references, metadata, permitted local indexes, and sync state. A shared network SQLite file is not a multi-device synchronization mechanism.

## Consequences
**Positive:** offline core workflows, transparent local backup/export, simpler transactions and Dashboard queries, less duplication of external knowledge.
**Negative:** workspace isolation is enforced logically by backend policy and requires comprehensive cross-scope tests; external data may be unavailable or stale offline; future multi-device use requires a separate synchronization protocol.

## Risks and mitigations
- **Risk:** Logical Workspace isolation is bypassed by an unscoped query, job, import, or search projection. **Mitigation:** centralize scope enforcement and test every access path, including negative cross-workspace cases.
- **Risk:** External knowledge references become stale or unavailable offline. **Mitigation:** retain last-known metadata and expose source availability/freshness without claiming remote content is current.

## Validation
Test workspace isolation across CRUD, FTS5, background jobs, imports, Dashboard queries, attachments, and backup/restore. Verify migration and restore on both desktop platforms.

## Revisit when
Independent workspace encryption/lifecycle, demonstrated single-database scale limits, or a server/multi-device architecture is designed. Any topology change requires a new ADR.
