/*
 * Dashboard — accurate summary of the active workspace (04-core-page-specification.md §1).
 * Every metric comes from the backend read model; nothing is hard-coded, no
 * progress percentages are invented, and Learning-domain counts stay hidden in
 * Alpha 0.1 (DR-003). Each section degrades independently.
 */
import { useState } from "react";
import { Circle, FolderKanban, Layers, ListTodo, Plus, Target } from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { PageHeader } from "../../components/layout/Shell";
import { MetricCard } from "../../components/data-display/MetricCard";
import { PROJECT_STATUS_META, TASK_STATUS_META } from "../../components/data-display/status";
import { CreateTaskDialog } from "../../components/forms/CreateTaskDialog";
import {
  Button,
  EmptyState,
  ErrorState,
  Panel,
  SkeletonRows,
  StatusBadge,
} from "../../components/ui/primitives";
import { useAsyncData } from "../../hooks/useAsyncData";
import { dashboardSummary } from "../../services/ipc/commands/dashboard";
import { listAreas } from "../../services/ipc/commands/areas";
import { listGoals } from "../../services/ipc/commands/goals";
import { listProjects } from "../../services/ipc/commands/projects";
import { listTasks } from "../../services/ipc/commands/tasks";
import type { Task, TaskStatus } from "../../services/ipc/contracts";

type TaskFocus = "all" | "open";

const OPEN_STATUSES: TaskStatus[] = ["todo", "in_progress"];

const PROJECT_LABEL = Object.fromEntries(
  Object.entries(PROJECT_STATUS_META).map(([key, meta]) => [key, meta.label]),
) as Record<string, string>;

export function DashboardPage({ navigate }: { navigate: (path: string) => void }) {
  const ws = useWorkspace();
  const workspaceId = ws.workspaceId;
  const [taskFocus, setTaskFocus] = useState<TaskFocus>("all");
  const [createOpen, setCreateOpen] = useState(false);

  const summary = useAsyncData(
    () => (workspaceId ? dashboardSummary(workspaceId) : Promise.resolve(null)),
    [workspaceId],
  );
  const projects = useAsyncData(
    () => (workspaceId ? listProjects(workspaceId) : Promise.resolve([])),
    [workspaceId],
  );
  const tasks = useAsyncData(() => (workspaceId ? listTasks(workspaceId) : Promise.resolve([])), [workspaceId]);
  const goals = useAsyncData(() => (workspaceId ? listGoals(workspaceId) : Promise.resolve([])), [workspaceId]);
  const areas = useAsyncData(() => (workspaceId ? listAreas(workspaceId) : Promise.resolve([])), [workspaceId]);

  const projectNameOf = (task: Task) => projects.data?.find((p) => p.id === task.project_id)?.name ?? null;

  const openCount = (summary.data?.tasks_by_status ?? [])
    .filter(([status]) => OPEN_STATUSES.includes(status as TaskStatus))
    .reduce((sum, [, count]) => sum + count, 0);
  const activeProjects = (summary.data?.projects_by_status ?? []).find(([status]) => status === "active")?.[1] ?? 0;

  const visibleTasks = (tasks.data ?? []).filter((task) => (taskFocus === "open" ? OPEN_STATUSES.includes(task.status) : true));
  const recentProjects = [...(projects.data ?? [])]
    .sort((a, b) => (a.status === "active" ? -1 : 1) - (b.status === "active" ? -1 : 1))
    .slice(0, 5);
  const areaNameOf = (areaId: string | null) => areas.data?.find((a) => a.id === areaId)?.name ?? null;

  return (
    <>
      <PageHeader
        title="仪表板"
        description={ws.workspace ? `工作空间「${ws.workspace.name}」的工作概览。` : undefined}
        actions={
          <Button variant="primary" onClick={() => setCreateOpen(true)}>
            <Plus size={15} aria-hidden />
            快速捕获
          </Button>
        }
      />

      <div className="metric-grid">
        <MetricCard
          label="待处理任务"
          value={summary.loading ? "…" : openCount}
          note="待办与进行中"
          icon={<ListTodo size={16} aria-hidden />}
          onClick={() => navigate("/tasks")}
        />
        <MetricCard
          label="进行中项目"
          value={summary.loading ? "…" : activeProjects}
          note="活跃项目总数"
          icon={<FolderKanban size={16} aria-hidden />}
          onClick={() => navigate("/projects")}
        />
        <MetricCard
          label="目标"
          value={summary.loading ? "…" : (summary.data?.goal_count ?? "…")}
          note="当前工作空间"
          icon={<Target size={16} aria-hidden />}
          onClick={() => navigate("/goals")}
        />
        <MetricCard
          label="领域"
          value={summary.loading ? "…" : (summary.data?.area_count ?? "…")}
          note="当前工作空间"
          icon={<Layers size={16} aria-hidden />}
          onClick={() => navigate("/areas")}
        />
      </div>

      <div className="dashboard-grid">
        <div className="stack">
          <Panel
            title="任务"
            actions={
              <div className="filter-row">
                <button
                  type="button"
                  className={taskFocus === "all" ? "filter active" : "filter"}
                  onClick={() => setTaskFocus("all")}
                >
                  全部
                </button>
                <button
                  type="button"
                  className={taskFocus === "open" ? "filter active" : "filter"}
                  onClick={() => setTaskFocus("open")}
                >
                  待处理
                </button>
              </div>
            }
          >
            {tasks.error ? (
              <ErrorState message={tasks.error} onRetry={tasks.reload} />
            ) : tasks.loading ? (
              <SkeletonRows rows={4} />
            ) : visibleTasks.length === 0 ? (
              <EmptyState
                icon={<Circle size={22} aria-hidden />}
                title="暂无任务"
                hint="从快速捕获开始记录要做的事。"
                action={
                  <Button size="small" variant="primary" onClick={() => setCreateOpen(true)}>
                    新建任务
                  </Button>
                }
              />
            ) : (
              <div className="list-rows" style={{ margin: "0 calc(var(--space-4) * -1)" }}>
                {visibleTasks.slice(0, 8).map((task) => (
                  <div key={task.id} className="list-row">
                    <div className="row-main">
                      <button type="button" className="row-title" onClick={() => navigate(`/tasks/${encodeURIComponent(task.id)}`)}>
                        {task.title}
                      </button>
                      <span className="row-sub">{projectNameOf(task) ?? "未关联项目"}</span>
                    </div>
                    <div className="row-side">
                      <StatusBadge
                        tone={TASK_STATUS_META[task.status].tone}
                        label={TASK_STATUS_META[task.status].label}
                      />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Panel>
        </div>

        <div className="stack">
          <Panel
            title="项目"
            actions={
              <Button variant="tertiary" size="small" onClick={() => navigate("/projects")}>
                全部项目
              </Button>
            }
          >
            {projects.error ? (
              <ErrorState message={projects.error} onRetry={projects.reload} />
            ) : projects.loading ? (
              <SkeletonRows rows={3} />
            ) : recentProjects.length === 0 ? (
              <EmptyState
                icon={<FolderKanban size={22} aria-hidden />}
                title="暂无项目"
                hint="创建第一个项目来组织任务。"
                action={
                  <Button size="small" variant="primary" onClick={() => navigate("/projects")}>
                    前往项目
                  </Button>
                }
              />
            ) : (
              <div className="list-rows" style={{ margin: "0 calc(var(--space-4) * -1)" }}>
                {recentProjects.map((project) => (
                  <div key={project.id} className="list-row">
                    <div className="row-main">
                      <button type="button" className="row-title" onClick={() => navigate(`/projects/${encodeURIComponent(project.id)}`)}>
                        {project.name}
                      </button>
                      <span className="row-sub">{areaNameOf(project.area_id) ?? "未关联领域"}</span>
                    </div>
                    <div className="row-side">
                      <StatusBadge
                        tone={project.status === "active" ? "success" : "neutral"}
                        label={PROJECT_LABEL[project.status]}
                      />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Panel>

          <Panel
            title="目标"
            actions={
              <Button variant="tertiary" size="small" onClick={() => navigate("/goals")}>
                全部目标
              </Button>
            }
          >
            {goals.error ? (
              <ErrorState message={goals.error} onRetry={goals.reload} />
            ) : goals.loading ? (
              <SkeletonRows rows={2} />
            ) : (goals.data ?? []).length === 0 ? (
              <EmptyState
                icon={<Target size={22} aria-hidden />}
                title="暂无目标"
                hint="目标用于组织长期方向。"
                action={
                  <Button size="small" variant="primary" onClick={() => navigate("/goals")}>
                    前往目标
                  </Button>
                }
              />
            ) : (
              <div className="list-rows" style={{ margin: "0 calc(var(--space-4) * -1)" }}>
                {(goals.data ?? []).slice(0, 4).map((goal) => (
                  <div key={goal.id} className="list-row">
                    <div className="row-main">
                      <span className="row-title" style={{ cursor: "default" }}>
                        {goal.name}
                      </span>
                      <span className="row-sub">{areaNameOf(goal.area_id) ?? "未关联领域"}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Panel>
        </div>
      </div>

      {workspaceId && (
        <CreateTaskDialog
          open={createOpen}
          onClose={() => setCreateOpen(false)}
          projects={projects.data ?? []}
          onCreated={() => {
            tasks.reload();
            summary.reload();
          }}
        />
      )}
    </>
  );
}
