# Data Architecture

**Architecture Baseline:** E0.2 Candidate
**Decision Status:** Accepted — see ADR-003 and ADR-006
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Ownership
The workbench owns workspace metadata, projects, tasks, workflow and Agent execution records, local content metadata, policies, and audit records. Hindsight and Lingshu own their knowledge content. The workbench stores references, identifiers, permitted local search projections, and synchronization state rather than silently duplicating entire knowledge repositories.

## 2. Logical layout
```text
application-data/
├── app.sqlite
├── attachments/
│   └── <workspace-id>/
│       └── <managed-file-id>
├── exports/
├── backups/
├── logs/
└── cache/
```
This is a logical model, not a hard-coded path contract. Use Windows known folders and macOS Application Support/Logs/Caches through platform APIs.

## 3. Database topology — decided
Use one local authoritative SQLite database, `app.sqlite`, for Alpha 0.1 through V1.0. Workspace isolation is logical and enforced by backend application/repository policy. Every workspace-scoped row carries `workspace_id`; relationships use composite constraints where needed to prevent cross-workspace references. All access paths, including search, background jobs, imports, and Dashboard queries, apply the same scope checks. Frontend filtering is not a security boundary.

Database-per-workspace is not used in this baseline. Reconsider only if independent encryption/keys, independent lifecycle/restore, scale, or a future multi-user server requirement justifies the added transaction, migration, and backup complexity. PostgreSQL is optional for a future server and is not a desktop prerequisite. Do not place the live SQLite database on a network share or treat shared-file access as synchronization.

## 4. Schema and migrations
Every database has an explicit schema version. Migrations are ordered, deterministic, and tested from supported prior versions. Startup checks compatibility before normal writes. Failed migrations preserve a recoverable pre-migration backup and report a clear error. Destructive migrations require an explicit rollback plan. Test clean install, upgrade, interruption, and restore.

## 5. Files and attachments
Store file metadata in SQLite and bytes in managed storage. Validate path components, canonicalize paths, prevent traversal and symlink escapes, and enforce workspace scope. Use atomic temp-write plus rename where supported. Content-addressed storage remains an optimization candidate; assess deduplication, privacy leakage, and deletion semantics before adoption.

## 6. Search
FTS5 indexes only content the application is permitted to index. Indexes are rebuildable projections, not authoritative data. Every query and result lookup enforces workspace authorization. Record source identity and last indexed version; expose freshness/source availability where relevant.

## 7. Backup and restore
A backup is a consistent snapshot of app.sqlite, required attachments, and a manifest containing schema/app versions, workspace IDs, checksums, and creation time. Use SQLite backup facilities or coordinated quiesce/snapshot; naively copying a live database is not accepted. Restore validates manifest and checksums before replacing active data. Test on both platforms and in a clean user profile.

## 8. Privacy and retention
Secrets belong in OS credential stores, not SQLite or logs. Logs/audit use data minimization and redaction. Export and deletion must state effects on attachments, indexes, backups, and external references.
