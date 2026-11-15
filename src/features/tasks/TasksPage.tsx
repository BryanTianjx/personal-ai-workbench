/*
 * Tasks — workspace task list with List/Kanban views (GUI-BIZ-004/005).
 * Transitions are backend-validated commands; failures restore the previous
 * position because no optimistic move is ever applied (08 §5).
 */
import { useEffect, useMemo, useState } from "react";
import { ListTodo, Plus } from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { PageHeader } from "../../components/layout/Shell";
import { KanbanBoard } from "../../components/data-display/KanbanBoard";
import { TaskTable } from "../../components/data-display/TaskTable";
import { TaskFilterBar, type TaskViewMode } from "../../components/data-display/TaskFilterBar";
import { CreateTaskDialog } from "../../components/forms/CreateTaskDialog";
import { Button, EmptyState, ErrorState, Panel, SkeletonRows } from "../../components/ui/primitives";
import { useToast } from "../../components/ui/toast";
import { TaskDrawer } from "./TaskDrawer";
import { useAsyncData } from "../../hooks/useAsyncData";
import { deleteTask, listTasks, transitionTask } from "../../services/ipc/commands/tasks";
import { listProjects } from "../../services/ipc/commands/projects";
import type { Project, Task, TaskStatus } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

export function TasksPage({
  focusTaskId,
  navigate,
}: {
  focusTaskId?: string;
  navigate: (path: string) => void;
}) {
  const ws = useWorkspace();
  const toast = useToast();
  const workspaceId = ws.workspaceId;

  const tasks = useAsyncData(
    () => (workspaceId ? listTasks(workspaceId) : Promise.resolve([] as Task[])),
    [workspaceId],
  );
  const projects = useAsyncData(
    () => (workspaceId ? listProjects(workspaceId) : Promise.resolve([] as Project[])),
    [workspaceId],
  );

  const [searchText, setSearchText] = useState("");
  const [statusFilter, setStatusFilter] = useState<TaskStatus | "all">("all");
  const [view, setView] = useState<TaskViewMode>("list");
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(focusTaskId ?? null);
  const [pendingTransitions, setPendingTransitions] = useState<ReadonlySet<string>>(new Set());
  const [createOpen, setCreateOpen] = useState(false);

  // Deep links (#/tasks/:id) drive the drawer; closing returns to /tasks.
  useEffect(() => {
    setSelectedTaskId(focusTaskId ?? null);
  }, [focusTaskId]);

  const projectNameOf = (task: Task) => projects.data?.find((p) => p.id === task.project_id)?.name ?? null;

  const filtered = useMemo(() => {
    const needle = searchText.trim().toLowerCase();
    return (tasks.data ?? []).filter((task) => {
      if (statusFilter !== "all" && task.status !== statusFilter) return false;
      if (needle && !task.title.toLowerCase().includes(needle)) return false;
      return true;
    });
  }, [tasks.data, searchText, statusFilter]);

  const openTask = (taskId: string) => {
    setSelectedTaskId(taskId);
    navigate(`/tasks/${encodeURIComponent(taskId)}`);
  };

  const closeTask = () => {
    setSelectedTaskId(null);
    navigate("/tasks");
  };

  const transition = async (taskId: string, to: TaskStatus) => {
    if (!workspaceId || pendingTransitions.has(taskId)) return;
    setPendingTransitions((prev) => new Set(prev).add(taskId));
    try {
      await transitionTask(workspaceId, taskId, to);
      toast.success("任务状态已更新。");
      tasks.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setPendingTransitions((prev) => {
        const next = new Set(prev);
        next.delete(taskId);
        return next;
      });
    }
  };

  const removeTask = async (taskId: string) => {
    if (!workspaceId) return;
    try {
      await deleteTask(workspaceId, taskId);
      toast.success("任务已删除。");
      closeTask();
      tasks.reload();
    } catch (cause) {
      toast.error(describe(cause));
    }
  };

  const selectedTask = tasks.data?.find((t) => t.id === selectedTaskId) ?? null;
  const drawerOpen = selectedTaskId != null && !tasks.loading;

  return (
    <>
      <PageHeader
        title="任务"
        description="当前工作空间的全部任务；状态变更由后端生命周期规则校验。"
        actions={
          <Button variant="primary" onClick={() => setCreateOpen(true)}>
            <Plus size={15} aria-hidden />
            新建任务
          </Button>
        }
      />

      <TaskFilterBar
        searchText={searchText}
        onSearchTextChange={setSearchText}
        statusFilter={statusFilter}
        onStatusFilterChange={setStatusFilter}
        view={view}
        onViewChange={setView}
      />

      <Panel>
        {tasks.error ? (
          <ErrorState message={tasks.error} onRetry={tasks.reload} />
        ) : tasks.loading ? (
          <SkeletonRows rows={6} />
        ) : view === "list" ? (
          <div style={{ margin: "0 calc(var(--space-4) * -1)" }}>
            <TaskTable
              tasks={filtered}
              projectNameOf={projectNameOf}
              selectedTaskId={selectedTaskId}
              onOpenTask={openTask}
              emptyState={
                <EmptyState
                  icon={<ListTodo size={24} aria-hidden />}
                  title={searchText || statusFilter !== "all" ? "没有匹配的任务" : "还没有任务"}
                  hint={searchText || statusFilter !== "all" ? "调整筛选条件，或新建一个任务。" : "用快速捕获或新建任务开始记录。"}
                  action={
                    <Button variant="primary" size="small" onClick={() => setCreateOpen(true)}>
                      新建任务
                    </Button>
                  }
                />
              }
            />
          </div>
        ) : (
          <KanbanBoard
            tasks={filtered}
            projectNameOf={projectNameOf}
            onOpenTask={openTask}
            onTransition={(taskId, to) => void transition(taskId, to)}
            pendingTransitionIds={pendingTransitions}
          />
        )}
      </Panel>

      {workspaceId && (
        <>
          <CreateTaskDialog
            open={createOpen}
            onClose={() => setCreateOpen(false)}
            projects={projects.data ?? []}
            onCreated={() => tasks.reload()}
          />
          <TaskDrawer
            open={drawerOpen}
            task={selectedTask}
            projects={projects.data ?? []}
            workspaceId={workspaceId}
            pendingTransitionIds={pendingTransitions}
            onClose={closeTask}
            onTransition={(taskId, to) => void transition(taskId, to)}
            onDelete={(taskId) => void removeTask(taskId)}
            onChanged={() => tasks.reload()}
          />
        </>
      )}
    </>
  );
}
