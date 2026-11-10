-- Migration 0005: scoped full-text search indexes (E2).
-- FTS5 tables are standalone rebuildable projections (data-architecture §7):
-- never authoritative, always workspace-scoped on query, maintained in the
-- same transaction as the source change by the repositories, with an
-- explicit rebuild path after upgrades. Rows store opaque ids as UNINDEXED
-- columns; no content beyond titles/descriptions/notes is indexed.

CREATE VIRTUAL TABLE fts_projects USING fts5(
    name,
    description,
    workspace_id UNINDEXED,
    project_id UNINDEXED
);

CREATE VIRTUAL TABLE fts_tasks USING fts5(
    title,
    notes,
    workspace_id UNINDEXED,
    task_id UNINDEXED
);
