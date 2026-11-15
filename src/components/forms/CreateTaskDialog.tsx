/*
 * Shared task-creation dialog (quick capture). Used by Dashboard, Tasks and the
 * Project workspace. Validation mirrors backend rules; the backend response
 * remains authoritative and the caller reconciles via onCreated.
 */
import { useState, type FormEvent } from "react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { createTask } from "../../services/ipc/commands/tasks";
import type { Project, Task } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";
import { Dialog } from "../ui/overlays";
import { Button, Field, Input, Select, Textarea } from "../ui/primitives";
import { useToast } from "../ui/toast";

export interface CreateTaskDialogProps {
  open: boolean;
  onClose: () => void;
  /** Workspace projects for the optional relationship select. */
  projects: Project[];
  /** Pre-select a project (e.g. when creating from a project context). */
  defaultProjectId?: string | null;
  /** Called after the backend confirms creation. */
  onCreated?: (task: Task) => void;
}

export function CreateTaskDialog({ open, onClose, projects, defaultProjectId = null, onCreated }: CreateTaskDialogProps) {
  const ws = useWorkspace();
  const toast = useToast();
  const [title, setTitle] = useState("");
  const [notes, setNotes] = useState("");
  const [projectId, setProjectId] = useState<string>(defaultProjectId ?? "");
  const [titleError, setTitleError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const reset = () => {
    setTitle("");
    setNotes("");
    setProjectId("");
    setTitleError(null);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!ws.workspaceId || saving) return;
    const trimmed = title.trim();
    if (!trimmed) {
      setTitleError("标题不能为空。");
      return;
    }
    setTitleError(null);
    setSaving(true);
    try {
      const created = await createTask(ws.workspaceId, {
        projectId: projectId || null,
        title: trimmed,
        notes: notes.trim() ? notes.trim() : null,
      });
      toast.success("任务已创建。");
      reset();
      onCreated?.(created);
      onClose();
    } catch (cause) {
      // Draft is preserved on recoverable errors (08 §7).
      toast.error(describe(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog
      open={open}
      title="新建任务"
      onClose={() => {
        if (!saving) {
          reset();
          onClose();
        }
      }}
      footer={
        <>
          <Button
            onClick={() => {
              reset();
              onClose();
            }}
            disabled={saving}
          >
            取消
          </Button>
          <Button variant="primary" onClick={submit} loading={saving}>
            创建任务
          </Button>
        </>
      }
    >
      <form onSubmit={submit} style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
        <Field label="标题" htmlFor="create-task-title" error={titleError}>
          <Input
            id="create-task-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="要完成什么？"
            autoFocus
          />
        </Field>
        <Field label="项目（可选）" htmlFor="create-task-project">
          <Select id="create-task-project" value={projectId} onChange={(e) => setProjectId(e.target.value)}>
            <option value="">未关联项目</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="备注（可选）" htmlFor="create-task-notes">
          <Textarea id="create-task-notes" value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="补充说明…" />
        </Field>
        <button type="submit" hidden aria-hidden />
      </form>
    </Dialog>
  );
}
