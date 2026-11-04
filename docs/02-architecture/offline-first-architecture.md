# Offline-First Architecture

**Architecture Baseline:** E0.2 Candidate  
**Decision Status:** Accepted runtime principle  
**Implementation Status:** Not Implemented  
**Verification Status:** Not Verified

## 1. Definition
Offline-first means core local workflows remain usable without network access. It does not mean every external integration works offline or that local inference is bundled. Offline installation and offline runtime are separate acceptance properties.

## 2. Capability classes
| Capability | Offline expectation |
|---|---|
| Workspace, project, task CRUD | Fully available |
| Local attachments/file organization | Available within permissions |
| Local FTS5 search | Available for indexed local content |
| Backup, export, restore | Available |
| Local audit and execution history | Available |
| Remote Agent providers | Unavailable unless a supported local provider is configured |
| Hindsight External / remote knowledge | Unavailable; retain references and last-known metadata |
| WeChat APIs/external publishing | Unavailable |
| Cloud AI/model calls | Unavailable |

## 3. Connectivity state
Each external adapter reports NotConfigured, Available, Unavailable, Unauthorized, Degraded, or Unknown. Checks are bounded and must not block core startup. UI explains unavailable operations and local alternatives, including last successful observation time where applicable.

## 4. Deferred operations and remote side effects
Do not queue remote side effects silently. Any future deferred execution must be explicit, visible, cancellable, and re-authorized at execution time. Distinguish saved-locally from submitted-remotely. After timeout/disconnection with an indeterminate result, show OutcomeUnknown and reconcile before retry; never automatically repeat a potentially non-idempotent publication or Agent submission.

## 5. Reconnection
On reconnect, refresh external status with bounded retries. Preserve local records and show stale data until refreshed. Reconciliation must use provider IDs/idempotency where available and must not create duplicate local records or remote side effects. A connection recovery is not proof that a previously indeterminate action succeeded.

## 6. Conflict and synchronization
Multi-device sync is out of scope for the initial desktop baseline. Multiple clients must not concurrently write a shared SQLite file over a network share. Future sync requires identity, versioning, conflict resolution, tombstones, encryption, retries, and audit. PostgreSQL alone is not a synchronization protocol.

## 7. Offline acceptance
For each Windows/macOS release candidate: verify installer availability without network; install in a network-isolated environment; launch and create/open workspace; create/edit/search projects and tasks; add/open permitted attachments; view audit/execution history; create and verify backup; restore into a clean profile; confirm remote integrations show explicit unavailable state and no false success; reconnect and verify no duplicate/corrupt records. See `../04-quality/offline-acceptance-matrix.md`.

## 8. Failure behavior
Remote timeouts preserve local state and expose actionable errors. Never silently switch provider, transmit data to an unconfigured service, or treat stale cache as fresh.
