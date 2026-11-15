/*
 * Goals — list, create, edit, delete (GUI-BIZ-001). Goals may reference an Area.
 */
import { useState } from "react";
import { Plus, SquarePen, Target, Trash2 } from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { PageHeader } from "../../components/layout/Shell";
import {
  Button,
  EmptyState,
  ErrorState,
  Field,
  IconButton,
  Input,
  Panel,
  Select,
  SkeletonRows,
  Textarea,
} from "../../components/ui/primitives";
import { ConfirmDialog, Dialog } from "../../components/ui/overlays";
import { useToast } from "../../components/ui/toast";
import { useAsyncData } from "../../hooks/useAsyncData";
import { createGoal, deleteGoal, listGoals, updateGoal } from "../../services/ipc/commands/goals";
import { listAreas } from "../../services/ipc/commands/areas";
import type { Area, Goal } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

interface GoalDraft {
  name: string;
  areaId: string;
  description: string;
}

const EMPTY_DRAFT: GoalDraft = { name: "", areaId: "", description: "" };

export function GoalsPage() {
  const ws = useWorkspace();
  const toast = useToast();
  const workspaceId = ws.workspaceId;
  const goals = useAsyncData(
    () => (workspaceId ? listGoals(workspaceId) : Promise.resolve([] as Goal[])),
    [workspaceId],
  );
  const areas = useAsyncData(
    () => (workspaceId ? listAreas(workspaceId) : Promise.resolve([] as Area[])),
    [workspaceId],
  );

  const [formOpen, setFormOpen] = useState(false);
  const [editingGoal, setEditingGoal] = useState<Goal | null>(null);
  const [draft, setDraft] = useState<GoalDraft>(EMPTY_DRAFT);
  const [nameError, setNameError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<Goal | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);

  const areaNameOf = (areaId: string | null) => areas.data?.find((a) => a.id === areaId)?.name ?? null;

  const openCreate = () => {
    setEditingGoal(null);
    setDraft(EMPTY_DRAFT);
    setNameError(null);
    setFormOpen(true);
  };

  const openEdit = (goal: Goal) => {
    setEditingGoal(goal);
    setDraft({ name: goal.name, areaId: goal.area_id ?? "", description: goal.description ?? "" });
    setNameError(null);
    setFormOpen(true);
  };

  const submit = async () => {
    if (!workspaceId || saving) return;
    const trimmed = draft.name.trim();
    if (!trimmed) {
      setNameError("名称不能为空。");
      return;
    }
    setSaving(true);
    try {
      const input = {
        name: trimmed,
        areaId: draft.areaId || null,
        description: draft.description.trim() || null,
      };
      if (editingGoal) {
        await updateGoal(workspaceId, editingGoal.id, input);
        toast.success("目标已更新。");
      } else {
        await createGoal(workspaceId, input);
        toast.success("目标已创建。");
      }
      setFormOpen(false);
      goals.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setSaving(false);
    }
  };

  const confirmDelete = async () => {
    if (!workspaceId || !deleting || deleteBusy) return;
    setDeleteBusy(true);
    try {
      await deleteGoal(workspaceId, deleting.id);
      toast.success("目标已删除。");
      setDeleting(null);
      goals.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setDeleteBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="目标"
        description="目标表达想要达成的结果，可关联到领域，并支撑项目。"
        actions={
          <Button variant="primary" onClick={openCreate}>
            <Plus size={15} aria-hidden />
            新建目标
          </Button>
        }
      />

      <Panel>
        {goals.error ? (
          <ErrorState message={goals.error} onRetry={goals.reload} />
        ) : goals.loading ? (
          <SkeletonRows rows={4} />
        ) : (goals.data ?? []).length === 0 ? (
          <EmptyState
            icon={<Target size={24} aria-hidden />}
            title="还没有目标"
            hint="先设定一个目标，再用项目和任务推进它。"
            action={
              <Button variant="primary" size="small" onClick={openCreate}>
                新建目标
              </Button>
            }
          />
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th scope="col">名称</th>
                <th scope="col">所属领域</th>
                <th scope="col">描述</th>
                <th scope="col" style={{ width: 96 }}>
                  操作
                </th>
              </tr>
            </thead>
            <tbody>
              {(goals.data ?? []).map((goal) => (
                <tr key={goal.id}>
                  <td style={{ fontWeight: 500 }}>{goal.name}</td>
                  <td className="cell-muted">{areaNameOf(goal.area_id) ?? "未关联"}</td>
                  <td className="cell-muted" style={{ whiteSpace: "normal", maxWidth: 320 }}>
                    {goal.description ?? "—"}
                  </td>
                  <td>
                    <div style={{ display: "flex", gap: "var(--space-1)" }}>
                      <IconButton label={`编辑目标 ${goal.name}`} onClick={() => openEdit(goal)}>
                        <SquarePen size={15} aria-hidden />
                      </IconButton>
                      <IconButton label={`删除目标 ${goal.name}`} onClick={() => setDeleting(goal)}>
                        <Trash2 size={15} aria-hidden />
                      </IconButton>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>

      <Dialog
        open={formOpen}
        title={editingGoal ? "编辑目标" : "新建目标"}
        onClose={() => !saving && setFormOpen(false)}
        footer={
          <>
            <Button onClick={() => setFormOpen(false)} disabled={saving}>
              取消
            </Button>
            <Button variant="primary" onClick={submit} loading={saving}>
              {editingGoal ? "保存" : "创建"}
            </Button>
          </>
        }
      >
        <Field label="名称" htmlFor="goal-name" error={nameError}>
          <Input
            id="goal-name"
            value={draft.name}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            maxLength={200}
            autoFocus
          />
        </Field>
        <Field label="所属领域（可选）" htmlFor="goal-area">
          <Select id="goal-area" value={draft.areaId} onChange={(e) => setDraft({ ...draft, areaId: e.target.value })}>
            <option value="">未关联</option>
            {(areas.data ?? []).map((area) => (
              <option key={area.id} value={area.id}>
                {area.name}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="描述（可选）" htmlFor="goal-description">
          <Textarea
            id="goal-description"
            value={draft.description}
            onChange={(e) => setDraft({ ...draft, description: e.target.value })}
          />
        </Field>
      </Dialog>

      <ConfirmDialog
        open={deleting != null}
        title="删除目标"
        danger
        busy={deleteBusy}
        message={
          <>
            将删除目标「<strong>{deleting?.name}</strong>」。此操作不可撤销。
            <br />
            关联的项目不会被删除，但会失去该目标归属。
          </>
        }
        confirmLabel="删除"
        onConfirm={confirmDelete}
        onCancel={() => setDeleting(null)}
      />
    </>
  );
}
