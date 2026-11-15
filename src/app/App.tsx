/*
 * Application entry — shell composition only (06-component-architecture.md §4).
 * Feature pages are wired here; all domain behavior lives in feature modules.
 */
import { Suspense, lazy, useEffect, useState } from "react";
import { AppHeader, AppShell, Sidebar } from "../components/layout/Shell";
import { Spinner } from "../components/ui/primitives";
import { ToastProvider } from "../components/ui/toast";
import { WorkspaceProvider, useWorkspace } from "./providers/WorkspaceProvider";
import { routePath, useHashRoute, type Route } from "./router";
import { NotFoundPage, UnavailablePage } from "./system-pages";
import { DashboardPage } from "../features/dashboard/DashboardPage";
import { GoalsPage } from "../features/goals/GoalsPage";
import { AreasPage } from "../features/areas/AreasPage";
import { ProjectsPage } from "../features/projects/ProjectsPage";
import { ProjectDetailPage } from "../features/projects/ProjectDetailPage";
import { TasksPage } from "../features/tasks/TasksPage";
import { SearchDialog, SearchPage } from "../features/search/SearchDialog";
import { SettingsPage } from "../features/settings/SettingsPage";
import { WorkspaceOnboarding } from "../features/workspace/WorkspaceOnboarding";

const AppLegacy = lazy(() => import("../legacy/AppLegacy"));

const ROUTE_LABELS: Record<Route["name"], string> = {
  dashboard: "仪表板",
  goals: "目标",
  areas: "领域",
  projects: "项目",
  "project-detail": "项目详情",
  tasks: "任务",
  search: "搜索",
  settings: "设置",
  legacy: "开发控制台",
  unavailable: "功能未开放",
  "not-found": "页面不存在",
};

function ShellRoot() {
  const { route, navigate } = useHashRoute();
  const ws = useWorkspace();
  const [searchOpen, setSearchOpen] = useState(false);
  const [collapsed, setCollapsed] = useState(false);

  // Ctrl+K opens global search — 03-navigation-and-routing.md §4.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearchOpen(true);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  // Reset viewport scroll when the route changes.
  useEffect(() => {
    window.scrollTo({ top: 0 });
  }, [route]);

  const activePath = routePath(route);

  const openSearch = () => setSearchOpen(true);

  if (route.name === "legacy") {
    return (
      <Suspense
        fallback={
          <div style={{ display: "grid", placeItems: "center", minHeight: "100vh" }}>
            <Spinner label="正在加载开发控制台" />
          </div>
        }
      >
        <AppLegacy />
      </Suspense>
    );
  }

  const renderPage = () => {
    switch (route.name) {
      case "dashboard":
        return <DashboardPage navigate={navigate} />;
      case "goals":
        return <GoalsPage />;
      case "areas":
        return <AreasPage />;
      case "projects":
        return <ProjectsPage navigate={navigate} />;
      case "project-detail":
        return <ProjectDetailPage projectId={route.id} navigate={navigate} />;
      case "tasks":
        return <TasksPage focusTaskId={route.focusTaskId} navigate={navigate} />;
      case "search":
        return <SearchPage navigate={navigate} />;
      case "settings":
        return <SettingsPage navigate={navigate} />;
      case "unavailable":
        return <UnavailablePage path={route.path} />;
      case "not-found":
        return <NotFoundPage path={route.path} />;
    }
  };

  return (
    <AppShell
      sidebar={
        <Sidebar
          collapsed={collapsed}
          activePath={activePath}
          workspaceName={ws.workspace?.name ?? null}
          onNavigate={navigate}
          onOpenSearch={openSearch}
          onToggleCollapse={() => setCollapsed((c) => !c)}
        />
      }
    >
      <AppHeader
        breadcrumbTail={ROUTE_LABELS[route.name]}
        collapsed={collapsed}
        onToggleSidebar={() => setCollapsed((c) => !c)}
        onOpenSearch={openSearch}
      />
      <main className="page-area">
        {ws.workspaceId == null ? <WorkspaceOnboarding /> : renderPage()}
      </main>
      <SearchDialog
        open={searchOpen}
        onClose={() => setSearchOpen(false)}
        onNavigate={(path) => {
          setSearchOpen(false);
          navigate(path);
        }}
      />
    </AppShell>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <WorkspaceProvider>
        <ShellRoot />
      </WorkspaceProvider>
    </ToastProvider>
  );
}
