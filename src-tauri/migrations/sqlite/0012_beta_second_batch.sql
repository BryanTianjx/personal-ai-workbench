-- Migration 0012: Beta 0.5 second batch (E6 第二批)
-- 1) creator_publish_records: manual publish facts bound to exact draft
--    versions (publishing automation stays gated; feature-matrix note 4).
-- 2) project_retrospectives + session backflow links: learning products
--    flowing back into a project's review (PRD §3.8/§5.3); sessions are
--    referenced by id (ADR-014), application layer validates existence and
--    linkage. Insert-only discipline (corrections are new records).
-- The schema_migrations ledger is maintained by the migration runner.

CREATE TABLE creator_publish_records (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    draft_id     TEXT NOT NULL,
    version_no   INTEGER NOT NULL CHECK (version_no >= 1),
    channel      TEXT NOT NULL CHECK (channel IN ('wechat','manual')),
    locator      TEXT CHECK (locator IS NULL OR length(locator) <= 500),
    notes        TEXT CHECK (notes IS NULL OR length(notes) <= 500),
    created_at   INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (draft_id, workspace_id)
        REFERENCES creator_drafts(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_creator_publish_records_draft
    ON creator_publish_records(draft_id, workspace_id, created_at);

CREATE TABLE project_retrospectives (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id   TEXT NOT NULL,
    summary      TEXT NOT NULL CHECK (length(summary) > 0 AND length(summary) <= 2000),
    created_at   INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (project_id, workspace_id)
        REFERENCES projects(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_project_retrospectives_project
    ON project_retrospectives(project_id, workspace_id, created_at DESC);

-- Learning sessions flowing back into the retrospective. RESTRICT on both
-- sides: history is never silently cascaded.
CREATE TABLE retrospective_session_refs (
    retrospective_id TEXT NOT NULL,
    workspace_id     TEXT NOT NULL,
    session_id       TEXT NOT NULL,
    PRIMARY KEY (retrospective_id, session_id),
    FOREIGN KEY (retrospective_id, workspace_id)
        REFERENCES project_retrospectives(id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (session_id, workspace_id)
        REFERENCES learning_sessions(id, workspace_id) ON DELETE RESTRICT
);
