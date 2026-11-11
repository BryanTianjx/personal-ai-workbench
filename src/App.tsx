// E2 vertical slice UI: the Work Management experience end to end —
// workspace picker (persisted selection), real-data Dashboard (PAW-F-004),
// Area/Goal/Project/Task management with typed status transitions, scoped
// search (E2-03) and task attachments (E2-04). This screen talks only to
// paw-desktop commands; it never touches storage directly (ADR-018).

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// ---- Serialized contract shapes (snake_case across the boundary) ----

type Workspace = { id: string; name: string };
type Area = { id: string; workspace_id: string; name: string; description: string | null };
type Goal = { id: string; workspace_id: string; area_id: string | null; name: string; description: string | null };
type Project = {
  id: string;
  workspace_id: string;
  goal_id: string | null;
  area_id: string | null;
  name: string;
  description: string | null;
  status: "active" | "paused" | "completed" | "archived";
};
type Task = {
  id: string;
  workspace_id: string;
  project_id: string | null;
  title: string;
  notes: string | null;
  status: "todo" | "in_progress" | "done" | "cancelled";
};
type Attachment = {
  id: string;
  workspace_id: string;
  owner_type: "project" | "task";
  owner_id: string;
  managed_id: string;
  file_name: string;
  size_bytes: number;
  sha256: string;
  created_at: number;
};
type SearchHit = { kind: "project" | "task"; id: string; workspace_id: string; title: string };
type Dashboard = {
  workspace_id: string;
  area_count: number;
  goal_count: number;
  projects_by_status: [string, number][];
  tasks_by_status: [string, number][];
  attachment_count: number;
};
type OperationRow = {
  id: string;
  kind: string;
  workspace_id: string | null;
  status:
    | "pending"
    | "running"
    | "awaiting_approval"
    | "retry_scheduled"
    | "outcome_unknown"
    | "succeeded"
    | "failed"
    | "cancelled";
  next_attempt_at: number | null;
  cancellation_requested_at: number | null;
};
type QuarantinedMessage = {
  id: string;
  event_type: string;
  attempts: number;
  redrive_count: number;
  last_error_category: string | null;
  last_redrive_by: string | null;
};

type CommandError =
  | "empty_name"
  | "name_too_long"
  | "notes_too_long"
  | "workspace_not_found"
  | "project_not_found"
  | "area_not_found"
  | "goal_not_found"
  | "task_not_found"
  | "invalid_workspace_id"
  | "task_status_invalid"
  | "invalid_search_query"
  | "attachment_not_found"
  | "owner_not_found"
  | "denied"
  | "unexpected";

const ERROR_TEXT: Record<CommandError, string> = {
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
  denied: "操作被拒绝。",
  unexpected: "操作失败，请重试。",
};

function describe(e: unknown): string {
  if (typeof e === "string" && e in ERROR_TEXT) {
    return ERROR_TEXT[e as CommandError];
  }
  return ERROR_TEXT.unexpected;
}

const WORKSPACE_KEY = "paw.workspace-id";

export default function App() {
  // ---- Workspace state ----
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [workspaceName, setWorkspaceName] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // ---- Record lists ----
  const [areas, setAreas] = useState<Area[]>([]);
  const [goals, setGoals] = useState<Goal[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);

  // ---- Form inputs ----
  const [areaName, setAreaName] = useState("");
  const [goalName, setGoalName] = useState("");
  const [goalArea, setGoalArea] = useState<string>("");
  const [projectName, setProjectName] = useState("");
  const [projectGoal, setProjectGoal] = useState<string>("");
  const [projectArea, setProjectArea] = useState<string>("");
  const [taskTitle, setTaskTitle] = useState("");
  const [taskProject, setTaskProject] = useState<string>("");
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [taskAttachments, setTaskAttachments] = useState<Record<string, Attachment[]>>({});
  const [operations, setOperations] = useState<OperationRow[]>([]);
  const [quarantined, setQuarantined] = useState<QuarantinedMessage[]>([]);

  const fail = (e: unknown) => setNotice(describe(e));

  // ---- Loaders ----
  const reloadWorkspaces = useCallback(() => {
    invoke<Workspace[]>("list_workspaces")
      .then((list) => {
        setWorkspaces(list);
        setWorkspaceId((current) => {
          if (current && list.some((w) => w.id === current)) return current;
          const saved = localStorage.getItem(WORKSPACE_KEY);
          if (saved && list.some((w) => w.id === saved)) return saved;
          return list[0]?.id ?? null;
        });
      })
      .catch(fail);
  }, []);

  const reloadAll = useCallback(() => {
    if (!workspaceId) {
      setAreas([]);
      setGoals([]);
      setProjects([]);
      setTasks([]);
      setDashboard(null);
      setTaskAttachments({});
      return;
    }
    localStorage.setItem(WORKSPACE_KEY, workspaceId);
    invoke<Area[]>("list_areas", { workspaceId })
      .then((r) => setAreas(r))
      .catch(fail);
    invoke<Goal[]>("list_goals", { workspaceId, areaId: null })
      .then((r) => setGoals(r))
      .catch(fail);
    invoke<Project[]>("list_projects", { workspaceId })
      .then((r) => setProjects(r))
      .catch(fail);
    invoke<Task[]>("list_tasks", { workspaceId, projectId: null, status: null })
      .then((r) => setTasks(r))
      .catch(fail);
    invoke<Dashboard>("dashboard", { workspaceId })
      .then((r) => setDashboard(r))
      .catch(fail);
    // Attachment lists per task (metadata only).
    invoke<Task[]>("list_tasks", { workspaceId, projectId: null, status: null })
      .then((list) =>
        Promise.all(
          list.map((t) =>
            invoke<Attachment[]>("list_attachments", {
              workspaceId,
              ownerType: "task",
              ownerId: t.id,
            }).then((atts) => [t.id, atts] as const),
          ),
        ),
      )
      .then((pairs) => setTaskAttachments(Object.fromEntries(pairs)))
      .catch(fail);
    // Operation queue view (E3-04): workspace-scoped operations + poison queue.
    invoke<OperationRow[]>("list_operations", { workspaceId })
      .then((r) => setOperations(r))
      .catch(fail);
    invoke<QuarantinedMessage[]>("list_quarantined_messages", { workspaceId })
      .then((r) => setQuarantined(r))
      .catch(fail);
  }, [workspaceId]);

  useEffect(() => {
    reloadWorkspaces();
  }, [reloadWorkspaces]);

  useEffect(() => {
    reloadAll();
  }, [reloadAll]);

  // ---- Actions ----
  const run = async (fn: () => Promise<unknown>, after?: () => void) => {
    setBusy(true);
    setNotice(null);
    try {
      await fn();
      after?.();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const createWorkspace = () =>
    run(
      () => invoke<Workspace>("create_workspace", { name: workspaceName }),
      () => {
        setWorkspaceName("");
        reloadWorkspaces();
      },
    );

  const createArea = () =>
    run(
      () =>
        invoke<Area>("create_area", {
          workspaceId,
          name: areaName,
          description: null,
        }),
      () => {
        setAreaName("");
        reloadAll();
      },
    );

  const createGoal = () =>
    run(
      () =>
        invoke<Goal>("create_goal", {
          workspaceId,
          areaId: goalArea || null,
          name: goalName,
          description: null,
        }),
      () => {
        setGoalName("");
        reloadAll();
      },
    );

  const createProject = () =>
    run(
      () => invoke<Project>("create_project", { workspaceId, name: projectName }),
      () => {
        setProjectName("");
        reloadAll();
      },
    );

  const linkProject = (p: Project) =>
    run(
      () =>
        invoke<Project>("update_project", {
          workspaceId,
          projectId: p.id,
          name: p.name,
          description: p.description,
          status: p.status,
          goalId: projectGoal || null,
          areaId: projectArea || null,
        }),
      reloadAll,
    );

  const createTask = () =>
    run(
      () =>
        invoke<Task>("create_task", {
          workspaceId,
          projectId: taskProject || null,
          title: taskTitle,
          notes: null,
        }),
      () => {
        setTaskTitle("");
        reloadAll();
      },
    );

  const transitionTask = (t: Task, to: Task["status"]) =>
    run(
      () =>
        invoke<Task>("transition_task", {
          workspaceId,
          taskId: t.id,
          to,
        }),
      reloadAll,
    );

  const deleteEntity = (kind: "area" | "goal" | "project" | "task", id: string) =>
    run(
      () =>
        invoke<void>(
          kind === "area"
            ? "delete_area"
            : kind === "goal"
              ? "delete_goal"
              : kind === "project"
                ? "delete_project"
                : "delete_task",
          kind === "task" ? { workspaceId, taskId: id } : { workspaceId, [`${kind}Id`]: id },
        ),
      reloadAll,
    );

  const addAttachment = (taskId: string, file: File) =>
    run(async () => {
      const bytes = new Uint8Array(await file.arrayBuffer());
      await invoke<Attachment>("add_attachment", {
        workspaceId,
        ownerType: "task",
        ownerId: taskId,
        fileName: file.name,
        bytes: Array.from(bytes),
      });
    }, reloadAll);

  const removeAttachment = (attachmentId: string) =>
    run(() => invoke<void>("remove_attachment", { workspaceId, attachmentId }), reloadAll);

  const openAttachment = (a: Attachment) =>
    run(async () => {
      // WebView2 blocks programmatic blob downloads; the Rust side exports
      // the bytes to a temp file and hands it to the OS default app.
      const path = await invoke<string>("open_attachment_externally", {
        workspaceId,
        attachmentId: a.id,
      });
      setNotice(`已在系统默认程序中打开：${a.file_name}（临时副本：${path}）`);
    });

  const runSearch = () =>
    run(async () => {
      const result = await invoke<SearchHit[]>("search", {
        workspaceId,
        query,
        limit: 20,
      });
      setHits(result);
    });

  // ---- Operation queue actions (E3-04) ----
  const cancelOperation = (op: OperationRow) =>
    run(async () => {
      await invoke<OperationRow>("request_operation_cancellation", {
        workspaceId,
        operationId: op.id,
      });
      setNotice(`已记录取消请求：${op.kind}（取消是否生效取决于能力声明与状态机）`);
      reloadAll();
    });

  const resumeOperation = (op: OperationRow) =>
    run(async () => {
      await invoke<OperationRow>("resume_operation", {
        workspaceId,
        operationId: op.id,
        note: "用户确认：结果已核对，可安全重试",
      });
      setNotice(`已恢复运行：${op.kind}`);
      reloadAll();
    });

  const redriveMessage = (m: QuarantinedMessage) =>
    run(async () => {
      const id = await invoke<string>("redrive_message", {
        workspaceId,
        messageId: m.id,
      });
      setNotice(`已重驱隔离事件 ${id}（保留事件 ID，操作者已记录）`);
      reloadAll();
    });

  const exportOperations = () =>
    run(async () => {
      const path = await invoke<string>("export_operations", { workspaceId });
      setNotice(`操作历史已导出：${path}`);
    });

  const purgeExpired = () =>
    run(async () => {
      const [operations, attempts, audit] = await invoke<[number, number, number]>(
        "purge_expired_data",
      );
      setNotice(`已清除过期数据：操作 ${operations}（含尝试 ${attempts}）、审计 ${audit}`);
      reloadAll();
    });

  // ---- Render ----
  return (
    <main>
      <h1>自磨工坊 · Personal AI Workbench</h1>
      <p>E2 纵向切片：Goal / Area / Project / Task、真实数据 Dashboard、范围化搜索与任务附件。</p>
      {notice && (
        <p role="alert">
          {notice}{" "}
          <button onClick={() => setNotice(null)}>关闭</button>
        </p>
      )}

      {/* Workspace picker + creation */}
      <section>
        <h2>工作空间</h2>
        {workspaces.length > 0 ? (
          <select
            value={workspaceId ?? ""}
            onChange={(e) => setWorkspaceId(e.target.value || null)}
            disabled={busy}
          >
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
          </select>
        ) : (
          <p>还没有工作空间——创建第一个：</p>
        )}
        {!workspaces.some((w) => w.id === workspaceId) || workspaces.length === 0 ? (
          <div>
            <input
              value={workspaceName}
              onChange={(e) => setWorkspaceName(e.target.value)}
              placeholder="工作空间名称（1–120 字符）"
              disabled={busy}
            />
            <button onClick={createWorkspace} disabled={busy || !workspaceName.trim()}>
              创建工作空间
            </button>
          </div>
        ) : null}
      </section>

      {workspaceId && (
        <>
          {/* Real-data dashboard */}
          <section>
            <h2>仪表板（真实数据）</h2>
            {dashboard ? (
              <ul>
                <li>领域：{dashboard.area_count}</li>
                <li>目标：{dashboard.goal_count}</li>
                <li>
                  项目：
                  {dashboard.projects_by_status.map(([s, n]) => `${s}=${n}`).join("，")}
                </li>
                <li>
                  任务：
                  {dashboard.tasks_by_status.map(([s, n]) => `${s}=${n}`).join("，")}
                </li>
                <li>附件：{dashboard.attachment_count}</li>
              </ul>
            ) : (
              <p>加载中…</p>
            )}
          </section>

          {/* Scoped search */}
          <section>
            <h2>搜索</h2>
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="搜索项目与任务（当前工作空间内）"
              disabled={busy}
            />
            <button onClick={runSearch} disabled={busy || !query.trim()}>
              搜索
            </button>
            {hits && (
              <ul>
                {hits.length === 0 && <li>无结果。</li>}
                {hits.map((h) => (
                  <li key={`${h.kind}-${h.id}`}>
                    [{h.kind === "project" ? "项目" : "任务"}] {h.title}
                  </li>
                ))}
              </ul>
            )}
          </section>

          {/* Areas */}
          <section>
            <h2>领域</h2>
            <input
              value={areaName}
              onChange={(e) => setAreaName(e.target.value)}
              placeholder="领域名称（1–120 字符）"
              disabled={busy}
            />
            <button onClick={createArea} disabled={busy || !areaName.trim()}>
              创建领域
            </button>
            <ul>
              {areas.map((a) => (
                <li key={a.id}>
                  {a.name}{" "}
                  <button onClick={() => deleteEntity("area", a.id)} disabled={busy}>
                    删除
                  </button>
                </li>
              ))}
            </ul>
          </section>

          {/* Goals */}
          <section>
            <h2>目标</h2>
            <input
              value={goalName}
              onChange={(e) => setGoalName(e.target.value)}
              placeholder="目标名称（1–200 字符）"
              disabled={busy}
            />
            <select
              value={goalArea}
              onChange={(e) => setGoalArea(e.target.value)}
              disabled={busy}
            >
              <option value="">（不归属领域）</option>
              {areas.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
            <button onClick={createGoal} disabled={busy || !goalName.trim()}>
              创建目标
            </button>
            <ul>
              {goals.map((g) => (
                <li key={g.id}>
                  {g.name}
                  {g.area_id && `（${areas.find((a) => a.id === g.area_id)?.name ?? "?"}）`}{" "}
                  <button onClick={() => deleteEntity("goal", g.id)} disabled={busy}>
                    删除
                  </button>
                </li>
              ))}
            </ul>
          </section>

          {/* Projects */}
          <section>
            <h2>项目</h2>
            <input
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="项目名称（1–200 字符）"
              disabled={busy}
            />
            <button onClick={createProject} disabled={busy || !projectName.trim()}>
              创建项目
            </button>
            <ul>
              {projects.map((p) => (
                <li key={p.id}>
                  <strong>{p.name}</strong>（{p.status}）{" "}
                  <button onClick={() => deleteEntity("project", p.id)} disabled={busy}>
                    删除
                  </button>
                  <div>
                    关联目标：
                    <select
                      value={p.goal_id ?? ""}
                      onChange={(e) => {
                        setProjectGoal(e.target.value);
                        setProjectArea(p.area_id ?? "");
                        linkProject({ ...p, goal_id: e.target.value || null });
                      }}
                      disabled={busy}
                    >
                      <option value="">（无）</option>
                      {goals.map((g) => (
                        <option key={g.id} value={g.id}>
                          {g.name}
                        </option>
                      ))}
                    </select>
                    关联领域：
                    <select
                      value={p.area_id ?? ""}
                      onChange={(e) => {
                        setProjectArea(e.target.value);
                        setProjectGoal(p.goal_id ?? "");
                        linkProject({ ...p, area_id: e.target.value || null });
                      }}
                      disabled={busy}
                    >
                      <option value="">（无）</option>
                      {areas.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.name}
                        </option>
                      ))}
                    </select>
                  </div>
                </li>
              ))}
            </ul>
          </section>

          {/* Tasks + attachments */}
          <section>
            <h2>任务</h2>
            <input
              value={taskTitle}
              onChange={(e) => setTaskTitle(e.target.value)}
              placeholder="任务标题（1–200 字符）"
              disabled={busy}
            />
            <select
              value={taskProject}
              onChange={(e) => setTaskProject(e.target.value)}
              disabled={busy}
            >
              <option value="">（独立任务）</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <button onClick={createTask} disabled={busy || !taskTitle.trim()}>
              创建任务
            </button>
            <ul>
              {tasks.map((t) => (
                <li key={t.id}>
                  <strong>{t.title}</strong>（{t.status}）{" "}
                  {t.status === "todo" && (
                    <button
                      onClick={() => transitionTask(t, "in_progress")}
                      disabled={busy}
                    >
                      开始
                    </button>
                  )}
                  {t.status === "in_progress" && (
                    <>
                      <button onClick={() => transitionTask(t, "done")} disabled={busy}>
                        完成
                      </button>
                      <button onClick={() => transitionTask(t, "todo")} disabled={busy}>
                        退回
                      </button>
                    </>
                  )}
                  {(t.status === "todo" || t.status === "in_progress") && (
                    <button onClick={() => transitionTask(t, "cancelled")} disabled={busy}>
                      取消
                    </button>
                  )}
                  <button onClick={() => deleteEntity("task", t.id)} disabled={busy}>
                    删除
                  </button>
                  <div>
                    <label>
                      添加附件：
                      <input
                        type="file"
                        disabled={busy}
                        onChange={(e) => {
                          const file = e.target.files?.[0];
                          if (file) addAttachment(t.id, file);
                          e.target.value = "";
                        }}
                      />
                    </label>
                    <ul>
                      {(taskAttachments[t.id] ?? []).map((a) => (
                        <li key={a.id}>
                          {a.file_name}（{a.size_bytes} 字节）{" "}
                          <button onClick={() => openAttachment(a)} disabled={busy}>
                            打开
                          </button>{" "}
                          <button onClick={() => removeAttachment(a.id)} disabled={busy}>
                            移除
                          </button>
                        </li>
                      ))}
                    </ul>
                  </div>
                </li>
              ))}
            </ul>
          </section>

          {/* Operation queue (E3-04) */}
          <section>
            <h2>操作队列</h2>
            <p>
              持久操作与隔离事件（当前工作空间）。导出先于清除；清除为显式动作，无后台自动清除。
            </p>
            <button onClick={exportOperations} disabled={busy}>
              导出操作历史（JSON）
            </button>{" "}
            <button onClick={purgeExpired} disabled={busy}>
              清除过期数据（默认策略）
            </button>
            <ul>
              {operations.length === 0 && <li>无操作记录。</li>}
              {operations.map((op) => (
                <li key={op.id}>
                  <strong>{op.kind}</strong>（{op.status}）
                  {op.next_attempt_at !== null &&
                    ` · 下次尝试 ${new Date(op.next_attempt_at).toLocaleString()}`}
                  {op.cancellation_requested_at !== null && " · 已请求取消"}
                  {op.status === "running" && (
                    <button onClick={() => cancelOperation(op)} disabled={busy}>
                      请求取消
                    </button>
                  )}
                  {op.status === "outcome_unknown" && (
                    <button onClick={() => resumeOperation(op)} disabled={busy}>
                      核对后恢复
                    </button>
                  )}
                </li>
              ))}
            </ul>
            <h3>隔离事件</h3>
            <ul>
              {quarantined.length === 0 && <li>无隔离事件。</li>}
              {quarantined.map((m) => (
                <li key={m.id}>
                  {m.event_type}（尝试 {m.attempts} 次，重驱 {m.redrive_count} 次
                  {m.last_error_category && `，原因：${m.last_error_category}`}
                  {m.last_redrive_by && `，上次操作者：${m.last_redrive_by}`}）{" "}
                  <button onClick={() => redriveMessage(m)} disabled={busy}>
                    重驱
                  </button>
                </li>
              ))}
            </ul>
          </section>
        </>
      )}
    </main>
  );
}
