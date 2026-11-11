# ADR-028: Plugin Dependency and Compatibility Governance

- **Status:** Proposed (R1)
- **Decision status:** Proposed / Not Accepted
- **Implementation status:** Not Implemented
- **Verification status:** Not Verified
- **Date:** 2026-11-11
- **Scope:** First-party plugin dependency declaration, compatibility, upgrade, deprecation and removal
- **Related:** ADR-002, ADR-006, ADR-017, ADR-018, ADR-019, ADR-020, ADR-023, ADR-026, ADR-027

## 1. Context

Static first-party plugins still form a dependency graph. A plugin may require another domain capability, a platform contract, a migration level, or a minimum application version. Without explicit compatibility governance, startup order, partial availability, upgrades, and removal can diverge between UI, workflow, and Agent invocation paths.

The initial product is a single-user, local-first modular monolith. It does not need a third-party package marketplace, remote plugin distribution, or dynamic code loading. It does need deterministic startup and a safe way to evolve statically linked plugin descriptors and capability contracts.

## 2. Decision drivers

1. Preserve bounded-context ownership and the four-crate modular-monolith boundary.
2. Make plugin activation deterministic and fail closed for unsafe dependency states.
3. Separate plugin package version, descriptor schema version, capability contract version, and data migration version.
4. Prevent a plugin upgrade or disable from silently corrupting domain data or stranding durable Operations.
5. Keep Alpha scope small: static built-ins, explicit compatibility checks, and migration discipline only.

## 3. Options considered

- **A. Implicit dependencies and startup ordering:** rejected; hidden coupling and non-deterministic behavior.
- **B. Explicit dependency DAG with static registration:** selected; sufficient for first-party modules and testable at startup.
- **C. Dynamic package resolver / marketplace:** deferred; introduces signing, provenance, sandboxing, supply-chain, rollback, and distribution concerns outside the current product stage.

## 4. Decision

### 4.1 Dependency declaration and graph

Each plugin descriptor MAY declare required and optional dependencies. A dependency identifies a stable plugin ID and a supported version range; it MUST NOT reference a UI route or concrete Rust implementation type. The registry validates the complete graph before exposing capabilities.

- Required dependency absent, disabled, or incompatible: dependent plugin is `blocked` and its capabilities are not invocable.
- Optional dependency absent or incompatible: plugin may be `available_with_degraded_features` only when its descriptor declares a safe fallback for the affected contribution.
- Cycles: reject the affected registration graph; do not attempt best-effort ordering.
- Dependency resolution: deterministic topological order with stable tie-breaking by plugin ID.
- A plugin MUST NOT depend on a higher-level presentation composition or Role Profile.

### 4.2 Version dimensions

Do not collapse these independent versions:

| Version dimension | Owner | Meaning |
|---|---|---|
| `plugin_version` | Plugin release | Implementation/descriptor release identity |
| `descriptor_schema_version` | Platform contract | Shape and semantics of plugin metadata |
| `capability_contract_version` | Owning domain | Input/output and behavioral contract for a capability |
| `data_migration_version` | Owning domain | Persistent data transformation sequence |
| `minimum_app_version` | Application | Minimum host version required by the plugin |

Version ranges are compatibility declarations, not proof. Compatibility is established by automated contract tests and release evidence. A package version bump MUST NOT be used to imply that data migration has run.

### 4.3 Compatibility policy

- Additive optional fields with defined defaults MAY be backward compatible.
- Removing/renaming fields, changing meaning, tightening accepted values, changing authorization scope, changing idempotency semantics, or changing side-effect behavior is breaking unless an explicit compatibility adapter proves otherwise.
- Security-critical fixes may require immediate blocking even when the ordinary semver range would permit activation.
- Unknown major versions, unknown required descriptor fields, and unsupported capability versions fail closed.
- The host MUST expose a structured incompatibility reason; it MUST NOT silently downgrade security policy.

### 4.4 Upgrade, migration, deprecation, removal

1. Validate descriptor and dependency graph before activation.
2. Verify compatibility against host and capability contracts.
3. Plan domain-owned migrations separately from plugin activation.
4. Apply migrations through the owning domain's migration mechanism, with backup/rollback policy governed by the data architecture.
5. Reconcile in-flight Operations before disabling/removing a plugin that owns their capability.
6. Deprecation MUST specify replacement, migration guidance, and a removal window or explicit no-removal rationale.
7. Removal MUST preserve historical records and audit evidence; deleting plugin code does not authorize deleting domain data.

A plugin cannot directly migrate another domain's tables. Cross-domain migration requires the owning domain's versioned contract and coordinated plan.

### 4.5 Runtime lifecycle states

The registry distinguishes at least `registered`, `available`, `blocked`, `disabled`, and `deprecated`. These are platform lifecycle states, not user preference or UI visibility. A user's enable/disable preference is a separate composition setting. A plugin may be registered but blocked; a plugin may be available but hidden in a particular composition.

## 5. Consequences

- Startup validation and dependency diagnostics become explicit and deterministic.
- Domain teams retain ownership of capability and data migration semantics.
- The initial static graph remains simple; future dynamic distribution requires a superseding ADR.
- A safe upgrade may require a staged migration and Operation reconciliation rather than an instantaneous toggle.

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Version range gives false confidence | Contract tests and compatibility evidence are release gates |
| Dependency cycle blocks unrelated plugins | Isolate invalid connected components where safe; report exact cycle; never activate a partially unsafe component |
| Migration partially completes | Domain-owned transactional/idempotent migration, backup, checkpoint and recovery evidence |
| Disabled plugin strands an Operation | Drain/reconcile policy; preserve operation history; no blind cancellation |
| Optional dependency hides a broken feature | Descriptor must declare fallback and feature-level degraded state |

## 7. Required validation

- Missing required dependency blocks dependent capabilities.
- Missing optional dependency activates only declared safe fallback.
- Cycles and unsupported ranges are rejected deterministically.
- Descriptor, capability, app and migration versions are independently checked.
- Breaking/security-critical contract change blocks incompatible consumer.
- Migration failure is recoverable and never silently marked complete.
- Disable/removal preserves data, audit history and Operation reconciliation.
- Deprecation metadata and replacement links are surfaced.

## 8. Implementation and verification boundary

This ADR defines policy only. It does not authorize Rust type design, SQL schema changes, migrations, plugin SDK APIs, runtime loading, or code modifications. Those require implementation specifications and separate human approval. Current state remains **Not Implemented / Not Verified**.

## 9. Revisit when

Third-party plugins, independent plugin distribution, remote updates, dynamic loading, multi-tenant shared plugin administration, or a separate plugin process is approved. Such a proposal must include signing, provenance, sandboxing, permission review, rollback, and supply-chain threat analysis.
