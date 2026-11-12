-- Migration 0011: Creator content pipeline (E6-03, Beta 0.5)
-- Scope per product-scope.md Creator Beta 0.5: 选题、草稿、版本、审核.
-- Publishing automation is out of scope until official API verification
-- (feature-matrix note 4) — no publish tables in this migration.
-- Draft versions are immutable snapshots; reviews bind one exact version.
-- The schema_migrations ledger is maintained by the migration runner.

CREATE TABLE creator_topics (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    title        TEXT NOT NULL CHECK (length(title) > 0 AND length(title) <= 200),
    notes        TEXT CHECK (notes IS NULL OR length(notes) <= 500),
    status       TEXT NOT NULL CHECK (status IN ('Idea','Selected','Parked')),
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_creator_topics_workspace_status
    ON creator_topics(workspace_id, status, updated_at DESC);

CREATE TABLE creator_drafts (
    id                 TEXT PRIMARY KEY,
    workspace_id       TEXT NOT NULL,
    topic_id           TEXT NOT NULL,
    title              TEXT NOT NULL CHECK (length(title) > 0 AND length(title) <= 200),
    current_version_no INTEGER NOT NULL CHECK (current_version_no >= 0),
    status             TEXT NOT NULL CHECK (status IN ('Writing','InReview','Approved','Published')),
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (topic_id, workspace_id)
        REFERENCES creator_topics(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_creator_drafts_workspace_status
    ON creator_drafts(workspace_id, status, updated_at DESC);

-- Immutable version snapshots: INSERT-only by contract (no UPDATE/DELETE
-- path in the repository; deleting a draft RESTRICTs while versions exist).
CREATE TABLE creator_draft_versions (
    id           TEXT NOT NULL UNIQUE,
    workspace_id TEXT NOT NULL,
    draft_id     TEXT NOT NULL,
    version_no   INTEGER NOT NULL CHECK (version_no >= 1),
    body         TEXT NOT NULL CHECK (length(body) <= 100000),
    created_at   INTEGER NOT NULL,
    PRIMARY KEY (draft_id, version_no),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (draft_id, workspace_id)
        REFERENCES creator_drafts(id, workspace_id) ON DELETE RESTRICT
);

-- Review history bound to exact versions; the draft status machine reflects
-- decisions but stays its own state machine (no shared transition table).
CREATE TABLE creator_draft_reviews (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    draft_id     TEXT NOT NULL,
    version_no   INTEGER NOT NULL CHECK (version_no >= 1),
    decision     TEXT NOT NULL CHECK (decision IN ('Approved','Rejected')),
    comment      TEXT CHECK (comment IS NULL OR length(comment) <= 500),
    created_at   INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (draft_id, workspace_id)
        REFERENCES creator_drafts(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_creator_draft_reviews_draft
    ON creator_draft_reviews(draft_id, workspace_id, created_at);
