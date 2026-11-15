/* Typed command wrappers — Dashboard read model. */
import { callCommand } from "../client";
import type { DashboardSummary } from "../contracts";

export function dashboardSummary(workspaceId: string): Promise<DashboardSummary> {
  return callCommand("dashboard", { workspaceId });
}
