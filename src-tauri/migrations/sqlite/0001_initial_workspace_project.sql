-- Migration 0001: workspaces and projects (Alpha 0.1 vertical slice)
-- Schema reference: docs/05-architecture-evolution/E1-sqlite-schema-migrations-workspace-isolation.md
-- Note: the schema_migrations ledger table is created and maintained by the
-- migration runner (paw-infrastructure::sqlite::migrate) in the same
-- transaction as each migration; it is intentionally not recreated here.
-- This file only carries the business tables and indexes for the first slice.

PRAGMA foreign_keys = ON;

CREATE TABLE workspaces (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 120),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE projects (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    description     TEXT,
    status          TEXT NOT NULL CHECK (status IN ('active','paused','completed','archived')),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE (id, workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);

CREATE INDEX idx_projects_workspace_updated
    ON projects(workspace_id, updated_at DESC);
