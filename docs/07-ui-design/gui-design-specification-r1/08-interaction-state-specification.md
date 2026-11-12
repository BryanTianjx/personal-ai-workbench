# 08 — Interaction and State Specification

**Status:** Proposed

## 1. Required states

| State | UI behavior |
|---|---|
| Initial loading | Stable skeleton or progress indicator; avoid layout shift |
| Loaded | Render authoritative data |
| Empty | Explain absence and provide a valid next action |
| Partial | Render available sections and identify unavailable sections |
| Saving | Lock duplicate submission; show local progress |
| Saved | Confirm success only after backend response |
| Validation error | Associate message with field; preserve draft |
| Permission denied | Explain that action is not permitted; no retry loop |
| Backend unavailable | Preserve current view/draft; show retry when safe |
| Offline | Keep local features usable; do not imply local data is unavailable |
| Conflict | Preserve user draft and present refresh/reconcile path |
| Outcome unknown | Do not label success/failure; reconcile or request user resolution |

## 2. Loading

Prefer skeletons for stable card/list structures. Use inline spinners for small local actions. Avoid a full-page spinner for a single secondary panel request.

## 3. Empty states

Every empty state should answer:
- what is empty;
- why it may be empty (when known);
- what the user can do next.

Do not invent a create operation if the backend contract does not support it.

## 4. Error handling

Errors should be actionable and non-sensitive. Include a retry action only if the operation is safe to repeat. For non-idempotent operations with uncertain outcomes, offer status reconciliation rather than retry.

## 5. Optimistic updates

Allowed only when:
- the operation is reversible in the UI;
- rollback is deterministic;
- duplicate submission is prevented;
- backend result is reconciled;
- no authorization or lifecycle rule is bypassed.

For task state transitions, restore prior state if backend rejects the transition.

## 6. Destructive operations

Before irreversible deletion or purge:
- state the target and consequence;
- distinguish archive from delete;
- require explicit confirmation;
- do not default focus to the destructive action;
- show success only after backend confirmation.

## 7. Forms

- Show unsaved-change indicator.
- Preserve draft on recoverable IPC error.
- Confirm navigation away only when changes would be lost.
- Server validation is authoritative.

## 8. Connectivity and health

A disconnected network does not imply local application failure. Present local runtime health and optional remote-provider status separately. Avoid a permanent red offline banner when offline is expected and local workflows remain healthy.

## 9. Keyboard and focus

- Escape closes the topmost dismissible overlay.
- Focus returns to the invoking control.
- Dialog traps focus.
- All actions reachable without drag-and-drop.
- Enter/Space semantics follow native control expectations.

## 10. Notification policy

Use inline feedback for field-level issues, toast for short completed operations, persistent banners for unresolved system conditions, and dialogs only for decisions requiring user attention. Avoid duplicate notifications for one operation.
