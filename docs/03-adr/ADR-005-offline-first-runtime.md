# ADR-005: Offline-First Desktop Runtime

- **Status:** Accepted for V1 planning
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering

## Context

The desktop product must remain useful when disconnected and must not require cloud services for basic personal workspace operations.

## Decision

Workspace, project, task, local file, local search, audit history, backup, and restore operate locally. Remote Agent providers, external knowledge services, and WeChat APIs are optional integrations that report explicit unavailable/degraded states offline. No silent fallback or false success is permitted.

## Consequences

**Positive:** predictable offline operation, user control, reduced runtime dependencies.

**Negative:** remote capabilities are unavailable without connectivity; cached external data may be stale and must be labeled.

## Revisit when

Local inference becomes a supported product requirement or a server/sync edition is approved. Preserve explicit data-transfer consent and offline core guarantees.
