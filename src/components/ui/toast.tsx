/*
 * Toast notifications — transient operation feedback (08-interaction-state-specification.md §10).
 * Field-level issues stay inline; toasts are only for short completed operations.
 */
import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { X } from "lucide-react";
import "./ui.css";

export type ToastKind = "info" | "success" | "error";

interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
}

interface ToastApi {
  info: (message: string) => void;
  success: (message: string) => void;
  error: (message: string) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

export function useToast(): ToastApi {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within ToastProvider");
  return ctx;
}

const AUTO_DISMISS_MS = 4200;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const nextId = useRef(1);

  const push = useCallback((kind: ToastKind, message: string) => {
    const id = nextId.current++;
    setItems((prev) => [...prev.slice(-4), { id, kind, message }]);
    window.setTimeout(() => {
      setItems((prev) => prev.filter((t) => t.id !== id));
    }, AUTO_DISMISS_MS);
  }, []);

  const api = useMemo<ToastApi>(
    () => ({
      info: (message) => push("info", message),
      success: (message) => push("success", message),
      error: (message) => push("error", message),
    }),
    [push],
  );

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="toast-stack" aria-live="polite">
        {items.map((toast) => (
          <div key={toast.id} className={toast.kind === "info" ? "toast" : `toast ${toast.kind}`} role={toast.kind === "error" ? "alert" : "status"}>
            <span style={{ minWidth: 0, overflowWrap: "anywhere" }}>{toast.message}</span>
            <button
              type="button"
              className="toast-close"
              aria-label="关闭通知"
              onClick={() => setItems((prev) => prev.filter((t) => t.id !== toast.id))}
            >
              <X size={13} aria-hidden />
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
