# ADR-021: Stage-to-Version Mapping and Windows-Only Alpha 0.1 Delivery

- **Status:** Accepted
- **Date:** 2026-11-08
- **Decision scope:** Milestone planning (E stages vs product versions) and platform delivery scope
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified
- **Related:** ADR-005, ADR-006, ADR-018, E0.3-architecture-adjustment-plan

## Context

The E0.3 delivery scope (`E0.3-architecture-adjustment-plan.md` §4) defines engineering stages E1 through E6+, while the roadmap defines product versions (Alpha 0.1, Alpha 0.2, Beta 0.5, V1.0, V1.1+, V1.3+). No authoritative mapping between the two existed. The roadmap's legacy milestone gates (Gate E1–E5) also used E-numbers whose semantics no longer match the stage definitions (for example, roadmap "Gate E3" described Agent/knowledge/learning integration, while stage E3 is backup/audit/operation primitives).

At the same time, Alpha 0.1 has been documented as a dual-platform (Windows and macOS) deliverable across the README, product scope, roadmap, product vision, the requirements baseline (PAW-N-001～004), platform architecture, the offline acceptance matrix, and ADR-005/006 validation sections. In contrast, every E0.3.2 engineering artifact and the E1 delivery plan are centered on a Windows vertical slice, and the PRD already states a Windows-first strategy. For a solo developer with AI-assisted delivery, dual-platform native acceptance (macOS signing/notarization, real-hardware verification, a doubled offline acceptance matrix) is a material recurring cost on the critical path to the first working product.

## Decision proposal

### 1. Stage-to-version mapping (authoritative)

Stage numbering follows the E0.3 delivery scope table. One version closes only when all of its stages have passed their gates.

| Stages | Product version | Closing gate |
|---|---|---|
| E1 (skeleton, Workspace context, SQLite init/migration) + E2 (Goal/Area/Project/Task vertical slice, scoped search, attachments) + E3 (backup/restore, audit, Operation/Outbox primitives) | Alpha 0.1 | Gate Alpha 0.1: Windows-native build/install/launch, offline core loop, restart persistence, backup/restore, migration and isolation tests pass with evidence |
| E4 (KnowledgeRef, basic Learning lifecycle) + E5 (Agent adapter, mediated authorization) | Alpha 0.2 | Gate Alpha 0.2: cross-domain contract tests, authorization gate, fake-provider conformance, unknown-outcome reconciliation; ADR-022 ratified before any external Agent execution is enabled |
| E6.x increments (Skill/Evidence/Assessment; Creator; Automation) | Beta 0.5 | Gate Beta 0.5: evidence-based assessment, creator review flow and automation governance accepted per increment |
| Remaining cross-domain closure (explainable growth reports, reliability hardening) | V1.0 | Gate V1.0: full tracked loop across goals/projects/learning/evidence/agents/knowledge/content |
| Server, multi-device sync, multi-agent collaboration | V1.1+ / V1.3+ (not mapped to E stages) | Requires separate planning and, where architectural, a superseding ADR |

The roadmap milestone gates are renamed to version-based gates (Gate Alpha 0.1, Gate Alpha 0.2, Gate Beta 0.5, release gate) to remove the numbering collision with stages E1–E6.

### 2. Alpha 0.1 is Windows-only

Alpha 0.1 delivery and acceptance cover the Windows desktop application only: native build, install, launch, offline operation, backup/restore, and upgrade verification on Windows. macOS becomes a later-version target delivered behind a dedicated macOS readiness milestone; no macOS acceptance is claimed for Alpha 0.1 or any version until that milestone closes.

The architecture remains platform-neutral. ADR-018 port boundaries, platform-architecture isolation rules, and platform-native path handling are unchanged; no macOS-specific code is required now, and nothing in this decision forecloses macOS delivery later.

Costs explicitly accepted: delayed macOS availability; deferred discovery of macOS-specific defects (later rework risk); a second full native acceptance pass at the macOS readiness milestone. Mitigations: keep platform behavior behind infrastructure ports; retain the offline acceptance matrix macOS column marked Deferred rather than deleting it; re-run the full matrix on real macOS hardware when the milestone activates.

## Alternatives

1. Keep dual-platform Alpha 0.1 and accept the cost: rejected — the doubled native acceptance matrix and macOS signing/notarization overhead sit on the critical path of the first working product without a corresponding near-term need.
2. Drop macOS entirely from the product vision: rejected — cross-platform boundaries stay reserved, consistent with the PRD's Windows-first strategy and the platform-neutral architecture.
3. Windows-only Alpha 0.1 with a macOS readiness milestone: proposed.

## Consequences

**Positive:** the first product version has one unambiguous platform target; E1–E3 engineering work and acceptance evidence stop being split across two platforms; the stage/version mapping removes gate-numbering ambiguity for planning and traceability.

**Negative:** the dual-platform promise in earlier documents is walked back; macOS users must wait; a later macOS milestone must re-validate the full offline acceptance matrix.

## Risks and mitigations

- Scope creep back into "dual-platform soon": version gates reference ADR-021; platform scope changes require a new ADR.
- Windows-only shortcuts leaking into shared code (hard-coded paths, Windows-only assumptions in domain/application): ADR-018 dependency rules and platform-architecture isolation rules still apply; architecture checks and path handling tests remain in scope.
- Deferred macOS defects accumulate silently: the macOS readiness milestone re-runs the complete offline acceptance matrix; the deferred matrix column is preserved, not deleted.
- Stage/version mapping drifts as new stages appear: this ADR is authoritative; new stages or re-mapping require a superseding ADR and roadmap update.

## Validation required

Documentation validator passes (requirement traceability, ADR index, links); a repository search confirms no remaining Alpha 0.1 dual-platform delivery claims outside reserved-architecture/future-milestone context; roadmap, product scope, README, product vision, platform architecture, offline acceptance matrix and PAW-N requirement text are consistent with this mapping; PAW-N-001 acceptance text updated through the requirements freeze rule with ADR-021 recorded as the P0 change review.

## Revisit when

The macOS readiness milestone activates, a second platform becomes a release requirement, or the E-stage structure changes. All three require a superseding ADR.

## E0.3.2-era ratification

**Decision:** Accepted on 2026-11-08 by the repository owner as part of the E0.3 post-ratification governance pass. Acceptance establishes the planning baseline only; implementation remains Not Implemented and verification remains Not Verified.
