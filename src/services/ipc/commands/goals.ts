/* Typed command wrappers — Goal domain. */
import { callCommand } from "../client";
import type { Goal } from "../contracts";

export function listGoals(workspaceId: string, areaId?: string | null): Promise<Goal[]> {
  return callCommand("list_goals", { workspaceId, areaId: areaId ?? null });
}

export function createGoal(
  workspaceId: string,
  input: { areaId?: string | null; name: string; description?: string | null },
): Promise<Goal> {
  return callCommand("create_goal", {
    workspaceId,
    areaId: input.areaId ?? null,
    name: input.name,
    description: input.description ?? null,
  });
}

export function updateGoal(
  workspaceId: string,
  goalId: string,
  input: { areaId?: string | null; name: string; description?: string | null },
): Promise<Goal> {
  return callCommand("update_goal", {
    workspaceId,
    goalId,
    areaId: input.areaId ?? null,
    name: input.name,
    description: input.description ?? null,
  });
}

export function deleteGoal(workspaceId: string, goalId: string): Promise<void> {
  return callCommand("delete_goal", { workspaceId, goalId });
}
