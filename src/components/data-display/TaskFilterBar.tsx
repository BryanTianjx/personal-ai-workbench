/*
 * Task toolbar: local text filter, status filter and List/Board view switch.
 * Client-side filtering only narrows already-authorized workspace records;
 * scoping to the workspace remains a backend responsibility (07-ipc-integration-contract.md §6).
 */
import { List, Search, Columns3 } from "lucide-react";
import type { TaskStatus } from "../../services/ipc/contracts";
import { TASK_STATUS_LABELS, TASK_STATUS_ORDER } from "../../services/ipc/commands/tasks";
import { Select } from "../ui/primitives";
import "./data-display.css";

export type TaskViewMode = "list" | "board";

export interface TaskFilterBarProps {
  searchText: string;
  onSearchTextChange: (value: string) => void;
  statusFilter: TaskStatus | "all";
  onStatusFilterChange: (value: TaskStatus | "all") => void;
  view: TaskViewMode;
  onViewChange: (view: TaskViewMode) => void;
}

export function TaskFilterBar({
  searchText,
  onSearchTextChange,
  statusFilter,
  onStatusFilterChange,
  view,
  onViewChange,
}: TaskFilterBarProps) {
  return (
    <div className="filter-row">
      <label className="search-field">
        <Search size={15} aria-hidden />
        <input
          type="search"
          placeholder="筛选任务标题…"
          value={searchText}
          onChange={(e) => onSearchTextChange(e.target.value)}
          aria-label="按标题筛选任务"
        />
      </label>
      <Select
        aria-label="按状态筛选"
        value={statusFilter}
        onChange={(e) => onStatusFilterChange(e.target.value as TaskStatus | "all")}
        style={{ width: 132, flex: "none" }}
      >
        <option value="all">全部状态</option>
        {TASK_STATUS_ORDER.map((status) => (
          <option key={status} value={status}>
            {TASK_STATUS_LABELS[status]}
          </option>
        ))}
      </Select>
      <div className="view-toggle" role="group" aria-label="视图切换" style={{ marginLeft: "auto" }}>
        <button
          type="button"
          className={view === "list" ? "selected" : undefined}
          onClick={() => onViewChange("list")}
          aria-pressed={view === "list"}
        >
          <List size={14} aria-hidden />
          列表
        </button>
        <button
          type="button"
          className={view === "board" ? "selected" : undefined}
          onClick={() => onViewChange("board")}
          aria-pressed={view === "board"}
        >
          <Columns3 size={14} aria-hidden />
          看板
        </button>
      </div>
    </div>
  );
}
