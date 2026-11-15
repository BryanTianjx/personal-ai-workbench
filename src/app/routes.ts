/*
 * Feature availability registry and navigation model — 03-navigation-and-routing.md.
 * The registry is presentation configuration only: it hides routes from the UI and
 * blocks deferred deep links, but it is never authorization (backend policy stays
 * authoritative for every read/write).
 */
import type { LucideIcon } from "lucide-react";
import { LayoutDashboard, Target, Layers, FolderKanban, ListTodo, Search, Settings } from "lucide-react";

export type FeatureKey =
  | "dashboard"
  | "inbox"
  | "goals"
  | "areas"
  | "projects"
  | "tasks"
  | "search"
  | "settings"
  | "learning"
  | "agents"
  | "workflows";

export interface FeatureAvailability {
  key: FeatureKey;
  release: "alpha-0.1" | "alpha-0.2" | "deferred";
  enabled: boolean;
  route?: string;
}

export const FEATURES: FeatureAvailability[] = [
  { key: "dashboard", release: "alpha-0.1", enabled: true, route: "/dashboard" },
  // Inbox is only navigable where an approved capture contract exists; Alpha 0.1
  // provides task quick-capture instead (04-core-page-specification.md §4).
  { key: "inbox", release: "alpha-0.1", enabled: false },
  { key: "goals", release: "alpha-0.1", enabled: true, route: "/goals" },
  { key: "areas", release: "alpha-0.1", enabled: true, route: "/areas" },
  { key: "projects", release: "alpha-0.1", enabled: true, route: "/projects" },
  { key: "tasks", release: "alpha-0.1", enabled: true, route: "/tasks" },
  { key: "search", release: "alpha-0.1", enabled: true, route: "/search" },
  { key: "settings", release: "alpha-0.1", enabled: true, route: "/settings" },
  // Deferred domains are registered so direct deep links resolve to a safe
  // "unavailable" page instead of an error (DR-002).
  { key: "learning", release: "deferred", enabled: false },
  { key: "agents", release: "deferred", enabled: false },
  { key: "workflows", release: "deferred", enabled: false },
];

const DEFERRED_PREFIXES = ["/learning", "/agents", "/workflows", "/inbox"];

export function isDeferredPath(path: string): boolean {
  return DEFERRED_PREFIXES.some((prefix) => path === prefix || path.startsWith(`${prefix}/`));
}

export interface NavEntry {
  feature: FeatureKey;
  route: string;
  label: string;
  icon: LucideIcon;
}

const enabled = (key: FeatureKey) => FEATURES.find((f) => f.key === key)?.enabled ?? false;

const WORKSPACE_NAV_ALL: NavEntry[] = [
  { feature: "dashboard", route: "/dashboard", label: "仪表板", icon: LayoutDashboard },
  { feature: "goals", route: "/goals", label: "目标", icon: Target },
  { feature: "areas", route: "/areas", label: "领域", icon: Layers },
  { feature: "projects", route: "/projects", label: "项目", icon: FolderKanban },
  { feature: "tasks", route: "/tasks", label: "任务", icon: ListTodo },
];

const TOOLS_NAV_ALL: NavEntry[] = [{ feature: "search", route: "/search", label: "全局搜索", icon: Search }];

const SYSTEM_NAV_ALL: NavEntry[] = [{ feature: "settings", route: "/settings", label: "设置", icon: Settings }];

export const WORKSPACE_NAV: NavEntry[] = WORKSPACE_NAV_ALL.filter((entry) => enabled(entry.feature));

export const TOOLS_NAV: NavEntry[] = TOOLS_NAV_ALL.filter((entry) => enabled(entry.feature));

export const SYSTEM_NAV: NavEntry[] = SYSTEM_NAV_ALL.filter((entry) => enabled(entry.feature));

export const APP_NAME = "自磨工坊";
export const APP_SUBTITLE = "PERSONAL AI WORKBENCH";
export const APP_VERSION = "Alpha 0.1";
