# ADR-027: First-Party Domain Plugin Policy and Learning & Growth Boundary

- **Status:** Accepted (R1)
- **Decision status:** Accepted (repository owner decision, 2026-11-12; R1 clarifications are normative; implementation and verification remain separate gates)
- **Implementation Status:** Not Implemented — no first-party plugin registration exists, so no domain is adopted as a plugin; the Learning & Growth domain (`paw-domain/src/learning.rs`) exists independently of plugin adoption.
- **Verification Status:** Not Verified — plugin-context validation (ownership, no cross-domain table writes through a registry, unavailable KnowledgeRef behaviour) is untested; no evidence record exists.
- **Date:** 2026-11-11 (accepted 2026-11-12)
- **Scope:** First-party domain plugin adoption
- **Related:** ADR-002, ADR-013, ADR-014, ADR-015, ADR-016, ADR-018, ADR-020

## Context

The product needs role-specific plugin combinations, but existing business domains already have accepted ownership boundaries. Learning & Growth is explicitly a first-class domain and must not be reduced to a generic tool plugin or absorbed by Core.

## Decision

1. Existing business domains remain the authoritative bounded contexts.
2. A first-party Domain Plugin is a registration and composition representation of a domain module, not a replacement for the domain model.
3. Learning & Growth remains a first-class domain and may be registered as a built-in first-party plugin.
4. Learning & Growth continues to own Roadmap, Plan, Item, Session, Skill, Evidence, and Assessment.
5. Work Management continues to own Project and Task; Knowledge & Context owns Note and KnowledgeRef; references and cross-domain commands/events follow ADR-014/016/020.
6. Learning completion does not automatically raise Skill level; assessment remains evidence-linked and user-confirmed.
7. Pluginization does not accelerate or expand Learning & Growth delivery beyond the accepted PRD stage scope.
8. The same policy applies to other domains: plugin composition does not transfer domain ownership to Core.

## Consequences

- Role profiles can combine Learning & Growth with Project, Knowledge, or Engineering views.
- The learning domain remains testable and evolvable independently at the module boundary.
- Plugin Registry cannot mutate learning entities or assessments.
- Existing cross-domain data contracts remain authoritative.

## Risks and controls

Risk register carried over from the [R1 cross-ADR review](../04-quality/ADR-023-029-R1-cross-adr-review.md) §3 (risk-control column) and §7 (blocking conditions); the controls are normative per the owner's acceptance conditions.

| Risk | Control |
|---|---|
| Pluginization transfers domain ownership to Core | The domain remains owner of entities, invariants, persistence and events; the registry mutates no domain data |
| Learning & Growth is reduced to a generic tool plugin | Learning stays a first-class domain under ADR-013–016; assessment remains evidence-linked and user-confirmed |
| Plugin adoption silently expands Alpha/Beta scope | Adoption follows domain readiness and roadmap gates, never a target count |
| Cross-domain direct table writes | Cross-domain access only through versioned contracts and events (ADR-014, ADR-016, ADR-020) |

## Validation required

Learning domain ownership tests, no cross-domain table writes, evidence provenance, assessment confirmation, unavailable KnowledgeRef behavior, and versioned contract compatibility.

## Revisit when

The product explicitly changes its learning product scope or adopts a separate learning service. Such a change requires a superseding ADR and migration plan.

## R1 clarification: first-party adoption policy

The initial built-in plugin set is an incremental composition layer over existing domain modules, not a domain redesign. Candidate mappings are subject to the current domain architecture and must not be used to infer new ownership:

| Domain area | Plugin adoption rule | Ownership invariant |
|---|---|---|
| Work Management | Built-in plugin over Work Management use cases | Project and Task remain domain-owned |
| Knowledge & Context | Built-in plugin over knowledge use cases | Note and KnowledgeRef remain domain-owned |
| Learning & Growth | First-class built-in plugin | Roadmap, Plan, Item, Session, Skill, Evidence and Assessment remain Learning-owned |
| Engineering / Delivery | Compose existing approved capabilities only | No direct writes into other domains |
| Personal Productivity | Adopt only after domain contract is explicit | No catch-all Core ownership |
| AI Collaboration | Expose Agent lifecycle through governed capabilities | Provider adapter and external runtime remain separate |
| System & Security | Expose policy/configuration surfaces under human-only admin constraints | Plugin metadata never grants permission |
| Core / Workbench | Platform shell and composition infrastructure | Must not absorb business-domain entities |

Learning & Growth remains governed by ADR-013–016. Learning completion does not automatically increase Skill level; assessment remains evidence-linked and user-confirmed. Pluginization MUST NOT accelerate, widen, or implicitly move Learning delivery into Alpha scope.

## R1 implementation boundary

Plugin adoption is incremental and must follow domain readiness, not a target count. A domain is not eligible merely because it has a page or menu item. It needs an explicit owner, capability contract, data boundary, authorization behavior, and testable lifecycle.

## R1 validation additions

Verify domain ownership and cross-domain contract boundaries; no registry/core direct table writes; Learning evidence provenance and assessment confirmation; unavailable KnowledgeRef behavior; no Alpha scope expansion; plugin adoption does not change existing project/learning milestones.
