/*
 * Areas — list, create, edit, delete (GUI-BIZ-002).
 * Deletion is destructive and requires explicit confirmation (08 §6).
 */
import { useState } from "react";
import { Layers, Plus, SquarePen, Trash2 } from "lucide-react";
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
  SkeletonRows,
  Textarea,
} from "../../components/ui/primitives";
import { ConfirmDialog, Dialog } from "../../components/ui/overlays";
import { useToast } from "../../components/ui/toast";
import { useAsyncData } from "../../hooks/useAsyncData";
import { createArea, deleteArea, listAreas, updateArea } from "../../services/ipc/commands/areas";
import type { Area } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

interface AreaDraft {
  name: string;
  description: string;
}

const EMPTY_DRAFT: AreaDraft = { name: "", description: "" };

export function AreasPage() {
  const ws = useWorkspace();
  const toast = useToast();
  const workspaceId = ws.workspaceId;
  const areas = useAsyncData(
    () => (workspaceId ? listAreas(workspaceId) : Promise.resolve([] as Area[])),
    [workspaceId],
  );

  const [formOpen, setFormOpen] = useState(false);
  const [editingArea, setEditingArea] = useState<Area | null>(null);
  const [draft, setDraft] = useState<AreaDraft>(EMPTY_DRAFT);
  const [nameError, setNameError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<Area | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);

  const openCreate = () => {
    setEditingArea(null);
    setDraft(EMPTY_DRAFT);
    setNameError(null);
    setFormOpen(true);
  };

  const openEdit = (area: Area) => {
    setEditingArea(area);
    setDraft({ name: area.name, description: area.description ?? "" });
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
      const input = { name: trimmed, description: draft.description.trim() || null };
      if (editingArea) {
        await updateArea(workspaceId, editingArea.id, input);
        toast.success("领域已更新。");
      } else {
        await createArea(workspaceId, input);
        toast.success("领域已创建。");
      }
      setFormOpen(false);
      areas.reload();
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
      await deleteArea(workspaceId, deleting.id);
      toast.success("领域已删除。");
      setDeleting(null);
      areas.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setDeleteBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="领域"
        description="领域是长期负责的范围，用于组织目标与项目。"
        actions={
          <Button variant="primary" onClick={openCreate}>
            <Plus size={15} aria-hidden />
            新建领域
          </Button>
        }
      />

      <Panel>
        {areas.error ? (
          <ErrorState message={areas.error} onRetry={areas.reload} />
        ) : areas.loading ? (
          <SkeletonRows rows={4} />
        ) : (areas.data ?? []).length === 0 ? (
          <EmptyState
            icon={<Layers size={24} aria-hidden />}
            title="还没有领域"
            hint="领域用于归类目标，例如「职业发展」「健康管理」。"
            action={
              <Button variant="primary" size="small" onClick={openCreate}>
                新建领域
              </Button>
            }
          />
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th scope="col">名称</th>
                <th scope="col">描述</th>
                <th scope="col" style={{ width: 96 }}>
                  操作
                </th>
              </tr>
            </thead>
            <tbody>
              {(areas.data ?? []).map((area) => (
                <tr key={area.id}>
                  <td style={{ fontWeight: 500 }}>{area.name}</td>
                  <td className="cell-muted" style={{ whiteSpace: "normal" }}>
                    {area.description ?? "—"}
                  </td>
                  <td>
                    <div style={{ display: "flex", gap: "var(--space-1)" }}>
                      <IconButton label={`编辑领域 ${area.name}`} onClick={() => openEdit(area)}>
                        <SquarePen size={15} aria-hidden />
                      </IconButton>
                      <IconButton label={`删除领域 ${area.name}`} onClick={() => setDeleting(area)}>
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
        title={editingArea ? "编辑领域" : "新建领域"}
        onClose={() => !saving && setFormOpen(false)}
        footer={
          <>
            <Button onClick={() => setFormOpen(false)} disabled={saving}>
              取消
            </Button>
            <Button variant="primary" onClick={submit} loading={saving}>
              {editingArea ? "保存" : "创建"}
            </Button>
          </>
        }
      >
        <Field label="名称" htmlFor="area-name" error={nameError}>
          <Input
            id="area-name"
            value={draft.name}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            maxLength={200}
            autoFocus
          />
        </Field>
        <Field label="描述（可选）" htmlFor="area-description">
          <Textarea
            id="area-description"
            value={draft.description}
            onChange={(e) => setDraft({ ...draft, description: e.target.value })}
          />
        </Field>
      </Dialog>

      <ConfirmDialog
        open={deleting != null}
        title="删除领域"
        danger
        busy={deleteBusy}
        message={
          <>
            将删除领域「<strong>{deleting?.name}</strong>」。此操作不可撤销。
            <br />
            关联的目标不会被删除，但会失去该领域归属。
          </>
        }
        confirmLabel="删除"
        onConfirm={confirmDelete}
        onCancel={() => setDeleting(null)}
      />
    </>
  );
}
