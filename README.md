# 自磨工坊 · 个人智能工作台

**Personal AI Workbench**
Git repository: personal-ai-workbench
微信公众号：AI智能体打磨工坊

> 自主构建 · 持续打磨 · 知行合一

自磨工坊是一款面向个人的本地优先工作与成长平台，统一组织目标、领域责任、项目、任务、学习、知识上下文、AI Agent、工程活动与内容创作。它负责工作组织、上下文关联、任务协调、权限控制和结果追溯；专业工具负责专业执行。

## 产品原则

- Local-first：个人核心工作数据由本地应用掌控。
- Offline-first：断网时仍可使用核心工作管理能力。
- Security-first：最小权限、明确授权、可审计。
- Agent-ready：通过适配器连接可验证的外部 Agent，不绑定单一厂商。
- Human-governed：高风险操作和内容发布保留人工决策。
- Modular monolith：先建立清晰模块边界，不为早期版本引入微服务复杂度。
- Evidence-based Growth：学习活动与技能评估分离；技能判断必须有证据并由用户确认。

## 核心业务域

1. Work Management：Goal、Area、Project、Task。
2. Learning & Growth：Learning Roadmap、Plan、Item、Session、Skill、Evidence、Assessment。
3. Knowledge & Context：本地笔记、工作上下文、外部知识引用。
4. AI Collaboration：Agent 注册、能力、执行状态和授权。
5. Engineering：代码仓库、工程活动和 Artifact 关联。
6. Creator：内容选题、研究、写作、审核和发布辅助。
7. Automation：可控的自动化任务与后续工作流。
8. System & Security：设置、权限、审计、备份、恢复和升级。

## 核心成长闭环

**Learn → Build → Verify → Deliver → Reflect**

学习域管理学习过程和技能证据；项目域管理工程工作和交付；知识域管理资料与引用。三者通过稳定 ID、应用契约和版本化事件协作，不共享私有表写权限。

## 技术基线

- Desktop：Tauri 2
- Frontend：React + TypeScript
- Backend：Rust
- Local database：SQLite
- Local search：SQLite FTS5
- Build：Cargo + pnpm
- Optional server：后续评估，不作为本地桌面运行前置依赖。

## 平台与离线承诺

Alpha 0.1 起交付与验收目标为 Windows（Windows-only，见 ADR-021）；macOS 保留为后续版本的 macOS readiness 里程碑目标，架构保持平台中立。核心工作管理、学习本地记录（在对应版本交付后）、知识引用元数据、搜索、附件管理、备份恢复应可在完全断网条件下运行。云端 AI、远程知识服务和微信平台接口不可用时，必须显示明确状态并提供降级路径。

## 项目状态

- **E0.2.2.3 Final Documentation Remediation & Freeze Gate：** Completed as documentation/design baseline only; software implementation and runtime verification are not implied.
- **E0.3：** Architecture Consolidation & Engineering Readiness — E0.3.1 review and E0.3.2 ratification completed on 2026-11-08 (work delivered on docs/e0.3-architecture-consolidation, merged into main; branch deleted). ADR-017～020 are Accepted as the design baseline; implementation remains Not Implemented and verification Not Verified.
- **E1：** Engineering skeleton and first Windows vertical slice — work packages E1-01…E1-10 implemented and merged to main with automated evidence (see [E1 engineering evidence report](docs/04-quality/E1-engineering-evidence-report.md); §5.1 records the Gate E windowed run: items 1–3 pass, offline check deferred, UI workspace-picker gap logged to E2 and repaired there).
- **E2：** Work Management vertical slice — work packages E2-01…E2-06 implemented on feat/e2-work-management with automated evidence (62 tests with --no-fail-fast, clippy/fmt clean; see [E2 engineering evidence report](docs/04-quality/E2-engineering-evidence-report.md)). Areas/Goals/Tasks with project links, scoped FTS5 search (CJK-aware), attachments with deletion guards, real-data Dashboard and the full Work Management UI are implemented; the windowed Gate E run and milestone acceptance remain explicit human steps.

## 文档入口

- [产品愿景](docs/00-overview/product-vision.md)
- [产品范围](docs/00-overview/product-scope.md)
- [术语表](docs/00-overview/terminology.md)
- [PRD 1.0](docs/01-product/PRD-v1.0.md)
- [Learning & Growth PRD](docs/01-product/learning-growth-PRD.md)
- [需求基线](docs/01-product/requirements-baseline.md)
- [功能矩阵](docs/01-product/feature-matrix.md)
- [产品路线图](docs/01-product/roadmap.md)
- [领域架构](docs/02-architecture/domain-architecture.md)
- [跨领域数据契约](docs/02-architecture/cross-domain-data-contracts.md)
- [ADR 索引](docs/03-adr/README.md)
- [E0.3 架构调整方案](docs/05-architecture-evolution/E0.3-architecture-adjustment-plan.md)
- [E0.3 GitHub 文档变更清单](docs/05-architecture-evolution/E0.3-github-document-change-list.md)

## 变更治理

需求变更必须记录影响范围、优先级、验收标准和版本归属；架构级变化通过 ADR 评审。未经明确授权，不自动提交、合并或强制推送。

## 许可证

本项目当前仓库包含 Apache License 2.0，详见 [LICENSE](LICENSE)。品牌名称和商标不因开源许可证而获得授权。
