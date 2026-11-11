# 03 — Navigation and Routing Specification

**Status:** Proposed  
**Scope:** Alpha 0.1

## 1. Navigation model

| Group | Entry | Route concept | Availability |
|---|---|---|---|
| Workspace | Dashboard | /dashboard | Alpha 0.1 |
| Workspace | Inbox | /inbox | Only where backed by approved capture contract |
| Workspace | Goals | /goals and /goals/:id | Alpha 0.1 |
| Workspace | Areas | /areas and /areas/:id | Alpha 0.1 |
| Workspace | Projects | /projects and /projects/:id | Alpha 0.1 |
| Workspace | Tasks | /tasks and /tasks/:id | Alpha 0.1 |
| Tools | Search | /search | Alpha 0.1 |
| System | Settings | /settings | Approved settings only |
| Deferred | Learning & Growth | /learning/* | Deferred |
| Deferred | Agent execution | /agents/* | Deferred / authorization gate |
| Deferred | Workflow editor | /workflows/* | Deferred |

Route names are conceptual. The production router must align with the existing application and should not be changed solely to match this proposal.

## 2. Navigation behavior

- Sidebar selection reflects the canonical active route.
- Detail routes retain parent navigation selection.
- Breadcrumbs provide parent navigation without replacing browser/router history.
- Unknown routes show a local not-found page.
- Disabled/deferred routes must not be activated by direct URL navigation.
- Workspace selection must be explicit and visible when multiple workspaces are supported.

## 3. Feature availability registry

A frontend registry may control whether a route and navigation item are visible:

```ts
type FeatureAvailability = {
  key: string;
  release: "alpha-0.1" | "alpha-0.2" | "deferred";
  enabled: boolean;
  route?: string;
};
```

This registry is presentation configuration only. It is not authorization. Backend policy remains authoritative and must reject unauthorized operations regardless of UI visibility.

## 4. Global search

- Windows shortcut: Ctrl+K.
- Search opens a command/search overlay.
- Results are grouped by supported entity type.
- Search result selection navigates to the relevant detail route.
- Empty query and no-result states are distinct.
- Search must not reveal records from another workspace; workspace scoping is enforced by backend query logic.

## 5. Keyboard navigation

Proposed shortcuts:
- Ctrl+K: global search.
- Ctrl+N: create task only when the current route and backend capability support it.
- Ctrl+Enter: submit a focused form only when unambiguous.
- Escape: close topmost non-blocking overlay.

Shortcuts must not intercept text-editing contexts or conflict with OS conventions.

## 6. Route guards

Route guards may check:
- application initialization;
- workspace selection;
- feature availability;
- required record identifier.

Route guards must not make security decisions based on client-side state. Every read/write is independently validated by the backend.

## 7. Deep links

A deep link to a missing, archived, inaccessible or wrong-workspace record must show a safe not-found/unavailable state without leaking whether a protected record exists.
