/*
 * Presentation metadata for domain statuses. Labels are Chinese; tones follow
 * 01-design-system.md §2 — color is never the sole indicator (each badge has a label).
 */
import type { BadgeTone } from "../ui/primitives";
import type { OperationStatus, ProjectStatus, TaskStatus } from "../../services/ipc/contracts";

export interface StatusMeta {
  label: string;
  tone: BadgeTone;
}

export const PROJECT_STATUS_META: Record<ProjectStatus, StatusMeta> = {
  active: { label: "进行中", tone: "success" },
  paused: { label: "已暂停", tone: "warning" },
  completed: { label: "已完成", tone: "primary" },
  archived: { label: "已归档", tone: "neutral" },
};

export const TASK_STATUS_META: Record<TaskStatus, StatusMeta> = {
  todo: { label: "待办", tone: "neutral" },
  in_progress: { label: "进行中", tone: "primary" },
  done: { label: "已完成", tone: "success" },
  cancelled: { label: "已取消", tone: "neutral" },
};

export const OPERATION_STATUS_META: Record<OperationStatus, StatusMeta> = {
  pending: { label: "排队中", tone: "neutral" },
  running: { label: "运行中", tone: "primary" },
  awaiting_approval: { label: "待审批", tone: "warning" },
  retry_scheduled: { label: "待重试", tone: "info" },
  outcome_unknown: { label: "结果未知", tone: "danger" },
  succeeded: { label: "已成功", tone: "success" },
  failed: { label: "已失败", tone: "danger" },
  cancelled: { label: "已取消", tone: "neutral" },
};

export function formatBytes(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}
