/*
 * Dashboard metric card. Values come exclusively from the backend read model —
 * production never hard-codes or fabricates metrics (04-core-page-specification.md §1).
 */
import type { ReactNode } from "react";
import "./data-display.css";

export interface MetricCardProps {
  label: string;
  value: number | string;
  note?: string;
  icon?: ReactNode;
  onClick?: () => void;
}

export function MetricCard({ label, value, note, icon, onClick }: MetricCardProps) {
  const content = (
    <>
      <span className="metric-top">
        {label}
        {icon && (
          <span className="metric-icon" aria-hidden>
            {icon}
          </span>
        )}
      </span>
      <span className="metric-value">{value}</span>
      {note && <span className="metric-note">{note}</span>}
    </>
  );
  if (onClick) {
    return (
      <button type="button" className="metric-card" onClick={onClick}>
        {content}
      </button>
    );
  }
  return <div className="metric-card">{content}</div>;
}
