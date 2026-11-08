# 产品路线图

**状态：** Accepted planning baseline · **版本：** 0.3.0 (E0.3 review extension)

## E0 — 产品与架构基线

完成需求、架构、数据、安全、视觉和验收规范；通过评审后冻结 Alpha 0.1 范围。Learning & Growth 作为正式领域纳入基线，具体交付分阶段。

## E0.3 — Architecture Consolidation & Engineering Readiness

将 E0.2 设计基线转化为可实施工程契约。评审 Coordination Layer、Rust Workspace/Crate 边界、Operation 生命周期与恢复、跨领域契约版本治理、Workspace 安全边界和 UI 信息架构。补齐 E1 纵向切片及验收证据要求。

**状态：E0.3.2 Architecture Ratification & E1 Engineering Specification completed as a design baseline。** ADR-017～020 已于 2026-11-08 批准为 Accepted，澄清项为规范性约束。E1 工程规格已形成；代码实现与运行时验证仍未完成。

## Alpha 0.1 — 可独立工作的本地个人工作台

Windows/macOS 桌面应用；Goal、Area、Project、Task；Dashboard、搜索和附件；SQLite Migration；审计、备份和恢复；完全离线验收。仅保留 Learning & Growth 的领域边界、稳定 ID 和通用关联扩展点，不交付完整 Learning Center。

## Alpha 0.2 — 可控的 Agent、知识与学习基础

统一 Agent Adapter 契约；Agent 注册和受控执行（通过安全门禁后）；KnowledgeRef；Lingshu/Hindsight 连接器评估与实现；Learning Roadmap、Plan、Item、Session 基础生命周期；Learning 与 Project/Task/KnowledgeRef/Artifact 的契约关联。

## Beta 0.5 — 内容生产与项目实践成长

微信公众号内容流程；Skill Matrix、SkillEvidence、用户确认的 LearningAssessment；项目提出学习缺口、用户确认后转化为学习计划；学习产物回流项目复盘。微信官方接口经验证后再启用自动化发布能力。

## V1.0 — 个人工作与成长平台

形成目标、项目、任务、学习、技能证据、Agent、知识和内容运营之间的可追踪闭环；提供可解释的学习复盘与跨项目成长报告；具备可靠的权限、安全和恢复能力。

## V1.1+ — 服务端与多设备

评估独立 Web 服务、身份认证、同步协议和冲突解决。禁止多个客户端直接并发操作同一 SQLite 文件。

## V1.3+ — Agent 协作与工作流

增加多 Agent 协作、任务委派、可视化工作流和更完整的执行治理。

## 里程碑门禁

- Gate E0：PRD、需求追踪、架构和验收基线通过评审。
- Gate E0.3：Coordination、模块边界、Operation 可靠性、契约版本和 E1 验收方案通过架构评审。
- Gate E1：Windows 工程骨架可构建；基础通信、Workspace 初始化、SQLite Migration 和 Project 纵向切片通过。
- Gate E2：本地业务闭环和离线场景通过。
- Gate E3：Agent/知识/学习集成权限、关联与失败降级通过。
- Gate E4：内容流程、审核和发布辅助通过。
- Gate E5：双平台安装、升级、恢复、安全和发布检查通过。

## E1 首个纵向切片

Tauri shell → React view → typed Tauri command → Rust application use case → Work Management domain → SQLite repository → restart application → persisted Workspace and Project restored.

验收必须包括 Windows 原生构建、数据持久化、重启恢复、基本迁移测试、跨 Workspace 访问拒绝和架构依赖检查。演示截图或仅编译成功不构成验收。

## 进度管理原则

路线图不预设未经验证的交付日期。每个里程碑应以可复现的验收证据关闭，不以代码行数、页面数量或演示截图代替验收。


### E0.3.1 — ADR Review & Consolidation

评审结论建议：

- ADR-017：建议接受；Coordination 是 paw-application 内的逻辑能力，不是新业务域或独立服务。
- ADR-018：建议接受；初始四 crate，发布的应用 DTO/Ports 属于 paw-application，领域事件与不变量属于 paw-domain。
- ADR-019：建议接受；Operation 作为执行信封，OperationAttempt 记录单次副作用尝试；Task/Workflow Run/Agent Execution 各自保留生命周期。
- ADR-020：建议接受；仅对公开跨域契约与持久化事件版本化，定义兼容性、幂等和隔离失败事件规则。

以上为技术评审建议，不代替仓库所有者批准。E0.3.1 验收还需架构文档一致性、契约测试方案和文档验证通过。
