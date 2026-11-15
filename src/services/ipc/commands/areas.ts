/* Typed command wrappers — Area domain. */
import { callCommand } from "../client";
import type { Area } from "../contracts";

export function listAreas(workspaceId: string): Promise<Area[]> {
  return callCommand("list_areas", { workspaceId });
}

export function createArea(
  workspaceId: string,
  input: { name: string; description?: string | null },
): Promise<Area> {
  return callCommand("create_area", {
    workspaceId,
    name: input.name,
    description: input.description ?? null,
  });
}

export function updateArea(
  workspaceId: string,
  areaId: string,
  input: { name: string; description?: string | null },
): Promise<Area> {
  return callCommand("update_area", {
    workspaceId,
    areaId,
    name: input.name,
    description: input.description ?? null,
  });
}

export function deleteArea(workspaceId: string, areaId: string): Promise<void> {
  return callCommand("delete_area", { workspaceId, areaId });
}
