-- Migration 0006: attachment metadata (E2).
-- Bytes live in managed storage (attachments/<workspace-id>/<managed-uuid>);
-- SQLite holds the metadata. The owner reference is polymorphic
-- (project|task) and therefore not a SQL foreign key: the repository
-- verifies owner existence inside the workspace before inserting, and owner
-- deletion refuses while attachments remain (explicit, no silent cascade —
-- ADR-020). sha256 is recorded at ingestion for integrity and future
-- content-addressing assessment.

CREATE TABLE attachments (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    owner_type      TEXT NOT NULL CHECK (owner_type IN ('project','task')),
    owner_id        TEXT NOT NULL,
    managed_id      TEXT NOT NULL,
    file_name       TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL CHECK (size_bytes >= 0),
    sha256          TEXT NOT NULL CHECK (length(sha256) = 64),
    created_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_attachments_owner
    ON attachments(owner_type, owner_id, workspace_id);
CREATE INDEX idx_attachments_workspace
    ON attachments(workspace_id, created_at DESC);
