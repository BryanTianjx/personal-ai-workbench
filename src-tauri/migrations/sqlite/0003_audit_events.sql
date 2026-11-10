-- Migration 0003: audit events (E1-09 security and audit gates)
-- System & Security owns audit semantics (system-architecture §4); E1-09
-- records consequential-action evidence for the protected side effects of
-- the first vertical slice. Rows carry no free-text payloads: only stable
-- actions, resource references, outcomes and correlation ids, so nothing
-- sensitive can leak into the audit trail by construction.

CREATE TABLE audit_events (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT,
    actor_id        TEXT NOT NULL,
    action          TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    resource_id     TEXT,
    outcome         TEXT NOT NULL CHECK (outcome IN ('allowed','denied','error')),
    correlation_id  TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_audit_workspace_created
    ON audit_events(workspace_id, created_at DESC);
CREATE INDEX idx_audit_action_created
    ON audit_events(action, created_at DESC);
