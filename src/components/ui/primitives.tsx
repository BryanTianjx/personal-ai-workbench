/*
 * Shared UI primitives (design-system layer). These components must never import
 * feature services (06-component-architecture.md §3) — data flows in via props.
 */
import {
  forwardRef,
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
  type SelectHTMLAttributes,
  type TextareaHTMLAttributes,
} from "react";
import { Loader2, RefreshCw } from "lucide-react";
import "./ui.css";

/* ---------- Button ---------- */
export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "tertiary" | "danger";
  size?: "default" | "small";
  loading?: boolean;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "default", loading = false, disabled, children, className, ...rest },
  ref,
) {
  const classes = ["btn"];
  if (variant !== "secondary") classes.push(variant);
  if (size !== "default") classes.push(size);
  if (className) classes.push(className);
  return (
    <button
      ref={ref}
      type="button"
      className={classes.join(" ")}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      {...rest}
    >
      {loading && <Loader2 className="spinner" aria-hidden />}
      {children}
    </button>
  );
});

/* ---------- Icon button (always needs an accessible name) ---------- */
export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Accessible name; also used as the tooltip. Icon-only buttons require it. */
  label: string;
}

export function IconButton({ label, className, children, ...rest }: IconButtonProps) {
  return (
    <button type="button" className={["icon-btn", className].filter(Boolean).join(" ")} aria-label={label} title={label} {...rest}>
      {children}
    </button>
  );
}

/* ---------- Form fields (labels are mandatory; placeholder is not a label) ---------- */
export interface FieldProps {
  label: string;
  htmlFor?: string;
  error?: string | null;
  hint?: string;
  children: ReactNode;
}

export function Field({ label, htmlFor, error, hint, children }: FieldProps) {
  return (
    <div className="field">
      <label htmlFor={htmlFor}>{label}</label>
      {children}
      {hint && !error && (
        <span style={{ fontSize: "var(--type-caption-size)", color: "var(--color-text-muted)" }}>{hint}</span>
      )}
      {error && (
        <span className="field-error" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...rest }, ref) {
    return <input ref={ref} className={["input", className].filter(Boolean).join(" ")} {...rest} />;
  },
);

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(
  function Textarea({ className, ...rest }, ref) {
    return <textarea ref={ref} className={["textarea", className].filter(Boolean).join(" ")} {...rest} />;
  },
);

export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(
  function Select({ className, ...rest }, ref) {
    return <select ref={ref} className={["select", className].filter(Boolean).join(" ")} {...rest} />;
  },
);

/* ---------- Status badge: label + color, never color alone ---------- */
export type BadgeTone = "neutral" | "primary" | "success" | "warning" | "danger" | "info";

export function StatusBadge({ tone = "neutral", label, dot = true }: { tone?: BadgeTone; label: string; dot?: boolean }) {
  return (
    <span className={tone === "neutral" ? "badge" : `badge ${tone}`}>
      {dot && <span className="dot" aria-hidden />}
      {label}
    </span>
  );
}

/* ---------- Panel ---------- */
export interface PanelProps {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  tight?: boolean;
  className?: string;
}

export function Panel({ title, description, actions, children, tight = false, className }: PanelProps) {
  return (
    <section className={["panel", className].filter(Boolean).join(" ")}>
      {(title || actions) && (
        <div className="panel-head">
          <div style={{ minWidth: 0 }}>
            {title && <h2 className="panel-title">{title}</h2>}
            {description && <p className="panel-desc">{description}</p>}
          </div>
          {actions && <div className="panel-actions">{actions}</div>}
        </div>
      )}
      <div className={tight ? "panel-body tight" : "panel-body"}>{children}</div>
    </section>
  );
}

/* ---------- Tabs ---------- */
export interface TabItem {
  key: string;
  label: string;
  count?: number;
}

export function Tabs({
  items,
  active,
  onChange,
  ariaLabel,
}: {
  items: TabItem[];
  active: string;
  onChange: (key: string) => void;
  ariaLabel: string;
}) {
  return (
    <div className="tabs" role="tablist" aria-label={ariaLabel}>
      {items.map((item) => (
        <button
          key={item.key}
          type="button"
          role="tab"
          aria-selected={item.key === active}
          className={item.key === active ? "tab active" : "tab"}
          onClick={() => onChange(item.key)}
        >
          {item.label}
          {typeof item.count === "number" && <span className="tab-count">{item.count}</span>}
        </button>
      ))}
    </div>
  );
}

/* ---------- Loading / empty / error states (08-interaction-state-specification.md) ---------- */
export function Skeleton({ variant = "text", style }: { variant?: "text" | "block"; style?: React.CSSProperties }) {
  return <div className={`skeleton ${variant}`} style={style} aria-hidden />;
}

export function SkeletonRows({ rows = 4 }: { rows?: number }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }} role="status" aria-label="加载中">
      {Array.from({ length: rows }, (_, i) => (
        <Skeleton key={i} variant="text" style={{ width: `${88 - i * 9}%` }} />
      ))}
    </div>
  );
}

export function Spinner({ label }: { label?: string }) {
  return <span className="spinner" role="status" aria-label={label ?? "加载中"} />;
}

export interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  hint?: string;
  action?: ReactNode;
}

export function EmptyState({ icon, title, hint, action }: EmptyStateProps) {
  return (
    <div className="empty-state">
      {icon}
      <strong>{title}</strong>
      {hint && <span>{hint}</span>}
      {action && <div style={{ marginTop: "var(--space-2)" }}>{action}</div>}
    </div>
  );
}

export interface ErrorStateProps {
  title?: string;
  message: string;
  /** Only offer retry when repeating the operation is safe (read-only reloads). */
  onRetry?: () => void;
}

export function ErrorState({ title = "加载失败", message, onRetry }: ErrorStateProps) {
  return (
    <div className="error-state" role="alert">
      <strong>{title}</strong>
      <span className="error-message">{message}</span>
      {onRetry && (
        <Button size="small" onClick={onRetry}>
          <RefreshCw size={13} aria-hidden />
          重试
        </Button>
      )}
    </div>
  );
}
