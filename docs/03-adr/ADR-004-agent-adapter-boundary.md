# ADR-004: Isolate Agent Providers Behind Adapters

- **Status:** Accepted
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified

## Context
Multiple Agent products have independently evolving CLI/API/MCP capabilities and lifecycle semantics. Binding core workflows to one provider creates lock-in and unreliable tests.

## Decision
Define a provider-neutral Agent execution contract and implement provider-specific adapters. Keep Agent Adapter separate from Workflow Engine. All side effects pass through the central workbench authorization service; neither adapter nor workflow orchestration may widen scope or bypass approval. Agent output and imported content are untrusted. Persist workbench-owned execution and approval records independently of provider storage. Integrate only through stable, documented, version-tested public interfaces.

## Consequences
**Positive:** replaceable providers, testable contracts, consistent authorization/audit, simulated adapters.
**Negative:** a lowest-common-denominator contract can hide provider-specific capabilities; adapters need capability declarations and explicit unsupported states.

## Risks and mitigations

- **Risk:** Provider adapters expose undocumented or unstable interfaces. **Mitigation:** require documented public interfaces, versioned contract tests, and explicit unsupported states.
- **Risk:** An adapter or workflow bypasses workbench authorization. **Mitigation:** mediate every side effect through the central policy service and test denial/approval paths at the adapter boundary.

## Validation
Adapter contract tests must cover authorization denial, scope escape, stale approval, timeout, unknown outcome, duplicate-submit protection, and secret redaction.

## Revisit when
A provider needs a materially different trust/execution model, or a new protocol makes the contract insufficient. Do not expose provider internals to domain modules.
