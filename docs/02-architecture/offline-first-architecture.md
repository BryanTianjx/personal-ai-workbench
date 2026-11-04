# Offline-First Architecture

**Status:** Proposed — E0.2

## 1. Definition

Offline-first means core local workflows remain usable without network access. It does not mean every external integration works offline or that local inference is bundled.

## 2. Capability classes

| Capability | Offline expectation |
|---|---|
| Workspace, project, task CRUD | Fully available |
| Local attachments/file organization | Available within permissions |
| Local FTS5 search | Available for indexed local content |
| Backup, export, restore | Available |
| Local audit/execution history | Available |
| Remote Agent providers | Unavailable unless supported local provider configured |
| Hindsight External / remote knowledge | Unavailable; retain references and last-known metadata |
| WeChat APIs/external publishing | Unavailable |
| Cloud AI/model calls | Unavailable |

## 3. Connectivity state

Each external adapter reports NotConfigured, Available, Unavailable, Unauthorized, Degraded, or Unknown. Checks are bounded and must not block core startup. UI explains unavailable operations and local alternatives.

## 4. Deferred operations

Do not queue remote side effects silently. Any future deferred execution must be explicit, visible, cancellable, and re-authorized at execution time because credentials, policy, and target state may change. Distinguish “saved locally” from “submitted remotely”.

## 5. Conflict and synchronization

Multi-device sync is out of scope for the initial desktop baseline. Multiple clients must not concurrently write a shared SQLite file over a network share. Future sync requires identity, versioning, conflict resolution, tombstones, encryption, retries, and audit. PostgreSQL alone is not a synchronization protocol.

## 6. Offline acceptance

For every Windows/macOS release candidate: install with network disabled; launch and create/open workspace; create/edit/search projects and tasks; add/open permitted attachments; view audit/execution history; create and verify backup; restore into a clean profile; confirm remote integrations show explicit unavailable state and no false success; reconnect and verify no duplicate/corrupt records.

## 7. Failure behavior

Remote timeouts preserve local state and expose actionable errors. Never silently switch provider, transmit data to an unconfigured service, or treat stale cache as fresh.
