# 附件备份保证（REL-006）

**Baseline:** Alpha 0.1
**Status:** Implemented — 显式排除语义已落在代码、测试与本文档
**Verification Status:** Verified（自动化测试，见 §5；未做人工 Gate 演练）

## 1. 结论：备份包含什么、排除什么

备份文件是**仅包含数据库**的单文件快照。附件字节**显式排除**在数据库备份之外（REL-006 owner 决策），
因此"备份里有没有附件"不再是隐含假设，而是备份文件自带、恢复端强制校验的声明。

| 内容 | 是否进入备份文件 | 说明 |
|---|---|---|
| `app.sqlite` 的全部表，包括 `attachments` 附件**元数据行** | 是 | 通过 SQLite 在线备份 API 复制整个数据库 |
| 附件**字节** `attachments/<workspace-id>/<managed-uuid>` | **否（显式排除）** | 托管文件位于数据库目录之外，复制数据库不可能带上它们 |
| 备份自描述元数据 `paw_backup_metadata` | 是 | 键值见 §2 |
| 数据库侧车文件 `-wal` / `-shm` | 否 | 快照强制切换为 rollback journal，保证单文件可移植 |

## 2. 备份文件自描述（`paw_backup_metadata`）

| key | value | 含义 |
|---|---|---|
| `schema_version` | 迁移版本号 | 备份时刻已应用的最高 `schema_migrations` 版本 |
| `created_at` | UTC 毫秒时间戳 | 快照创建时间 |
| `app_version` | 应用版本 | 产生快照的应用版本 |
| `attachments_excluded` | `true` | 附件字节不在快照中；本格式只写、也只接受 `true` |

`attachments_excluded` 是本格式对外的保证声明：写入端始终写 `true`，读取端**必须**读到 `true`，
否则拒绝该备份。这样旧备份、被改动过的备份或第三方 SQLite 文件都不会被当成"含附件的备份"静默恢复。

## 3. 恢复行为（`restore_backup`）

`restore_backup(backup_path, active_path)` 的顺序是：

1. 只读校验备份：`PRAGMA integrity_check`、`PRAGMA foreign_key_check`、`paw_backup_metadata` 存在且含
   `attachments_excluded = true`；
2. 复制到同卷临时文件并再次校验（防止复制过程中损坏）；
3. 删除目标数据库残留的 `-wal` / `-shm` 侧车文件；
4. 原子 rename 覆盖目标数据库（提交点）；
5. **清空目标数据库同目录下 `attachments/` 的全部内容**，保留 `attachments/` 根目录本身。

第 5 步是排除语义的落地：快照不含附件字节，所以恢复出来的 `attachments` 元数据行必然指向不存在的
文件。清空目录避免"元数据与字节看似匹配、实际错位"的孤儿文件，恢复后的画像从"零附件字节"开始。

拒绝条件（任一命中即整体拒绝，目标数据库与附件目录都**不会**被触碰）：

| 情形 | 错误 |
|---|---|
| 备份文件损坏 / 非 SQLite | `BackupError::Sql` / `BackupError::Integrity` |
| 没有 `paw_backup_metadata` 表 | `BackupError::MissingMetadata` |
| 缺少 `attachments_excluded` 键 | `BackupError::MissingAttachmentsExcluded` |
| `attachments_excluded` 值不是 `true` | `BackupError::AttachmentsNotExcluded` |

## 4. 恢复后用户需要做什么

- 恢复完成后，项目/任务上的附件**元数据仍在**（列表里能看到文件名和大小），但**字节已不存在**，
  打开附件会得到"文件不存在"。
- 需要**重新添加附件**：从原始文件重新导入到对应的项目或任务。备份不提供附件还原。
- 需要保留附件时，请在恢复前**单独复制**应用数据目录下的 `attachments/` 目录；该目录结构与
  `<workspace-id>/<managed-uuid>` 一一对应，可离线拷贝。

## 5. 测试证据

均位于 `src-tauri/crates/paw-infrastructure/tests/backup_restore.rs`，使用一次性临时目录
（`target/tmp-e1-06-*`，测试结束自动删除），不触碰真实数据库或附件库。

| 测试 | 断言的可判定行为 |
|---|---|
| `backup_declares_attachment_bytes_excluded` | `BackupInfo.attachments_excluded == true`；备份文件里 `attachments_excluded` 值为 `true`；其余元数据键不变 |
| `restore_clears_attachment_bytes_and_keeps_metadata_rows` | 快照内含 1 条附件元数据行；恢复后该元数据行仍为 1 条，但托管文件已删除（`ManagedFileStore::open` 返回 `NotFound`），`attachments/` 根目录保留且内容为空 |
| `restore_rejects_snapshot_without_attachment_exclusion_declaration` | 删除该键的备份被 `MissingAttachmentsExcluded` 拒绝；目标数据库未创建，附件字节仍在 |
| `restore_rejects_snapshot_claiming_attachment_bytes_included` | 该键被改为 `false` 的备份被 `AttachmentsNotExcluded` 拒绝；目标数据库未创建，附件字节仍在 |
| `restore_rejects_corrupt_backup_file` | 非 SQLite 备份被类型化拒绝；目标数据库未创建，附件字节未被清理 |
| `backup_round_trip_restores_workspaces_and_projects` | 既有回归：备份/恢复后工作区与项目数据完整 |

验证命令：

```bash
cd src-tauri && CARGO_TARGET_DIR=/tmp/paw-fix-rel006-target cargo test -p paw-infrastructure --no-fail-fast
cd src-tauri && cargo clippy -p paw-infrastructure --all-targets -- -D warnings
cd src-tauri && cargo fmt --all -- --check
```

## 6. 限制与后续

- 目前**没有**"附件单独备份/打包"功能：附件字节既不在备份里，也没有独立的备份产物，
  需要用户自行复制 `attachments/` 目录（§4）。若 Alpha 之后要求可恢复附件，应新增独立附件归档，
  而不是把字节塞回数据库备份格式。
- 兼容性：本次之前生成的备份文件没有 `attachments_excluded` 键，会被新版本 `restore_backup` 一律拒绝。
  这符合"不假设附件保证"的要求；如需恢复这类旧备份，应使用生成它的旧版本，或确认其确实不含附件字节后
  手工补写该键再恢复。
- 恢复清空 `attachments/` 是破坏性操作，调用方应在执行前向用户明确确认。当前 `restore_backup`
  尚未接入桌面命令层（仅有测试调用方），接入时必须先做这一步确认。
- 清空动作发生在数据库替换（提交点）之后：若清空失败（例如文件被占用），函数返回 `Io` 错误，
  数据库已完成恢复，残留的旧附件文件属于可重试清理的孤儿文件，不会造成元数据被错误满足的假象。
