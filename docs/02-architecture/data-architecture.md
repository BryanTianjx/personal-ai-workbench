# Data Architecture

**Status:** Proposed — E0.2; storage partitioning requires prototype validation

## 1. Ownership

The workbench owns workspace metadata, projects, tasks, execution records, local content metadata, policies, and audit records. Hindsight and Lingshu own their knowledge content. The workbench stores references, identifiers, permitted local search projections, and synchronization state rather than silently duplicating entire knowledge repositories.

## 2. Logical layout

```text
application-data/
├── app.sqlite
├── workspaces/<workspace-id>/
│   ├── workspace.sqlite   # partitioning option; not yet frozen
│   ├── attachments/
│   ├── exports/
│   └── backups/
├── logs/
└── cache/
```

This is a logical model, not a hard-coded path contract. Use Windows known folders and macOS Application Support/Logs/Caches through platform APIs.

## 3. Database strategy

SQLite is the desktop authoritative store; FTS5 provides local full-text search. PostgreSQL is optional for a future server deployment and is not a desktop prerequisite.

Two options remain:
- **Single database:** simpler atomic transactions, migrations, and cross-domain queries.
- **Database per workspace:** stronger isolation and workspace portability, but more complex cross-workspace queries, migrations, and backup coordination.

The app.sqlite plus workspace.sqlite layout is not final until a prototype evaluates transaction boundaries, backup/restore, migrations, and workspace switching. An accepted ADR freezes the choice.

## 4. Schema and migrations

Every database has an explicit schema version. Migrations are ordered, deterministic, and tested from supported prior versions. Startup checks compatibility before normal writes. Failed migrations preserve a recoverable pre-migration backup and report a clear error. Destructive migrations require an explicit rollback plan. Test clean install, upgrade, interruption, and restore.

## 5. Files and attachments

Store file metadata in the database and bytes in managed storage. Validate path components, canonicalize paths, prevent traversal and symlink escapes, and enforce workspace scope. Use atomic temp-write plus rename where supported. Content-addressed storage is an optimization candidate; assess deduplication, privacy leakage, and deletion semantics first.

## 6. Search

FTS5 indexes only content the application is permitted to index. Indexes are rebuildable projections, not authoritative data. Record source identity and last indexed version; expose freshness/source availability where relevant.

## 7. Backup and restore

A backup is a consistent snapshot of database(s), required attachments, and a manifest containing schema/app versions, workspace IDs, checksums, and creation time. Use SQLite backup facilities or coordinated quiesce/snapshot; naively copying a live database is not accepted. Restore validates manifest and checksums before replacing active data. Test on both platforms.

## 8. Privacy and retention

Secrets belong in OS credential stores, not SQLite or logs. Logs/audit use data minimization and redaction. Export and deletion must state effects on attachments, indexes, backups, and external references.
