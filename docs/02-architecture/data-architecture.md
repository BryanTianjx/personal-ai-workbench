# Data Architecture

**Architecture Baseline:** E0.2 + Learning & Growth extension
**Decision Status:** Accepted — see ADR-003, ADR-006, ADR-013～016
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Ownership

The workbench owns workspace metadata, Goals, Areas, Projects, Tasks, Learning records, local context metadata, Agent/Workflow execution records, policies, and audit records. Hindsight and Lingshu own their canonical knowledge content. Git repositories own source code and commit history. The workbench stores references, identifiers, permitted local projections, and synchronization state rather than silently duplicating entire external repositories.

Learning & Growth owns the learning lifecycle and competency assessment history. Knowledge & Context owns notes and knowledge references. Engineering owns engineering activity metadata and Artifact reference/metadata as defined by its contract. A cross-domain association never transfers ownership.

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

Database-per-workspace is not used in this baseline. PostgreSQL is optional for a future server and is not a desktop prerequisite. Do not place the live SQLite database on a network share or treat shared-file access as synchronization.

## 4. Learning data additions

Logical entities introduced by ADR-013～015:

- `learning_roadmaps`
- `learning_plans`
- `learning_items`
- `learning_sessions`
- `skills`
- `skill_evidence`
- `learning_assessments`
- `learning_associations` (or equivalent typed association tables)
- `domain_events` / outbox records where durable local event delivery is required

All workspace-owned records include `workspace_id`. Associations must enforce workspace consistency. Skill assessment history and Learning Sessions are historical records and must not be cascade-deleted when a Project or KnowledgeRef becomes unavailable.

The logical entity names above are design guidance; exact physical schema and migration sequence remain implementation decisions subject to ADR and migration review.

## 5. Schema and migrations

Every database has an explicit schema version. Migrations are ordered, deterministic, and tested from supported prior versions. Startup checks compatibility before normal writes. Failed migrations preserve a recoverable pre-migration backup and report a clear error. Destructive migrations require an explicit rollback plan. Test clean install, upgrade, interruption, and restore.

## 6. Files and attachments

Store file metadata in SQLite and bytes in managed storage. Validate path components, canonicalize paths, prevent traversal and symlink escapes, and enforce workspace scope. Use atomic temp-write plus rename where supported. Content-addressed storage remains an optimization candidate; assess deduplication, privacy leakage, and deletion semantics before adoption.

## 7. Search

FTS5 indexes only content the application is permitted to index. Indexes are rebuildable projections, not authoritative data. Every query and result lookup enforces workspace authorization. Learning search may index user-authored Learning Item titles, summaries, and Session notes after explicit scope definition; it must not ingest full external knowledge repositories by default.

## 8. Backup and restore

A backup is a consistent snapshot of app.sqlite, required attachments, and a manifest containing schema/app versions, workspace IDs, checksums, and creation time. Learning associations, evidence references, assessment history, and durable event metadata must be included. Use SQLite backup facilities or coordinated quiesce/snapshot; naively copying a live database is not accepted. Restore validates manifest and checksums before replacing active data.

## 9. Privacy and retention

Secrets belong in OS credential stores, not SQLite or logs. Logs/audit use data minimization and redaction. Export and deletion must state effects on attachments, indexes, backups, external references, Learning history, and evidence provenance.

## 10. E0.3 operation and Outbox persistence guidance

E0.3 adds logical persistence requirements, not a frozen physical schema. The implementation may use normalized tables, but must preserve these distinct records:

- Operation: durable cross-cutting execution envelope linked to its owning domain object.
- OperationAttempt: one attempt to perform a potentially side-effecting step, including attempt number, start/end time, classification and external reference where available.
- OutboxMessage: event/message committed atomically with the owning local state transition.
- ProcessedMessage: consumer deduplication marker committed with the consumer's local effect where feasible.

Every workspace-scoped record carries workspace_id and is queried under the same backend authorization boundary. System-wide operations may have an explicit null/non-workspace scope only where the owning use case documents why. Do not encode null as a wildcard grant.

Outbox dispatch is at-least-once. Retain message ID, contract type/version, payload-minimization policy, attempts, next-attempt time, delivery status, and safe error category. Poison messages are quarantined for diagnosis; they must not retry forever. Credentials, prompts, full documents and raw attachments are not event payloads.

Operation and attempt retention, redaction, export, and deletion behavior must be specified before production use. Do not persist secrets or rely on provider transcript retention as the workbench's source of truth.
