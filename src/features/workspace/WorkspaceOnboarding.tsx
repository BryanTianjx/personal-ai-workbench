/*
 * Workspace onboarding — shown when no workspace is selected (02-application-shell.md §10:
 * shell initialization failure renders a blocking, actionable error).
 */
import { useState, type FormEvent } from "react";
import { Plus } from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { useToast } from "../../components/ui/toast";
import { Button, ErrorState, Field, Input, Panel, Spinner } from "../../components/ui/primitives";
import { describe } from "../../services/ipc/errors";

export function WorkspaceOnboarding() {
  const ws = useWorkspace();
  const toast = useToast();
  const [name, setName] = useState("");
  const [creating, setCreating] = useState(false);

  if (ws.loading) {
    return (
      <div style={{ display: "grid", placeItems: "center", minHeight: "40vh" }}>
        <Spinner label="正在加载工作空间" />
      </div>
    );
  }

  if (ws.error) {
    // Blocking startup failure: the rest of the shell has nothing to show.
    return (
      <Panel title="无法连接本地服务" className="onboarding-panel">
        <ErrorState title="初始化失败" message={ws.error} onRetry={() => void ws.reload()} />
      </Panel>
    );
  }

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed || creating) return;
    setCreating(true);
    try {
      await ws.createAndSelect(trimmed);
      toast.success("工作空间已创建。");
      setName("");
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setCreating(false);
    }
  };

  return (
    <Panel title="选择或创建工作空间" description="工作空间是所有记录的顶层作用域。">
      {ws.workspaces.length > 0 && (
        <div className="list-rows" style={{ marginBottom: "var(--space-4)" }}>
          {ws.workspaces.map((workspace) => (
            <button
              key={workspace.id}
              type="button"
              className="list-row"
              style={{ border: "var(--border-default)", borderRadius: "var(--radius-md)", marginBottom: "var(--space-2)", cursor: "pointer", background: "var(--color-surface)", width: "100%", textAlign: "left" }}
              onClick={() => ws.selectWorkspace(workspace.id)}
            >
              <span className="avatar" style={{ width: 28, height: 28, borderRadius: "50%", background: "var(--color-primary-050)", color: "var(--color-primary-700)", display: "grid", placeItems: "center", fontWeight: 700, flex: "none" }}>
                {workspace.name.slice(0, 1)}
              </span>
              <span className="row-main">
                <strong style={{ fontWeight: 600, fontSize: "var(--type-body-small-size)" }}>{workspace.name}</strong>
                <span className="row-sub">进入此工作空间</span>
              </span>
            </button>
          ))}
        </div>
      )}
      <form onSubmit={submit} style={{ display: "flex", gap: "var(--space-2)", alignItems: "flex-end", maxWidth: 440 }}>
        <Field label="新建工作空间" htmlFor="onboarding-workspace-name">
          <Input
            id="onboarding-workspace-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="例如：个人事务"
            maxLength={200}
          />
        </Field>
        <Button variant="primary" type="submit" loading={creating} disabled={!name.trim()}>
          <Plus size={15} aria-hidden />
          创建
        </Button>
      </form>
    </Panel>
  );
}
