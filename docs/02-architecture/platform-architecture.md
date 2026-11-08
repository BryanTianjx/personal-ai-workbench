# Platform Architecture

**Architecture Baseline:** E0.2 Candidate
**Decision Status:** Accepted platform strategy
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Supported targets
Windows is the delivery and acceptance target from Alpha 0.1 onward (ADR-021). macOS remains a reserved target behind the platform-neutral boundaries in this document and is delivered and accepted at its own macOS readiness milestone in a later version. The current delivery platform requires native build, install, launch, offline, backup/restore, and upgrade verification. Compilation alone is not acceptance.

## 2. Technology responsibilities

- Tauri 2: desktop shell, lifecycle, controlled IPC, packaging integration.
- React + TypeScript: presentation and client interaction.
- Rust: application/domain execution, persistence, policy, adapters, platform abstractions.
- SQLite + FTS5: local persistence and search.
- OS credential facilities: secret storage.
- Platform APIs: application data, logs, cache paths, dialogs, notifications.

## 3. Platform isolation
Keep OS-specific behavior behind Rust interfaces. Domain/use cases should not branch on platform except for genuine platform behavior. Use platform-native per-user data locations: Windows known folders; macOS Application Support, Logs, and Caches. Never hard-code drive letters, usernames, or home paths.

## 4. Packaging and signing
Windows release packaging defines installer format, upgrade/uninstall behavior, signing policy, and runtime prerequisites. macOS defines bundle identity, architecture targets, signing, notarization, Gatekeeper behavior, and upgrade path. Release credentials are not stored in the repository. Exact installer formats and distribution channels remain release decisions.

## 5. Offline installation
The core app installer must not require downloading Docker, PostgreSQL, browser runtimes, or remote AI components. Runtime core workflows must remain available with networking disabled. Optional model/integration downloads are explicit and separate from base installation. Test installation offline from a locally available signed installer and dependencies.

## 6. Native validation matrix
macOS rows are deferred to the macOS readiness milestone (ADR-021); the column is retained, not deleted.

| Test | Windows | macOS |
|---|---:|---:|
| Native clean install and launch | Required | Deferred |
| Offline installer execution | Required | Deferred |
| Offline core workflow | Required | Deferred |
| File dialog/path scope | Required | Deferred |
| OS credential storage | Required | Deferred |
| SQLite migration and FTS5 | Required | Deferred |
| Backup and restore | Required | Deferred |
| Upgrade and uninstall | Required | Deferred |
| Signing/notarization and OS trust prompts | Required | Deferred |
| Reconnect and remote status reconciliation | Required | Deferred |

macOS acceptance, when the readiness milestone activates, must run on actual macOS hardware/VM with the release candidate package, signature/notarization and Gatekeeper checks, first-launch behavior, and data preservation. Windows acceptance must run on actual Windows with the release candidate installer. Cross-compilation is not acceptance.

## 7. Accessibility and Web boundary
Shared UI components provide consistent keyboard navigation, focus handling, and error semantics. Native dialogs may differ but preserve equivalent security and intent. A future Web client requires a server, authentication, authorization, API boundary, and data ownership model; it must not expose desktop IPC or raw SQLite over a network.
