# ADR-016: Learning and Knowledge Ownership Boundary

- **Status:** Accepted
- **Decision date:** 2026-11-08
- **Decision scope:** Alpha 0.1 through V1.0
Implementation Status: Not Implemented
Verification Status: Not Verified
- **Related:** ADR-003, ADR-013, ADR-015

## Context

Learning activities depend on books, standards, articles, notes, code, experiments, and external knowledge repositories. The workbench must connect these materials to learning without becoming a second canonical knowledge store or duplicating entire repositories.

## Decision

Separate **learning process ownership** from **knowledge asset ownership**.

Learning & Growth owns:
- What the user intends to learn.
- Learning plans, objectives, sessions, reflections, and assessments.
- The relationship between learning outcomes and competency evidence.

Knowledge & Context owns:
- Local notes and work-context records.
- KnowledgeRef metadata and external source references.
- Permitted local indexing/projection metadata and source freshness state.

External systems retain authority over their own canonical content:
- Lingshu Knowledge remains the curated knowledge repository.
- Hindsight remains an Agent memory service.
- Protocol standards and source documents remain in their authoritative repositories.
- Git repositories remain authoritative for source code and engineering history.

Learning records may reference a KnowledgeRef, Note, Artifact, Project, or Task by stable ID. They must not embed or silently mirror the full source content. A user may explicitly create a local excerpt or learning note; that excerpt is a new user-owned record with provenance, not an implicit copy of the source.

## Reference contract

A reference should carry, where available: `ref_id`, `workspace_id`, `source_domain`, `source_system`, `source_object_id`, `title_snapshot`, `uri_or_locator`, `version_or_revision`, `observed_at`, `availability_status`, and `access_policy`. Optional snapshots are display aids and must not be treated as canonical source content.

If a source becomes unavailable, the reference remains and is marked unavailable/stale; learning history and assessment evidence are not deleted.

## Consequences

- The workbench remains a coordination and context layer.
- Learning sessions can cite source materials without owning them.
- External knowledge services can be replaced without rewriting learning history, provided stable reference identity is preserved.
- Reference resolution and permissions must be checked at access time; storing a URI is not authorization.

## Alternatives considered

- Copy all Lingshu/Hindsight content into the workbench: rejected due to duplication, ownership ambiguity, privacy, and synchronization cost.
- Store only raw URLs: rejected because URLs are mutable and insufficient for provenance.
- Make Knowledge own learning plans: rejected because learning lifecycle is not source-material lifecycle.

## Verification requirements

Test unavailable-source behavior, reference provenance, permission checks at resolution time, and preservation of learning records when external references are stale or deleted upstream.


## Risks

- Cross-domain coupling could grow if consumers bypass published contracts; enforce ownership through module visibility and contract tests.
- Evidence quality or assessment semantics may be misunderstood; keep user confirmation and provenance visible.

## Validation

- Validate workspace isolation, reference authorization, lifecycle behavior, and idempotent event consumption.
- Keep implementation and verification marked as pending until corresponding code and repeatable tests exist.
