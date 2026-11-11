# ADR-025: Workspace Composition Is Separate from the Data Workspace Boundary

- **Status:** Accepted (R1)
- **Decision status:** Accepted (repository owner decision, 2026-11-12; R1 clarifications are normative; implementation and verification remain separate gates)
- **Implementation Status:** Not Implemented — no `WorkspaceComposition` type, layout serialisation or persistence exists (the E2 dashboard read model is unrelated to this ADR).
- **Verification Status:** Not Verified — cross-workspace composition denial, malformed-layout fallback and no-authorization-change-on-switch are untested because no implementation exists.
- **Date:** 2026-11-11 (accepted 2026-11-12)
- **Scope:** Role-based workspace composition and persistence
- **Related:** ADR-006, ADR-013, ADR-017, ADR-020, ADR-024

## Context

PAW already uses Workspace as a logical data and security isolation boundary. The product proposal introduces role-oriented workspaces that arrange plugins and views. Reusing the existing Workspace entity for both concepts would conflate data ownership, access scope, and UI layout.

## Decision

1. Preserve the existing `Workspace` semantics as a data/security scope.
2. Introduce `WorkspaceComposition` as a separate configuration object scoped to an Actor and one existing `data_workspace_id`.
3. Composition stores layout, navigation, widget configuration, enabled plugin preferences, workflow shortcuts, and an optional RoleProfile reference.
4. Composition does not own or duplicate domain business records.
5. Every command/query continues to resolve the actual Data Workspace and enforce backend authorization; frontend filtering is never an isolation control.
6. A missing or invalid composition falls back to a safe default without changing business data.

## Consequences

- Multiple role experiences can operate over the same data Workspace.
- A Role Profile can be reused across workspaces, while each WorkspaceComposition remains explicitly scoped.
- Composition migration/versioning is separate from domain data migration.

## Risks and controls

Risk register carried over from the [R1 cross-ADR review](../04-quality/ADR-023-029-R1-cross-adr-review.md) §3 (risk-control column) and §7 (blocking conditions); the controls are normative per the owner's acceptance conditions.

| Risk | Control |
|---|---|
| WorkspaceComposition is used as an isolation or tenancy control | Composition stores references and presentation configuration only; Data Workspace scope and backend authorization stay authoritative (ADR-003, ADR-006, ADR-017) |
| Composition references unauthorized or cross-workspace records | Every referenced ID is resolved against the active Data Workspace and current policy at use time |
| A malformed or incompatible composition blocks the UI or corrupts data | Safe-default fallback on missing/corrupt/unknown schema; composition never owns or duplicates domain records |
| Composition migration is conflated with domain data migration | Separate versioning and migration tracks (ADR-020, ADR-028) |

## Validation required

Cross-workspace composition denial, malformed-layout fallback, profile reuse without data leakage, and no permission change on composition switch.

## Revisit when

A distinct multi-user collaboration or organization model is approved. Such a model must preserve the difference between data tenancy and presentation composition.

## R1 clarification: composition contract

`WorkspaceComposition` is a presentation/configuration aggregate scoped to an Actor and exactly one existing `data_workspace_id`. It is not the Data Workspace, not a tenant boundary, and not a new owner of domain data. A profile may be reused as a template, but each composition is separately scoped and versioned.

The composition may contain five categories of references/configuration:

1. Plugin selection and ordering preferences.
2. Navigation structure and selected entry points.
3. Dashboard/view/widget layout and presentation settings.
4. Workflow shortcuts and references to existing workflow definitions.
5. Agent composition references, never credentials or grants.

All referenced IDs MUST be resolved against the active Data Workspace and current authorization policy at use time. Composition filtering is not access control. A malformed, missing, incompatible or unauthorized composition MUST fall back to a safe default and MUST NOT change business data.

## R1 implementation boundary

This ADR does not introduce a new data tenancy model, cross-workspace sharing, organization-wide composition, arbitrary executable widgets, or a second database. Exact persistence schema, layout serialization, version migration and UI contribution API require later implementation specifications.

## R1 validation additions

Test cross-workspace composition reads/writes; verify composition cannot reference unauthorized records; verify changing composition does not change query/command authorization; test corrupt and unknown schema fallback; test profile reuse with distinct composition scope; confirm composition contains references/configuration only.
