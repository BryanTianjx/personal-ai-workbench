/*
 * Global search — Ctrl+K command palette and the /search page (03 §4, 04 §5).
 * Results come exclusively from the backend workspace-scoped search command and
 * are grouped by entity type. Empty query and no-result states are distinct.
 */
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { FolderKanban, ListTodo, Search, SearchX } from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { PageHeader } from "../../components/layout/Shell";
import { Panel, Spinner } from "../../components/ui/primitives";
import { searchWorkspace } from "../../services/ipc/commands/search";
import type { SearchHit } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";

/* ---------- shared query lifecycle (debounced) ---------- */
type SearchStatus = "idle" | "loading" | "done" | "error";

function useWorkspaceSearch(workspaceId: string | null, query: string) {
  const trimmed = query.trim();
  const [status, setStatus] = useState<SearchStatus>("idle");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!workspaceId || trimmed.length === 0) {
      setStatus("idle");
      setHits([]);
      setError(null);
      return;
    }
    let active = true;
    setStatus("loading");
    const timer = window.setTimeout(() => {
      searchWorkspace(workspaceId, trimmed, 20)
        .then((result) => {
          if (!active) return;
          setHits(result);
          setError(null);
          setStatus("done");
        })
        .catch((cause) => {
          if (!active) return;
          setHits([]);
          setError(describe(cause));
          setStatus("error");
        });
    }, 250);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [workspaceId, trimmed]);

  return { status, hits, error };
}

interface HitGroup {
  key: string;
  label: string;
  items: SearchHit[];
  routeOf: (hit: SearchHit) => string;
}

function useGroupedHits(hits: SearchHit[]): HitGroup[] {
  return useMemo(
    () => [
      {
        key: "project",
        label: "项目",
        items: hits.filter((h) => h.kind === "project"),
        routeOf: (hit: SearchHit) => `/projects/${encodeURIComponent(hit.id)}`,
      },
      {
        key: "task",
        label: "任务",
        items: hits.filter((h) => h.kind === "task"),
        routeOf: (hit: SearchHit) => `/tasks/${encodeURIComponent(hit.id)}`,
      },
    ],
    [hits],
  );
}

/* ---------- grouped rendering (no hooks — safe after early returns) ---------- */
function ResultsBody({
  status,
  error,
  groups,
  activeIndex,
  onHover,
  onSelect,
}: {
  status: SearchStatus;
  error: string | null;
  groups: HitGroup[];
  activeIndex: number;
  onHover?: (index: number) => void;
  onSelect?: (hit: SearchHit, route: string) => void;
}) {
  if (status === "idle") {
    return <div className="search-hint">输入关键词，搜索当前工作空间的项目与任务。</div>;
  }
  if (status === "loading") {
    return (
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-4)" }}>
        <Spinner label="搜索中" />
        <span style={{ color: "var(--color-text-muted)", fontSize: "var(--type-body-small-size)" }}>搜索中…</span>
      </div>
    );
  }
  if (status === "error") {
    return (
      <div className="search-hint" style={{ color: "var(--color-danger)" }}>
        {error}
      </div>
    );
  }
  const total = groups.reduce((sum, group) => sum + group.items.length, 0);
  if (total === 0) {
    return (
      <div className="search-hint">
        <SearchX size={18} aria-hidden />
        没有匹配的项目或任务，换个关键词试试。
      </div>
    );
  }
  let flatIndex = -1;
  return (
    <div className="search-results" role="listbox" aria-label="搜索结果">
      {groups.map((group) =>
        group.items.length === 0 ? null : (
          <div key={group.key}>
            <div className="search-group-label">{group.label}</div>
            {group.items.map((hit) => {
              flatIndex += 1;
              const index = flatIndex;
              const icon = group.key === "project" ? FolderKanban : ListTodo;
              const GroupIcon = icon;
              const route = group.routeOf(hit);
              return (
                <button
                  key={`${hit.kind}-${hit.id}`}
                  type="button"
                  role="option"
                  aria-selected={index === activeIndex}
                  className={index === activeIndex ? "search-result active" : "search-result"}
                  onMouseEnter={onHover ? () => onHover(index) : undefined}
                  onClick={() => onSelect?.(hit, route)}
                >
                  <GroupIcon size={15} aria-hidden style={{ flex: "none" }} />
                  <span className="search-result-title">{hit.title}</span>
                </button>
              );
            })}
          </div>
        ),
      )}
    </div>
  );
}

/* ---------- command palette (Ctrl+K) ---------- */
export function SearchDialog({
  open,
  onClose,
  onNavigate,
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (path: string) => void;
}) {
  const ws = useWorkspace();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const { status, hits, error } = useWorkspaceSearch(ws.workspaceId, open ? query : "");
  const groups = useGroupedHits(hits);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      window.setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [open]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query, hits]);

  const flatHits = groups.flatMap((group) => group.items);

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, Math.max(flatHits.length - 1, 0)));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
    } else if (event.key === "Enter") {
      const hit = flatHits[activeIndex];
      if (hit) {
        event.preventDefault();
        const group = groups.find((g) => g.items.includes(hit));
        if (group) onNavigate(group.routeOf(hit));
      }
    }
  };

  if (!open) return null;

  return (
    <div
      className="overlay-backdrop"
      style={{ paddingTop: "12vh" }}
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="search-modal" role="dialog" aria-modal="true" aria-label="全局搜索">
        <div className="search-input-row">
          <Search size={17} aria-hidden />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="搜索项目、任务…"
            aria-label="搜索项目、任务"
            spellCheck={false}
            autoFocus
          />
          <kbd>ESC</kbd>
        </div>
        <ResultsBody
          status={status}
          error={error}
          groups={groups}
          activeIndex={activeIndex}
          onHover={setActiveIndex}
          onSelect={(_hit, route) => onNavigate(route)}
        />
      </div>
    </div>
  );
}

/* ---------- standalone /search page ---------- */
export function SearchPage({ navigate }: { navigate: (path: string) => void }) {
  const ws = useWorkspace();
  const [query, setQuery] = useState("");
  const { status, hits, error } = useWorkspaceSearch(ws.workspaceId, query);
  const groups = useGroupedHits(hits);

  return (
    <>
      <PageHeader title="搜索" description="在当前工作空间内查找项目与任务。" />
      <Panel>
        <label className="search-field" style={{ maxWidth: "none", marginBottom: "var(--space-3)" }}>
          <Search size={16} aria-hidden />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="输入关键词…"
            aria-label="搜索项目、任务"
            autoFocus
          />
        </label>
        <ResultsBody
          status={status}
          error={error}
          groups={groups}
          activeIndex={0}
          onSelect={(_hit, route) => navigate(route)}
        />
      </Panel>
    </>
  );
}
