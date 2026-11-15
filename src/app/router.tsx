/*
 * Minimal hash router for the Tauri WebView — 03-navigation-and-routing.md.
 * Route names follow the spec's conceptual table; unknown paths resolve to a
 * local not-found page and deferred prefixes resolve to a safe unavailable page.
 */
import { useCallback, useEffect, useState } from "react";
import { isDeferredPath } from "./routes";

export type Route =
  | { name: "dashboard" }
  | { name: "goals" }
  | { name: "areas" }
  | { name: "projects" }
  | { name: "project-detail"; id: string }
  | { name: "tasks"; focusTaskId?: string }
  | { name: "search" }
  | { name: "settings" }
  | { name: "legacy" }
  | { name: "unavailable"; path: string }
  | { name: "not-found"; path: string };

export function parseRoute(rawHash: string): Route {
  const path = rawHash.replace(/^#/, "") || "/dashboard";
  const segments = path.split("?")[0].split("/").filter(Boolean);

  if (segments.length === 0) return { name: "dashboard" };
  const [head, second] = segments;

  switch (head) {
    case "dashboard":
      return { name: "dashboard" };
    case "goals":
      return { name: "goals" };
    case "areas":
      return { name: "areas" };
    case "projects":
      return second ? { name: "project-detail", id: decodeURIComponent(second) } : { name: "projects" };
    case "tasks":
      return { name: "tasks", focusTaskId: second ? decodeURIComponent(second) : undefined };
    case "search":
      return { name: "search" };
    case "settings":
      return { name: "settings" };
    case "legacy":
      return { name: "legacy" };
    default:
      if (isDeferredPath(path)) return { name: "unavailable", path };
      return { name: "not-found", path };
  }
}

export function routePath(route: Route): string {
  switch (route.name) {
    case "dashboard":
      return "/dashboard";
    case "goals":
      return "/goals";
    case "areas":
      return "/areas";
    case "projects":
      return "/projects";
    case "project-detail":
      return `/projects/${encodeURIComponent(route.id)}`;
    case "tasks":
      return route.focusTaskId ? `/tasks/${encodeURIComponent(route.focusTaskId)}` : "/tasks";
    case "search":
      return "/search";
    case "settings":
      return "/settings";
    case "legacy":
      return "/legacy";
    case "unavailable":
    case "not-found":
      return route.path;
  }
}

export interface HashRoute {
  route: Route;
  navigate: (path: string) => void;
}

export function useHashRoute(): HashRoute {
  const [hash, setHash] = useState(() => window.location.hash);

  useEffect(() => {
    const onHashChange = () => setHash(window.location.hash);
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  const navigate = useCallback((path: string) => {
    window.location.hash = `#${path}`;
  }, []);

  const route = parseRoute(hash);
  return { route, navigate };
}
