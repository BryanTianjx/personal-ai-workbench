# E1 — Rust Workspace Layout and Dependency Contract

**Decision baseline:** ADR-018 Accepted (2026-11-08)
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Initial repository layout

```text
src-tauri/
├── Cargo.toml                    # workspace root; resolver = "2"
├── crates/
│   ├── paw-domain/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── shared/           # IDs, domain errors, clock-independent primitives
│   │       ├── work_management/  # Workspace, Project, Task entities/invariants/events
│   │       ├── learning/
│   │       ├── knowledge/
│   │       ├── ai_collaboration/
│   │       ├── engineering/
│   │       ├── creator/
│   │       ├── automation/
│   │       └── system_security/
│   ├── paw-application/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ports/            # repository, transaction, clock, provider, filesystem
│   │       ├── contracts/        # published commands, queries, DTOs and event envelopes
│   │       ├── context/          # actor/workspace/correlation/causation context
│   │       ├── authorization/
│   │       ├── coordination/     # Operation, Outbox orchestration and recovery use cases
│   │       └── use_cases/
│   │           └── work_management/
│   ├── paw-infrastructure/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── sqlite/           # connection, migrations, transaction manager
│   │       ├── repositories/
│   │       ├── outbox/
│   │       ├── filesystem/
│   │       ├── credential_store/
│   │       └── adapters/
│   └── paw-desktop/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── lib.rs
│           ├── commands/
│           ├── events/
│           ├── state.rs
│           └── composition.rs
├── migrations/
│   └── sqlite/
│       ├── 0001_initial_workspace_project.sql
│       └── 0002_durable_operations_outbox.sql
└── tests/
    ├── architecture/
    ├── integration/
    └── fixtures/
```

This is a target layout, not a claim that these files already exist. Keep all eight business domains as modules initially; create additional crates only with an ADR-backed extraction reason.

## 2. Allowed dependency graph

```text
paw-desktop ───────► paw-application ───────► paw-domain
      │                       ▲
      └────► paw-infrastructure┘
                  │
                  └────────────────────────► paw-domain (domain types only)
```

Allowed direct dependencies:
- paw-domain: standard library and narrowly selected pure utility crates only. No SQL, Tauri, filesystem/OS APIs, network clients, provider SDKs, or application crate.
- paw-application: paw-domain; serialization/validation primitives where they do not leak infrastructure types. Defines ports and published application contracts.
- paw-infrastructure: paw-application (implements its ports) and paw-domain. Contains SQLx/rusqlite choice, filesystem and provider SDK details.
- paw-desktop: paw-application and paw-infrastructure. It is the only composition root and Tauri command/event boundary.

Forbidden:
- paw-domain → any other PAW crate.
- paw-application → paw-infrastructure or paw-desktop.
- paw-infrastructure → paw-desktop.
- React/TypeScript → SQLite or repository internals directly.
- One domain module → another domain module's private entity/repository/table.
- Tauri types, SQL row types, provider SDK types crossing into domain contracts.

## 3. Ownership by crate

| Concern | Owner |
|---|---|
| Entities, value objects, invariants, domain events | paw-domain owning domain module |
| Use cases, orchestration, ports, published DTOs | paw-application |
| SQLite schema/repositories, filesystem and provider implementations | paw-infrastructure |
| Tauri commands, app state construction, dependency wiring | paw-desktop |
| React screens and view state | frontend |
| Cross-domain read composition | application query/read-model layer; no direct private-table reads |

The Coordination Layer is a module in paw-application, not a fifth crate. Domain events belong to paw-domain; the persisted/public event envelope and consumer-facing contracts belong to paw-application.

## 4. Rust API and visibility rules

- Default to private modules and types; expose only deliberate crate APIs.
- Use newtypes for WorkspaceId, ProjectId, OperationId, EventId and other stable IDs.
- Keep SQL row structs private to paw-infrastructure.
- Ports are defined by the application layer that consumes them; adapters implement them.
- Return typed application errors; map them to stable Tauri-safe errors at paw-desktop.
- Do not use a global service locator. Construct dependencies explicitly in composition.rs.
- Avoid async traits or runtime dependencies in paw-domain. Async belongs at application/adapter boundaries.

## 5. Automated enforcement

E1 must include:
1. `cargo metadata --no-deps --format-version 1` dependency inspection.
2. A repository architecture test/script that fails on forbidden crate edges.
3. A source scan preventing Tauri/SQL/provider imports in paw-domain.
4. A check that only paw-desktop registers Tauri commands and constructs infrastructure adapters.
5. A CI job running formatting, clippy, unit tests and architecture checks on Windows.

The architecture check must inspect actual Cargo dependencies and source imports; folder naming alone is insufficient. A failure blocks merge.

## 6. Crate extraction rule

A domain can become a crate only when at least one measurable driver exists: independent compilation/test boundary, stable API consumed by multiple crates, clear independent ownership, or parallel development blocked by module visibility. The proposal must include dependency graph, migration cost, test plan and rollback path. No extraction is planned for E1.
