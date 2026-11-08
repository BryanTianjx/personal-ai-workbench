# Cross-Domain Data Contracts

**Baseline:** E0.2 + Learning & Growth extension
**Status:** Accepted design contract
**Implementation:** Not Implemented
**Verification:** Not Verified
**Related ADRs:** ADR-013, ADR-014, ADR-015, ADR-016

## 1. Purpose

Define stable cross-domain references and interaction rules for Work Management, Learning & Growth, Knowledge & Context, Engineering, AI Collaboration, Automation, Creator, and System & Security. These contracts do not authorize direct access to another module's tables.

## 2. Domain ownership

| Domain | Authoritative records | Published references |
|---|---|---|
| Work Management | Workspace, Goal, Area, Project, Task | WorkspaceId, ProjectId, TaskId |
| Learning & Growth | Roadmap, Plan, Item, Session, Skill, Evidence, Assessment | LearningPlanId, LearningItemId, LearningSessionId, SkillId, EvidenceId |
| Knowledge & Context | Note, KnowledgeRef, local context metadata | NoteId, KnowledgeRefId |
| Engineering | Repository association, engineering activity metadata, Artifact metadata where designated | RepositoryRefId, ArtifactId |
| AI Collaboration | Agent registration, capability declarations, Agent Execution | AgentId, AgentExecutionId |
| Automation | Trigger, Workflow Definition, Workflow Run | WorkflowId, WorkflowRunId |
| Creator | Idea, Draft, Content Version, Review, Publication Record | ContentId, ContentVersionId |
| System & Security | Policy, Grant, Approval, Audit Event, Backup metadata | PolicyId, ApprovalId, AuditEventId |

A domain is the sole authoritative writer of its records. A reference does not transfer ownership.

## 3. Common reference envelope

```json
{
  "ref_id": "opaque-stable-id",
  "workspace_id": "opaque-workspace-id",
  "source_domain": "engineering",
  "source_type": "artifact",
  "source_object_id": "opaque-source-id",
  "relationship": "evidence",
  "created_at": "RFC3339 timestamp",
  "created_by": "actor-id",
  "status": "active"
}
```

Rules:

- IDs are opaque and stable; names, paths, URLs, and titles are not primary keys.
- `workspace_id` is mandatory for workspace-scoped records.
- Source domain validates the referenced object and workspace.
- Relationship values are controlled per consuming domain, not arbitrary SQL joins.
- Optional `title_snapshot` and `locator_snapshot` are display hints, not canonical content.
- Access must be re-authorized when resolving a reference.
- A stale/missing target changes reference availability, not the historical record that created it.

## 4. Learning-specific association contract

Learning may associate to:

- `Project` with relationship `applies_to` or `practice_context`.
- `Task` with relationship `action_step`.
- `Note` with relationship `reflection` or `learning_note`.
- `KnowledgeRef` with relationship `source_material`.
- `Artifact` with relationship `evidence` or `output`.
- `Skill` with relationship `develops` or `demonstrates`.

Association cardinality:

- Learning Plan ↔ Project: many-to-many.
- Learning Item ↔ Project: many-to-many.
- Learning Item → Task: zero or many action-step references; Task remains Work Management-owned.
- Learning Session ↔ Note/Artifact/KnowledgeRef: many-to-many references.
- Skill ↔ SkillEvidence: one-to-many.
- SkillEvidence → Artifact/Note/KnowledgeRef/Project/Task: one or more source references, subject to evidence type and provenance rules.

All relationships must be workspace-consistent. Cross-workspace association is rejected by default.

## 5. Commands and queries

Cross-domain writes use the source domain's published command:

- `CreateTaskForLearningItem` is a Work Management command requested by Learning.
- `ProposeLearningNeedFromProject` is a Learning application command invoked from a Project review interaction or event consumer.
- `AttachArtifactAsSkillEvidence` is a Learning command after Engineering/Knowledge confirms the Artifact reference.
- `ResolveKnowledgeReference` is a Knowledge query with authorization and freshness checks.

These are conceptual contract names; implementation may use Rust traits/application services but must preserve semantics.

Cross-domain reads use query DTOs/read models. Do not expose another module's repository or private ORM entity.

## 6. Domain events

Events are versioned, workspace-scoped, minimal, and idempotently consumable.

| Event | Producer | Consumer | Meaning |
|---|---|---|---|
| `ProjectLearningNeedRaised.v1` | Work Management / Project | Learning & Growth | Project review proposes a learning gap; not an approved plan |
| `LearningPlanAssociatedWithProject.v1` | Learning & Growth | Dashboard / Project read model | A plan is linked to a project |
| `LearningOutcomeRecorded.v1` | Learning & Growth | Project / Dashboard | Learning result is available for review |
| `SkillEvidenceAdded.v1` | Learning & Growth | Dashboard | Evidence record added; not necessarily verified |
| `SkillAssessmentConfirmed.v1` | Learning & Growth | Dashboard | User-confirmed assessment changed |
| `KnowledgeReferenceAvailabilityChanged.v1` | Knowledge & Context | Learning & Growth | A referenced source became stale/unavailable |

Event envelope:

```json
{
  "event_id": "opaque-event-id",
  "event_type": "ProjectLearningNeedRaised",
  "schema_version": 1,
  "workspace_id": "opaque-workspace-id",
  "aggregate_id": "opaque-source-id",
  "occurred_at": "RFC3339 timestamp",
  "correlation_id": "opaque-correlation-id",
  "payload": {}
}
```

Delivery semantics are at-least-once. Consumers must deduplicate by `event_id`, validate `schema_version`, and record processing outcome. Event payloads must not contain credentials, full notes, document bodies, or raw source files.

## 7. Lifecycle and deletion rules

- Archive is not delete.
- Detaching a relationship does not delete either endpoint.
- Deleting a source object must invoke its domain's deletion policy and publish a tombstone/availability event where needed.
- Learning Sessions, Assessments, and Audit Events are historical records; they must not be silently cascade-deleted because a Project or KnowledgeRef is removed.
- Permanent deletion of user data must explicitly report downstream effects and respect retention/legal constraints.

## 8. Transaction and consistency rules

- A module owns its transaction and invariants.
- No database transaction may remain open across network/provider calls.
- Local command orchestration may coordinate multiple module commands only through an application use case with documented rollback/compensation.
- Use a transactional outbox or durable local event log for events that must survive process restart.
- Dashboard projections are rebuildable and never authoritative.
- Eventual consistency is acceptable for suggestions and summaries, not for authorization or workspace isolation.

## 9. SQLite mapping guidance

For workspace-scoped tables:

- Include `workspace_id NOT NULL`.
- Use composite foreign keys or equivalent repository validation to prevent cross-workspace links.
- Index common association lookups by `(workspace_id, target_id)` and `(workspace_id, source_id)`.
- Store event schema version and processing status.
- Keep external URI fields separate from opaque IDs.
- Do not introduce a vector database, PostgreSQL, or remote service as a prerequisite for Alpha 0.1.

Exact table names and SQL migrations are implementation design, not fixed by this logical contract.

## 10. Security and privacy

- Reference resolution is an authorization boundary.
- A stored reference or URL does not grant permission.
- Learning summaries should avoid copying confidential project data.
- Agent-produced learning suggestions are untrusted input until validated and accepted.
- Audit records capture actor, action, target, outcome, timestamp, and correlation ID without secrets or unnecessary content.

## 11. Contract test requirements

1. Reject cross-workspace references.
2. Reject unknown source domain/type combinations.
3. Confirm that consumer modules cannot mutate source-owned records.
4. Deliver duplicate events without duplicate Learning Items or assessments.
5. Preserve sessions and assessments when referenced projects or sources are archived/unavailable.
6. Enforce authorization at reference resolution.
7. Validate event schema-version compatibility and failure handling.
8. Restore database backups with all Learning associations and event metadata intact.
