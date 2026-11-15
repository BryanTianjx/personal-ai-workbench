/*
 * Shell layout components — 02-application-shell.md §1.
 * The shell owns layout composition only; it must not hold domain rules or
 * duplicate backend state.
 */
import { ChevronRight, PanelLeftClose, PanelLeftOpen, Search } from "lucide-react";
import type { ReactNode } from "react";
import { APP_NAME, APP_SUBTITLE, SYSTEM_NAV, TOOLS_NAV, WORKSPACE_NAV, type NavEntry } from "../../app/routes";
import { IconButton } from "../ui/primitives";
import "./shell.css";

/* ---------- App shell composition ---------- */
export function AppShell({ sidebar, children }: { sidebar: ReactNode; children: ReactNode }) {
  return (
    <div className="app-shell">
      {sidebar}
      <div className="main-shell">{children}</div>
    </div>
  );
}

/* ---------- Primary sidebar ---------- */
export interface SidebarProps {
  collapsed: boolean;
  activePath: string;
  workspaceName: string | null;
  onNavigate: (path: string) => void;
  onOpenSearch: () => void;
  onToggleCollapse: () => void;
}

function NavGroup({ entries, label, activePath, onNavigate }: {
  entries: NavEntry[];
  label: string;
  activePath: string;
  onNavigate: (path: string) => void;
}) {
  if (entries.length === 0) return null;
  return (
    <nav className="nav-group" aria-label={label}>
      <div className="nav-group-label">{label}</div>
      {entries.map((entry) => {
        const Icon = entry.icon;
        const active = activePath === entry.route || activePath.startsWith(`${entry.route}/`);
        return (
          <button
            key={entry.feature}
            type="button"
            className={active ? "nav-item active" : "nav-item"}
            onClick={() => onNavigate(entry.route)}
            aria-current={active ? "page" : undefined}
            title={entry.label}
          >
            <Icon size={17} aria-hidden />
            <span>{entry.label}</span>
          </button>
        );
      })}
    </nav>
  );
}

export function Sidebar({ collapsed, activePath, workspaceName, onNavigate, onOpenSearch, onToggleCollapse }: SidebarProps) {
  const initial = workspaceName ? workspaceName.slice(0, 1).toUpperCase() : "?";
  return (
    <aside className={collapsed ? "sidebar collapsed" : "sidebar"} aria-label="主导航">
      <div className="sidebar-brand">
        <div className="brand-mark" aria-hidden>
          磨
        </div>
        <div className="brand-name">
          <strong>{APP_NAME}</strong>
          <small>{APP_SUBTITLE}</small>
        </div>
      </div>

      <NavGroup entries={WORKSPACE_NAV} label="工作空间" activePath={activePath} onNavigate={onNavigate} />
      <NavGroup entries={TOOLS_NAV} label="工具" activePath={activePath} onNavigate={onNavigate} />

      <div className="sidebar-foot">
        {TOOLS_NAV.some((e) => e.feature === "search") && (
          <button type="button" className="nav-item" onClick={onOpenSearch} title="全局搜索 (Ctrl+K)">
            <Search size={17} aria-hidden />
            <span>全局搜索</span>
            <kbd>Ctrl K</kbd>
          </button>
        )}
        <NavGroup entries={SYSTEM_NAV} label="系统" activePath={activePath} onNavigate={onNavigate} />
        <div className="workspace-chip">
          <span className="avatar" aria-hidden>
            {initial}
          </span>
          <div className="chip-text">
            <strong>{workspaceName ?? "未选择工作空间"}</strong>
            <small>本地工作空间</small>
          </div>
        </div>
      </div>

      <IconButton
        label={collapsed ? "展开侧边栏" : "收起侧边栏"}
        onClick={onToggleCollapse}
        style={{ position: "absolute", right: 10, top: 12 }}
      >
        {collapsed ? <PanelLeftOpen size={16} aria-hidden /> : <PanelLeftClose size={16} aria-hidden />}
      </IconButton>
    </aside>
  );
}

/* ---------- App header ---------- */
export interface AppHeaderProps {
  breadcrumbTail: string;
  collapsed: boolean;
  onToggleSidebar: () => void;
  onOpenSearch: () => void;
}

export function AppHeader({ breadcrumbTail, collapsed, onToggleSidebar, onOpenSearch }: AppHeaderProps) {
  return (
    <header className="topbar">
      <IconButton label={collapsed ? "展开侧边栏" : "收起侧边栏"} onClick={onToggleSidebar}>
        {collapsed ? <PanelLeftOpen size={16} aria-hidden /> : <PanelLeftClose size={16} aria-hidden />}
      </IconButton>
      <div className="topbar-breadcrumb">
        <span>工作空间</span>
        <span className="crumb-sep">/</span>
        <strong>{breadcrumbTail}</strong>
      </div>
      <div className="topbar-actions">
        <button type="button" className="search-trigger" onClick={onOpenSearch} aria-label="打开全局搜索 (Ctrl+K)">
          <Search size={14} aria-hidden />
          搜索
          <kbd>Ctrl K</kbd>
        </button>
      </div>
    </header>
  );
}

/* ---------- Page header contract (02-application-shell.md §6) ---------- */
export interface Crumb {
  label: string;
  route?: string;
}

export interface PageHeaderProps {
  crumbs?: Crumb[];
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  /** Optional row under the title (view switchers, filters). */
  children?: ReactNode;
}

export function PageHeader({ crumbs = [], title, description, actions, children }: PageHeaderProps) {
  return (
    <div className="page-header">
      {crumbs.length > 0 && (
        <nav className="breadcrumbs" aria-label="面包屑">
          {crumbs.map((crumb, index) => (
            <span key={`${crumb.label}-${index}`} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)" }}>
              {index > 0 && <ChevronRight className="crumb-sep" size={12} aria-hidden />}
              {crumb.route ? (
                <button type="button" className="crumb-link" onClick={() => crumb.route && (window.location.hash = `#${crumb.route}`)}>
                  {crumb.label}
                </button>
              ) : (
                <span className="crumb-current">{crumb.label}</span>
              )}
            </span>
          ))}
        </nav>
      )}
      <div className="page-title-row">
        <div style={{ minWidth: 0 }}>
          <h1 className="page-title">{title}</h1>
          {description && <p className="page-description">{description}</p>}
        </div>
        {actions && <div className="page-actions">{actions}</div>}
      </div>
      {children}
    </div>
  );
}
