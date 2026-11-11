# ADR-022: Workbench ↔ Hermes Execution Boundary

- **Status:** Accepted
- **Date:** 2026-11-08
- **Decision scope:** External Agent runtime boundary for the first mediated provider (Hermes), prerequisite for Alpha 0.2 Agent enablement
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified
- **Related:** ADR-004, ADR-017, ADR-019, ADR-020, ADR-021

## Context

Alpha 0.2 plans mediated Agent enablement (stages E4–E5). Hermes is the first candidate external Agent runtime: the product vision names delegating tasks to Hermes, and `agent-adapter-architecture.md` lists Hermes among provider candidates alongside Codex and Cursor. ADR-004 fixes the adapter principle (provider-specific capability behind adapters, central authorization), but no decision yet defines the concrete Workbench ↔ Hermes boundary: process model, interface contract, credential handling, data access, execution recording, and failure semantics. Enabling any external Agent execution before this boundary is decided would put the authorization, audit, and reliability guarantees of ADR-017/019/020 at risk.

## Decision proposal

Hermes is integrated as a separate, untrusted local process behind the Agent Adapter boundary. The Workbench owns all authoritative state; Hermes owns nothing inside `app.sqlite`.

1. **Process and interface boundary.** Hermes runs as its own local process, invoked only through stable, documented, public interfaces (CLI, API, MCP, or file exchange) pinned to verified versions. Undocumented internals, private databases, and assumptions about provider persistence are prohibited. A versioned compatibility test and operational guide must exist before any integration is supported, consistent with `agent-adapter-architecture.md`.
2. **Data boundary.** Hermes never receives raw SQLite access, direct filesystem access to Workbench-managed records, or broad path grants. Inputs are explicit payloads assembled by the application; outputs return as published, versioned DTOs/events (ADR-020) and artifacts stored through the Workbench attachment policy with reference metadata.
3. **Authorization.** Every side-effecting Hermes call passes central authorization immediately before execution (ADR-017). An approval binds to the digest of the execution plan it approved; changing the plan invalidates the approval. Approval never expands grants beyond the approved plan.
4. **Execution recording.** One Hermes invocation produces one Agent Execution (owned by AI Collaboration) carried by one Operation envelope (coordination layer) with OperationAttempt entries recording request/plan digest, idempotency key where supported, external execution reference, timing, and outcome classification (ADR-019). Local intent and Outbox delivery are distinct from provider execution.
5. **Credentials.** Hermes credentials live in the OS credential store through the platform integration. The Workbench mediates access; tokens never appear in command lines, logs, events, exports, or Hermes-visible configuration beyond what the invocation requires.
6. **Failure semantics.** A timeout after possible submission is indeterminate: the Operation moves to OutcomeUnknown and reconciliation or explicit human resolution precedes any non-idempotent resubmission. Exactly-once remote effects are not claimed. Cancellation is cooperative and capability-dependent; unsupported cancellation is explicit and never presented as proof the provider stopped.
7. **Offline.** Hermes-dependent capabilities are unavailable offline with an explicit degraded state; locally recorded intent survives and no false success is reported, consistent with ADR-005.

## Alternatives

1. Embed Hermes in-process or give it shared storage access: rejected — destroys the provider boundary, couples Workbench failures to provider failures, and bypasses authorization/audit.
2. A generic multi-provider runtime ADR now: deferred — Hermes is the first concrete case; the pattern generalizes through ADR-004 adapters, and premature generality would repeat the premature-crate-sprawl mistake ADR-018 avoided.
3. Defer all Agent integration beyond Alpha 0.2: possible fallback, but the product vision requires mediated delegation; this ADR keeps that path gated rather than open-ended.

## Consequences

**Positive:** a concrete, testable boundary for the first external Agent; consistent authorization/audit/recovery semantics from day one of Agent execution; Hermes remains replaceable behind the adapter contract.

**Negative:** adapter and mediation work lands in E5; some Hermes-native capabilities may be unreachable through the constrained interface; compatibility maintenance follows Hermes release cadence.

## Risks and mitigations

- Interface drift across Hermes versions: pinned versions, versioned compatibility tests, and explicit unsupported-version rejection.
- Over-permissive file or payload access: path allowlists, workspace-scoped payloads, redaction tests, and audit of every mediated call.
- Secret leakage: credential-store-only secrets, command-line/log/event redaction tests.
- Indeterminate side effects on the user's behalf: OutcomeUnknown handling, idempotency keys where supported, and approval binding to plan digests.

## Validation required

Fake-Hermes conformance suite against the adapter contract; approval-binding tests (plan change invalidates approval); unknown-outcome reconciliation tests; credential redaction tests; contract fixture tests under ADR-020 rules; workspace-scope denial tests for every Hermes-facing path; documented rollback/cleanup for a failed or removed Hermes integration.

## Revisit when

Hermes interface versions change materially, additional providers onboard (generalize the pattern), or a separately deployed Agent service is proposed. Any change requires a superseding ADR preserving ADR-004/017/019/020 semantics.

## Governance

This ADR is Proposed. It must be ratified as Accepted before Alpha 0.2 enables any external Agent execution (roadmap Gate Alpha 0.2, stages E4–E5). Ratification does not imply implementation.
