-- Migration 0002: durable operations and outbox messaging (E1-07/E1-08, Tier 1)
-- Schema reference: docs/05-architecture-evolution/E1-sqlite-schema-migrations-workspace-isolation.md §3
-- Logical contract: docs/05-architecture-evolution/E0.3.2-engineering-specification.md
-- The schema_migrations ledger is maintained by the migration runner.

CREATE TABLE operations (
    id                          TEXT PRIMARY KEY,
    kind                        TEXT NOT NULL,
    owner_domain                TEXT NOT NULL,
    owner_type                  TEXT NOT NULL,
    owner_id                    TEXT NOT NULL,
    workspace_id                TEXT,
    status                      TEXT NOT NULL CHECK (status IN
      ('Pending','Running','AwaitingApproval','RetryScheduled','OutcomeUnknown',
       'Succeeded','Failed','Cancelled')),
    actor_id                    TEXT NOT NULL,
    correlation_id              TEXT NOT NULL,
    causation_id                TEXT,
    idempotency_key             TEXT,
    request_digest              TEXT,
    cancellation_support        TEXT NOT NULL CHECK (cancellation_support IN
      ('supported','unsupported','unknown')),
    cancellation_requested_at   INTEGER,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    started_at                  INTEGER,
    finished_at                 INTEGER,
    next_attempt_at             INTEGER,
    last_error_category         TEXT,
    last_error_code             TEXT,
    resolution_note             TEXT,
    CHECK (request_digest IS NULL OR length(request_digest) = 64),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_operations_workspace_status_updated
    ON operations(workspace_id, status, updated_at DESC);
CREATE INDEX idx_operations_due
    ON operations(status, next_attempt_at)
    WHERE status = 'RetryScheduled';
CREATE INDEX idx_operations_owner
    ON operations(owner_domain, owner_type, owner_id);
CREATE UNIQUE INDEX idx_operations_idempotency
    ON operations(COALESCE(workspace_id, '__system__'), kind, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE TABLE operation_attempts (
    id                  TEXT PRIMARY KEY,
    operation_id        TEXT NOT NULL,
    attempt_no          INTEGER NOT NULL CHECK (attempt_no > 0),
    phase               TEXT NOT NULL CHECK (phase IN
      ('prepare','submit','poll','cancel','reconcile')),
    started_at          INTEGER NOT NULL,
    finished_at         INTEGER,
    outcome             TEXT NOT NULL CHECK (outcome IN
      ('started','rejected','accepted','completed','failed','indeterminate','cancelled')),
    request_digest      TEXT,
    idempotency_key     TEXT,
    external_reference  TEXT,
    retryable           INTEGER NOT NULL CHECK (retryable IN (0,1)),
    error_category      TEXT,
    error_code          TEXT,
    evidence_ref        TEXT,
    UNIQUE (operation_id, attempt_no),
    FOREIGN KEY (operation_id) REFERENCES operations(id) ON DELETE RESTRICT
);
CREATE INDEX idx_operation_attempts_operation
    ON operation_attempts(operation_id, attempt_no DESC);

CREATE TABLE outbox_messages (
    id                  TEXT PRIMARY KEY,
    workspace_id        TEXT,
    producer_domain     TEXT NOT NULL,
    aggregate_type      TEXT NOT NULL,
    aggregate_id        TEXT NOT NULL,
    event_type          TEXT NOT NULL,
    schema_version      INTEGER NOT NULL CHECK (schema_version > 0),
    occurred_at         INTEGER NOT NULL,
    correlation_id      TEXT NOT NULL,
    causation_id        TEXT,
    payload_json        TEXT NOT NULL CHECK (json_valid(payload_json)),
    status              TEXT NOT NULL CHECK (status IN
      ('Pending','Leased','Delivered','Quarantined')),
    attempts            INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    available_at        INTEGER NOT NULL,
    lease_owner         TEXT,
    lease_until         INTEGER,
    created_at          INTEGER NOT NULL,
    delivered_at        INTEGER,
    last_error_category TEXT,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_outbox_dispatch
    ON outbox_messages(status, available_at, created_at);
CREATE INDEX idx_outbox_workspace
    ON outbox_messages(workspace_id, created_at DESC);

CREATE TABLE processed_messages (
    consumer_name       TEXT NOT NULL,
    message_id          TEXT NOT NULL,
    processed_at        INTEGER NOT NULL,
    outcome             TEXT NOT NULL CHECK (outcome IN ('Applied','Ignored')),
    handler_version     TEXT,
    PRIMARY KEY (consumer_name, message_id),
    FOREIGN KEY (message_id) REFERENCES outbox_messages(id) ON DELETE RESTRICT
);
