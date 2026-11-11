# System Architecture

**Architecture Baseline:** E0.2 + Learning & Growth extension; E0.3.2 ratified coordination/operation/contract baseline (2026-11-08)
**Decision Status:** Accepted design baseline — see ADR-002 through ADR-006, ADR-013 through ADR-016, and ADR-017 through ADR-020
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified
**Applies to:** Alpha 0.1 through V1.0

## 1. Architectural objectives

The system is a local-first personal work and growth platform. Its architecture must preserve offline operation for core workflows, local ownership and export of user data, human governance over consequential Agent actions, evidence-based learning assessment, replaceable integrations, and a Windows-first delivery strategy with cross-platform boundaries reserved. Use a modular monolith until independent deployment or scaling is justified.

## 2. Logical architecture

```text
Presentation: Tauri 2 · React · TypeScript
        ↓ commands/events
Application: use cases · validation · authorization · coordination
        ↓ ports
Domain:
  Work Management · Learning & Growth · Knowledge & Context
  AI Collaboration · Engineering · Creator · Automation
  System & Security
        ↑ implemented by
Infrastructure: SQLite/FTS5 · files · backup · logging · OS credential store
Integration: Agent adapters · knowledge adapters · Git · optional remote services
```

UI code does not access SQLite or invoke external tools directly. Domain rules do not depend on Tauri, SQL drivers, or vendor SDKs. Infrastructure and adapters implement inward-facing ports.

## 3. Runtime topology

Alpha is a single desktop application with a local Rust backend and a React UI hosted by Tauri. Core operations require no daemon, Docker, PostgreSQL, or network service. Optional integrations communicate with configured local CLI processes or remote endpoints.

A future Web entry point is a separate deployment concern. It must reuse application/domain contracts where practical, but never expose the desktop database or filesystem directly.

## 4. Domain interaction model

- Work Management owns Goal, Area, Project, and Task.
- Learning & Growth owns Roadmap, Plan, Item, Session, Skill, Evidence, and Assessment.
- Knowledge & Context owns local notes, KnowledgeRef metadata, and permitted indexing state.
- Engineering owns engineering activity metadata and Artifact references as specified by its contract.
- AI Collaboration owns Agent registrations, capabilities, and Agent Executions.
- Automation owns Workflow definitions and Workflow Runs.
- System & Security owns authorization, audit, backup metadata, and security policy.

Cross-domain interaction uses published application commands, query DTOs, stable opaque identifiers, and versioned domain events. Direct cross-domain SQL writes are prohibited. See `cross-domain-data-contracts.md`.

## 5. Cross-cutting constraints

- Workbench-owned records are authoritative in local databases and managed files.
- Hindsight and Lingshu remain authoritative for their own knowledge content; the workbench stores references, metadata, permitted indexes, and sync state.
- Git repositories remain authoritative for source code and commit history.
- Learning completion does not automatically change a Skill assessment.
- Project-generated learning gaps are proposals until user acceptance.
- Agent execution and filesystem access pass through authorization and policy enforcement.
- Operations carry correlation IDs; structured logs redact secrets.
- Remote failures retain local intent and diagnostic state; no silent success.
- Schema changes are versioned migrations; adapters are independently testable.

## 6. Quality attributes and evidence

| Attribute | Requirement | Acceptance evidence |
|---|---|---|
| Offline | Core work and delivered local learning workflows work disconnected | Network-disabled test |
| Portability | Windows-first native build; cross-platform targets verified individually | Install and smoke-test on each supported OS |
| Security | Least privilege and explicit approval | Permission/path tests |
| Reliability | Transactional writes and recoverable backup | Restore drill |
| Maintainability | Explicit module ownership | Architecture review and CI checks |
| Interoperability | External tools behind adapters | Contract tests with fake providers |
| Learning integrity | Evidence-based, user-confirmed assessments | Domain and provenance tests |

## 7. V1 non-goals

No microservices, mandatory cloud account, mandatory PostgreSQL, shared writable SQLite over a network share, autonomous high-risk execution, automatic skill scoring, or dependency on undocumented vendor internals.

## 8. Accepted architecture decisions and remaining open items

The following decisions are part of the current baseline:

- Modular Monolith for Alpha 0.1 through V1.0; see ADR-002.
- Workbench-owned desktop records remain local-first; external knowledge systems retain authority over their own content; see ADR-003.
- Provider-specific capabilities remain behind Agent Adapters and central authorization; see ADR-004.
- Core workflows operate locally without mandatory network services; see ADR-005.
- One authoritative local `app.sqlite` for Alpha 0.1 through V1.0, with logical Workspace isolation enforced by backend policy; see ADR-006.
- Learning & Growth is a first-class domain; see ADR-013.
- Learning-to-Project integration uses explicit contracts and events; see ADR-014.
- Skill assessment is evidence-linked and user-confirmed; see ADR-015.
- Learning and Knowledge retain separate ownership; see ADR-016.
- Cross-domain orchestration is a logical Coordination Layer inside paw-application, not a new domain or service; see ADR-017.
- The Rust workspace starts with four crates (paw-domain, paw-application, paw-infrastructure, paw-desktop) and inward dependencies enforced by automated checks; see ADR-018.
- Operation is a durable execution envelope with per-attempt records; Task, Workflow Run, Agent Execution and Operation keep separate owners and state machines; see ADR-019.
- Published cross-domain DTOs and persisted event envelopes are versioned with explicit compatibility, idempotency and quarantine rules; see ADR-020.
- Stages E1–E3 map to Alpha 0.1, E4–E5 to Alpha 0.2, E6.x to Beta 0.5; E7 (platform composition) precedes the V1.0 closure; Alpha 0.1 onward is Windows-only delivery with macOS reserved behind platform-neutral boundaries; see ADR-021 and the roadmap v0.5.0 stage mapping.

Remaining items requiring implementation design or prototype evidence include attachment content-addressing, plugin descriptor serialization and registration mechanics (ADR-023 implementation specification, pending acceptance), representative SQLite performance, and the future multi-device synchronization protocol. None of the above decisions implies implementation or verification is complete.

## 9. E0.3 coordination and execution model

E0.3 (ratified 2026-11-08 via E0.3.2) establishes a logical Coordination Layer inside the Application Layer. It is not a ninth business domain, a separate process, or a deployment tier. It owns cross-cutting orchestration mechanics only: operation/correlation context, named cross-domain use-case coordination, authorization invocation, local transaction coordination, Outbox dispatch, retry classification, cancellation, reconciliation, and recovery.

The owning business domain remains authoritative for its entities and invariants. The Coordination Layer must not expose private repositories, own business records, or become a generic service locator. Cross-domain flows use published application contracts. External/provider calls occur outside SQLite transactions.

### Execution object ownership

| Object | Owner | Responsibility |
|---|---|---|
| Task | Work Management | User-facing business work and status |
| Workflow Run | Automation | Workbench-defined workflow instance |
| Agent Execution | AI Collaboration | Provider invocation and provider-facing lifecycle |
| Operation | Application coordination capability | Durable execution envelope, attempts, correlation and recovery |
| Audit Event | System & Security | Security and consequential-action evidence |
| Outbox Message | Infrastructure/application persistence | Reliable delivery of committed local events |

Operation references its owner and does not shadow its business state. A single workflow may produce multiple operations and Agent Executions; a Task may be related to a Workflow Run without sharing its state machine.

### Rust workspace boundary target

The initial workspace is intentionally small: `paw-domain`, `paw-application`, `paw-infrastructure`, and `paw-desktop`. Domain modules remain explicit within these crates until extraction has demonstrated value. Published application contracts belong to paw-application; domain events and invariants belong to their owning domain module. The dependency graph points inward and is tested.

## 10. E0.3 review disposition

ADR-017–020 were ratified as Accepted on 2026-11-08 through the repository owner's E0.3.2 direction. The E0.3.1 clarifications are normative and form part of each decision. Accepted establishes the design baseline only; implementation and verification remain Not Implemented and Not Verified.

Open items: [ADR-021](../03-adr/ADR-021-stage-version-mapping-windows-only-alpha.md) fixes the stage-to-version mapping and Windows-only Alpha 0.1 delivery. [ADR-022](../03-adr/ADR-022-workbench-hermes-boundary.md) (Workbench ↔ Hermes execution boundary) is Proposed and must be ratified before Alpha 0.2 enables external Agent execution.

## 11. Proposed direction: platform composition and governed capability invocation (ADR-023–029, R1)

The R1 ADR package (2026-11-11; **all seven Accepted by the repository owner on 2026-11-12** with the cross-ADR review conditions normative — implementation and verification remain Not Implemented / Not Verified, and the E7 implementation specification is not yet authorized; decision record in the [owner-review-checklist](../architecture/adr-023-029-hermes-task-package/owner-review-checklist.md)) extends this architecture along two axes. The four-crate modular monolith, single SQLite, domain ownership and ADR-019/020 reliability semantics are unchanged:

1. **Composition axis (ADR-023/024/025/027/028 → stage E7, V1.0 前置):** first-party domains register as statically linked plugins — the plugin is the registration/composition unit, never a replacement domain owner. Four lifecycle dimensions stay independent (registration, availability, composition preference, presentation visibility). `RoleProfile` is an experience template and `WorkspaceComposition` a presentation aggregate scoped to one Data Workspace; neither carries grants, credentials or approvals. Dependency DAG validation, independent version dimensions and fail-closed compatibility govern evolution (ADR-028). Learning & Growth keeps its ADR-013–016 first-class ownership; pluginization does not widen Alpha/Beta scope (ADR-027).
2. **Invocation axis (ADR-026/029 → absorbed into stage E5, Alpha 0.2):** domains publish versioned capability descriptors; a metadata-only Capability Registry resolves IDs/versions; authorization, approval policy, durable Operations, correlation and outbox stay in Application Governance & Coordination; AI Collaboration/Adapter owns Agent lifecycle and provider translation only; the external Agent Runtime stays untrusted with no repository access. Approvals bind to the canonical invocation-plan digest, identity, workspace, scope, target, expiry and attempt policy; policy is re-checked before every protected side effect; uncertain non-idempotent outcomes hold at OutcomeUnknown for reconciliation.

The five-layer ownership model (Domain Capability Contract → Capability Registry → Application Governance & Coordination → AI Collaboration & Adapter → External Agent Runtime) refines, not replaces, the §2 logical architecture: the Registry is catalog infrastructure, dispatch is always mediated through the owning domain's use case. Cross-ADR consistency evidence: [ADR-023～029 R1 跨 ADR 一致性审查报告](../04-quality/ADR-023-029-R1-cross-adr-review.md).
