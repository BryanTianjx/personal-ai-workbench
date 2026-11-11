-- Migration 0008: Learning & Growth basic lifecycle + KnowledgeRef metadata
-- (E4, Alpha 0.2 front segment).
-- Model: learning-growth-PRD §3.1–3.4; associations per cross-domain-data-
-- contracts §4 (Item↔Project many-to-many, Item→Task zero-or-many
-- action-step references, Session↔KnowledgeRef many-to-many). All
-- relationships are workspace-consistent via composite (id, workspace_id)
-- keys. Sessions are historical records: deleting an item or referenced
-- project/task is RESTRICT-blocked while sessions remain.

CREATE TABLE learning_roadmaps (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    description     TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_roadmaps_workspace_updated
    ON learning_roadmaps(workspace_id, updated_at DESC);

CREATE TABLE learning_plans (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    roadmap_id      TEXT NOT NULL,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    description     TEXT,
    status          TEXT NOT NULL CHECK (status IN
      ('draft','active','paused','completed','archived')),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (roadmap_id, workspace_id)
        REFERENCES learning_roadmaps(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_plans_workspace_updated
    ON learning_plans(workspace_id, updated_at DESC);
CREATE INDEX idx_plans_roadmap ON learning_plans(roadmap_id, workspace_id);

CREATE TABLE learning_items (
    id                  TEXT PRIMARY KEY,
    workspace_id        TEXT NOT NULL,
    plan_id             TEXT NOT NULL,
    kind                TEXT NOT NULL CHECK (kind IN
      ('concept','reading','exercise','experiment','implementation','review')),
    title               TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    expected_outcome    TEXT,
    acceptance_criteria TEXT,
    notes               TEXT,
    status              TEXT NOT NULL CHECK (status IN
      ('todo','in_progress','done','cancelled')),
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (plan_id, workspace_id)
        REFERENCES learning_plans(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_items_workspace_status_updated
    ON learning_items(workspace_id, status, updated_at DESC);
CREATE INDEX idx_items_plan ON learning_items(plan_id, workspace_id);

CREATE TABLE learning_sessions (
    id                  TEXT PRIMARY KEY,
    workspace_id        TEXT NOT NULL,
    item_id             TEXT NOT NULL,
    task_id             TEXT,
    project_id          TEXT,
    method              TEXT,
    duration_minutes    INTEGER CHECK (duration_minutes IS NULL OR duration_minutes >= 0),
    summary             TEXT,
    difficulty          TEXT,
    takeaway            TEXT,
    next_step           TEXT,
    occurred_at         INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (item_id, workspace_id)
        REFERENCES learning_items(id, workspace_id) ON DELETE RESTRICT,
    FOREIGN KEY (task_id, workspace_id)
        REFERENCES tasks(id, workspace_id) ON DELETE RESTRICT,
    FOREIGN KEY (project_id, workspace_id)
        REFERENCES projects(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_sessions_item ON learning_sessions(item_id, workspace_id);
CREATE INDEX idx_sessions_workspace
    ON learning_sessions(workspace_id, occurred_at DESC);

-- Item ↔ Project many-to-many (relationship: applies_to).
CREATE TABLE learning_item_projects (
    item_id         TEXT NOT NULL,
    workspace_id    TEXT NOT NULL,
    project_id      TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    PRIMARY KEY (item_id, project_id),
    FOREIGN KEY (item_id, workspace_id)
        REFERENCES learning_items(id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (project_id, workspace_id)
        REFERENCES projects(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_item_projects_project
    ON learning_item_projects(project_id, workspace_id);

-- Item → Task zero-or-many action-step references (relationship:
-- action_step). The Task remains Work Management-owned.
CREATE TABLE learning_item_tasks (
    item_id         TEXT NOT NULL,
    workspace_id    TEXT NOT NULL,
    task_id         TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    PRIMARY KEY (item_id, task_id),
    FOREIGN KEY (item_id, workspace_id)
        REFERENCES learning_items(id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (task_id, workspace_id)
        REFERENCES tasks(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_item_tasks_task ON learning_item_tasks(task_id, workspace_id);

-- Session ↔ KnowledgeRef many-to-many (relationship: source_material).
CREATE TABLE learning_session_refs (
    session_id      TEXT NOT NULL,
    workspace_id    TEXT NOT NULL,
    knowledge_ref_id TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    PRIMARY KEY (session_id, knowledge_ref_id),
    FOREIGN KEY (session_id, workspace_id)
        REFERENCES learning_sessions(id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (knowledge_ref_id, workspace_id)
        REFERENCES knowledge_refs(id, workspace_id) ON DELETE CASCADE
);

CREATE TABLE knowledge_refs (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    title           TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    source          TEXT NOT NULL CHECK (source IN
      ('lingshu','hindsight','web','local')),
    external_uri    TEXT,
    locator         TEXT,
    version_label   TEXT,
    notes           TEXT,
    availability    TEXT NOT NULL CHECK (availability IN
      ('available','stale','unavailable')),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_knowledge_refs_workspace
    ON knowledge_refs(workspace_id, updated_at DESC);
