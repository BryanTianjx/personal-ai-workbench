-- Migration 0009: AI Collaboration agent governance (E5-02c/E5-03/E5-04)
-- Schema reference: docs/05-architecture-evolution/E5-implementation-specification.md §7/§10
-- Governance source: ADR-026 (capability registry), ADR-029 (approval binding),
-- agent-authorization-model.md §2 (grants), agent-adapter-architecture.md §3
-- (execution lifecycle). State strings mirror the paw_domain::ai_collaboration
-- state machines exactly. The schema_migrations ledger is maintained by the
-- migration runner.

-- Explicit agent grants (authorization model §2). Grants are workspace-bound
-- by construction: a grant without a workspace would widen scope and cannot
-- be represented. Rows are read fresh per evaluation so revocation takes
-- effect at the next side effect (ADR-029 §4.4).
CREATE TABLE agent_grants (
    id              TEXT PRIMARY KEY,
    subject_id      TEXT NOT NULL,             -- 'agent:<registration_id>'
    capabilities    TEXT NOT NULL,             -- JSON array of capability ids
    workspace_id    TEXT NOT NULL,
    tools           TEXT NOT NULL,             -- JSON array of tool keys ('[]' = none)
    valid_from_ms   INTEGER NOT NULL,
    valid_until_ms  INTEGER,
    created_by      TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    CHECK (json_valid(capabilities) AND json_valid(tools)),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_agent_grants_subject
    ON agent_grants(subject_id, workspace_id);

-- Workbench-owned agent registrations (AI Collaboration domain). The
-- declared capability list is metadata for governance and display — never
-- a grant (ADR-026 R1).
CREATE TABLE agent_registrations (
    id                     TEXT PRIMARY KEY,
    workspace_id           TEXT NOT NULL,
    display_name           TEXT NOT NULL,
    adapter_kind           TEXT NOT NULL CHECK (adapter_kind IN
      ('fake','cli_hermes','cli_codex','api_custom')),
    declared_capabilities  TEXT NOT NULL,      -- JSON array of capability ids
    status                 TEXT NOT NULL CHECK (status IN ('Active','Suspended','Retired')),
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    CHECK (json_valid(declared_capabilities) AND length(display_name) > 0),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT
);
CREATE INDEX idx_agent_registrations_workspace_status
    ON agent_registrations(workspace_id, status);

-- Agent execution records: the provider-facing lifecycle (adapter §3).
-- The durable Operation is the cross-cutting envelope; executions link to
-- it by id without sharing a state machine (ADR-019). Execution history is
-- never cascade-deleted: a registration is queryable only while its
-- workspace exists, and statuses TimedOut/OutcomeUnknown stay visible until
-- reconciled.
CREATE TABLE agent_executions (
    id                          TEXT PRIMARY KEY,
    registration_id             TEXT NOT NULL,
    workspace_id                TEXT NOT NULL,
    capability_id               TEXT NOT NULL,
    capability_contract_version TEXT NOT NULL,
    plan_digest                 TEXT NOT NULL CHECK (length(plan_digest) = 64),
    tool_adapter                TEXT NOT NULL,
    target_identity             TEXT NOT NULL,
    material_params_digest      TEXT NOT NULL CHECK (length(material_params_digest) = 64),
    parameters_json             TEXT NOT NULL CHECK (json_valid(parameters_json)),
    status                      TEXT NOT NULL CHECK (status IN
      ('Created','Validating','AwaitingApproval','Approved','Submitted','Running',
       'Completed','Failed','Cancelled','TimedOut','OutcomeUnknown')),
    operation_id                TEXT,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    FOREIGN KEY (registration_id) REFERENCES agent_registrations(id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (operation_id) REFERENCES operations(id) ON DELETE RESTRICT
);
CREATE INDEX idx_agent_executions_workspace_status_updated
    ON agent_executions(workspace_id, status, updated_at DESC);
CREATE INDEX idx_agent_executions_registration
    ON agent_executions(registration_id);
-- The reconciliation queue: unresolved executions by workspace.
CREATE INDEX idx_agent_executions_unresolved
    ON agent_executions(workspace_id, updated_at)
    WHERE status IN ('TimedOut','OutcomeUnknown');

-- Human approval records (ADR-029 §4.3). The record binds the canonical
-- plan digest, actor, agent, workspace, capability + contract version,
-- tool/adapter, target identity and the material-parameters digest; secret
-- values and raw payloads are never stored. Expiry is enforced at use time
-- from expires_at_ms even while the row reads Active.
CREATE TABLE approvals (
    id                          TEXT PRIMARY KEY,
    workspace_id                TEXT NOT NULL,
    canonical_digest            TEXT NOT NULL CHECK (length(canonical_digest) = 64),
    actor_id                    TEXT NOT NULL,
    agent_registration_id       TEXT,
    capability_id               TEXT NOT NULL,
    capability_contract_version TEXT NOT NULL,
    tool_adapter                TEXT NOT NULL,
    target_identity             TEXT NOT NULL,
    material_params_digest      TEXT NOT NULL CHECK (length(material_params_digest) = 64),
    side_effect_set             TEXT NOT NULL,   -- JSON array of effect ids
    risk_class                  TEXT NOT NULL,
    expires_at_ms               INTEGER NOT NULL,
    attempt_policy              TEXT NOT NULL CHECK (attempt_policy IN ('single','bounded')),
    attempts_allowed            INTEGER NOT NULL CHECK (attempts_allowed >= 1),
    consumed_attempts           INTEGER NOT NULL DEFAULT 0 CHECK (consumed_attempts >= 0),
    status                      TEXT NOT NULL CHECK (status IN
      ('Active','Consumed','Expired','Revoked')),
    created_at                  INTEGER NOT NULL,
    CHECK (json_valid(side_effect_set)),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (agent_registration_id) REFERENCES agent_registrations(id) ON DELETE RESTRICT
);
CREATE INDEX idx_approvals_workspace_status_expires
    ON approvals(workspace_id, status, expires_at_ms);
-- Idempotent decision surface: one open approval per canonical digest.
CREATE UNIQUE INDEX idx_approvals_open_digest
    ON approvals(canonical_digest)
    WHERE status = 'Active';
