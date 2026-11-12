-- Migration 0013: Platform composition (E7-02/E7-03/E7-04)
-- Governance: ADR-023 (plugin four-state dimensions — user disable is the
-- composition/availability preference, persisted separately from the static
-- registry), ADR-024 (RoleProfile = experience template, no grants/secrets/
-- approval tokens by construction), ADR-025 (WorkspaceComposition scoped to
-- one Data Workspace; composition filtering is never access control).
-- The schema_migrations ledger is maintained by the migration runner.

-- User disable preference per plugin (static registration itself lives in
-- code; only the runtime preference is persisted). Disabling blocks NEW
-- capability invocations via the E5 pipeline gate; domain data, audit
-- history and in-flight Operations are preserved (ADR-019 drain/reconcile).
CREATE TABLE plugin_states (
    plugin_id    TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    disabled     INTEGER NOT NULL CHECK (disabled IN (0, 1)),
    disabled_at  INTEGER,
    updated_at   INTEGER NOT NULL,
    UNIQUE (plugin_id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);

-- Role profiles: experience templates. No grant/credential/approval fields
-- exist by construction; references are validated at use time.
CREATE TABLE role_profiles (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    name            TEXT NOT NULL CHECK (length(name) > 0 AND length(name) <= 100),
    plugin_preferences TEXT NOT NULL CHECK (json_valid(plugin_preferences)),
    dashboard_layout   TEXT CHECK (dashboard_layout IS NULL OR length(dashboard_layout) <= 200),
    agent_profile_refs TEXT NOT NULL CHECK (json_valid(agent_profile_refs)),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_role_profiles_workspace ON role_profiles(workspace_id, updated_at DESC);

-- Workspace compositions: presentation aggregates scoped to an actor and
-- exactly one Data Workspace. Composition filtering is never access control.
CREATE TABLE workspace_compositions (
    id               TEXT PRIMARY KEY,
    workspace_id     TEXT NOT NULL,
    actor_id         TEXT NOT NULL,
    name             TEXT NOT NULL CHECK (length(name) > 0 AND length(name) <= 100),
    profile_ref      TEXT,
    plugin_selection TEXT NOT NULL CHECK (json_valid(plugin_selection)),
    navigation_config TEXT CHECK (navigation_config IS NULL OR length(navigation_config) <= 200),
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (profile_ref, workspace_id)
        REFERENCES role_profiles(id, workspace_id) ON DELETE SET NULL
);
CREATE INDEX idx_workspace_compositions_actor
    ON workspace_compositions(workspace_id, actor_id, updated_at DESC);
