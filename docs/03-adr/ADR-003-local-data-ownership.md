# ADR-003: Local-First Data Ownership

- **Status:** Accepted for V1 planning
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering

## Context

The workbench must function offline and preserve user control. It integrates with systems such as Hindsight and Lingshu that own their own knowledge stores.

## Decision

SQLite and managed local files are authoritative for workbench-owned desktop records. PostgreSQL is optional for a future server deployment, not a desktop prerequisite. External knowledge systems remain authoritative for their content; the workbench stores references, metadata, permitted local indexes, and sync state. A shared network SQLite file is not a multi-device synchronization mechanism.

## Consequences

**Positive:** offline core workflows, transparent local backup/export, less duplication of external knowledge.

**Negative:** external data may be unavailable or stale offline; future multi-device use requires a separate synchronization protocol and conflict model.

## Revisit when

A server edition or multi-device sync is designed. Specify data authority, migration, conflict resolution, encryption, and export guarantees.
