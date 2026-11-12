# R2-EXEC-02 — Backend Architecture & Rust Quality Audit

- Parent: [Engineering Audit R2.0](./README.md)
- Dependency: R2-EXEC-01 baseline frozen
- Mode: read-only; tests/builds only when isolated

## Objective

Assess the four-crate architecture, domain/application boundaries, infrastructure adapters, transactional behavior, error handling, and Rust engineering quality at the agreed SHA.

## Scope

Discover actual workspace paths first. Expected architectural areas:
- `paw-domain`
- `paw-application`
- `paw-infrastructure`
- `paw-desktop`

Do not assume names or file paths if the audited tree differs.

## Audit tasks

### A. Dependency architecture

1. Map crate and module dependencies.
2. Verify dependency direction: domain should not depend on infrastructure or desktop concerns.
3. Identify circular dependencies, dependency inversion violations, and duplicated domain rules.
4. Inspect public API boundaries and visibility.
5. Inspect oversized modules, cross-layer imports, and hidden global state.
6. Compare actual dependency graph with ADRs and architecture documentation.

### B. Domain and application

1. Inspect entities, value objects, invariants, validation, and domain errors.
2. Trace critical use cases from command/API to domain and persistence.
3. Identify transaction ownership and multi-repository atomicity.
4. Inspect idempotency, concurrency, retry, cancellation, and error propagation.
5. Inspect event/Outbox creation and dispatch consistency.
6. Identify use cases that bypass application-level policy.

### C. Infrastructure and desktop composition

1. Inspect SQLite, filesystem, search, audit, backup, and Agent adapters.
2. Inspect application composition root and production dependency wiring.
3. Identify fakes or in-memory adapters wired into production.
4. Inspect connection lifecycle, resource cleanup, logging, and error translation.
5. Check Tauri commands remain thin and do not duplicate business rules.

### D. Rust quality and tests

Discover project-supported commands, then run non-mutating checks as applicable:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Record exact command, exit code, duration, output, target SHA, and environment. If a command is unsupported, mark NOT_CONFIGURED. If blocked by dependency/network/environment, mark BLOCKED. Do not run cargo fix or format write mode.

Inspect test quality: critical-path coverage, assertions, fixtures, fake/mock use, production adapter coverage, concurrency tests, flaky/order-dependent tests, and ignored tests.

## Required outputs

- `R2-EXEC-02-backend-architecture-report.md`
- `R2-EXEC-02-rust-quality-report.md`
- `R2-EXEC-02-test-evidence.md`
- `R2-EXEC-02-findings.md`

## Finding focus

Prioritize architectural boundary violations, silent error suppression, non-atomic multi-step operations, unsafe concurrency, unbounded resource use, production fake adapters, and test evidence that overstates real integration.

## Acceptance criteria

All four actual crate boundaries are mapped; critical flows are traced; available Rust checks have execution evidence; each finding points to source/test evidence; historical test counts are not presented as current results.

## Stop conditions

Stop if tests access production data, require destructive operations, modify tracked files, or need unapproved system/dependency changes. Preserve logs and report BLOCKED.
