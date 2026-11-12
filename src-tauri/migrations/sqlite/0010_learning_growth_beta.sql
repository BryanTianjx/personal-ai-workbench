-- Migration 0010: Learning & Growth Beta (E6-01/E6-02)
-- Schema reference: learning-growth-PRD §3.5–3.8; ADR-015 (skill evidence and
-- assessment); PAW-LG-007 (project learning-gap proposal).
-- Governance invariants: skill levels are user-confirmed facts (an
-- assessment must be evidence-linked, link table enforces >= 1 evidence);
-- assessments snapshot the skill's scale_version (history never reinterpreted
-- after a scale bump); gap proposals reference projects without ownership
-- transfer (ADR-014). State strings mirror paw_domain entities exactly.
-- The schema_migrations ledger is maintained by the migration runner.

CREATE TABLE skills (
    id             TEXT PRIMARY KEY,
    workspace_id   TEXT NOT NULL,
    name           TEXT NOT NULL CHECK (length(name) > 0 AND length(name) <= 100),
    category       TEXT NOT NULL CHECK (length(category) > 0 AND length(category) <= 60),
    scale_version  INTEGER NOT NULL CHECK (scale_version >= 1),
    notes          TEXT,
    CHECK (notes IS NULL OR length(notes) <= 500),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_skills_workspace_category ON skills(workspace_id, category);

-- Evidence records point at their source by opaque id without copying
-- source content (PRD §3.6). No cross-domain FK: source ids are validated
-- at use time (domain-architecture reference discipline).
CREATE TABLE skill_evidence (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    skill_id     TEXT NOT NULL,
    kind         TEXT NOT NULL CHECK (kind IN
      ('concept_explanation','practice','experiment','implementation',
       'test_verification','project_delivery','tech_review','teaching_documentation')),
    source_kind  TEXT NOT NULL CHECK (source_kind IN
      ('artifact','note','knowledge_ref','task','project')),
    source_id    TEXT NOT NULL CHECK (length(source_id) > 0 AND length(source_id) <= 100),
    summary      TEXT,
    created_at   INTEGER NOT NULL,
    CHECK (summary IS NULL OR length(summary) <= 500),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (skill_id, workspace_id)
        REFERENCES skills(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_skill_evidence_skill ON skill_evidence(skill_id, workspace_id);

-- User-confirmed assessments (append-only; no UPDATE path). The link table
-- enforces the >= 1 evidence invariant at the schema level.
CREATE TABLE learning_assessments (
    id             TEXT PRIMARY KEY,
    workspace_id   TEXT NOT NULL,
    skill_id       TEXT NOT NULL,
    level          INTEGER NOT NULL CHECK (level BETWEEN 0 AND 5),
    scale_version  INTEGER NOT NULL CHECK (scale_version >= 1),
    confidence     TEXT NOT NULL CHECK (confidence IN ('low','medium','high')),
    rationale      TEXT,
    created_at     INTEGER NOT NULL,
    CHECK (rationale IS NULL OR length(rationale) <= 500),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (skill_id, workspace_id)
        REFERENCES skills(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_learning_assessments_skill
    ON learning_assessments(skill_id, workspace_id, created_at);

CREATE TABLE learning_assessment_evidence (
    assessment_id  TEXT NOT NULL,
    workspace_id   TEXT NOT NULL,
    evidence_id    TEXT NOT NULL,
    PRIMARY KEY (assessment_id, evidence_id),
    FOREIGN KEY (assessment_id, workspace_id)
        REFERENCES learning_assessments(id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (evidence_id, workspace_id)
        REFERENCES skill_evidence(id, workspace_id) ON DELETE RESTRICT
);

-- Project learning-gap proposals (PAW-LG-007). Learning owns the proposal;
-- the project reference is RESTRICTed so deleting a project with a live
-- proposal is an explicit decision, never a silent cascade (ADR-014).
CREATE TABLE learning_gap_proposals (
    id                 TEXT PRIMARY KEY,
    workspace_id       TEXT NOT NULL,
    project_id         TEXT NOT NULL,
    gap                TEXT NOT NULL CHECK (length(gap) > 0 AND length(gap) <= 500),
    status             TEXT NOT NULL CHECK (status IN ('Proposed','Confirmed','Dismissed')),
    confirmed_item_id  TEXT,
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (project_id, workspace_id)
        REFERENCES projects(id, workspace_id) ON DELETE RESTRICT,
    FOREIGN KEY (confirmed_item_id, workspace_id)
        REFERENCES learning_items(id, workspace_id) ON DELETE RESTRICT
);
CREATE INDEX idx_learning_gap_proposals_workspace_status
    ON learning_gap_proposals(workspace_id, status, updated_at DESC);
CREATE INDEX idx_learning_gap_proposals_project
    ON learning_gap_proposals(project_id, workspace_id);
