/*
 * Typed command wrappers — Backup & restore (PAW-AUDIT-002 remediation).
 * The frontend identifies backups by FILE NAME only; paths are resolved
 * inside the managed <app_data_dir>/backups directory by the backend.
 * request_restore stages the snapshot and restarts the shell — the invoke
 * intentionally never resolves (the apply happens at next bootstrap).
 */
import { callCommand } from "../client";

export interface BackupInfoDto {
  schema_version: number;
  created_at: number;
  app_version: string;
  attachments_excluded: boolean;
}

export interface BackupSnapshotDto {
  file_name: string;
  size_bytes: number;
  info: BackupInfoDto;
}

export interface BackupFileMetaDto {
  file_name: string;
  size_bytes: number;
  modified_at: number | null;
  info: BackupInfoDto | null;
  valid: boolean;
}

export interface PendingRestoreDto {
  pending: boolean;
}

export function createBackup(): Promise<BackupSnapshotDto> {
  return callCommand("create_backup");
}

export function listBackups(): Promise<BackupFileMetaDto[]> {
  return callCommand("list_backups");
}

export function readBackupInfo(backupName: string): Promise<BackupInfoDto> {
  return callCommand("read_backup_info", { backupName });
}

export function requestRestore(backupName: string): Promise<PendingRestoreDto> {
  return callCommand("request_restore", { backupName });
}

export function pendingRestoreStatus(): Promise<PendingRestoreDto> {
  return callCommand("pending_restore_status");
}

export function cancelPendingRestore(): Promise<PendingRestoreDto> {
  return callCommand("cancel_pending_restore");
}
