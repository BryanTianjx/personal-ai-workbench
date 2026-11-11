# ADR-026: Capability Registry and Governed AI Invocation

- **Status:** Accepted (R1)
- **Decision status:** Accepted (repository owner decision, 2026-11-12; R1 clarifications are normative; implementation and verification remain separate gates)
- **Implementation status:** Not Implemented
- **Verification status:** Not Verified
- **Date:** 2026-11-11 (accepted 2026-11-12)
- **Scope:** Domain capabilities, Agent invocation, adapter boundary
- **Related:** ADR-004, ADR-017, ADR-018, ADR-019, ADR-020, ADR-022

## Context

Domain plugins need to expose useful business operations to UI, workflows, and Agents without exposing private repositories or allowing Agents to invoke arbitrary internal functions. Provider adapters must remain separate from business capabilities and authorization.

## Decision

1. Each domain publishes explicitly versioned application capabilities routed to its owning use cases.
2. A Capability Registry indexes descriptors and resolves supported versions; it is not a general service locator.
3. Descriptors include capability ID, owner domain, contract version, input/output schema, required scope, risk class, idempotency semantics, offline behavior, and approval-policy hint.
4. Declared permission requirements and approval hints are not grants. System & Security policy remains authoritative; every protected side effect is re-authorized immediately before execution.
5. AI Collaboration owns Agent registrations, Agent Executions, and Agent-facing lifecycle. AI Adapter translates the governed request to supported external provider interfaces.
6. Application Coordination owns Operation, correlation/causation, Outbox, recovery, and cross-domain use-case orchestration.
7. Agent Runtime performs its specialized planning/tool execution. PAW does not duplicate a professional Agent Runtime.
8. OutcomeUnknown, approval binding, no-blind-retry, and at-least-once semantics remain governed by ADR-019/020/022.

## Consequences

- Agents can consume domain capabilities without direct database or repository access.
- Every invocation is auditable and workspace-scoped.
- Capability catalog governance and compatibility tests become mandatory.
- Provider adapters remain replaceable and do not own domain semantics.

## Required validation

Capability schema compatibility, default-deny, cross-workspace denial, approval digest binding, adapter conformance, idempotency, duplicate delivery, OutcomeUnknown reconciliation, and secret redaction.

## Revisit when

A separate Agent service/runtime or externally published capability API is proposed. Preserve the current authorization and ownership invariants.

## R1 clarification: five-layer ownership model

The invocation architecture has five distinct responsibilities:

1. **Domain Capability Contract:** the owning domain defines the stable, versioned use-case contract and enforces business invariants.
2. **Capability Registry:** stores descriptors and resolves capability IDs/versions. It is metadata/catalog infrastructure, not a general Service Locator, dependency injection container, or execution engine.
3. **Application Governance & Coordination:** resolves actor/workspace, authorizes, applies approval policy, creates and advances durable Operations, correlates events, and owns outbox/recovery.
4. **AI Collaboration & AI Adapter:** owns Agent registration/execution lifecycle and translates governed requests to provider-specific interfaces. The adapter does not authorize or own domain semantics.
5. **External Agent Runtime:** performs specialized planning and tool orchestration. Its output is untrusted and it does not receive direct repository/database access.

Every capability descriptor MUST include stable ID, owning domain, contract version, input/output schema references, required permission/scope metadata, risk class, idempotency semantics, offline behavior, and approval-policy hint. Descriptor metadata is advisory input to governance; it is never a grant. Effective authorization is evaluated by backend policy on each protected side effect.

A registry lookup MUST return a descriptor/resolution result, not an arbitrary callable implementation. Dispatch is mediated through the owning domain's application use case and the Application Coordination path.

## R1 implementation boundary

This ADR does not authorize a fifth crate, generic dynamic service locator, arbitrary tool execution, provider-specific authorization bypass, or a duplicate Agent Runtime. Concrete Rust traits, schema formats, registry storage, and provider adapters remain implementation-specification work.

## R1 validation additions

Test descriptor compatibility, duplicate IDs, unsupported versions, default-deny, cross-workspace denial, no direct database/repository access, approval digest binding, per-side-effect reauthorization, idempotency, duplicate outbox delivery, OutcomeUnknown reconciliation, adapter conformance and secret redaction.
