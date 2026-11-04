# Platform Architecture

**Status:** Proposed — E0.2

## 1. Supported targets

Windows and macOS are first-class targets. Each requires native build, install, launch, offline, backup/restore, and upgrade verification. Compilation alone is not acceptance.

## 2. Technology responsibilities

- **Tauri 2:** desktop shell, lifecycle, controlled IPC, packaging integration.
- **React + TypeScript:** presentation and client interaction.
- **Rust:** application/domain execution, persistence, policy, adapters, platform abstractions.
- **SQLite + FTS5:** local persistence and search.
- **OS credential facilities:** secret storage.
- **Platform APIs:** application data, logs, cache paths, dialogs, notifications.

## 3. Platform isolation

Keep OS-specific behavior behind Rust interfaces. Domain/use cases should not branch on platform except for genuine platform behavior. Use platform-native per-user data locations: Windows known folders; macOS Application Support, Logs, and Caches. Never hard-code drive letters, usernames, or home paths.

## 4. Packaging and signing

Windows release packaging defines installer format, upgrade/uninstall behavior, signing policy, and runtime prerequisites. macOS defines bundle identity, architecture targets, signing, notarization, Gatekeeper behavior, and upgrade path. Release credentials are not stored in the repository. Exact installer formats and distribution channels remain release decisions.

## 5. Offline installation

The core app installs and starts without Docker, PostgreSQL, browser downloads, or remote AI. Bundle dependencies only after license/security review. Optional model/integration downloads are explicit and separate from base installation.

## 6. Validation matrix

| Test | Windows | macOS |
|---|---:|---:|
| Clean install and launch | Required | Required |
| Offline core workflow | Required | Required |
| File dialog/path scope | Required | Required |
| Credential storage | Required | Required |
| SQLite migration and FTS5 | Required | Required |
| Backup and restore | Required | Required |
| Upgrade and uninstall | Required | Required |
| Signing/notarization behavior | Required | Required |

## 7. Accessibility and Web boundary

Shared UI components provide consistent keyboard navigation, focus handling, and error semantics. Native dialogs may differ but preserve equivalent security and intent. A future Web client requires a server, authentication, authorization, API boundary, and data ownership model; it must not expose desktop IPC or raw SQLite over a network.
