/* Typed command wrappers — Workspace-scoped search. */
import { callCommand } from "../client";
import type { SearchHit } from "../contracts";

export function searchWorkspace(
  workspaceId: string,
  query: string,
  limit = 20,
): Promise<SearchHit[]> {
  return callCommand("search", { workspaceId, query, limit });
}
