# ADR-013: Learning & Growth as a First-Class Domain

- **Status:** Accepted
- **Decision date:** 2026-11-08
- **Decision scope:** Alpha 0.1 through V1.0

Implementation Status: Not Implemented
Verification Status: Not Verified

- **Related:** ADR-002, ADR-003, ADR-006, ADR-007, ADR-014, ADR-015, ADR-016

## Context

Personal AI Workbench coordinates personal work and long-running self-directed development. Learning is not merely a note category or a task label: it has its own lifecycle, planning hierarchy, competency model, practice records, and evidence-based assessment. It also produces useful outputs for projects, while projects reveal capability gaps that should feed back into learning plans.

Putting all learning records inside Task would conflate the intent to learn with an individual action. Putting them inside Knowledge would conflate learning progress with source-material ownership. Neither model can express a roadmap, learning plan, session history, skill evidence, and assessment without violating domain ownership.

## Decision drivers

- Preserve explicit domain ownership in the modular monolith.
- Connect learning outcomes to real project practice and delivery.
- Keep learning records useful even when external knowledge services or Agents are unavailable.
- Avoid asserting skill levels without user assessment and traceable evidence.
- Keep Alpha 0.1 delivery small and testable.

## Decision

Introduce **Learning & Growth** as an independent business domain.

The domain owns:

- Learning Roadmaps and Learning Plans.
- Learning Items and Learning Sessions.
- Skills and user-confirmed proficiency assessments.
- Skill Evidence and assessment history.
- Learning progress and learning-specific review state.

The domain does not own:

- General-purpose tasks or project lifecycle.
- Source documents, notes, external knowledge assets, or their canonical content.
- Code repositories, engineering artifacts, or Agent execution internals.
- Generic audit, authorization, file storage, or notification infrastructure.

Learning & Growth interacts with other domains only through published application contracts, stable identifiers, and versioned domain events. It may request a Task for an actionable learning activity, but the Task remains owned by Work Management. It may associate an Artifact or KnowledgeRef as evidence or material, but the referenced object remains owned by its source domain.

## Domain model

- `LearningRoadmap`: a durable direction or curriculum.
- `LearningPlan`: a time-bounded or milestone-bounded plan within a roadmap.
- `LearningItem`: a planned topic, objective, practice, or checkpoint.
- `LearningSession`: an actual learning activity record.
- `Skill`: a competency definition.
- `SkillEvidence`: a traceable piece of evidence supporting a skill assessment.
- `LearningAssessment`: a user-confirmed assessment at a point in time.

A learning record may be associated with zero or more Projects. A Project may be associated with multiple Learning Plans and Learning Items.

## Consequences

### Positive

- Learning becomes a durable first-class capability rather than a note/task convention.
- Project-driven learning gaps can be captured without making Project depend on Learning internals.
- Skill claims can be tied to evidence and explicit assessment.
- The product can support personal growth without replacing a curated knowledge base.

### Trade-offs

- Adds domain concepts, associations, migrations, and UI complexity.
- Requires explicit cross-domain contracts and lifecycle semantics.
- Full Learning Center delivery must be phased to avoid expanding Alpha 0.1.

## Alternatives considered

1. **Learning as Task type:** rejected; does not model roadmaps, sessions, skills, and evidence cleanly.
2. **Learning inside Knowledge:** rejected; learning progress and knowledge ownership are different concerns.
3. **External LMS integration only:** rejected; the product needs a personal project-linked learning loop even when offline.

## Implementation and verification requirements

- Alpha 0.1: reserve identifiers/contracts and permit generic Task/Project/Artifact associations; do not implement the full learning UI or competency engine.
- Alpha 0.2: implement Roadmap, Plan, Item, and Session minimum viable lifecycle.
- Beta 0.5: implement Skill, SkillEvidence, and user-confirmed assessment.
- V1.0+: add recommendations and analytics only after evidence quality and privacy rules are validated.

Acceptance requires domain-boundary tests, migration tests, workspace-isolation tests, and tests proving Learning cannot directly mutate Task, Project, Knowledge, or Artifact records.


## Risks

- Cross-domain coupling could grow if consumers bypass published contracts; enforce ownership through module visibility and contract tests.
- Evidence quality or assessment semantics may be misunderstood; keep user confirmation and provenance visible.

## Validation

- Validate workspace isolation, reference authorization, lifecycle behavior, and idempotent event consumption.
- Keep implementation and verification marked as pending until corresponding code and repeatable tests exist.
