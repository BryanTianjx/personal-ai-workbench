# 07 — Frontend IPC Integration Contract

**Status:** Contract mapping candidate; command DTO verification required

## 1. Boundary

React communicates with the Rust application through Tauri IPC. React must not access SQLite, filesystem paths or provider credentials directly.

Expected flow:

```text
React Page → Feature Service → Typed IPC Client
 → Tauri Command → Application Use Case
 → Domain / Infrastructure
```

## 2. Existing command inventory observed in src/App.tsx

The following command names were observed in the current App.tsx source during repository inspection. This inventory is not a substitute for inspecting Rust command definitions, DTO schemas, authorization and error semantics.

| Feature | Observed command names |
|---|---|
| Workspace | list_workspaces, create_workspace |
| Areas | list_areas, create_area |
| Goals | list_goals, create_goal |
| Projects | list_projects, create_project, update_project |
| Tasks | list_tasks, create_task, transition_task |
| Dashboard | dashboard |
| Attachments | list_attachments, add_attachment, remove_attachment, open_attachment_externally |
| Search | search |
| Operations | list_operations, request_operation_cancellation, resume_operation, export_operations |
| Other existing UI surfaces | list_quarantined_messages, list_roadmaps, list_plans, list_items, list_sessions, list_knowledge_refs, create_roadmap, create_plan, update_plan_status, create_item, link_item_to_project, create_task_for_item, append_session, create_knowledge_ref, set_knowledge_availability, delete_knowledge_ref, transition_item, create_workspace, purge_expired_data |

Some observed commands belong to deferred or future domains and must not be exposed in Alpha 0.1 merely because they exist.

## 3. Required contract verification

Before implementation, inspect:
- command registration and argument casing;
- request DTO and response DTO types;
- error enum/code and user-safe messages;
- workspace scoping and authorization;
- lifecycle transition validation;
- attachment path and symlink protections;
- pagination/filter semantics;
- cancellation and uncertain-outcome behavior.

Do not infer any command signature from its name.

## 4. Typed IPC client proposal

```ts
import { invoke } from "@tauri-apps/api/core";

export async function callCommand<TRequest, TResponse>(
  command: string,
  args?: TRequest
): Promise<TResponse> {
  try {
    return await invoke<TResponse>(command, args as Record<string, unknown>);
  } catch (cause) {
    throw mapIpcError(cause);
  }
}
```

The production implementation should prefer named wrappers over arbitrary string command calls. Use generated/shared types only if a reliable source-of-truth generation process exists.

## 5. Error normalization

Normalize into a presentation-safe shape:
- code;
- category (validation, not-found, conflict, permission, unavailable, internal);
- safe message;
- optional field errors;
- retryability;
- correlation/operation reference where safe.

Do not display raw Rust debug output or secret-bearing payloads.

## 6. Workspace isolation

- Pass the active workspace only according to the verified command contract.
- Never trust localStorage or route parameters as authorization.
- Backend must validate workspace membership/ownership for every protected operation.
- Search and dashboard queries must be scoped server-side.
- On workspace switch, invalidate or reload workspace-bound caches.

## 7. Mutation semantics

- Treat backend response as authoritative.
- Do not optimistically report success.
- Disable duplicate submissions.
- Reconcile after success.
- On conflict, preserve draft and offer refresh/retry only when safe.
- Never blindly retry a non-idempotent create/transition after an uncertain timeout.

## 8. Attachments

Use only approved Tauri commands and native file selection. UI file selection does not establish a persistent authorization grant. Render filenames and metadata safely; never trust file paths supplied by arbitrary frontend state.

## 9. Offline semantics

Distinguish:
- Offline: network unavailable while local application remains operational.
- Local backend unavailable: IPC/Rust service cannot be reached.
- Remote integration unavailable: optional provider is unavailable.
- Unauthorized: policy rejected the operation.

Core local operations should not depend on remote network status.

## 10. Contract completion gate

Before a production feature is implemented, add a command mapping record containing command name, Rust handler, request DTO, response DTO, error mapping, scope validation, idempotency/retry rule, and tests. Any mismatch requires an ADR or contract update before UI assumptions change.
