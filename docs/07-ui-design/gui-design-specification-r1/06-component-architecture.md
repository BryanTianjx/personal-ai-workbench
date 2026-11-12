# 06 — Frontend Component Architecture

**Status:** Proposed target architecture; migration required

## 1. Architectural principles

- Organize by feature/domain, not by technical file type alone.
- Keep page components thin; move data orchestration into feature services/hooks.
- Reuse visual primitives through a small design-system layer.
- Keep backend DTOs distinct from presentation ViewModels.
- Do not reproduce domain state machines in React.
- No direct database access from frontend code.

## 2. Target structure

```text
src/
  app/
    App.tsx
    router/
    providers/
    layouts/
  features/
    dashboard/
      pages/
      components/
      hooks/
      services/
      types/
    goals/
    areas/
    projects/
    tasks/
    inbox/
    search/
    assistant/        # deferred; no provider execution in Alpha 0.1
  components/
    ui/
    forms/
    feedback/
    data-display/
  services/
    ipc/
      client.ts
      commands.ts
      errors.ts
    contracts/
  stores/
  hooks/
  styles/
    tokens.css
    global.css
  types/
```

This is a target structure, not a claim that the current repository already follows it.

## 3. Dependency direction

```text
app/layouts
   ↓
feature pages/components
   ↓
feature hooks/services
   ↓
shared IPC client + contract types
   ↓
Tauri commands
   ↓
Rust application/domain/infrastructure
```

Shared UI components must not import feature services. Features must not import one another's private implementation files; communicate through route navigation or explicitly shared contracts.

## 4. Component ownership

| Component | Responsibility |
|---|---|
| AppShell | Layout composition only |
| Sidebar | Navigation presentation |
| PageHeader | Breadcrumb/title/actions |
| DataTable/List | Generic data presentation |
| StatusBadge | Labelled state presentation |
| ConfirmDialog | Explicit confirmation |
| TaskCard | Task summary and interaction affordance |
| TaskDrawer | Task inspection/edit composition |
| ProjectHeader | Project identity and actions |
| EmptyState | Actionable no-data presentation |
| ErrorState | Recoverable/non-recoverable failure |
| Toast | Transient operation feedback |
| SearchDialog | Global search interaction |
| AssistantPanel | Deferred contextual assistant surface |

## 5. State management

Start with React state and query lifecycle hooks already present in the repository. Introduce a global state library only when a demonstrated cross-feature state problem warrants it.

Separate:
- server state: authoritative records returned by IPC;
- UI state: selected tab, drawer visibility, filters;
- draft state: unsaved form values;
- ephemeral state: toast, hover, command palette;
- preference state: non-sensitive presentation preferences.

Never store authorization decisions or secrets as client-authoritative state.

## 6. Form behavior

- Controlled draft values.
- Explicit dirty state.
- Validation errors mapped from backend field/error codes.
- Disable duplicate submit.
- Preserve draft after recoverable errors.
- Reconcile from server response after successful mutation.

## 7. Testing seams

Feature services should be injectable/mockable for component tests. Production IPC and prototype mock data must remain separate. Do not allow prototype mock implementations to enter the production bundle by accidental import.
