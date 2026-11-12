# R2-EXEC-04 — Database, Migration & Recovery Audit

- Parent: [Engineering Audit R2.0](./README.md)
- Dependency: R2-EXEC-01 baseline frozen
- Mode: isolated temporary databases only

## Objective

Verify SQLite configuration, schema constraints, migration safety, transactional consistency, Outbox behavior, attachment lifecycle, and end-to-end backup/restore readiness.

## Scope

Database initialization, connection configuration, migrations, repositories, Workspace constraints, Outbox, attachment storage, backup/restore, retention and deletion flows.

## Audit tasks

### A. SQLite configuration

Inspect and verify actual runtime settings for:
- foreign key enforcement;
- journal mode;
- synchronous mode;
- busy timeout;
- connection and transaction lifecycle;
- WAL/SHM behavior;
- file permissions and database path selection.

Do not infer runtime PRAGMA values solely from source defaults.

### B. Schema and Workspace integrity

1. Inventory tables, columns, primary keys, foreign keys, unique constraints, checks, and indexes.
2. Inspect composite Workspace ownership constraints.
3. Identify nullable ownership fields and potential cross-workspace joins.
4. Check cascade/restrict semantics and orphan risks.
5. Run integrity and foreign-key checks on disposable databases.

### C. Migration safety

1. Verify migration ordering and schema version tracking.
2. Verify checksums and tamper detection.
3. Inspect failure/rollback behavior.
4. Test fresh initialization.
5. Test every supported historical upgrade path using generated fixtures or documented snapshots.
6. Test interrupted migration only where safely reproducible.
7. Verify resulting schema and data invariants.

Never point migration tests at the user's existing database.

### D. Transactions and Outbox

Test partial failure, rollback, duplicate delivery, retry, idempotency, concurrent writes, crash boundaries, and event-to-state consistency. Identify whether event publication can occur without committed state or vice versa.

### E. Backup and restore — distinguish layers

Assess separately:
1. backup/restore implementation in infrastructure;
2. application use case or port;
3. desktop command/API;
4. GUI entry point and confirmation;
5. backup integrity/manifest;
6. restore validation and failure atomicity;
7. attachment consistency;
8. recovery documentation and user-facing behavior.

Test with disposable DB and attachments:
- create synthetic records;
- produce backup;
- verify backup contents and integrity;
- restore into a new isolated destination;
- compare records, relationships, migrations, and attachments;
- test invalid/truncated backup;
- confirm failed restore does not damage source or existing destination.

Do not restore over the user's database or application profile.

### F. Data lifecycle

Inspect deletion, retention, attachment cleanup, orphan-file detection, and privacy implications.

## Required outputs

- `R2-EXEC-04-database-integrity-report.md`
- `R2-EXEC-04-migration-report.md`
- `R2-EXEC-04-transaction-outbox-report.md`
- `R2-EXEC-04-backup-restore-report.md`
- `R2-EXEC-04-data-lifecycle-report.md`

## Acceptance criteria

Fresh DB, supported migrations, constraints, transactions, backup, restore, and attachments are assessed independently. All write tests use temporary paths. Evidence includes schema/PRAGMA output and test logs.

## Stop conditions

Stop if the DB path resolves to user data, backup/restore could overwrite an existing file, a destructive migration cannot be isolated, or test code attempts external data access.
