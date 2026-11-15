/*
 * Overlay primitives (dialog / confirm / drawer) with focus management per
 * 02-application-shell.md §8 and 01-design-system.md §7: focus is trapped while
 * open, Escape dismisses only when safe, and focus returns to the invoker.
 */
import { useCallback, useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { Button, IconButton } from "./primitives";
import "./ui.css";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Shared overlay behavior: focus first focusable on open, trap Tab, call
 * onClose on Escape, restore focus to the invoking control on close.
 */
function useOverlay(ref: React.RefObject<HTMLDivElement | null>, open: boolean, onClose: () => void) {
  useEffect(() => {
    if (!open) return;
    const previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const container = ref.current;
    const focusables = container?.querySelectorAll<HTMLElement>(FOCUSABLE);
    const first = focusables && focusables.length > 0 ? focusables[0] : container;
    // Defer so the element is mounted before focusing.
    window.setTimeout(() => first?.focus(), 0);

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
        return;
      }
      if (event.key === "Tab" && container) {
        const items = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
          (el) => el.offsetParent !== null,
        );
        if (items.length === 0) return;
        const activeIndex = items.indexOf(document.activeElement as HTMLElement);
        const nextIndex = event.shiftKey
          ? (activeIndex <= 0 ? items.length - 1 : activeIndex - 1)
          : (activeIndex === items.length - 1 || activeIndex < 0 ? 0 : activeIndex + 1);
        event.preventDefault();
        items[nextIndex].focus();
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      previouslyFocused?.focus();
    };
  }, [open, onClose, ref]);
}

export interface DialogProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  wide?: boolean;
}

export function Dialog({ open, title, onClose, children, footer, wide = false }: DialogProps) {
  const ref = useRef<HTMLDivElement | null>(null);
  useOverlay(ref, open, onClose);
  if (!open) return null;
  return (
    <div
      className="overlay-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className={wide ? "dialog wide" : "dialog"} role="dialog" aria-modal="true" aria-label={title} ref={ref}>
        <div className="dialog-head">
          <h2 className="dialog-title">{title}</h2>
          <IconButton label="关闭" onClick={onClose}>
            <X size={16} aria-hidden />
          </IconButton>
        </div>
        <div className="dialog-body">{children}</div>
        {footer && <div className="dialog-foot">{footer}</div>}
      </div>
    </div>
  );
}

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/** Destructive confirmation: default focus lands on the safe action, never on the destructive one. */
export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = "确认",
  cancelLabel = "取消",
  danger = false,
  busy = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    if (open) window.setTimeout(() => cancelRef.current?.focus(), 0);
  }, [open]);
  return (
    <Dialog
      open={open}
      title={title}
      onClose={() => {
        if (!busy) onCancel();
      }}
      footer={
        <>
          <Button ref={cancelRef} onClick={onCancel} disabled={busy}>
            {cancelLabel}
          </Button>
          <Button variant={danger ? "danger" : "primary"} onClick={onConfirm} loading={busy}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      <div style={{ fontSize: "var(--type-body-size)", color: "var(--color-text-secondary)", lineHeight: 1.7 }}>
        {message}
      </div>
    </Dialog>
  );
}

export interface DrawerProps {
  open: boolean;
  title: ReactNode;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

export function Drawer({ open, title, onClose, children, footer }: DrawerProps) {
  const ref = useRef<HTMLDivElement | null>(null);
  useOverlay(ref, open, onClose);
  if (!open) return null;
  return (
    <div className="drawer-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <aside className="drawer" role="dialog" aria-modal="true" ref={ref}>
        <div className="drawer-head">
          <h2 className="drawer-title">{title}</h2>
          <IconButton label="关闭" onClick={onClose}>
            <X size={16} aria-hidden />
          </IconButton>
        </div>
        <div className="drawer-body">{children}</div>
        {footer && <div className="drawer-foot">{footer}</div>}
      </aside>
    </div>
  );
}

/** Hook form helper: tracks whether unsaved draft changes exist. */
export function useDirty<T>(current: T): (draft: T) => boolean {
  return useCallback((draft: T) => JSON.stringify(draft) !== JSON.stringify(current), [current]);
}
