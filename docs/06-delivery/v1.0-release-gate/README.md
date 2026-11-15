# V1.0 Release Gate

本目录承载 Gate V1.0（E7 + 收口）与发布门禁的检查清单与证据。纪律沿用 [Alpha 0.1 Release Closure](../alpha-0.1-release-closure/README.md)：**任何报告不得预填 PASS**——记录确切 commit、环境、过程、实际结果与证据文件。

## 范围（roadmap v0.5.1）

- Gate V1.0（阶段 E7 + 收口）：平台组合验收（已在 E7 窗口化运行完成，见 [E7 evidence report](../../04-quality/E7-engineering-evidence-report.md) §6.1）+ 跨域闭环收口确认。
- 发布门禁：Windows 安装、升级、恢复、安全和发布检查通过；macOS 独立 readiness 里程碑。

## 检查清单

| # | 检查项 | 执行者 | 状态 | 证据 |
|---|---|---|---|---|
| R1 | 发布构建：`pnpm tauri build` 产出 NSIS/MSI 安装包 | agent/owner | ✅ 2026-11-15（见 §R1） | 本文档 §R1 + 构建日志 |
| R2 | 安装演练：NSIS 安装器全新安装 → 启动 → 数据目录初始化 → 卸载 | owner | ⏳ 待执行 | `windows-install-report.md`（待建） |
| R3 | 升级演练：安装旧构建 → 覆盖安装新构建 → 数据保留（迁移 0001→0013 顺序应用） | owner | ⏳ 待执行 | 同上 |
| R4 | 恢复演练：已安装应用内执行备份 → 破坏 → 恢复（E3 恢复演练在安装态复跑） | owner | ⏳ 待执行 | 同上 |
| R5 | 安全检查：默认无公网监听、敏感日志脱敏、文件访问限制（PAW-S-001～003 在安装态复跑） | owner | ⏳ 待执行 | security-architecture 对应测试 |
| R6 | 离线核心闭环（E1/E2/E3 遗留项在发布门禁一并关闭） | owner | ⏳ 待执行 | offline-acceptance-matrix |
| R7 | 发布证据报告：汇总 R1–R6 + commit + 环境 | owner | ⏳ R1–R6 后 | `release-evidence-report.md`（待建） |

## R1 — 发布构建记录（2026-11-15）

- **环境：** Windows，toolchain 1.98.1，node/pnpm per repo lockfiles，main 分支。
- **过程：** `pnpm tauri build`（beforeBuildCommand: `pnpm build` → tauri release 编译 → NSIS + MSI 打包）。
- **发现缺陷 R1-a：** `build.frontendDist` 为 `"../../dist"`——相对 tauri.conf.json 所在目录（`src-tauri/crates/paw-desktop/`）解析为 `src-tauri/dist`（不存在），而 vite 实际输出在仓库根 `dist/`。dev 模式走 devUrl 故从未暴露。修复为 `"../../../dist"`。
- **结果：✅ 构建成功（2026-11-15，commit 284845e）。** 产物：
  - NSIS：`src-tauri/target/release/bundle/nsis/Personal AI Workbench_0.1.0_x64-setup.exe`（2.88 MiB，SHA-256 `cd4eb8fc17dbccb50b43dd9b4663fe56b32d0bbb522d705505bd1aac1134449b`）
  - MSI：`src-tauri/target/release/bundle/msi/Personal AI Workbench_0.1.0_x64_en-US.msi`（4.09 MiB，SHA-256 `619f110af0f164394104d87ab2804de7c1900b1a82f889254d3338a3d5196b31`）
  - 附带产物：release 可执行文件 `target/release/paw-desktop.exe`。R2–R7 演练以这两个安装包为输入。

## 升级/恢复演练注意

- 覆盖安装前备份 `%APPDATA%/com.bryantianjx.personal-ai-workbench/app.sqlite`。
- 迁移按版本顺序应用（schema_migrations 账本），升级演练应观察到 0001→0013 全部执行且 checksum 匹配。
- 恢复演练使用应用内备份/恢复命令（E3），不直接拷贝 DB 文件。
