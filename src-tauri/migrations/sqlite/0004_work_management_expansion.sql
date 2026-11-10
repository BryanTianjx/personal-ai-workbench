-- Migration 0004: Work Management expansion — areas, goals, tasks and
-- project links (E2 vertical slice).
-- Model: PRD §3.1/§4.2 (Project may link Goal/Area; Task may link Project;
-- standalone tasks allowed). All relationships use composite
-- (id, workspace_id) keys so a cross-workspace reference fails at the
-- constraint level. The projects table is rebuilt: SQLite cannot attach a
-- composite foreign key to an existing table via ALTER TABLE.
-- The schema_migrations ledger is maintained by the migration runner.

CREATE TABLE areas (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 120),
    description     TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_areas_workspace_updated
    ON areas(workspace_id, updated_at DESC);

CREATE TABLE goals (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    area_id         TEXT,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    description     TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (area_id, workspace_id)
        REFERENCES areas(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_goals_workspace_updated
    ON goals(workspace_id, updated_at DESC);
CREATE INDEX idx_goals_area ON goals(area_id, workspace_id);

-- Rebuild projects: same columns plus optional Goal/Area links. Rows are
-- copied verbatim (no defaults invented); integrity holds because the
-- composite parents exist and copied rows have NULL links.
CREATE TABLE projects_rebuilt (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    description     TEXT,
    status          TEXT NOT NULL CHECK (status IN ('active','paused','completed','archived')),
    goal_id         TEXT,
    area_id         TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (goal_id, workspace_id)
        REFERENCES goals(id, workspace_id) ON DELETE RESTRICT,
    FOREIGN KEY (area_id, workspace_id)
        REFERENCES areas(id, workspace_id) ON DELETE RESTRICT
);
INSERT INTO projects_rebuilt (id, workspace_id, name, description, status, created_at, updated_at)
    SELECT id, workspace_id, name, description, status, created_at, updated_at
    FROM projects;
DROP TABLE projects;
ALTER TABLE projects_rebuilt RENAME TO projects;
CREATE INDEX idx_projects_workspace_updated
    ON projects(workspace_id, updated_at DESC);
CREATE INDEX idx_projects_goal ON projects(goal_id, workspace_id);
CREATE INDEX idx_projects_area ON projects(area_id, workspace_id);

CREATE TABLE tasks (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    project_id      TEXT,
    title           TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    notes           TEXT,
    status          TEXT NOT NULL CHECK (status IN ('todo','in_progress','done','cancelled')),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (project_id, workspace_id)
        REFERENCES projects(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_tasks_workspace_status_updated
    ON tasks(workspace_id, status, updated_at DESC);
CREATE INDEX idx_tasks_project ON tasks(project_id, workspace_id);
