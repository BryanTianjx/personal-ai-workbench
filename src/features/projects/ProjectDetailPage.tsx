/*
 * Project workspace — one place for project identity, relationships, work items
 * and resources (04-core-page-specification.md §2). Completion, archival and
 * deletion are distinct operations through their verified backend commands.
 */
import { useState } from "react";
import { FolderKanban, ListTodo, Plus, SquarePen, Trash2 } from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { PageHeader } from "../../components/layout/Shell";
import { PROJECT_STATUS_META } from "../../components/data-display/status";
import { KanbanBoard } from "../../components/data-display/KanbanBoard";
import { TaskTable } from "../../components/data-display/TaskTable";
import { CreateTaskDialog } from "../../components/forms/CreateTaskDialog";
import { AttachmentSection } from "../../components/forms/AttachmentSection";
import {
  Button,
  EmptyState,
  ErrorState,
  Field,
  Input,
  Panel,
  Select,
  Skeleton,
  SkeletonRows,
  StatusBadge,
  Tabs,
  Textarea,
} from "../../components/ui/primitives";
import { ConfirmDialog, Dialog } from "../../components/ui/overlays";
import { useToast } from "../../components/ui/toast";
import { useAsyncData } from "../../hooks/useAsyncData";
import { deleteProject, listProjects, updateProject } from "../../services/ipc/commands/projects";
import { listAreas } from "../../services/ipc/commands/areas";
import { listGoals } from "../../services/ipc/commands/goals";
import { listTasks, TASK_STATUS_LABELS, TASK_STATUS_ORDER, transitionTask } from "../../services/ipc/commands/tasks";
import type { Area, Goal, ProjectStatus, Task, TaskStatus } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

const PROJECT_STATUSES: ProjectStatus[] = ["active", "paused", "completed", "archived"];

export function ProjectDetailPage({ projectId, navigate }: { projectId: string; navigate: (path: string) => void }) {
  const ws = useWorkspace();
  const toast = useToast();
  const workspaceId = ws.workspaceId;

  const projects = useAsyncData(
    () => (workspaceId ? listProjects(workspaceId) : Promise.resolve([])),
    [workspaceId],
  );
  const tasks = useAsyncData(
    () => (workspaceId ? listTasks(workspaceId, { projectId }) : Promise.resolve([] as Task[])),
    [workspaceId, projectId],
  );
  const goals = useAsyncData(
    () => (workspaceId ? listGoals(workspaceId) : Promise.resolve([] as Goal[])),
    [workspaceId],
  );
  const areas = useAsyncData(
    () => (workspaceId ? listAreas(workspaceId) : Promise.resolve([] as Area[])),
    [workspaceId],
  );

  const project = projects.data?.find((p) => p.id === projectId) ?? null;

  const [tab, setTab] = useState("overview");
  const [createTaskOpen, setCreateTaskOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [pendingTransitions, setPendingTransitions] = useState<ReadonlySet<string>>(new Set());

  if (projects.error) {
    return (
      <Panel>
        <ErrorState message={projects.error} onRetry={projects.reload} />
      </Panel>
    );
  }
  if (projects.loading) {
    return (
      <>
        <Skeleton variant="text" style={{ width: "40%", height: 28 }} />
        <Panel>
          <SkeletonRows rows={5} />
        </Panel>
      </>
    );
  }
  if (!project) {
    return (
      <Panel>
        <EmptyState
          icon={<FolderKanban size={26} aria-hidden />}
          title="项目不存在"
          hint="它可能已被删除，或链接指向了其他工作空间的记录。"
          action={
            <Button variant="primary" onClick={() => navigate("/projects")}>
              返回项目列表
            </Button>
          }
        />
      </Panel>
    );
  }

  const goalName = goals.data?.find((g) => g.id === project.goal_id)?.name ?? null;
  const areaName = areas.data?.find((a) => a.id === project.area_id)?.name ?? null;
  const projectNameOf = (task: Task) =>
    task.project_id === project.id ? project.name : null;

  const transition = async (taskId: string, to: TaskStatus) => {
    if (!workspaceId || pendingTransitions.has(taskId)) return;
    setPendingTransitions((prev) => new Set(prev).add(taskId));
    try {
      await transitionTask(workspaceId, taskId, to);
      toast.success("任务状态已更新。");
      tasks.reload();
    } catch (cause) {
      // No optimistic move was made; authoritative state is unchanged.
      toast.error(describe(cause));
    } finally {
      setPendingTransitions((prev) => {
        const next = new Set(prev);
        next.delete(taskId);
        return next;
      });
    }
  };

  const confirmDelete = async () => {
    if (!workspaceId || deleteBusy) return;
    setDeleteBusy(true);
    try {
      await deleteProject(workspaceId, project.id);
      toast.success("项目已删除。");
      navigate("/projects");
    } catch (cause) {
      toast.error(describe(cause));
      setDeleteBusy(false);
      setDeleteOpen(false);
    }
  };

  const taskCounts = TASK_STATUS_ORDER.map((status) => ({
    status,
    count: (tasks.data ?? []).filter((t) => t.status === status).length,
  }));

  return (
    <>
      <PageHeader
        crumbs={[{ label: "项目", route: "/projects" }, { label: project.name }]}
        title={project.name}
        description={project.description ?? undefined}
        actions={
          <>
            <Button onClick={() => setEditOpen(true)}>
              <SquarePen size={14} aria-hidden />
              编辑项目
            </Button>
            <Button variant="danger" onClick={() => setDeleteOpen(true)}>
              <Trash2 size={14} aria-hidden />
              删除项目
            </Button>
            <Button variant="primary" onClick={() => setCreateTaskOpen(true)}>
              <Plus size={15} aria-hidden />
              新建任务
            </Button>
          </>
        }
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
          <StatusBadge
            tone={PROJECT_STATUS_META[project.status].tone}
            label={PROJECT_STATUS_META[project.status].label}
          />
        </div>
      </PageHeader>

      <Tabs
        ariaLabel="项目视图"
        active={tab}
        onChange={setTab}
        items={[
          { key: "overview", label: "概览" },
          { key: "tasks", label: "任务", count: tasks.data?.length },
          { key: "board", label: "看板", count: tasks.data?.length },
        ]}
      />

      {tab === "overview" && (
        <div className="dashboard-grid">
          <div className="stack">
            <Panel title="项目信息">
              <div className="detail-list">
                <div className="detail-row">
                  <span className="detail-label">描述</span>
                  <span className="detail-value">{project.description ?? "暂无描述"}</span>
                </div>
                <div className="detail-row">
                  <span className="detail-label">关联目标</span>
                  <span className="detail-value">{goalName ?? "未关联"}</span>
                </div>
                <div className="detail-row">
                  <span className="detail-label">所属领域</span>
                  <span className="detail-value">{areaName ?? "未关联"}</span>
                </div>
                <div className="detail-row">
                  <span className="detail-label">状态</span>
                  <span className="detail-value">
                    <StatusBadge tone={PROJECT_STATUS_META[project.status].tone} label={PROJECT_STATUS_META[project.status].label} />
                  </span>
                </div>
              </div>
            </Panel>
            <Panel title="附件">
              {workspaceId && <AttachmentSection workspaceId={workspaceId} ownerType="project" ownerId={project.id} />}
            </Panel>
          </div>
          <div className="stack">
            <Panel title="任务概览" description="按后端返回的任务状态统计。">
              {tasks.error ? (
                <ErrorState message={tasks.error} onRetry={tasks.reload} />
              ) : tasks.loading ? (
                <SkeletonRows rows={4} />
              ) : (
                <div className="detail-list">
                  {taskCounts.map(({ status, count }) => (
                    <div key={status} className="detail-row">
                      <span className="detail-label">{TASK_STATUS_LABELS[status]}</span>
                      <span className="detail-value" style={{ fontWeight: 600 }}>
                        {count}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </Panel>
          </div>
        </div>
      )}

      {tab === "tasks" && (
        <Panel
          title="任务列表"
          actions={
            <Button size="small" variant="primary" onClick={() => setCreateTaskOpen(true)}>
              <Plus size={14} aria-hidden />
              新建任务
            </Button>
          }
        >
          {tasks.error ? (
            <ErrorState message={tasks.error} onRetry={tasks.reload} />
          ) : tasks.loading ? (
            <SkeletonRows rows={4} />
          ) : (
            <TaskTable
              tasks={tasks.data ?? []}
              projectNameOf={projectNameOf}
              onOpenTask={(taskId) => navigate(`/tasks/${encodeURIComponent(taskId)}`)}
              emptyState={
                <EmptyState
                  icon={<ListTodo size={24} aria-hidden />}
                  title="项目暂无任务"
                  hint="为这个项目创建第一批任务。"
                  action={
                    <Button variant="primary" size="small" onClick={() => setCreateTaskOpen(true)}>
                      新建任务
                    </Button>
                  }
                />
              }
            />
          )}
        </Panel>
      )}

      {tab === "board" && (
        <Panel title="任务看板" description="通过状态选择器提交状态变更请求；变更由后端校验并确认。">
          {tasks.error ? (
            <ErrorState message={tasks.error} onRetry={tasks.reload} />
          ) : tasks.loading ? (
            <SkeletonRows rows={4} />
          ) : (
            <KanbanBoard
              tasks={tasks.data ?? []}
              projectNameOf={projectNameOf}
              onOpenTask={(taskId) => navigate(`/tasks/${encodeURIComponent(taskId)}`)}
              onTransition={(taskId, to) => void transition(taskId, to)}
              pendingTransitionIds={pendingTransitions}
            />
          )}
        </Panel>
      )}

      {workspaceId && (
        <CreateTaskDialog
          open={createTaskOpen}
          onClose={() => setCreateTaskOpen(false)}
          projects={[project]}
          defaultProjectId={project.id}
          onCreated={() => tasks.reload()}
        />
      )}

      <EditProjectDialog
        open={editOpen}
        project={project}
        goals={goals.data ?? []}
        areas={areas.data ?? []}
        workspaceId={workspaceId}
        onClose={() => setEditOpen(false)}
        onSaved={() => {
          setEditOpen(false);
          projects.reload();
        }}
      />

      <ConfirmDialog
        open={deleteOpen}
        title="删除项目"
        danger
        busy={deleteBusy}
        message={
          <>
            将永久删除项目「<strong>{project.name}</strong>」。此操作不可撤销。
          </>
        }
        confirmLabel="删除"
        onConfirm={confirmDelete}
        onCancel={() => setDeleteOpen(false)}
      />
    </>
  );
}

/* ---------- Edit dialog ---------- */
interface EditProjectDialogProps {
  open: boolean;
  project: { id: string; name: string; description: string | null; status: ProjectStatus; goal_id: string | null; area_id: string | null };
  goals: Goal[];
  areas: Area[];
  workspaceId: string | null;
  onClose: () => void;
  onSaved: () => void;
}

function EditProjectDialog({ open, project, goals, areas, workspaceId, onClose, onSaved }: EditProjectDialogProps) {
  const toast = useToast();
  const [draft, setDraft] = useState({
    name: project.name,
    description: project.description ?? "",
    status: project.status,
    goalId: project.goal_id ?? "",
    areaId: project.area_id ?? "",
  });
  const [nameError, setNameError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Re-seed the draft whenever a different project is edited.
  const [seededId, setSeededId] = useState(project.id);
  if (seededId !== project.id) {
    setSeededId(project.id);
    setDraft({
      name: project.name,
      description: project.description ?? "",
      status: project.status,
      goalId: project.goal_id ?? "",
      areaId: project.area_id ?? "",
    });
  }

  const submit = async () => {
    if (!workspaceId || saving) return;
    const trimmed = draft.name.trim();
    if (!trimmed) {
      setNameError("名称不能为空。");
      return;
    }
    setNameError(null);
    setSaving(true);
    try {
      await updateProject(workspaceId, project.id, {
        name: trimmed,
        description: draft.description.trim() || null,
        status: draft.status,
        goalId: draft.goalId || null,
        areaId: draft.areaId || null,
      });
      toast.success("项目已更新。");
      onSaved();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog
      open={open}
      title="编辑项目"
      onClose={() => !saving && onClose()}
      footer={
        <>
          <Button onClick={onClose} disabled={saving}>
            取消
          </Button>
          <Button variant="primary" onClick={submit} loading={saving}>
            保存
          </Button>
        </>
      }
    >
      <Field label="名称" htmlFor="edit-project-name" error={nameError}>
        <Input
          id="edit-project-name"
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
          maxLength={200}
          autoFocus
        />
      </Field>
      <Field label="描述（可选）" htmlFor="edit-project-description">
        <Textarea
          id="edit-project-description"
          value={draft.description}
          onChange={(e) => setDraft({ ...draft, description: e.target.value })}
        />
      </Field>
      <Field label="状态" htmlFor="edit-project-status" hint="完成与归档是不同的状态，按实际进度选择。">
        <Select
          id="edit-project-status"
          value={draft.status}
          onChange={(e) => setDraft({ ...draft, status: e.target.value as ProjectStatus })}
        >
          {PROJECT_STATUSES.map((status) => (
            <option key={status} value={status}>
              {PROJECT_STATUS_META[status].label}
            </option>
          ))}
        </Select>
      </Field>
      <Field label="关联目标（可选）" htmlFor="edit-project-goal">
        <Select id="edit-project-goal" value={draft.goalId} onChange={(e) => setDraft({ ...draft, goalId: e.target.value })}>
          <option value="">未关联</option>
          {goals.map((goal) => (
            <option key={goal.id} value={goal.id}>
              {goal.name}
            </option>
          ))}
        </Select>
      </Field>
      <Field label="所属领域（可选）" htmlFor="edit-project-area">
        <Select id="edit-project-area" value={draft.areaId} onChange={(e) => setDraft({ ...draft, areaId: e.target.value })}>
          <option value="">未关联</option>
          {areas.map((area) => (
            <option key={area.id} value={area.id}>
              {area.name}
            </option>
          ))}
        </Select>
      </Field>
    </Dialog>
  );
}
