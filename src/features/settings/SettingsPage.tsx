/*
 * Settings — approved Alpha 0.1 surfaces only (03 §1): workspace management and
 * the E3 system/data contracts (operation queue, quarantine redrive, export,
 * purge). The legacy development console is reachable from here (DR-002 gate).
 */
import { useState } from "react";
import {
  Database,
  Download,
  Hammer,
  Plus,
  RefreshCw,
  Server,
  StopCircle,
  TriangleAlert,
} from "lucide-react";
import { useWorkspace } from "../../app/providers/WorkspaceProvider";
import { PageHeader } from "../../components/layout/Shell";
import { formatBytes, OPERATION_STATUS_META } from "../../components/data-display/status";
import {
  cancelPendingRestore,
  createBackup,
  listBackups,
  pendingRestoreStatus,
  requestRestore,
  type BackupFileMetaDto,
} from "../../services/ipc/commands/backup";
import {
  Button,
  EmptyState,
  ErrorState,
  Field,
  Input,
  Panel,
  SkeletonRows,
  StatusBadge,
} from "../../components/ui/primitives";
import { ConfirmDialog } from "../../components/ui/overlays";
import { useToast } from "../../components/ui/toast";
import { useAsyncData } from "../../hooks/useAsyncData";
import {
  exportOperations,
  listOperations,
  listQuarantinedMessages,
  purgeExpiredData,
  redriveMessage,
  requestOperationCancellation,
  resumeOperation,
} from "../../services/ipc/commands/operations";
import { createWorkspace } from "../../services/ipc/commands/workspaces";
import { APP_NAME, APP_VERSION } from "../../app/routes";
import { describe } from "../../services/ipc/errors";

const RESUME_NOTE = "用户确认：结果已核对，可安全重试。";

export function SettingsPage({ navigate }: { navigate: (path: string) => void }) {
  const ws = useWorkspace();
  const toast = useToast();
  const workspaceId = ws.workspaceId;

  const [newWorkspaceName, setNewWorkspaceName] = useState("");
  const [creatingWorkspace, setCreatingWorkspace] = useState(false);
  const [purgeOpen, setPurgeOpen] = useState(false);
  const [purgeBusy, setPurgeBusy] = useState(false);

  const operations = useAsyncData(
    () => (workspaceId ? listOperations(workspaceId) : Promise.resolve([])),
    [workspaceId],
  );
  const quarantined = useAsyncData(
    () => (workspaceId ? listQuarantinedMessages(workspaceId) : Promise.resolve([])),
    [workspaceId],
  );
  // Backup state is device-scoped, not workspace-scoped.
  const backups = useAsyncData(() => listBackups(), []);
  const restoreStatus = useAsyncData(() => pendingRestoreStatus(), []);
  const [creatingBackup, setCreatingBackup] = useState(false);
  const [restoringBackup, setRestoringBackup] = useState<BackupFileMetaDto | null>(null);
  const [restoreBusy, setRestoreBusy] = useState(false);

  const runCreateBackup = async () => {
    if (creatingBackup) return;
    setCreatingBackup(true);
    try {
      const snapshot = await createBackup();
      toast.success(`备份已创建：${snapshot.file_name}`);
      backups.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setCreatingBackup(false);
    }
  };

  const runRestore = async () => {
    if (!restoringBackup || restoreBusy) return;
    setRestoreBusy(true);
    try {
      // Stages the snapshot and restarts the shell; on success this promise
      // never resolves (the apply happens at next bootstrap).
      await requestRestore(restoringBackup.file_name);
      toast.info("恢复已暂存，重启应用后生效。");
      setRestoringBackup(null);
      restoreStatus.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setRestoreBusy(false);
    }
  };

  const runCancelRestore = async () => {
    try {
      await cancelPendingRestore();
      toast.success("已放弃暂存的恢复。");
      restoreStatus.reload();
    } catch (cause) {
      toast.error(describe(cause));
    }
  };

  const createWorkspaceSubmit = async () => {
    const trimmed = newWorkspaceName.trim();
    if (!trimmed || creatingWorkspace) return;
    setCreatingWorkspace(true);
    try {
      await createWorkspace(trimmed);
      toast.success("工作空间已创建。");
      setNewWorkspaceName("");
      await ws.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setCreatingWorkspace(false);
    }
  };

  const cancelOperation = async (operationId: string) => {
    if (!workspaceId) return;
    try {
      await requestOperationCancellation(workspaceId, operationId);
      toast.success("已请求取消该操作。");
      operations.reload();
    } catch (cause) {
      toast.error(describe(cause));
    }
  };

  const resume = async (operationId: string) => {
    if (!workspaceId) return;
    try {
      await resumeOperation(workspaceId, operationId, RESUME_NOTE);
      toast.success("操作已排入重试。");
      operations.reload();
    } catch (cause) {
      toast.error(describe(cause));
    }
  };

  const exportAll = async () => {
    if (!workspaceId) return;
    try {
      const path = await exportOperations(workspaceId);
      toast.info(`操作记录已导出：${path}`);
    } catch (cause) {
      toast.error(describe(cause));
    }
  };

  const redrive = async (messageId: string) => {
    if (!workspaceId) return;
    try {
      await redriveMessage(workspaceId, messageId);
      toast.success("隔离事件已重新投递。");
      quarantined.reload();
    } catch (cause) {
      toast.error(describe(cause));
    }
  };

  const purge = async () => {
    setPurgeBusy(true);
    try {
      const [ops, attempts, audit] = await purgeExpiredData();
      toast.success(`已清理过期数据：操作 ${ops} 条、尝试记录 ${attempts} 条、审计 ${audit} 条。`);
      setPurgeOpen(false);
      operations.reload();
      quarantined.reload();
    } catch (cause) {
      toast.error(describe(cause));
    } finally {
      setPurgeBusy(false);
    }
  };

  return (
    <>
      <PageHeader title="设置" description="工作空间与系统级配置。" />

      <Panel title="工作空间" description="所有记录的顶层作用域；切换后各页面将重新加载当前工作空间的数据。">
        {ws.error ? (
          <ErrorState message={ws.error} onRetry={() => void ws.reload()} />
        ) : (
          <div className="list-rows" style={{ margin: "0 calc(var(--space-4) * -1)" }}>
            {ws.workspaces.map((workspace) => {
              const active = workspace.id === ws.workspaceId;
              return (
                <div key={workspace.id} className={active ? "list-row selected" : "list-row"}>
                  <div className="row-main">
                    <span className="row-title" style={{ cursor: "default", display: "inline-flex", alignItems: "center", gap: "var(--space-2)" }}>
                      {workspace.name}
                      {active && <StatusBadge tone="primary" label="当前" />}
                    </span>
                  </div>
                  <div className="row-side">
                    {!active && (
                      <Button size="small" onClick={() => ws.selectWorkspace(workspace.id)}>
                        切换
                      </Button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void createWorkspaceSubmit();
          }}
          style={{ display: "flex", gap: "var(--space-2)", alignItems: "flex-end", marginTop: "var(--space-4)", maxWidth: 440 }}
        >
          <Field label="新建工作空间" htmlFor="settings-workspace-name">
            <Input
              id="settings-workspace-name"
              value={newWorkspaceName}
              onChange={(e) => setNewWorkspaceName(e.target.value)}
              placeholder="工作空间名称"
              maxLength={200}
            />
          </Field>
          <Button variant="primary" type="submit" loading={creatingWorkspace} disabled={!newWorkspaceName.trim()}>
            <Plus size={15} aria-hidden />
            创建
          </Button>
        </form>
      </Panel>

      <Panel
        title="操作队列"
        description="异步操作的执行记录（E3 契约）。"
        actions={
          <Button size="small" onClick={() => void exportAll()}>
            <Download size={14} aria-hidden />
            导出记录
          </Button>
        }
      >
        {operations.error ? (
          <ErrorState message={operations.error} onRetry={operations.reload} />
        ) : operations.loading ? (
          <SkeletonRows rows={3} />
        ) : (operations.data ?? []).length === 0 ? (
          <EmptyState icon={<Server size={22} aria-hidden />} title="暂无操作记录" hint="异步操作执行后会出现在这里。" />
        ) : (
          <div style={{ margin: "0 calc(var(--space-4) * -1)" }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th scope="col">类型</th>
                  <th scope="col">状态</th>
                  <th scope="col">更新时间</th>
                  <th scope="col" style={{ width: 96 }}>
                    操作
                  </th>
                </tr>
              </thead>
              <tbody>
                {(operations.data ?? []).map((operation) => (
                  <tr key={operation.id}>
                    <td style={{ fontWeight: 500 }}>{operation.kind}</td>
                    <td className="cell-badge">
                      <StatusBadge
                        tone={OPERATION_STATUS_META[operation.status].tone}
                        label={OPERATION_STATUS_META[operation.status].label}
                      />
                    </td>
                    <td className="cell-muted">
                      {new Date(operation.updated_at).toLocaleString("zh-CN", { hour12: false })}
                    </td>
                    <td>
                      {operation.status === "running" && (
                        <Button size="small" onClick={() => void cancelOperation(operation.id)}>
                          <StopCircle size={13} aria-hidden />
                          取消
                        </Button>
                      )}
                      {operation.status === "outcome_unknown" && (
                        <Button size="small" onClick={() => void resume(operation.id)}>
                          <RefreshCw size={13} aria-hidden />
                          恢复
                        </Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Panel>

      <Panel
        title="隔离事件"
        description="未能投递的事件；核对后可重新投递。"
      >
        {quarantined.error ? (
          <ErrorState message={quarantined.error} onRetry={quarantined.reload} />
        ) : quarantined.loading ? (
          <SkeletonRows rows={2} />
        ) : (quarantined.data ?? []).length === 0 ? (
          <EmptyState icon={<Database size={22} aria-hidden />} title="暂无隔离事件" hint="一切正常。" />
        ) : (
          <div style={{ margin: "0 calc(var(--space-4) * -1)" }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th scope="col">事件</th>
                  <th scope="col">尝试次数</th>
                  <th scope="col">错误类别</th>
                  <th scope="col" style={{ width: 110 }}>
                    操作
                  </th>
                </tr>
              </thead>
              <tbody>
                {(quarantined.data ?? []).map((message) => (
                  <tr key={message.id}>
                    <td style={{ fontWeight: 500 }}>{message.event_type}</td>
                    <td className="cell-muted">
                      {message.attempts} 次 / 已重投 {message.redrive_count} 次
                    </td>
                    <td className="cell-muted">{message.last_error_category ?? "—"}</td>
                    <td>
                      <Button size="small" onClick={() => void redrive(message.id)}>
                        <RefreshCw size={13} aria-hidden />
                        重新投递
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <div style={{ marginTop: "var(--space-4)", display: "flex", justifyContent: "flex-end" }}>
          <Button variant="danger" onClick={() => setPurgeOpen(true)}>
            <TriangleAlert size={14} aria-hidden />
            清理过期数据
          </Button>
        </div>
      </Panel>

      <Panel
        title="数据保护"
        description="数据库快照（在线备份 API，自校验）。附件字节不在快照内（REL-006），恢复后附件目录会被清空。"
        actions={
          <Button size="small" variant="primary" onClick={() => void runCreateBackup()} loading={creatingBackup}>
            <Database size={14} aria-hidden />
            立即备份
          </Button>
        }
      >
        {restoreStatus.data?.pending && (
          <div
            role="status"
            style={{
              display: "flex",
              alignItems: "center",
              gap: "var(--space-2)",
              padding: "var(--space-3)",
              background: "var(--color-warning-bg)",
              color: "var(--color-warning)",
              borderRadius: "var(--radius-sm)",
              fontSize: "var(--type-body-small-size)",
              marginBottom: "var(--space-3)",
            }}
          >
            <TriangleAlert size={15} aria-hidden />
            存在待恢复的数据库快照，将在下次启动应用时生效。
            <Button size="small" style={{ marginLeft: "auto" }} onClick={() => void runCancelRestore()}>
              放弃恢复
            </Button>
          </div>
        )}
        {backups.error ? (
          <ErrorState message={backups.error} onRetry={backups.reload} />
        ) : backups.loading ? (
          <SkeletonRows rows={2} />
        ) : (backups.data ?? []).length === 0 ? (
          <EmptyState
            icon={<Database size={22} aria-hidden />}
            title="暂无备份"
            hint="立即备份会生成一个自校验的数据库快照，保存在应用数据目录的 backups/ 下。"
          />
        ) : (
          <div style={{ margin: "0 calc(var(--space-4) * -1)" }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th scope="col">备份文件</th>
                  <th scope="col">快照信息</th>
                  <th scope="col">大小</th>
                  <th scope="col" style={{ width: 96 }}>
                    操作
                  </th>
                </tr>
              </thead>
              <tbody>
                {(backups.data ?? []).map((backup) => (
                  <tr key={backup.file_name}>
                    <td style={{ fontWeight: 500, overflowWrap: "anywhere" }}>{backup.file_name}</td>
                    <td className="cell-muted">
                      {backup.valid && backup.info
                        ? `schema v${backup.info.schema_version} · ${new Date(backup.info.created_at).toLocaleString("zh-CN", { hour12: false })}`
                        : "不可用（校验失败）"}
                    </td>
                    <td className="cell-muted">{formatBytes(backup.size_bytes)}</td>
                    <td>
                      {backup.valid ? (
                        <Button size="small" onClick={() => setRestoringBackup(backup)}>
                          恢复
                        </Button>
                      ) : (
                        <span className="badge danger">不可用</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Panel>

      <Panel title="开发控制台" description="旧版功能验证界面（E4–E7 证据流程使用），非生产界面。">
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
          <Hammer size={18} aria-hidden style={{ color: "var(--color-text-muted)" }} />
          <span style={{ fontSize: "var(--type-body-small-size)", color: "var(--color-text-secondary)", flex: 1 }}>
            包含学习、知识、Agent 协作、技能评估等延期领域的原始界面，仅用于内部验证。
          </span>
          <Button onClick={() => navigate("/legacy")}>打开开发控制台</Button>
        </div>
      </Panel>

      <Panel title="关于">
        <div className="detail-list">
          <div className="detail-row">
            <span className="detail-label">应用</span>
            <span className="detail-value">{APP_NAME}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">版本</span>
            <span className="detail-value">{APP_VERSION}（Windows）</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">运行方式</span>
            <span className="detail-value">本地优先 · 数据存储在本机</span>
          </div>
        </div>
      </Panel>

      <ConfirmDialog
        open={purgeOpen}
        title="清理过期数据"
        danger
        busy={purgeBusy}
        message={
          <>
            将清理已过期的操作记录、尝试记录与审计数据。此操作不可撤销，且不区分工作空间。
          </>
        }
        confirmLabel="立即清理"
        onConfirm={() => void purge()}
        onCancel={() => setPurgeOpen(false)}
      />

      <ConfirmDialog
        open={restoringBackup != null}
        title="恢复数据库"
        danger
        busy={restoreBusy}
        message={
          <>
            将用快照「<strong>{restoringBackup?.file_name}</strong>」替换当前数据库（
            {restoringBackup?.info
              ? `schema v${restoringBackup.info.schema_version}，创建于 ${new Date(restoringBackup.info.created_at).toLocaleString("zh-CN", { hour12: false })}`
              : "信息不可用"}
            ）。
            <br />
            <br />
            附件文件不随备份恢复（REL-006）：恢复后当前附件目录将被清空，需要重新添加附件。
            <br />
            快照会在应用下次启动时生效；现在将自动重启应用。
          </>
        }
        confirmLabel="恢复并重启"
        onConfirm={() => void runRestore()}
        onCancel={() => {
          if (!restoreBusy) setRestoringBackup(null);
        }}
      />
    </>
  );
}
