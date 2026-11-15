/*
 * Kanban board — columns mirror the backend task lifecycle. The status select is
 * a *request* to transition (04-core-page-specification.md §3): the component
 * never mutates task state locally; the parent submits the command and
 * reconciles from the authoritative response, restoring position on failure.
 * The select is also the keyboard-operable alternative to future drag-and-drop.
 */
import { Inbox } from "lucide-react";
import type { Task, TaskStatus } from "../../services/ipc/contracts";
import { TASK_STATUS_LABELS, TASK_STATUS_ORDER, TASK_TRANSITIONS } from "../../services/ipc/commands/tasks";
import { TASK_STATUS_META } from "./status";
import { Select, StatusBadge, EmptyState } from "../ui/primitives";
import "./data-display.css";

export interface KanbanBoardProps {
  tasks: Task[];
  projectNameOf: (task: Task) => string | null;
  onOpenTask: (taskId: string) => void;
  onTransition: (taskId: string, to: TaskStatus) => void;
  /** Task ids with a transition currently in flight — controls stay disabled. */
  pendingTransitionIds?: ReadonlySet<string>;
}

export function KanbanBoard({ tasks, projectNameOf, onOpenTask, onTransition, pendingTransitionIds }: KanbanBoardProps) {
  return (
    <div className="kanban-scroll">
      <div className="kanban-grid">
        {TASK_STATUS_ORDER.map((status) => {
          const columnTasks = tasks.filter((t) => t.status === status);
          return (
            <section key={status} className="kanban-column" aria-label={TASK_STATUS_LABELS[status]}>
              <div className="kanban-column-head">
                <span>{TASK_STATUS_META[status].label}</span>
                <span className="column-count">{columnTasks.length}</span>
              </div>
              <div className="kanban-column-body">
                {columnTasks.length === 0 ? (
                  <div style={{ display: "grid", placeItems: "center", flex: 1, padding: "var(--space-4) 0" }}>
                    <EmptyState icon={<Inbox size={20} aria-hidden />} title="暂无任务" />
                  </div>
                ) : (
                  columnTasks.map((task) => {
                    const targets = TASK_TRANSITIONS[task.status];
                    const pending = pendingTransitionIds?.has(task.id) ?? false;
                    return (
                      <article key={task.id} className="kanban-card">
                        <button type="button" className="kanban-card-title" onClick={() => onOpenTask(task.id)} title={task.title}>
                          {task.title}
                        </button>
                        <span className="kanban-card-meta">{projectNameOf(task) ?? "未关联项目"}</span>
                        <div className="kanban-card-foot">
                          <StatusBadge tone={TASK_STATUS_META[task.status].tone} label={TASK_STATUS_META[task.status].label} />
                          {targets.length > 0 && (
                            <Select
                              aria-label={`变更任务状态：${task.title}`}
                              value=""
                              disabled={pending}
                              onChange={(e) => {
                                const to = e.target.value as TaskStatus;
                                if (to) onTransition(task.id, to);
                              }}
                            >
                              <option value="" disabled>
                                {pending ? "提交中…" : "变更状态"}
                              </option>
                              {targets.map((to) => (
                                <option key={to} value={to}>
                                  → {TASK_STATUS_LABELS[to]}
                                </option>
                              ))}
                            </Select>
                          )}
                        </div>
                      </article>
                    );
                  })
                )}
              </div>
            </section>
          );
        })}
      </div>
    </div>
  );
}
