/*
 * Typed command wrappers — Workspace domain.
 * Named wrappers over callCommand; request keys are camelCase, responses are the
 * snake_case wire types from contracts.ts. Never infer a signature from a name:
 * each wrapper mirrors a verified Rust command (07-ipc-integration-contract.md §3).
 */
import { callCommand } from "../client";
import type { Workspace } from "../contracts";

/** No workspace scoping: the workspace list is the tenant root. */
export function listWorkspaces(): Promise<Workspace[]> {
  return callCommand("list_workspaces");
}

export function createWorkspace(name: string): Promise<Workspace> {
  return callCommand("create_workspace", { name });
}
