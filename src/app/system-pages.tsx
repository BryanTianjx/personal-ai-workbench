/*
 * Safe route fallbacks — 03-navigation-and-routing.md §2/§7: unknown routes show a
 * local not-found page; deferred routes are never activated, they explain and exit.
 */
import { Compass, ShieldOff } from "lucide-react";
import { Button, EmptyState, Panel } from "../components/ui/primitives";

function goHome() {
  window.location.hash = "#/dashboard";
}

export function NotFoundPage({ path }: { path: string }) {
  return (
    <Panel>
      <EmptyState
        icon={<Compass size={28} aria-hidden />}
        title="页面不存在"
        hint={`没有找到 ${path}。它可能已被移动，或输入的地址有误。`}
        action={
          <Button variant="primary" onClick={goHome}>
            返回仪表板
          </Button>
        }
      />
    </Panel>
  );
}

export function UnavailablePage({ path }: { path: string }) {
  return (
    <Panel>
      <EmptyState
        icon={<ShieldOff size={28} aria-hidden />}
        title="该功能尚未开放"
        hint={`${path} 属于后续版本规划，当前版本未启用。`}
        action={
          <Button variant="primary" onClick={goHome}>
            返回仪表板
          </Button>
        }
      />
    </Panel>
  );
}
