/*
 * Error normalization — GUI Design Specification R1 (07-ipc-integration-contract.md §5).
 * The backend rejects invokes with a bare snake_case error code string; nothing else
 * crosses the boundary (Gate C safe-errors policy). This module maps codes to a
 * presentation-safe category and a user-facing Chinese message.
 */

export const COMMAND_ERROR_CODES = [
  "empty_name",
  "name_too_long",
  "notes_too_long",
  "workspace_not_found",
  "project_not_found",
  "area_not_found",
  "goal_not_found",
  "task_not_found",
  "invalid_workspace_id",
  "task_status_invalid",
  "invalid_search_query",
  "attachment_not_found",
  "owner_not_found",
  "operation_not_found",
  "learning_record_not_found",
  "item_not_found",
  "knowledge_ref_not_found",
  "agent_registration_not_found",
  "agent_execution_not_found",
  "approval_not_found",
  "approval_not_active",
  "capability_unsupported",
  "skill_not_found",
  "plugin_disabled",
  "creator_not_found",
  "creator_state_invalid",
  "denied",
  "unexpected",
] as const;

export type CommandError = (typeof COMMAND_ERROR_CODES)[number];

export type ErrorCategory =
  | "validation"
  | "not_found"
  | "conflict"
  | "permission"
  | "internal";

export const ERROR_TEXT: Record<CommandError, string> = {
  empty_name: "名称不能为空。",
  name_too_long: "名称超出长度限制。",
  notes_too_long: "备注超出长度限制。",
  workspace_not_found: "工作空间不存在。",
  project_not_found: "项目不存在。",
  area_not_found: "领域不存在。",
  goal_not_found: "目标不存在。",
  task_not_found: "任务不存在。",
  invalid_workspace_id: "工作空间标识格式无效。",
  task_status_invalid: "该状态变更不被允许。",
  invalid_search_query: "搜索词无效。",
  attachment_not_found: "附件不存在。",
  owner_not_found: "附件所属记录不存在。",
  operation_not_found: "操作记录不存在。",
  learning_record_not_found: "学习记录不存在。",
  item_not_found: "学习条目不存在。",
  knowledge_ref_not_found: "知识引用不存在。",
  agent_registration_not_found: "Agent 注册不存在。",
  agent_execution_not_found: "Agent 执行记录不存在。",
  approval_not_found: "审批记录不存在。",
  approval_not_active: "审批已失效，需重新发起。",
  capability_unsupported: "该能力不受支持。",
  skill_not_found: "技能不存在。",
  plugin_disabled: "相关插件已被禁用。",
  creator_not_found: "创作记录不存在。",
  creator_state_invalid: "创作流程状态不允许该操作。",
  denied: "操作被拒绝。",
  unexpected: "操作失败，请重试。",
};

const ERROR_CATEGORY: Record<CommandError, ErrorCategory> = {
  empty_name: "validation",
  name_too_long: "validation",
  notes_too_long: "validation",
  workspace_not_found: "not_found",
  project_not_found: "not_found",
  area_not_found: "not_found",
  goal_not_found: "not_found",
  task_not_found: "not_found",
  invalid_workspace_id: "validation",
  task_status_invalid: "conflict",
  invalid_search_query: "validation",
  attachment_not_found: "not_found",
  owner_not_found: "not_found",
  operation_not_found: "not_found",
  learning_record_not_found: "not_found",
  item_not_found: "not_found",
  knowledge_ref_not_found: "not_found",
  agent_registration_not_found: "not_found",
  agent_execution_not_found: "not_found",
  approval_not_found: "not_found",
  approval_not_active: "conflict",
  capability_unsupported: "validation",
  skill_not_found: "not_found",
  plugin_disabled: "conflict",
  creator_not_found: "not_found",
  creator_state_invalid: "conflict",
  denied: "permission",
  unexpected: "internal",
};

/** Presentation-safe error carrying code, category and a user-facing message. */
export class WorkbenchError extends Error {
  readonly code: CommandError;
  readonly category: ErrorCategory;

  constructor(code: CommandError) {
    super(ERROR_TEXT[code]);
    this.name = "WorkbenchError";
    this.code = code;
    this.category = ERROR_CATEGORY[code];
  }
}

export function isWorkbenchError(e: unknown): e is WorkbenchError {
  return e instanceof WorkbenchError;
}

/** Normalize any IPC rejection into a WorkbenchError (never raw debug output). */
export function toWorkbenchError(cause: unknown): WorkbenchError {
  if (isWorkbenchError(cause)) return cause;
  if (typeof cause === "string" && (COMMAND_ERROR_CODES as readonly string[]).includes(cause)) {
    return new WorkbenchError(cause as CommandError);
  }
  return new WorkbenchError("unexpected");
}

/** Best-effort human message for any rejection (never leaks raw payloads). */
export function describe(e: unknown): string {
  return toWorkbenchError(e).message;
}
