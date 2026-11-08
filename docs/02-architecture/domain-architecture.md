# Domain Architecture

**Architecture Baseline:** E0.2 + Learning & Growth extension  
**Decision Status:** Accepted design rules (ADR-001～016)  
**Implementation Status:** Not Implemented  
**Verification Status:** Not Verified

## 1. Domain map

The workbench is a modular monolith. Modules own their concepts and use cases; they communicate through explicit application contracts or domain events, not by writing one another's tables.

| Domain | Owns | Does not own |
|---|---|---|
| Work Management | Workspace, Goal, Area, Project, Task and their lifecycle | Agent provider internals, learning assessments |
| Learning & Growth | Roadmaps, Plans, Items, Sessions, Skills, Skill Evidence, Assessments | Task/Project lifecycle, source documents, code repositories |
| Knowledge & Context | Local notes, context records, KnowledgeRef metadata, permitted indexing/freshness state | Canonical Lingshu/Hindsight content or learning progress |
| AI Collaboration | Agent registration, capabilities, Agent Executions, approvals/integration state | Learning plans, Workflow definitions, provider private databases |
| Engineering | Repository associations, engineering activity metadata, Artifact references/metadata | Canonical Git history or generic Project lifecycle |
| Automation | Triggers, Workflow definitions, Workflow Runs | Unrestricted execution or Agent authorization bypass |
| Creator | Account metadata, ideas, drafts, review and publication records | Unverified platform capabilities |
| System & Security | Preferences, credential references, policies, audit, backup metadata | Plaintext secrets |
| Dashboard | Composed read models | Authoritative business records or direct writes |

## 2. Core relationships

A Workspace scopes Goals, Areas, Projects, Tasks, learning records, and other workspace-owned resources. Projects may reference Tasks, Agent Executions, KnowledgeRefs, Learning Plans/Items, Engineering Artifacts, and Creator assets through published contracts.

Learning Plans and Items may associate with multiple Projects. Learning Items may request Tasks through Work Management commands. Learning Sessions may reference Notes, KnowledgeRefs, Tasks, Projects, and Artifacts. Skill Evidence references source-owned artifacts; it does not take ownership of them.

A Task may reference an Agent Execution or Workflow Run, but their states remain distinct. Each scoped record carries workspace identity and is checked by backend policy. Audit events identify actor, action, target, decision/outcome, timestamp, and correlation ID.

## 3. State and write ownership

Each domain is the sole writer for its authoritative records. Cross-domain views use application queries. Direct cross-domain SQL writes are prohibited. A coordinated transaction is allowed only through an application use case that documents participating repositories and invariants. See `module-boundary-rules.md` and `cross-domain-data-contracts.md`.

## 4. Lifecycle and invariants

- Deletion semantics are explicit: archive/soft-delete where recovery is appropriate; permanent deletion requires a user-visible operation and dependency checks.
- An Agent Execution cannot be submitted before capability validation, resource-scope authorization, policy evaluation, and required approval.
- Provider connectivity is distinct from workbench execution status.
- Knowledge sync failures do not delete last-known reference metadata.
- Learning completion does not automatically increase a Skill assessment.
- Skill assessments are user-confirmed and evidence-linked; Agent recommendations are advisory.
- Project-generated learning needs remain proposals until user acceptance.
- A stale KnowledgeRef does not delete Learning Sessions or evidence history.
- Creator content is not marked published without a publication result or an explicitly user-recorded external publication event.
- Audit records do not contain credentials or unrestricted sensitive payloads.

## 5. Future domains

Calendar, finance, health, and other sensitive domains are not implicitly included. Add them only through explicit scope, data classification, and permission review.