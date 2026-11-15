/*
 * Attachment section (GUI-BIZ-007). Uses only approved Tauri attachment commands:
 * bytes are read locally and uploaded via add_attachment; opening exports to a
 * temp file path returned by the backend (02-application-shell.md §9).
 */
import { useRef, useState } from "react";
import { Download, FileText, Trash2, Upload } from "lucide-react";
import {
  addAttachment,
  listAttachments,
  openAttachmentExternally,
  removeAttachment,
} from "../../services/ipc/commands/attachments";
import type { AttachmentOwnerType } from "../../services/ipc/contracts";
import { describe } from "../../services/ipc/errors";
import { useAsyncData } from "../../hooks/useAsyncData";
import { Button, EmptyState, ErrorState, IconButton, SkeletonRows } from "../ui/primitives";
import { ConfirmDialog } from "../ui/overlays";
import { useToast } from "../ui/toast";
import { formatBytes } from "../data-display/status";

export interface AttachmentSectionProps {
  workspaceId: string;
  ownerType: AttachmentOwnerType;
  ownerId: string;
}

export function AttachmentSection({ workspaceId, ownerType, ownerId }: AttachmentSectionProps) {
  const toast = useToast();
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [uploading, setUploading] = useState(false);
  const [deleting, setDeleting] = useState<{ id: string; name: string } | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);

  const attachments = useAsyncData(
    () => listAttachments(workspaceId, ownerType, ownerId),
    [workspaceId, ownerType, ownerId],
  );

  const upload = async (file: File) => {
    if (uploading) return;
    setUploading(true);
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      await addAttachment(workspaceId, ownerType, ownerId, file.name, bytes);
      toast.success("附件已上传。");
      attachments.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setUploading(false);
    }
  };

  const open = async (attachmentId: string) => {
    try {
      const path = await openAttachmentExternally(workspaceId, attachmentId);
      toast.info(`已导出到临时文件：${path}`);
    } catch (cause) {
      toast.error(describe(cause));
    }
  };

  const confirmDelete = async () => {
    if (!deleting || deleteBusy) return;
    setDeleteBusy(true);
    try {
      await removeAttachment(workspaceId, deleting.id);
      toast.success("附件已删除。");
      setDeleting(null);
      attachments.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setDeleteBusy(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
      <div style={{ display: "flex", justifyContent: "flex-end" }}>
        <input
          ref={fileInputRef}
          type="file"
          hidden
          onChange={(e) => {
            const file = e.target.files?.[0];
            e.target.value = "";
            if (file) void upload(file);
          }}
        />
        <Button size="small" onClick={() => fileInputRef.current?.click()} loading={uploading}>
          <Upload size={14} aria-hidden />
          上传附件
        </Button>
      </div>

      {attachments.error ? (
        <ErrorState message={attachments.error} onRetry={attachments.reload} />
      ) : attachments.loading ? (
        <SkeletonRows rows={2} />
      ) : (attachments.data ?? []).length === 0 ? (
        <EmptyState icon={<FileText size={20} aria-hidden />} title="暂无附件" hint="上传文件后可随时导出查看。" />
      ) : (
        <div className="list-rows" style={{ margin: "0 calc(var(--space-4) * -1)" }}>
          {(attachments.data ?? []).map((attachment) => (
            <div key={attachment.id} className="list-row">
              <FileText size={16} aria-hidden style={{ color: "var(--color-text-muted)", flex: "none" }} />
              <div className="row-main">
                <span className="row-title" style={{ cursor: "default" }} title={attachment.file_name}>
                  {attachment.file_name}
                </span>
                <span className="row-sub">
                  {formatBytes(attachment.size_bytes)} · {new Date(attachment.created_at).toLocaleDateString("zh-CN")}
                </span>
              </div>
              <div className="row-side">
                <IconButton label={`打开附件 ${attachment.file_name}`} onClick={() => void open(attachment.id)}>
                  <Download size={15} aria-hidden />
                </IconButton>
                <IconButton
                  label={`删除附件 ${attachment.file_name}`}
                  onClick={() => setDeleting({ id: attachment.id, name: attachment.file_name })}
                >
                  <Trash2 size={15} aria-hidden />
                </IconButton>
              </div>
            </div>
          ))}
        </div>
      )}

      <ConfirmDialog
        open={deleting != null}
        title="删除附件"
        danger
        busy={deleteBusy}
        message={
          <>
            将删除附件「<strong>{deleting?.name}</strong>」。此操作不可撤销。
          </>
        }
        confirmLabel="删除"
        onConfirm={confirmDelete}
        onCancel={() => setDeleting(null)}
      />
    </div>
  );
}
