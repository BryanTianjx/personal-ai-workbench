/*
 * Projects — workspace list with create. Completion/archival are status writes
 * through updateProject; deletion is a separate hard delete (04 §2 constraints).
 */
import { useState } from "react";
import { ArrowRight, FolderKanban, Plus } from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { PageHeader } from "../../components/layout/Shell";
import { PROJECT_STATUS_META } from "../../components/data-display/status";
import {
  Button,
  EmptyState,
  ErrorState,
  Field,
  Input,
  Panel,
  SkeletonRows,
  StatusBadge,
} from "../../components/ui/primitives";
import { Dialog } from "../../components/ui/overlays";
import { useToast } from "../../components/ui/toast";
import { useAsyncData } from "../../hooks/useAsyncData";
import { createProject, listProjects } from "../../services/ipc/commands/projects";
import { listAreas } from "../../services/ipc/commands/areas";
import { listGoals } from "../../services/ipc/commands/goals";
import type { Area, Goal, Project } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

export function ProjectsPage({ navigate }: { navigate: (path: string) => void }) {
  const ws = useWorkspace();
  const toast = useToast();
  const workspaceId = ws.workspaceId;
  const projects = useAsyncData(
    () => (workspaceId ? listProjects(workspaceId) : Promise.resolve([] as Project[])),
    [workspaceId],
  );
  const goals = useAsyncData(
    () => (workspaceId ? listGoals(workspaceId) : Promise.resolve([] as Goal[])),
    [workspaceId],
  );
  const areas = useAsyncData(
    () => (workspaceId ? listAreas(workspaceId) : Promise.resolve([] as Area[])),
    [workspaceId],
  );

  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState("");
  const [nameError, setNameError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const goalNameOf = (goalId: string | null) => goals.data?.find((g) => g.id === goalId)?.name ?? null;
  const areaNameOf = (areaId: string | null) => areas.data?.find((a) => a.id === areaId)?.name ?? null;

  const submit = async () => {
    if (!workspaceId || saving) return;
    const trimmed = name.trim();
    if (!trimmed) {
      setNameError("名称不能为空。");
      return;
    }
    setSaving(true);
    try {
      await createProject(workspaceId, trimmed);
      toast.success("项目已创建。");
      setCreateOpen(false);
      setName("");
      setNameError(null);
      projects.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <PageHeader
        title="项目"
        description="项目是一组有明确产出的工作，可关联目标与领域。"
        actions={
          <Button
            variant="primary"
            onClick={() => {
              setName("");
              setNameError(null);
              setCreateOpen(true);
            }}
          >
            <Plus size={15} aria-hidden />
            新建项目
          </Button>
        }
      />

      <Panel>
        {projects.error ? (
          <ErrorState message={projects.error} onRetry={projects.reload} />
        ) : projects.loading ? (
          <SkeletonRows rows={4} />
        ) : (projects.data ?? []).length === 0 ? (
          <EmptyState
            icon={<FolderKanban size={24} aria-hidden />}
            title="还没有项目"
            hint="创建项目后，可以在其中组织任务与附件。"
            action={
              <Button variant="primary" size="small" onClick={() => setCreateOpen(true)}>
                新建项目
              </Button>
            }
          />
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th scope="col">名称</th>
                <th scope="col">状态</th>
                <th scope="col">目标</th>
                <th scope="col">领域</th>
                <th scope="col" style={{ width: 88 }} />
              </tr>
            </thead>
            <tbody>
              {(projects.data ?? []).map((project) => (
                <tr key={project.id}>
                  <td>
                    <button
                      type="button"
                      className="row-link"
                      onClick={() => navigate(`/projects/${encodeURIComponent(project.id)}`)}
                    >
                      {project.name}
                    </button>
                  </td>
                  <td className="cell-badge">
                    <StatusBadge
                      tone={PROJECT_STATUS_META[project.status].tone}
                      label={PROJECT_STATUS_META[project.status].label}
                    />
                  </td>
                  <td className="cell-muted">{goalNameOf(project.goal_id) ?? "未关联"}</td>
                  <td className="cell-muted">{areaNameOf(project.area_id) ?? "未关联"}</td>
                  <td>
                    <Button
                      size="small"
                      variant="tertiary"
                      onClick={() => navigate(`/projects/${encodeURIComponent(project.id)}`)}
                      aria-label={`打开项目 ${project.name}`}
                    >
                      打开
                      <ArrowRight size={13} aria-hidden />
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>

      <Dialog
        open={createOpen}
        title="新建项目"
        onClose={() => !saving && setCreateOpen(false)}
        footer={
          <>
            <Button onClick={() => setCreateOpen(false)} disabled={saving}>
              取消
            </Button>
            <Button variant="primary" onClick={submit} loading={saving}>
              创建
            </Button>
          </>
        }
      >
        <Field label="名称" htmlFor="project-name" error={nameError} hint="创建后可在项目详情中补充描述、目标与领域。">
          <Input
            id="project-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={200}
            autoFocus
            onKeyDown={(e) => e.key === "Enter" && submit()}
          />
        </Field>
      </Dialog>
    </>
  );
}
