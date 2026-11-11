-- Migration 0007: audit retention policy and outbox re-drive support (E3).
--
-- 1) audit_events is rebuilt with workspace_id ON DELETE SET NULL: deleting
--    a workspace detaches its audit rows into system scope instead of
--    blocking deletion (or destroying evidence). Audit rows are System &
--    Security records; they survive the workspace as workspace-less
--    evidence and fall under the audit retention policy (E3-03).
-- 2) outbox_messages gains re-drive bookkeeping: an explicit, audited
--    re-drive preserves the event id and records who re-drove it and how
--    often (E0.3.2 spec §4 — never silently a different event).

CREATE TABLE audit_events_rebuilt (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT,
    actor_id        TEXT NOT NULL,
    action          TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    resource_id     TEXT,
    outcome         TEXT NOT NULL CHECK (outcome IN ('allowed','denied','error')),
    correlation_id  TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);
INSERT INTO audit_events_rebuilt (id, workspace_id, actor_id, action, resource_type,
    resource_id, outcome, correlation_id, created_at)
    SELECT id, workspace_id, actor_id, action, resource_type,
    resource_id, outcome, correlation_id, created_at FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_rebuilt RENAME TO audit_events;
CREATE INDEX idx_audit_workspace_created
    ON audit_events(workspace_id, created_at DESC);
CREATE INDEX idx_audit_action_created
    ON audit_events(action, created_at DESC);

ALTER TABLE outbox_messages ADD COLUMN redrive_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE outbox_messages ADD COLUMN last_redrive_by TEXT;
