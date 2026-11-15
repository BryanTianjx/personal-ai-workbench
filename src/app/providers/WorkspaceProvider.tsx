/*
 * Workspace selection state. The persisted id is only a presentation preference:
 * it is validated against the workspace list before use, and never treated as
 * authorization (07-ipc-integration-contract.md §6 — backend validates every op).
 */
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { createWorkspace, listWorkspaces } from "../../services/ipc/commands/workspaces";
import type { Workspace } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

const WORKSPACE_KEY = "paw.workspace-id";

interface WorkspaceContextValue {
  workspaces: Workspace[];
  workspaceId: string | null;
  workspace: Workspace | null;
  loading: boolean;
  error: string | null;
  reload: () => Promise<void>;
  selectWorkspace: (id: string) => void;
  createAndSelect: (name: string) => Promise<Workspace>;
}

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

export function useWorkspace(): WorkspaceContextValue {
  const ctx = useContext(WorkspaceContext);
  if (!ctx) throw new Error("useWorkspace must be used within WorkspaceProvider");
  return ctx;
}

export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await listWorkspaces();
      setWorkspaces(list);
      setWorkspaceId((current) => {
        const valid = current != null && list.some((w) => w.id === current);
        if (valid) return current;
        const persisted = window.localStorage.getItem(WORKSPACE_KEY);
        if (persisted && list.some((w) => w.id === persisted)) return persisted;
        return list[0]?.id ?? null;
      });
    } catch (cause) {
      setError(describe(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  // Persist as presentation preference only.
  useEffect(() => {
    if (workspaceId) window.localStorage.setItem(WORKSPACE_KEY, workspaceId);
  }, [workspaceId]);

  const selectWorkspace = useCallback((id: string) => {
    setWorkspaceId(id);
  }, []);

  const createAndSelect = useCallback(
    async (name: string) => {
      const created = await createWorkspace(name);
      setWorkspaces((prev) => [...prev, created]);
      setWorkspaceId(created.id);
      return created;
    },
    [],
  );

  const value = useMemo<WorkspaceContextValue>(
    () => ({
      workspaces,
      workspaceId,
      workspace: workspaces.find((w) => w.id === workspaceId) ?? null,
      loading,
      error,
      reload,
      selectWorkspace,
      createAndSelect,
    }),
    [workspaces, workspaceId, loading, error, reload, selectWorkspace, createAndSelect],
  );

  return <WorkspaceContext.Provider value={value}>{children}</WorkspaceContext.Provider>;
}
