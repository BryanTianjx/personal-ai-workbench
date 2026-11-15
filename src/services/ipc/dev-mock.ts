/*
 * DEV-ONLY mock transport. Never imported by production code paths: client.ts
 * reaches it exclusively through a dynamic import guarded by `import.meta.env.DEV`.
 *
 * Implements the work-management command surface with backend-faithful semantics
 * (validation codes, task lifecycle, snake_case responses) over in-memory data.
 * This is a development harness, not a product feature (06-component-architecture.md §7).
 */
import type {
  Area,
  Attachment,
  DashboardSummary,
  Goal,
  OperationRow,
  Project,
  ProjectStatus,
  QuarantinedMessage,
  SearchHit,
  Task,
  TaskStatus,
  Workspace,
} from "./contracts";
import { WorkbenchError, type CommandError } from "./errors";

interface MockState {
  workspaces: Workspace[];
  areas: Area[];
  goals: Goal[];
  projects: Project[];
  tasks: Task[];
  attachments: Attachment[];
  operations: OperationRow[];
  quarantined: QuarantinedMessage[];
}

function reject(code: CommandError): never {
  throw new WorkbenchError(code);
}

function id(): string {
  return crypto.randomUUID();
}

function requireWorkspace(state: MockState, workspaceId: unknown): string {
  if (typeof workspaceId !== "string" || workspaceId.length === 0) {
    reject("invalid_workspace_id");
  }
  if (!state.workspaces.some((w) => w.id === workspaceId)) {
    reject("workspace_not_found");
  }
  return workspaceId;
}

function cleanName(name: unknown): string {
  if (typeof name !== "string" || name.trim().length === 0) reject("empty_name");
  if (name.trim().length > 200) reject("name_too_long");
  return name.trim();
}

function optionalText(value: unknown): string | null {
  if (value == null) return null;
  if (typeof value !== "string" || value.trim().length === 0) return null;
  return value.trim();
}

function seed(): MockState {
  const ws: Workspace = { id: id(), name: "个人工作空间" };
  const areaA: Area = { id: id(), workspace_id: ws.id, name: "个人效能", description: "工具与流程的自我改进" };
  const areaB: Area = { id: id(), workspace_id: ws.id, name: "技术成长", description: "工程能力与知识体系" };
  const goalA: Goal = {
    id: id(), workspace_id: ws.id, area_id: areaA.id,
    name: "建立可靠的个人工作台", description: "把日常事务沉淀为可复用的系统",
  };
  const goalB: Goal = {
    id: id(), workspace_id: ws.id, area_id: areaB.id,
    name: "补齐前端工程能力", description: "按路线图推进学习计划",
  };
  const projA: Project = {
    id: id(), workspace_id: ws.id, goal_id: goalA.id, area_id: areaA.id,
    name: "Personal AI Workbench", description: "本地优先的个人 AI 工作台（Alpha 0.1）", status: "active",
  };
  const projB: Project = {
    id: id(), workspace_id: ws.id, goal_id: goalB.id, area_id: areaB.id,
    name: "前端设计系统整理", description: "沉淀设计规范与组件库", status: "active",
  };
  const projC: Project = {
    id: id(), workspace_id: ws.id, goal_id: null, area_id: areaA.id,
    name: "家庭网络改造", description: null, status: "paused",
  };
  const tasks: Task[] = [
    { id: id(), workspace_id: ws.id, project_id: projA.id, title: "完成 GUI 设计规范 R1 实现评审", notes: "核对 IPC 契约与状态机映射", status: "in_progress" },
    { id: id(), workspace_id: ws.id, project_id: projA.id, title: "整理 Alpha 0.1 验收清单", notes: null, status: "todo" },
    { id: id(), workspace_id: ws.id, project_id: projA.id, title: "准备发布说明草稿", notes: null, status: "todo" },
    { id: id(), workspace_id: ws.id, project_id: projB.id, title: "梳理设计 token 命名", notes: "颜色/间距/圆角/阴影", status: "done" },
    { id: id(), workspace_id: ws.id, project_id: projB.id, title: "评审四个交互原型", notes: null, status: "in_progress" },
    { id: id(), workspace_id: ws.id, project_id: null, title: "整理书房线缆", notes: null, status: "todo" },
    { id: id(), workspace_id: ws.id, project_id: projC.id, title: "绘制新拓扑草图", notes: null, status: "cancelled" },
  ];
  const attachments: Attachment[] = [
    {
      id: id(), workspace_id: ws.id, owner: "task", owner_id: tasks[0].id,
      managed_id: "mock-managed-1", file_name: "评审记录.md", size_bytes: 2048,
      sha256: "0".repeat(64), created_at: Date.now() - 86_400_000,
    },
  ];
  const operations: OperationRow[] = [
    {
      id: id(), kind: "export", owner_domain: "work_management", owner_type: "workspace",
      owner_id: ws.id, workspace_id: ws.id, status: "succeeded", correlation_id: id(),
      next_attempt_at: null, cancellation_requested_at: null, updated_at: Date.now() - 3_600_000,
    },
    {
      id: id(), kind: "attachment.write", owner_domain: "work_management", owner_type: "task",
      owner_id: tasks[0].id, workspace_id: ws.id, status: "outcome_unknown", correlation_id: id(),
      next_attempt_at: Date.now() + 600_000, cancellation_requested_at: null, updated_at: Date.now() - 60_000,
    },
  ];
  const quarantined: QuarantinedMessage[] = [
    {
      id: id(), event_type: "attachment.write_failed", producer_domain: "work_management",
      aggregate_type: "task", aggregate_id: tasks[0].id, attempts: 2, redrive_count: 0,
      last_error_category: "unexpected", last_redrive_by: null, payload_json: "{}",
    },
  ];
  return { workspaces: [ws], areas: [areaA, areaB], goals: [goalA, goalB], projects: [projA, projB, projC], tasks, attachments, operations, quarantined };
}

const state: MockState = seed();

const TASK_TRANSITIONS: Record<TaskStatus, TaskStatus[]> = {
  todo: ["in_progress", "cancelled"],
  in_progress: ["done", "todo", "cancelled"],
  done: [],
  cancelled: [],
};

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function simulate<T>(result: T, ms = 160): Promise<T> {
  await delay(ms + Math.random() * 120);
  return structuredClone(result);
}

function dashboardSummary(workspaceId: string): DashboardSummary {
  const count = (rows: Array<{ status: string }>, key: string) => rows.filter((r) => r.status === key).length;
  return {
    workspace_id: workspaceId,
    area_count: state.areas.length,
    goal_count: state.goals.length,
    projects_by_status: (["active", "paused", "completed", "archived"] as ProjectStatus[]).map(
      (s) => [s, count(state.projects, s)] as [string, number],
    ),
    tasks_by_status: (["todo", "in_progress", "done", "cancelled"] as TaskStatus[]).map(
      (s) => [s, count(state.tasks, s)] as [string, number],
    ),
    attachment_count: state.attachments.length,
    roadmap_count: 0,
    learning_plan_count: 0,
    learning_item_count: 0,
    learning_session_count: 0,
  };
}

export async function mockInvoke<TResponse>(command: string, args: Record<string, unknown>): Promise<TResponse> {
  switch (command) {
    case "list_workspaces":
      return simulate(state.workspaces) as Promise<TResponse>;
    case "create_workspace": {
      const ws: Workspace = { id: id(), name: cleanName(args.name) };
      state.workspaces.push(ws);
      return simulate(ws) as Promise<TResponse>;
    }
    case "list_areas": {
      const wsId = requireWorkspace(state, args.workspaceId);
      return simulate(state.areas.filter((a) => a.workspace_id === wsId)) as Promise<TResponse>;
    }
    case "create_area": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const area: Area = { id: id(), workspace_id: wsId, name: cleanName(args.name), description: optionalText(args.description) };
      state.areas.push(area);
      return simulate(area) as Promise<TResponse>;
    }
    case "update_area": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const area = state.areas.find((a) => a.id === args.areaId && a.workspace_id === wsId);
      if (!area) reject("area_not_found");
      area.name = cleanName(args.name);
      area.description = optionalText(args.description);
      return simulate(area) as Promise<TResponse>;
    }
    case "delete_area": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const idx = state.areas.findIndex((a) => a.id === args.areaId && a.workspace_id === wsId);
      if (idx < 0) reject("area_not_found");
      state.areas.splice(idx, 1);
      return simulate(null, 120) as Promise<TResponse>;
    }
    case "list_goals": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const areaId = optionalText(args.areaId);
      return simulate(state.goals.filter((g) => g.workspace_id === wsId && (areaId == null || g.area_id === areaId))) as Promise<TResponse>;
    }
    case "create_goal": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const goal: Goal = {
        id: id(), workspace_id: wsId, area_id: optionalText(args.areaId),
        name: cleanName(args.name), description: optionalText(args.description),
      };
      state.goals.push(goal);
      return simulate(goal) as Promise<TResponse>;
    }
    case "update_goal": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const goal = state.goals.find((g) => g.id === args.goalId && g.workspace_id === wsId);
      if (!goal) reject("goal_not_found");
      goal.area_id = optionalText(args.areaId);
      goal.name = cleanName(args.name);
      goal.description = optionalText(args.description);
      return simulate(goal) as Promise<TResponse>;
    }
    case "delete_goal": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const idx = state.goals.findIndex((g) => g.id === args.goalId && g.workspace_id === wsId);
      if (idx < 0) reject("goal_not_found");
      state.goals.splice(idx, 1);
      return simulate(null, 120) as Promise<TResponse>;
    }
    case "list_projects": {
      const wsId = requireWorkspace(state, args.workspaceId);
      return simulate(state.projects.filter((p) => p.workspace_id === wsId)) as Promise<TResponse>;
    }
    case "create_project": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const project: Project = {
        id: id(), workspace_id: wsId, goal_id: null, area_id: null,
        name: cleanName(args.name), description: null, status: "active",
      };
      state.projects.push(project);
      return simulate(project) as Promise<TResponse>;
    }
    case "update_project": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const project = state.projects.find((p) => p.id === args.projectId && p.workspace_id === wsId);
      if (!project) reject("project_not_found");
      project.name = cleanName(args.name);
      project.description = optionalText(args.description);
      project.goal_id = optionalText(args.goalId);
      project.area_id = optionalText(args.areaId);
      project.status = args.status as ProjectStatus;
      return simulate(project) as Promise<TResponse>;
    }
    case "delete_project": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const idx = state.projects.findIndex((p) => p.id === args.projectId && p.workspace_id === wsId);
      if (idx < 0) reject("project_not_found");
      state.projects.splice(idx, 1);
      return simulate(null, 120) as Promise<TResponse>;
    }
    case "list_tasks": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const projectId = optionalText(args.projectId);
      const status = optionalText(args.status);
      return simulate(
        state.tasks.filter(
          (t) =>
            t.workspace_id === wsId &&
            (projectId == null || t.project_id === projectId) &&
            (status == null || t.status === status),
        ),
      ) as Promise<TResponse>;
    }
    case "create_task": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const task: Task = {
        id: id(), workspace_id: wsId, project_id: optionalText(args.projectId),
        title: cleanName(args.title), notes: optionalText(args.notes), status: "todo",
      };
      state.tasks.push(task);
      return simulate(task) as Promise<TResponse>;
    }
    case "update_task_details": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const task = state.tasks.find((t) => t.id === args.taskId && t.workspace_id === wsId);
      if (!task) reject("task_not_found");
      task.title = cleanName(args.title);
      task.notes = optionalText(args.notes);
      task.project_id = optionalText(args.projectId);
      return simulate(task) as Promise<TResponse>;
    }
    case "transition_task": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const task = state.tasks.find((t) => t.id === args.taskId && t.workspace_id === wsId);
      if (!task) reject("task_not_found");
      const to = args.to as TaskStatus;
      if (!TASK_TRANSITIONS[task.status].includes(to)) reject("task_status_invalid");
      task.status = to;
      return simulate(task) as Promise<TResponse>;
    }
    case "delete_task": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const idx = state.tasks.findIndex((t) => t.id === args.taskId && t.workspace_id === wsId);
      if (idx < 0) reject("task_not_found");
      state.tasks.splice(idx, 1);
      return simulate(null, 120) as Promise<TResponse>;
    }
    case "dashboard": {
      const wsId = requireWorkspace(state, args.workspaceId);
      return simulate(dashboardSummary(wsId)) as Promise<TResponse>;
    }
    case "search": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const query = typeof args.query === "string" ? args.query.trim() : "";
      if (query.length === 0) reject("invalid_search_query");
      const limit = typeof args.limit === "number" ? args.limit : 20;
      const hits: SearchHit[] = [];
      for (const p of state.projects) {
        if (p.workspace_id === wsId && p.name.toLowerCase().includes(query.toLowerCase())) {
          hits.push({ kind: "project", id: p.id, workspace_id: wsId, title: p.name });
        }
      }
      for (const t of state.tasks) {
        if (t.workspace_id === wsId && t.title.toLowerCase().includes(query.toLowerCase())) {
          hits.push({ kind: "task", id: t.id, workspace_id: wsId, title: t.title });
        }
      }
      return simulate(hits.slice(0, limit)) as Promise<TResponse>;
    }
    case "list_attachments": {
      const wsId = requireWorkspace(state, args.workspaceId);
      return simulate(
        state.attachments.filter(
          (a) => a.workspace_id === wsId && a.owner === args.ownerType && a.owner_id === args.ownerId,
        ),
      ) as Promise<TResponse>;
    }
    case "add_attachment": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const ownerType = args.ownerType;
      const ownerId = args.ownerId;
      const ownerExists =
        (ownerType === "project" && state.projects.some((p) => p.id === ownerId && p.workspace_id === wsId)) ||
        (ownerType === "task" && state.tasks.some((t) => t.id === ownerId && t.workspace_id === wsId));
      if (!ownerExists) reject("owner_not_found");
      const fileName = cleanName(args.fileName);
      // Mirror backend validation: no separators, no "..", no control chars.
      if (/[\\/]|\.\.|[\x00-\x1f]/.test(fileName)) reject("unexpected");
      const bytes = args.bytes;
      const attachment: Attachment = {
        id: id(), workspace_id: wsId, owner: ownerType as "project" | "task", owner_id: ownerId as string,
        managed_id: id(), file_name: fileName,
        size_bytes: Array.isArray(bytes) ? bytes.length : 0,
        sha256: "1".repeat(64), created_at: Date.now(),
      };
      state.attachments.push(attachment);
      return simulate(attachment, 240) as Promise<TResponse>;
    }
    case "remove_attachment": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const idx = state.attachments.findIndex((a) => a.id === args.attachmentId && a.workspace_id === wsId);
      if (idx < 0) reject("attachment_not_found");
      state.attachments.splice(idx, 1);
      return simulate(null, 120) as Promise<TResponse>;
    }
    case "open_attachment_externally": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const attachment = state.attachments.find((a) => a.id === args.attachmentId && a.workspace_id === wsId);
      if (!attachment) reject("attachment_not_found");
      return simulate(`C:\\Users\\Public\\paw-attachments\\${attachment.file_name}`) as Promise<TResponse>;
    }
    case "list_operations": {
      const wsId = requireWorkspace(state, args.workspaceId);
      return simulate(state.operations.filter((o) => o.workspace_id === wsId)) as Promise<TResponse>;
    }
    case "request_operation_cancellation": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const op = state.operations.find((o) => o.id === args.operationId && o.workspace_id === wsId);
      if (!op) reject("operation_not_found");
      op.status = "cancelled";
      op.cancellation_requested_at = Date.now();
      return simulate(op) as Promise<TResponse>;
    }
    case "resume_operation": {
      const wsId = requireWorkspace(state, args.workspaceId);
      const op = state.operations.find((o) => o.id === args.operationId && o.workspace_id === wsId);
      if (!op) reject("operation_not_found");
      op.status = "retry_scheduled";
      op.next_attempt_at = Date.now() + 60_000;
      return simulate(op) as Promise<TResponse>;
    }
    case "list_quarantined_messages": {
      requireWorkspace(state, args.workspaceId);
      return simulate(state.quarantined) as Promise<TResponse>;
    }
    case "redrive_message": {
      requireWorkspace(state, args.workspaceId);
      const msg = state.quarantined.find((q) => q.id === args.messageId);
      if (!msg) reject("operation_not_found");
      msg.redrive_count += 1;
      msg.last_redrive_by = "user";
      return simulate(id()) as Promise<TResponse>;
    }
    case "export_operations": {
      requireWorkspace(state, args.workspaceId);
      return simulate("C:\\Users\\Public\\paw-exports\\operations.json") as Promise<TResponse>;
    }    case "purge_expired_data":
      return simulate([2, 0, 1]) as Promise<TResponse>;
    default:
      reject("unexpected");
  }
}
