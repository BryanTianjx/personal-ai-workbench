# Domain Architecture

**Status:** Proposed — E0.2

## 1. Domain map

The workbench is a modular monolith. Modules own their concepts and use cases; they communicate through explicit application contracts or domain events, not by writing one another's tables.

| Module | Owns | Does not own |
|---|---|---|
| Workspace | Workspace identity, settings, context | Project execution details |
| Project | Project metadata, lifecycle, relationships | Agent provider internals |
| Task | Tasks, status, dependencies, assignments | External scheduler internals |
| AI/Agent | Provider connections, capabilities, executions, approvals | Provider private databases |
| Knowledge | External references, metadata, indexing/sync state | Hindsight/Lingshu source content |
| Creator | Account metadata, ideas, drafts, review and publication records | Unverified platform capabilities |
| Automation | Triggers, workflow definitions, run records | Unrestricted execution |
| System & Security | Preferences, credential references, policies, audit, backup metadata | Plaintext secrets |

Dashboard is a composition/read-model surface, not a source of truth.

## 2. Core relationships

A Workspace contains Projects and workspace-scoped resources. Projects may contain Tasks, Agent executions, Knowledge references, and Creator assets. A Task may reference an Agent execution, but task and execution states remain distinct. Executions belong to a workspace and may optionally reference a project/task. Knowledge references store source identity, provider, scope, observed version/hash when available, and sync status. Audit events identify actor, action, target, outcome, timestamp, and correlation ID.

Use stable opaque IDs, not display names or filesystem paths as identity.

## 3. State ownership

Each module is the sole writer for its authoritative records. Cross-module views use application queries. Avoid direct cross-module SQL writes. A coordinated transaction is allowed only through an application use case that documents participating repositories and invariants.

## 4. Lifecycle and invariants

- Deletion semantics are explicit: archive/soft-delete where recovery is appropriate; permanent deletion requires a user-visible operation and dependency checks.
- An Agent execution cannot be submitted before validation, capability checks, scope authorization, and required approval.
- Provider status is distinct from workbench execution status.
- Knowledge sync failures do not delete last-known reference metadata.
- Creator content is not marked published without a publication result or an explicitly user-recorded external publication event.
- Audit records do not contain credentials or unrestricted sensitive payloads.

## 5. Future domains

Calendar, finance, health, and other sensitive domains are not implicitly included. Add them only through explicit scope, data classification, and permission review.
