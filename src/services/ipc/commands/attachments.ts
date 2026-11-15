/* Typed command wrappers — Attachment domain.
 * Bytes cross the IPC boundary as a plain number[] (backend Vec<u8>). */
import { callCommand } from "../client";
import type { Attachment, AttachmentOwnerType } from "../contracts";

export function listAttachments(
  workspaceId: string,
  ownerType: AttachmentOwnerType,
  ownerId: string,
): Promise<Attachment[]> {
  return callCommand("list_attachments", { workspaceId, ownerType, ownerId });
}

export function addAttachment(
  workspaceId: string,
  ownerType: AttachmentOwnerType,
  ownerId: string,
  fileName: string,
  bytes: number[],
): Promise<Attachment> {
  return callCommand("add_attachment", {
    workspaceId,
    ownerType,
    ownerId,
    fileName,
    bytes,
  });
}

export function removeAttachment(workspaceId: string, attachmentId: string): Promise<void> {
  return callCommand("remove_attachment", { workspaceId, attachmentId });
}

/** Exports to a temp file and returns its path (WebView2 blocks blob downloads). */
export function openAttachmentExternally(
  workspaceId: string,
  attachmentId: string,
): Promise<string> {
  return callCommand("open_attachment_externally", { workspaceId, attachmentId });
}
