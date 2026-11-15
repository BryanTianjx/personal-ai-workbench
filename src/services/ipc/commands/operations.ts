/* Typed command wrappers — Operation queue & data maintenance (E3 contract). */
import { callCommand } from "../client";
import type { OperationRow, QuarantinedMessage } from "../contracts";

export function listOperations(workspaceId: string): Promise<OperationRow[]> {
  return callCommand("list_operations", { workspaceId });
}

export function requestOperationCancellation(
  workspaceId: string,
  operationId: string,
): Promise<OperationRow> {
  return callCommand("request_operation_cancellation", { workspaceId, operationId });
}

export function resumeOperation(
  workspaceId: string,
  operationId: string,
  note: string,
): Promise<OperationRow> {
  return callCommand("resume_operation", { workspaceId, operationId, note });
}

export function listQuarantinedMessages(workspaceId: string): Promise<QuarantinedMessage[]> {
  return callCommand("list_quarantined_messages", { workspaceId });
}

export function redriveMessage(workspaceId: string, messageId: string): Promise<string> {
  return callCommand("redrive_message", { workspaceId, messageId });
}

export function exportOperations(workspaceId: string): Promise<string> {
  return callCommand("export_operations", { workspaceId });
}

/** Returns [operations, attempts, audit] purge counts. Takes no workspace arg. */
export function purgeExpiredData(): Promise<[number, number, number]> {
  return callCommand("purge_expired_data");
}
