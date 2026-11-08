# ADR-005: Offline-First Desktop Runtime

- **Status:** Accepted
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified

## Context
The desktop product must remain useful when disconnected and must not require cloud services for basic personal workspace operations.

## Decision
Workspace, project, task, local file, local search, audit history, backup, and restore operate locally. Remote Agent providers, external knowledge services, and WeChat APIs are optional integrations that report explicit unavailable/degraded states offline. No silent fallback or false success is permitted. Installation and runtime offline behavior are separately tested. Indeterminate remote side effects are reconciled before retry and are never silently replayed.

## Consequences
**Positive:** predictable offline operation, user control, reduced runtime dependencies.
**Negative:** remote capabilities are unavailable without connectivity; cached external data may be stale and must be labeled.

## Risks and mitigations

- **Risk:** UI presents stale external data as current or queues a remote side effect silently. **Mitigation:** expose connectivity/freshness state and require explicit, re-authorized deferred execution.
- **Risk:** Offline claims are inferred from build success rather than real disconnected operation. **Mitigation:** execute the platform-specific offline matrix on actual Windows and macOS environments and retain evidence.

## Validation
Run the complete offline acceptance matrix independently on Windows and macOS, including install, core workflows, backup/restore, reconnect, and duplicate-side-effect checks.

## Revisit when
Local inference becomes a supported product requirement or a server/sync edition is approved. Preserve explicit data-transfer consent and offline core guarantees.
