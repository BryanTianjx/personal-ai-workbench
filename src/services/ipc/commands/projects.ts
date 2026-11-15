/* Typed command wrappers — Project domain.
 * Project status transitions are free-form per the E2 slice contract; completion
 * and archival are distinct status writes through updateProject, never relabeled. */
import { callCommand } from "../client";
import type { Project, ProjectStatus } from "../contracts";

export function listProjects(workspaceId: string): Promise<Project[]> {
  return callCommand("list_projects", { workspaceId });
}

export function createProject(workspaceId: string, name: string): Promise<Project> {
  return callCommand("create_project", { workspaceId, name });
}

export function updateProject(
  workspaceId: string,
  projectId: string,
  input: {
    name: string;
    description?: string | null;
    status: ProjectStatus;
    goalId?: string | null;
    areaId?: string | null;
  },
): Promise<Project> {
  return callCommand("update_project", {
    workspaceId,
    projectId,
    name: input.name,
    description: input.description ?? null,
    status: input.status,
    goalId: input.goalId ?? null,
    areaId: input.areaId ?? null,
  });
}

export function deleteProject(workspaceId: string, projectId: string): Promise<void> {
  return callCommand("delete_project", { workspaceId, projectId });
}
