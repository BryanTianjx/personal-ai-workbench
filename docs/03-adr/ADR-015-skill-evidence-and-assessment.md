# ADR-015: Skill, Evidence, and Assessment Model

- **Status:** Accepted
- **Decision date:** 2026-11-08
- **Decision scope:** Beta 0.5 onward; schema extension points may be reserved earlier
- **Implementation status:** Not Implemented
- **Verification status:** Not Verified
- **Related:** ADR-013, ADR-014, ADR-016

## Context

A learning system becomes misleading if it treats completed reading or time spent as proof of competence. Skills should be assessed from explicit, reviewable evidence. A single numeric level without context, date, assessor, or evidence can imply more certainty than the data supports.

## Decision

Model skill definitions, evidence, and assessments as separate records.

### Skill
A workspace-scoped competency definition with a stable ID, name, description, optional domain/category, and optional framework/source reference. Skills are not inferred automatically from project names or job titles.

### SkillEvidence
A traceable association between a Skill and one or more evidence references. Evidence kinds include:
- `concept_explanation`
- `guided_exercise`
- `experiment`
- `implementation`
- `test_verification`
- `project_delivery`
- `technical_review`
- `teaching_or_documentation`

Evidence stores a short user-authored summary, evidence kind, source reference(s), observed date, and verification state. It does not copy the entire source artifact.

### LearningAssessment
A time-stamped, user-confirmed assessment that references the Skill, selected evidence, assessment scale/version, assessor (`self` or an explicitly identified reviewer), rationale, and confidence. Assessment history is append-only at the domain level; a newer assessment supersedes the current view but does not erase history.

## Proficiency policy

- No skill level is assumed at initial setup.
- The product may provide an optional scale (for example: Awareness, Working Knowledge, Applied, Independent, Advanced), but the scale and definitions must be versioned and user-visible.
- Completion of a Learning Item or Session does not automatically increase proficiency.
- Agent-generated suggestions are advisory and cannot silently change the user's assessed level.
- Evidence may be marked `proposed`, `reviewed`, or `verified`; `verified` requires a defined verification method and traceable source.
- Users can correct an assessment while retaining an audit/history record.

## Consequences

- Skill claims are explainable and reviewable.
- Evidence can originate from learning exercises or real project delivery.
- Assessment is more complex than a simple progress percentage.
- The product must avoid gamified certainty and fabricated competency scores.

## Alternatives considered

- Automatically increment skill level on course completion: rejected.
- Derive skill from time spent: rejected.
- Store a single mutable level on Skill: rejected; use assessment history.
- Use Agent self-rating as authoritative: rejected.

## Verification requirements

Test evidence provenance, assessment history retention, scale versioning, workspace isolation, and the rule that learning completion alone cannot mutate proficiency.
