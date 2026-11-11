# 02 — Application Shell Specification

**Status:** Proposed  
**Target:** Desktop-first Windows Alpha 0.1

## 1. Shell composition

The application shell consists of:
1. AppHeader — workspace identity, global search shortcut, status and settings access.
2. PrimarySidebar — top-level feature navigation.
3. MainOutlet — active page content.
4. Optional ContextPanel — reserved for contextual tools; AI execution is not enabled by this layout alone.
5. Global overlays — dialogs, command palette, toast and attachment preview.

The shell must not own domain rules or duplicate backend state.

## 2. Window and viewport targets

Design review sizes:
- Primary: 1440 × 900.
- Secondary: 1280 × 720.
- Compact desktop: 1024 × 700 (degraded layout review only).

The actual supported minimum window size must be confirmed against Tauri configuration and usability tests; this document does not set a release minimum.

## 3. Layout behavior

| Viewport | Sidebar | Main content | Context panel |
|---|---|---|---|
| >= 1360 px | Expanded | Flexible | Optional side-by-side |
| 1120–1359 px | Collapsible | Flexible | Overlay/drawer |
| 1024–1119 px | Collapsed by default | Flexible | Overlay |
| < 1024 px | Compact navigation | Single column | Full-height drawer |

Avoid horizontal scrolling for ordinary forms and dashboard cards. Wide tables and Kanban boards may use a bounded horizontal scroll region with clear affordance.

## 4. Header

Header elements:
- Sidebar toggle.
- Product/workspace label.
- Global Search trigger (Ctrl+K on Windows).
- Connectivity/runtime status when meaningful.
- Settings entry.
- User/workspace menu only if supported by the product model.

Do not display a fake cloud-sync indicator when no synchronization capability exists.

## 5. Sidebar

Navigation is grouped by function:
- Workspace: Dashboard, Inbox, Goals, Areas, Projects, Tasks.
- Tools: Search.
- System: Settings.

Only routes enabled for the current release may be navigable. Hidden/deferred features must not be reachable through stale deep links.

## 6. Main content frame

Each page should use a common PageHeader contract:
- Breadcrumb (where hierarchy exists).
- Title and optional description.
- Primary action aligned to the right.
- Optional view switcher/filter row.
- Content body with consistent spacing.

Pages must preserve scroll position when switching between related views where feasible.

## 7. Context panel

The panel is an optional layout slot, not an Alpha 0.1 commitment to AI capability. When no supported context tool is available, the panel is absent rather than an empty permanent column.

Future AI panel requirements are in 05-ai-assistant-ui.md.

## 8. Overlay hierarchy

1. Tooltip/popover.
2. Menu/command palette.
3. Drawer.
4. Dialog.
5. Critical blocking error.

Overlays must trap focus appropriately and restore focus to the invoking control on close.

## 9. Window and OS integration

- Use Tauri-supported native file selection for attachment workflows.
- Do not assume browser APIs or browser storage are equivalent to native OS facilities.
- Respect platform path semantics in display and error messages.
- Alpha 0.1 acceptance is Windows-only per ADR-021. macOS design compatibility must not be described as completed acceptance.

## 10. Failure handling

If the frontend cannot initialize its IPC bridge, show a blocking, actionable application error. If one feature request fails after shell initialization, isolate the error to that feature and preserve the rest of the shell.
