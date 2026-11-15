/*
 * Task table — generic presentational component shared by the Tasks page and
 * the Project workspace. Props in, callbacks out; no feature-service imports.
 */
import type { ReactNode } from "react";
import type { Task } from "../../services/ipc/contracts";
import { TASK_STATUS_META } from "./status";
import { StatusBadge, EmptyState } from "../ui/primitives";
import "./data-display.css";

export interface TaskTableProps {
  tasks: Task[];
  projectNameOf: (task: Task) => string | null;
  selectedTaskId?: string | null;
  onOpenTask: (taskId: string) => void;
  emptyState?: ReactNode;
}

export function TaskTable({ tasks, projectNameOf, selectedTaskId, onOpenTask, emptyState }: TaskTableProps) {
  if (tasks.length === 0) {
    return <>{emptyState ?? <EmptyState title="没有任务" hint="当前筛选条件下没有可显示的任务。" />}</>;
  }
  return (
    <table className="data-table">
      <thead>
        <tr>
          <th scope="col">任务</th>
          <th scope="col">项目</th>
          <th scope="col">状态</th>
        </tr>
      </thead>
      <tbody>
        {tasks.map((task) => (
          <tr
            key={task.id}
            className={selectedTaskId === task.id ? "selected" : undefined}
            aria-selected={selectedTaskId === task.id || undefined}
          >
            <td>
              <div className="cell-title">
                <button type="button" className="row-link" onClick={() => onOpenTask(task.id)} title={task.title}>
                  {task.title}
                </button>
              </div>
            </td>
            <td className="cell-muted">{projectNameOf(task) ?? "未关联项目"}</td>
            <td className="cell-badge">
              <StatusBadge tone={TASK_STATUS_META[task.status].tone} label={TASK_STATUS_META[task.status].label} />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
