# ADR-024: Role Profile Is a User Experience Profile, Not an Authorization Role

- **Status:** Proposed (R1)
- **Decision status:** Proposed / Not Accepted
- **Implementation status:** Not Implemented
- **Verification status:** Not Verified
- **Date:** 2026-11-11
- **Scope:** Role templates and persona-based experience
- **Related:** ADR-003, ADR-005, ADR-013, ADR-017, ADR-020

## Context

Different users and duties need different plugin combinations, navigation, dashboards, workflows, and Agent presets. Reusing security roles for this purpose would couple presentation preferences to access control and create privilege-escalation risk.

## Decision

1. Introduce `RoleProfile` as a non-security persona/experience template.
2. A Role Profile may declare recommended plugins, navigation, views, workflow shortcuts, and Agent Profile references.
3. A Role Profile must not grant Read/Write/Execute/Admin permissions, widen Workspace scope, expose credentials, or bypass approval.
4. Switching or editing a Role Profile has no effect on effective authorization.
5. A user may have multiple profiles and may clone/customize a profile.
6. Security authorization remains governed by the existing System & Security policy and backend authorization pipeline.

## Consequences

- Role-based experiences can be personalized without changing security semantics.
- UI must distinguish “feature is configured” from “action is authorized”.
- The system must compute visible actions from both composition preferences and current backend policy; the latter remains authoritative.

## Required validation

Role switching does not alter grants; unauthorized actions remain denied through direct command invocation; profile export/import contains no secrets or permission grants.

## Revisit when

A multi-user organization/enterprise authorization model is explicitly introduced. That work must define a separate security-role/relationship model and must not repurpose RoleProfile.

## R1 clarification: template lifecycle and security separation

A `RoleProfile` is an experience template, not an authenticated principal, security role, permission set, or Agent identity. It may reference plugin IDs, navigation identifiers, dashboard layouts, workflow shortcuts and Agent Profile identifiers. It MUST NOT contain grants, credential values, approval tokens, data-scope expansion, or executable code.

Profiles are versioned templates. Applying a template copies or derives user-owned composition settings; it does not create a live authorization link. A user may maintain multiple profiles, but a UI context has one active profile at a time. The system MUST define deterministic behavior for missing, stale, malformed or unsupported profile versions and fall back to a safe default.

Import/export MUST validate schema and references, strip forbidden security fields, and treat imported values as untrusted configuration. A profile update MUST NOT mutate an existing Data Workspace or domain record.

## R1 implementation boundary

This ADR does not prescribe exact Rust types, profile inheritance, team sharing, database tables, or migration. Profile inheritance and collaborative profile administration are deferred.

## R1 validation additions

Switching profiles leaves effective grants and Data Workspace scope unchanged; imported profiles cannot carry permissions or secrets; stale profile schema uses safe fallback; invalid plugin references are reported without widening access; multiple profiles remain isolated.
