# ADR-014: Learning and Project Integration Contract

- **Status:** Accepted
- **Decision date:** 2026-11-08
- **Decision scope:** Alpha 0.1 through V1.0

Implementation Status: Not Implemented
Verification Status: Not Verified

- **Related:** ADR-013, ADR-015, ADR-016

## Context

Learning should be driven by both deliberate plans and real engineering work. Project work can expose a knowledge or skill gap; learning can then produce practice results that improve a project. A direct dependency between Project and Learning modules would create a cycle and make both domains difficult to evolve.

## Decision

Use **loose, identifier-based, event-assisted integration**.

1. Learning & Growth may associate a Learning Plan or Learning Item with a Project through a published association contract.
2. Work Management remains the sole owner of Project and Task records.
3. Learning & Growth may request creation of a Task through the Work Management application command. It must not write Task tables directly.
4. A Project may emit a versioned `ProjectLearningNeedRaised` event or equivalent application notification containing only the minimum necessary metadata.
5. Learning & Growth may consume that event idempotently and create a proposed Learning Item or inbox entry. It must not silently create a committed Learning Plan or change the user's learning priority.
6. A completed Learning Item or Session may publish `LearningOutcomeRecorded`, with references to evidence artifacts and related Project IDs. Project may consume this for a read model or review prompt; it must not rewrite learning assessment state.

## Contract semantics

- Associations use stable opaque IDs, not project names, local paths, or repository URLs as primary identity.
- Every association is workspace-scoped and validated by the owning module.
- Associations have explicit role and lifecycle (`active`, `completed`, `detached`); deleting or archiving one side must not silently delete the other side.
- Event delivery is at-least-once; consumers must be idempotent and retain event/version metadata.
- Event payloads must not include source document bodies, credentials, unrestricted notes, or large artifact contents.
- Project-generated learning needs are proposals until user acceptance.
- Learning-to-Task links express an actionable step; the Learning Item remains the learning objective and owns learning progress.

## Consequences

- Project and Learning remain independently evolvable.
- The user can navigate in both directions without shared table ownership.
- Eventual consistency is acceptable for suggestions and summaries; critical invariants remain in the source module's transaction.
- The application needs a small event/outbox mechanism or equivalent reliable local dispatch before event-driven automation is enabled.

## Alternatives considered

- Direct cross-module SQL joins and writes: rejected.
- Project owns learning plans: rejected; learning plans span multiple projects and personal goals.
- Learning owns Project lifecycle: rejected; violates Work Management ownership.
- Synchronous network-style event bus for local workflows: rejected for Alpha; use in-process contracts and durable local event records only where reliability requires them.

## Verification requirements

Test workspace isolation, duplicate event delivery, project archive/detach behavior, task creation through the owning command, and preservation of Learning records when a Project is archived or removed.


## Risks

- Cross-domain coupling could grow if consumers bypass published contracts; enforce ownership through module visibility and contract tests.
- Evidence quality or assessment semantics may be misunderstood; keep user confirmation and provenance visible.

## Validation

- Validate workspace isolation, reference authorization, lifecycle behavior, and idempotent event consumption.
- Keep implementation and verification marked as pending until corresponding code and repeatable tests exist.
