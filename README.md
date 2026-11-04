# 自磨工坊 · 个人智能工作台

**Personal AI Workbench**  
Git repository: `personal-ai-workbench`  
微信公众号：AI智能体打磨工坊

> 自主构建 · 持续打磨 · 知行合一

自磨工坊是一款面向个人的本地优先智能工作平台，统一组织目标、项目、任务、知识、AI Agent、工程活动与内容创作。它负责工作组织、上下文关联、任务协调、权限控制和结果追溯；专业工具负责专业执行。

## 产品原则

- Local-first：个人核心工作数据由本地应用掌控。
- Offline-first：断网时仍可使用核心工作管理能力。
- Security-first：最小权限、明确授权、可审计。
- Agent-ready：通过适配器连接可验证的外部 Agent，不绑定单一厂商。
- Human-governed：高风险操作和内容发布保留人工决策。
- Modular monolith：先建立清晰模块边界，不为早期版本引入微服务复杂度。

## 核心工作域

1. Personal Workspace：目标、项目、任务和工作记录。
2. AI Workspace：助手、Agent 连接、任务执行与状态。
3. Knowledge Workspace：本地知识入口、外部知识引用和检索。
4. Engineering Workspace：代码项目、Git 与工程活动入口。
5. Creator Workspace：微信公众号选题、研究、写作、审核和发布辅助。
6. Automation Workspace：可控的自动化任务与后续工作流。
7. System & Security：设置、权限、审计、备份、恢复和升级。

## 技术基线

- Desktop：Tauri 2
- Frontend：React + TypeScript
- Backend：Rust
- Local database：SQLite
- Local search：SQLite FTS5
- Build：Cargo + pnpm
- Optional server：后续评估，不作为本地桌面运行前置依赖。

## 平台与离线承诺

目标平台为 Windows 和 macOS。核心工作管理、知识引用元数据、搜索、附件管理、备份恢复应可在完全断网条件下运行。云端 AI、远程知识服务和微信平台接口不可用时，必须显示明确状态并提供降级路径。

## 项目状态

当前处于 E0 产品基线阶段。本仓库文档描述目标和验收要求，不代表相关功能已经实现或测试通过。工程代码将在需求与架构基线评审后分阶段加入。

## 文档入口

- [产品愿景](docs/00-overview/product-vision.md)
- [产品范围](docs/00-overview/product-scope.md)
- [术语表](docs/00-overview/terminology.md)
- [PRD 1.0](docs/01-product/PRD-v1.0.md)
- [需求基线](docs/01-product/requirements-baseline.md)
- [功能矩阵](docs/01-product/feature-matrix.md)
- [产品路线图](docs/01-product/roadmap.md)
- [ADR 索引](docs/03-adr/README.md)

## 变更治理

需求变更必须记录影响范围、优先级、验收标准和版本归属；架构级变化通过 ADR 评审。未经明确授权，不自动提交、合并或强制推送。

## 许可证

本项目当前仓库包含 Apache License 2.0，详见 [LICENSE](LICENSE)。品牌名称和商标不因开源许可证而获得授权。
