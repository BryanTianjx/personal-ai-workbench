/* Typed command wrappers — Task domain.
 * Status changes only through transitionTask (updateTaskDetails never touches status).
 * TASK_TRANSITIONS mirrors TaskStatus::can_transition_to in paw-domain exactly. */
import { callCommand } from "../client";
import type { Task, TaskStatus } from "../contracts";

export const TASK_TRANSITIONS: Record<TaskStatus, TaskStatus[]> = {
  todo: ["in_progress", "cancelled"],
  in_progress: ["done", "todo", "cancelled"],
  done: [],
  cancelled: [],
};

export const TASK_STATUS_LABELS: Record<TaskStatus, string> = {
  todo: "待办",
  in_progress: "进行中",
  done: "已完成",
  cancelled: "已取消",
};

export const TASK_STATUS_ORDER: TaskStatus[] = ["todo", "in_progress", "done", "cancelled"];

export function listTasks(
  workspaceId: string,
  filter?: { projectId?: string | null; status?: TaskStatus | null },
): Promise<Task[]> {
  return callCommand("list_tasks", {
    workspaceId,
    projectId: filter?.projectId ?? null,
    status: filter?.status ?? null,
  });
}

export function createTask(
  workspaceId: string,
  input: { projectId?: string | null; title: string; notes?: string | null },
): Promise<Task> {
  return callCommand("create_task", {
    workspaceId,
    projectId: input.projectId ?? null,
    title: input.title,
    notes: input.notes ?? null,
  });
}

export function updateTaskDetails(
  workspaceId: string,
  taskId: string,
  input: { title: string; notes?: string | null; projectId?: string | null },
): Promise<Task> {
  return callCommand("update_task_details", {
    workspaceId,
    taskId,
    title: input.title,
    notes: input.notes ?? null,
    projectId: input.projectId ?? null,
  });
}

export function transitionTask(
  workspaceId: string,
  taskId: string,
  to: TaskStatus,
): Promise<Task> {
  return callCommand("transition_task", { workspaceId, taskId, to });
}

export function deleteTask(workspaceId: string, taskId: string): Promise<void> {
  return callCommand("delete_task", { workspaceId, taskId });
}
