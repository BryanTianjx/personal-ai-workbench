// E1 vertical slice UI (Tauri-shell unit): exercises the typed Tauri command
// bridge end to end — create Workspace → create Project → list Projects —
// against the authoritative local SQLite store. This screen talks only to
// paw-desktop commands; it never touches storage directly (ADR-018).

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Serialized shapes of paw-application contract types (Workspace/Project).
// Errors arrive as the CommandError snake_case code enum from the command
// bridge (Gate C: safe errors only).
type Workspace = { id: string; name: string };
type Project = { id: string; workspace_id: string; name: string };
type CommandError =
  | "empty_name"
  | "name_too_long"
  | "workspace_not_found"
  | "invalid_workspace_id"
  | "unexpected";

const ERROR_TEXT: Record<CommandError, string> = {
  empty_name: "名称不能为空。",
  name_too_long: "名称超出长度限制。",
  workspace_not_found: "工作空间不存在。",
  invalid_workspace_id: "工作空间标识格式无效。",
  unexpected: "操作失败，请重试。",
};

function describe(e: unknown): string {
  if (typeof e === "string" && e in ERROR_TEXT) {
    return ERROR_TEXT[e as CommandError];
  }
  return ERROR_TEXT.unexpected;
}

export default function App() {
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [workspaceName, setWorkspaceName] = useState("");
  const [projectName, setProjectName] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const [projects, setProjects] = useState<Project[]>([]);
  const [projectsError, setProjectsError] = useState<string | null>(null);

  const reloadProjects = useCallback(() => {
    if (!workspace) {
      setProjects([]);
      setProjectsError(null);
      return;
    }
    invoke<Project[]>("list_projects", { workspaceId: workspace.id })
      .then((list) => {
        setProjects(list);
        setProjectsError(null);
      })
      .catch((e: unknown) => setProjectsError(describe(e)));
  }, [workspace?.id]);

  useEffect(() => {
    reloadProjects();
  }, [reloadProjects]);

  const createWorkspace = async () => {
    setBusy(true);
    setNotice(null);
    try {
      const ws = await invoke<Workspace>("create_workspace", {
        name: workspaceName,
      });
      setWorkspace(ws);
      setWorkspaceName("");
    } catch (e) {
      setNotice(describe(e));
    } finally {
      setBusy(false);
    }
  };

  const createProject = async () => {
    if (!workspace) return;
    setBusy(true);
    setNotice(null);
    try {
      await invoke<Project>("create_project", {
        workspaceId: workspace.id,
        name: projectName,
      });
      setProjectName("");
      reloadProjects();
    } catch (e) {
      setNotice(describe(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <main>
      <h1>自磨工坊 · Personal AI Workbench</h1>
      <p>E1 纵向切片：创建工作空间与项目，验证 Tauri 命令桥到 SQLite 的完整链路。</p>

      <section>
        <h2>工作空间</h2>
        {!workspace ? (
          <>
            <input
              value={workspaceName}
              onChange={(e) => setWorkspaceName(e.target.value)}
              placeholder="工作空间名称（1–120 字符）"
              disabled={busy}
            />
            <button onClick={createWorkspace} disabled={busy || !workspaceName.trim()}>
              创建工作空间
            </button>
          </>
        ) : (
          <p>
            当前工作空间：<strong>{workspace.name}</strong>
            <br />
            <code>{workspace.id}</code>
          </p>
        )}
      </section>

      <section>
        <h2>项目</h2>
        {workspace && (
          <>
            <input
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="项目名称（1–200 字符）"
              disabled={busy}
            />
            <button onClick={createProject} disabled={busy || !projectName.trim()}>
              创建项目
            </button>
          </>
        )}
        {projectsError ? (
          <p role="alert">{projectsError}</p>
        ) : (
          <ul>
            {projects.map((p) => (
              <li key={p.id}>
                {p.name} <code>{p.id}</code>
              </li>
            ))}
          </ul>
        )}
      </section>

      {notice && <p role="alert">{notice}</p>}
    </main>
  );
}
