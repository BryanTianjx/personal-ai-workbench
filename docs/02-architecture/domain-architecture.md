# Domain Architecture

**Architecture Baseline:** E0.2 Candidate  
**Decision Status:** Accepted design rules  
**Implementation Status:** Not Implemented  
**Verification Status:** Not Verified

## 1. Domain map
The workbench is a modular monolith. Modules own their concepts and use cases; they communicate through explicit application contracts or domain events, not by writing one another's tables.

| Module | Owns | Does not own |
|---|---|---|
| Workspace | Workspace identity, settings, membership/context | Project execution details |
| Project | Project metadata, lifecycle, relationships | Agent provider internals |
| Task | Tasks, status, dependencies, assignments | Workflow scheduling or Agent execution state |
| AI/Agent | Provider connections, capabilities, Agent Executions, approvals | Workflow definitions, provider private databases |
| Knowledge | External references, metadata, permitted indexing/sync state | Hindsight/Lingshu source content |
| Creator | Account metadata, ideas, drafts, review and publication records | Unverified platform capabilities |
| Automation | Triggers, workflow definitions, Workflow Runs | Unrestricted execution or Agent authorization bypass |
| System & Security | Preferences, credential references, policies, audit, backup metadata | Plaintext secrets |
| Dashboard | No authoritative business records; composed read model | Direct writes or private-table ownership |

## 2. Core relationships
A Workspace contains Projects and workspace-scoped resources. Projects may reference Tasks, Agent Executions, Knowledge references, and Creator assets. A Task may reference an Agent Execution or Workflow Run, but their states remain distinct. Each scoped record carries workspace identity and is checked by backend policy. Audit events identify actor, action, target, decision/outcome, timestamp, and correlation ID.

Use stable opaque IDs, not display names or filesystem paths as identity.

## 3. State and write ownership
Each module is the sole writer for its authoritative records. Cross-module views use application queries. Direct cross-module SQL writes are prohibited. A coordinated transaction is allowed only through an application use case that documents participating repositories and invariants. See `module-boundary-rules.md`.

## 4. Lifecycle and invariants
- Deletion semantics are explicit: archive/soft-delete where recovery is appropriate; permanent deletion requires a user-visible operation and dependency checks.
- An Agent Execution cannot be submitted before capability validation, resource-scope authorization, policy evaluation, and required approval.
- Provider connectivity is distinct from workbench execution status.
- Knowledge sync failures do not delete last-known reference metadata.
- Creator content is not marked published without a publication result or an explicitly user-recorded external publication event.
- Audit records do not contain credentials or unrestricted sensitive payloads.

## 5. Future domains
Calendar, finance, health, and other sensitive domains are not implicitly included. Add them only through explicit scope, data classification, and permission review.
