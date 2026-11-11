# 04 — Core Page Design Specification

**Status:** Proposed  
**Scope:** Alpha 0.1 core pages plus one deferred assistant prototype

## 1. Dashboard

### Purpose
Provide an accurate, actionable summary of the active workspace.

### Information hierarchy
1. Page title and current workspace.
2. Work summary metrics derived from authoritative backend read models.
3. Active goals and projects.
4. Due/active tasks.
5. Recent activity where supported by contract.

### Interactions
- Metric card opens the corresponding filtered list.
- Project card opens project detail.
- Task row opens task detail.
- Empty sections provide a relevant create/capture action only when supported.

### Constraints
- Never hard-code sample metrics in production.
- Do not invent progress percentages.
- Do not surface Learning-domain metrics in Alpha 0.1 unless explicitly included by the frozen feature matrix.
- Partial data must be labelled; one failed panel should not blank the whole dashboard.

## 2. Project Workspace

### Purpose
Provide one place to inspect project identity, relationships, work items and project-level resources.

### Header
- Project name and status.
- Description.
- Goal and Area references when present.
- Primary action based on actual lifecycle contract.
- Overflow actions for supported operations only.

### Views
- Overview.
- Tasks/List.
- Kanban.
- Activity only if the backend supplies authoritative activity data.
- Attachments where supported.

### Interactions
- Create/edit project.
- Navigate to related Goal/Area.
- Create task in project context.
- Switch List/Kanban without changing task data.
- Archive/complete only through valid backend transitions.

### Constraints
Project completion, archival and deletion are distinct operations. Do not map one to another in the UI.

## 3. Task Management

### List view
Columns/fields may include title, status, priority, due date and project. Keep a useful compact mode for narrow windows.

### Kanban view
Columns reflect the backend task lifecycle. Drag-and-drop is a request to transition state, not a local-only mutation.

### Task detail drawer
- Title and description.
- Status and priority.
- Due date.
- Project relationship (optional where domain permits).
- Goal/Area context only if supported by contract.
- Attachments where supported.
- Created/updated metadata where returned.

### Transition behavior
1. Preserve the current authoritative state.
2. Show optimistic movement only if rollback is implemented.
3. Submit transition command.
4. On success, reconcile with returned authoritative data.
5. On failure, restore the previous position and show a reason.
6. Prevent duplicate submissions while a transition is pending.

Do not assume arbitrary status values; obtain allowed transitions from domain contract or explicit frontend mapping ratified against the backend.

## 4. Inbox / Capture

Inbox is a workflow entry, not automatically a new domain entity. The UI may only expose it if an approved capture contract exists. If current backend supports only Task creation, use a clear quick-create flow that creates a Task; do not imply a separate persisted Inbox object.

## 5. Search

Use the existing backend search capability. Search results must include safe display labels and identifiers sufficient for navigation. The UI must not perform broad client-side filtering over records it was not authorized to receive.

## 6. AI Assistant prototype (deferred)

The fourth prototype demonstrates future interaction only. It is labelled "Prototype — not enabled in Alpha 0.1". It may show a simulated suggestion and a disabled approval control, but must not call real Agent providers or mutate backend data.

## 7. Shared page states

All pages implement loading, loaded, empty, partial, error, offline, permission-denied and conflict states as applicable. See 08-interaction-state-specification.md.
