/*
 * Task detail drawer — inspection and edit composition (04-core-page-specification.md §3).
 * Draft state is explicit; Escape only closes when no unsaved change would be lost;
 * transitions submit commands and reconcile from the authoritative response.
 */
import { useState } from "react";
import { ArrowRightLeft, Pencil, Trash2 } from "lucide-react";
import { AttachmentSection } from "../../components/forms/AttachmentSection";
import { TASK_STATUS_META } from "../../components/data-display/status";
import {
  Button,
  EmptyState,
  Field,
  Input,
  Panel,
  Select,
  StatusBadge,
  Textarea,
} from "../../components/ui/primitives";
import { ConfirmDialog, Drawer } from "../../components/ui/overlays";
import { useToast } from "../../components/ui/toast";
import { TASK_STATUS_LABELS, TASK_TRANSITIONS, updateTaskDetails } from "../../services/ipc/commands/tasks";
import type { Project, Task, TaskStatus } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

export interface TaskDrawerProps {
  open: boolean;
  task: Task | null;
  projects: Project[];
  workspaceId: string | null;
  pendingTransitionIds: ReadonlySet<string>;
  onClose: () => void;
  onTransition: (taskId: string, to: TaskStatus) => void;
  onDelete: (taskId: string) => void;
  onChanged: () => void;
}

interface TaskDraft {
  title: string;
  notes: string;
  projectId: string;
}

export function TaskDrawer({
  open,
  task,
  projects,
  workspaceId,
  pendingTransitionIds,
  onClose,
  onTransition,
  onDelete,
  onChanged,
}: TaskDrawerProps) {
  const toast = useToast();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<TaskDraft>({ title: "", notes: "", projectId: "" });
  const [saving, setSaving] = useState(false);
  const [nameError, setNameError] = useState<string | null>(null);
  const [discardOpen, setDiscardOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);

  // Seed the draft per task id — never re-seed mid-edit when the list reloads.
  const [seededTaskId, setSeededTaskId] = useState<string | null>(null);
  if (open && task && seededTaskId !== task.id) {
    setSeededTaskId(task.id);
    setDraft({ title: task.title, notes: task.notes ?? "", projectId: task.project_id ?? "" });
    setEditing(false);
    setNameError(null);
  }

  const dirty =
    editing &&
    task != null &&
    (draft.title.trim() !== task.title ||
      draft.notes.trim() !== (task.notes ?? "") ||
      draft.projectId !== (task.project_id ?? ""));

  const requestClose = () => {
    if (dirty) {
      setDiscardOpen(true);
      return;
    }
    onClose();
  };

  const startEdit = () => {
    if (!task) return;
    setDraft({ title: task.title, notes: task.notes ?? "", projectId: task.project_id ?? "" });
    setNameError(null);
    setEditing(true);
  };

  const save = async () => {
    if (!workspaceId || !task || saving) return;
    const trimmed = draft.title.trim();
    if (!trimmed) {
      setNameError("标题不能为空。");
      return;
    }
    setSaving(true);
    try {
      await updateTaskDetails(workspaceId, task.id, {
        title: trimmed,
        notes: draft.notes.trim() || null,
        projectId: draft.projectId || null,
      });
      toast.success("任务已保存。");
      setEditing(false);
      onChanged();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setSaving(false);
    }
  };

  const confirmDelete = () => {
    if (!task || deleteBusy) return;
    setDeleteBusy(true);
    // Deletion flow is owned by the page (it closes the drawer and reloads).
    Promise.resolve(onDelete(task.id)).finally(() => setDeleteBusy(false));
  };

  const projectName = task ? projects.find((p) => p.id === task.project_id)?.name ?? null : null;
  const transitionPending = task ? pendingTransitionIds.has(task.id) : false;

  return (
    <>
      <Drawer
        open={open}
        title={task?.title ?? "任务详情"}
        onClose={requestClose}
        footer={
          task && (
            <>
              {dirty ? (
                <span className="dirty-hint">● 有未保存的更改</span>
              ) : (
                <span />
              )}
              <div style={{ display: "flex", gap: "var(--space-2)" }}>
                <Button variant="danger" onClick={() => setDeleteOpen(true)} disabled={saving}>
                  <Trash2 size={14} aria-hidden />
                  删除
                </Button>
                {editing ? (
                  <>
                    <Button
                      onClick={() => {
                        setEditing(false);
                        setNameError(null);
                      }}
                      disabled={saving}
                    >
                      取消
                    </Button>
                    <Button variant="primary" onClick={save} loading={saving}>
                      保存
                    </Button>
                  </>
                ) : (
                  <Button variant="primary" onClick={startEdit}>
                    <Pencil size={14} aria-hidden />
                    编辑
                  </Button>
                )}
              </div>
            </>
          )
        }
      >
        {!task ? (
          <EmptyState title="任务不存在" hint="它可能已被删除，或链接指向了其他工作空间的记录。" />
        ) : editing ? (
          <>
            <Field label="标题" htmlFor="task-drawer-title" error={nameError}>
              <Input
                id="task-drawer-title"
                value={draft.title}
                onChange={(e) => setDraft({ ...draft, title: e.target.value })}
                maxLength={200}
                autoFocus
              />
            </Field>
            <Field label="项目（可选）" htmlFor="task-drawer-project">
              <Select
                id="task-drawer-project"
                value={draft.projectId}
                onChange={(e) => setDraft({ ...draft, projectId: e.target.value })}
              >
                <option value="">未关联项目</option>
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="备注（可选）" htmlFor="task-drawer-notes">
              <Textarea
                id="task-drawer-notes"
                value={draft.notes}
                onChange={(e) => setDraft({ ...draft, notes: e.target.value })}
              />
            </Field>
          </>
        ) : (
          <>
            <div className="detail-list">
              <div className="detail-row">
                <span className="detail-label">状态</span>
                <span className="detail-value">
                  <StatusBadge tone={TASK_STATUS_META[task.status].tone} label={TASK_STATUS_META[task.status].label} />
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-label">项目</span>
                <span className="detail-value">{projectName ?? "未关联项目"}</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">备注</span>
                <span className="detail-value" style={{ whiteSpace: "pre-wrap" }}>
                  {task.notes ?? "暂无备注"}
                </span>
              </div>
            </div>

            {TASK_TRANSITIONS[task.status].length > 0 && (
              <Panel title="状态变更" description="变更将提交后端校验；非法变更会被拒绝并保持原状态。">
                <div style={{ display: "flex", flexWrap: "wrap", gap: "var(--space-2)" }}>
                  {TASK_TRANSITIONS[task.status].map((to) => (
                    <Button
                      key={to}
                      size="small"
                      disabled={transitionPending}
                      onClick={() => onTransition(task.id, to)}
                    >
                      <ArrowRightLeft size={13} aria-hidden />
                      {transitionPending ? "提交中…" : `设为「${TASK_STATUS_LABELS[to]}」`}
                    </Button>
                  ))}
                </div>
              </Panel>
            )}

            {workspaceId && (
              <Panel title="附件">
                <AttachmentSection workspaceId={workspaceId} ownerType="task" ownerId={task.id} />
              </Panel>
            )}
          </>
        )}
      </Drawer>

      <ConfirmDialog
        open={discardOpen}
        title="放弃更改？"
        message="任务的未保存修改将被丢弃。"
        confirmLabel="放弃更改"
        onCancel={() => setDiscardOpen(false)}
        onConfirm={() => {
          setDiscardOpen(false);
          setEditing(false);
          onClose();
        }}
      />

      <ConfirmDialog
        open={deleteOpen}
        title="删除任务"
        danger
        busy={deleteBusy}
        message={
          <>
            将永久删除任务「<strong>{task?.title}</strong>」。此操作不可撤销。
          </>
        }
        confirmLabel="删除"
        onConfirm={confirmDelete}
        onCancel={() => setDeleteOpen(false)}
      />
    </>
  );
}
