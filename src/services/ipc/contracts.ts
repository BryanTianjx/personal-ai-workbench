/*
 * Serialized wire contracts (snake_case across the IPC boundary).
 * Shapes mirror the Rust DTOs exactly — see docs/07-ui-design/07-ipc-integration-contract.md §3.
 * Request args are camelCase (Tauri maps them to snake_case Rust params);
 * every response field below is the serde-serialized Rust field name.
 */

export type ProjectStatus = "active" | "paused" | "completed" | "archived";
export type TaskStatus = "todo" | "in_progress" | "done" | "cancelled";
export type AttachmentOwnerType = "project" | "task";
export type SearchKind = "project" | "task";
export type OperationStatus =
  | "pending"
  | "running"
  | "awaiting_approval"
  | "retry_scheduled"
  | "outcome_unknown"
  | "succeeded"
  | "failed"
  | "cancelled";

export interface Workspace {
  id: string;
  name: string;
}

export interface Area {
  id: string;
  workspace_id: string;
  name: string;
  description: string | null;
}

export interface Goal {
  id: string;
  workspace_id: string;
  area_id: string | null;
  name: string;
  description: string | null;
}

export interface Project {
  id: string;
  workspace_id: string;
  goal_id: string | null;
  area_id: string | null;
  name: string;
  description: string | null;
  status: ProjectStatus;
}

export interface Task {
  id: string;
  workspace_id: string;
  project_id: string | null;
  title: string;
  notes: string | null;
  status: TaskStatus;
}

export interface Attachment {
  id: string;
  workspace_id: string;
  owner: AttachmentOwnerType;
  owner_id: string;
  managed_id: string;
  file_name: string;
  size_bytes: number;
  sha256: string;
  created_at: number;
}

export interface SearchHit {
  kind: SearchKind;
  id: string;
  workspace_id: string;
  title: string;
}

export interface DashboardSummary {
  workspace_id: string;
  area_count: number;
  goal_count: number;
  projects_by_status: Array<[string, number]>;
  tasks_by_status: Array<[string, number]>;
  attachment_count: number;
  roadmap_count: number;
  learning_plan_count: number;
  learning_item_count: number;
  learning_session_count: number;
}

export interface OperationRow {
  id: string;
  kind: string;
  owner_domain: string;
  owner_type: string;
  owner_id: string;
  workspace_id: string | null;
  status: OperationStatus;
  correlation_id: string;
  next_attempt_at: number | null;
  cancellation_requested_at: number | null;
  updated_at: number;
}

export interface QuarantinedMessage {
  id: string;
  event_type: string;
  producer_domain: string;
  aggregate_type: string;
  aggregate_id: string;
  attempts: number;
  redrive_count: number;
  last_error_category: string | null;
  last_redrive_by: string | null;
  payload_json: string;
}
