# 产品需求文档（PRD）1.0

**产品：** 自磨工坊 · 个人智能工作台  
**英文描述：** Personal AI Workbench  
**状态：** Accepted architecture/product baseline; implementation is phased  
**版本：** 0.2.0  
**目标平台：** Windows-first，跨平台架构预留（Windows/macOS）

## 1. 产品概述

自磨工坊是一款本地优先的个人工作与成长平台，统一组织个人目标、责任领域、项目、任务、学习、知识上下文、AI Agent、工程活动和内容创作。它负责工作组织、上下文关联、任务协调、权限控制和结果追溯；专业工具负责专业执行。

核心工作模型为 **Goal + Area + Project + Task**：
- Goal：用户希望达成的长期或阶段性结果。
- Area：需要持续维护的责任领域。
- Project：有明确目标、范围、状态和成果的工作单元。
- Task：可执行、可跟踪的具体工作项。

核心成长循环为 **Learn → Build → Verify → Deliver → Reflect**。

## 2. 用户与核心场景

首要用户为需要管理复杂个人知识工作、软件工程项目和 AI 辅助任务的个人专业用户。关键场景：
- 管理目标、持续责任领域、项目和任务。
- 将学习路线和学习计划与自研项目结合。
- 记录学习活动，并将实验、测试、代码、文档和交付物作为技能证据。
- 关联本地知识、外部知识库和工程资料。
- 注册外部 Agent，追踪能力和执行状态；执行能力必须经过授权门禁。
- 管理内容选题、草稿、审核和发布辅助。
- 断网时继续使用本地数据。

## 3. 业务领域

### 3.1 Work Management
拥有 Goal、Area、Project、Task 及其生命周期。Alpha 0.1 的核心工作闭环以 Goal + Area + Project + Task 为模型。Project 与 Task 可建立关系；Task 可独立存在。跨领域模块只能通过 Work Management 应用契约创建或修改这些对象。

### 3.2 Learning & Growth
作为独立业务域，拥有 Learning Roadmap、Learning Plan、Learning Item、Learning Session、Skill、Skill Evidence 和 Learning Assessment。学习域管理学习过程和能力发展，不拥有 Project、Task、Note、KnowledgeRef 或 Artifact 的源记录。

- Roadmap：长期学习方向。
- Plan：阶段性学习计划。
- Item：主题、练习、实验、实现或检查点。
- Session：实际学习活动记录。
- Skill/Evidence/Assessment：Beta 0.5 起提供基于证据、用户确认的技能评估。

Learning Plan/Item 可关联 Project；Learning Item 可通过 Work Management 命令请求创建 Task；Session 可关联 Note、KnowledgeRef 和 Artifact。项目产生的学习缺口先作为建议，需用户确认后进入正式计划。

### 3.3 Knowledge & Context
管理本地笔记、工作上下文、KnowledgeRef 元数据和被允许的本地索引。Lingshu Knowledge、Hindsight、协议标准仓库和 Git 仓库保留其源内容权威。Workbench 保存引用、标识、允许的快照和同步状态，不默认复制完整知识内容。

### 3.4 AI Collaboration
管理 Agent 注册、能力声明、适配器、执行状态和审批关联。Agent 执行与 Task、Workflow Run 分离。Alpha 0.1 不开放外部 Agent 执行；后续执行能力必须通过权限、审批和审计门禁。

### 3.5 Engineering
管理代码仓库关联、工程活动入口和 Artifact 引用/元数据，不替代 Git、IDE 或 CI 平台。

### 3.6 Creator
管理内容选题、研究素材、草稿、版本、审核和发布记录。未经人工确认不得自动发布。

### 3.7 Automation
管理触发器、Workflow 定义和执行记录。不得绕过 Agent 授权和安全策略。

### 3.8 System & Security
管理设置、凭据引用、权限、策略、审计、备份、恢复和迁移状态。

Dashboard 是跨领域只读汇总，不拥有权威业务记录。

## 4. 功能需求

### 4.1 Dashboard
- 展示真实目标、项目、任务、学习计划、Agent 和系统状态。
- 支持从汇总项进入对应业务对象。
- 区分计划进度、学习活动、技能评估和项目交付，不将它们合并为单一虚构分数。
- 明确区分本地状态、远程连接状态和数据更新时间。

### 4.2 Work Management
- 创建和切换 Workspace。
- 管理 Goal、Area、Project、Task。
- Project 可关联 Goal/Area；Task 可关联 Project；允许无项目 Task。
- 记录创建、更新时间和必要的状态变更。

### 4.3 Learning & Growth
- Alpha 0.1：只预留领域边界、稳定 ID 和通用关联扩展点；不交付完整学习中心。
- Alpha 0.2：Roadmap、Plan、Item、Session 基础生命周期；与 Project、Task、KnowledgeRef、Artifact 关联。
- Beta 0.5：Skill Matrix、Skill Evidence、用户确认的 Assessment；项目学习缺口反馈闭环。
- V1.0：可解释的学习复盘、跨项目成长报告和差距分析。
- 不得根据学习时长、课程完成或任务完成自动升级 Skill。
- Agent 生成的学习建议必须由用户确认后成为正式计划。

### 4.4 AI Collaboration
- 定义供应商无关的 Agent Adapter 契约。
- 展示适配器连接状态和能力声明。
- 记录 Agent 任务、执行状态、时间和结果引用。
- 对超时、取消、失败和不可用状态提供明确语义。
- 外部 Agent 断开不得影响本地项目、任务和学习记录。
- 高风险操作必须经过权限检查和人工审批。

### 4.5 Knowledge & Context
- Alpha 0.1：支持本地附件/文件管理及项目、任务的本地搜索；仅预留知识引用入口。
- Alpha 0.2：引入正式 KnowledgeRef 模型，支持外部知识资产引用及来源、标识、标题、位置、版本/更新时间和可用状态等元数据。
- 外部知识服务不可用时保留引用记录并提示状态。
- 不未经授权复制、覆盖或删除外部知识库原始数据。

### 4.6 Engineering
- 项目可关联本地代码目录和 Git 仓库标识。
- Artifact 可被学习记录引用为实践或评估证据。
- 首期不替代 IDE、Git 客户端或 CI 平台。

### 4.7 Creator
目标公众号：**AI智能体打磨工坊**。内容生命周期：选题 → 研究素材 → 文章草稿 → 版本修订 → 审核 → 人工确认 → 发布/人工发布 → 运营记录。
- 管理选题状态、栏目、优先级和关联项目。
- 管理文章草稿、版本和附件。
- 保存内容来源、引用和 Evidence Status。
- 记录审核意见、审核人和时间。
- 未经人工确认不得自动发布。

### 4.8 Automation
首期支持记录自动化任务和执行状态；任务触发、权限、执行结果和错误应可追踪。可视化工作流编辑器和自主编排属于后续版本。

### 4.9 System & Security
展示数据目录和运行配置；保护敏感凭据；默认不开放公网监听；文件访问限制在用户授权目录；支持审计、备份、恢复和迁移状态检查。

## 5. 非功能需求

- Windows-first，架构为后续 macOS 等平台保留适配边界。
- 首期核心业务数据以本地 SQLite 为权威数据源。
- 完全断网时核心工作管理仍可用。
- 数据库使用版本化 Migration。
- 数据库和附件备份可恢复并验证完整性。
- 关键操作可审计。
- Agent Adapter 可替换，不绑定单一 AI 厂商。
- 网络服务不可用时不得静默挂起或破坏本地数据。
- 所有 workspace-scoped 跨域关联必须验证 workspace 一致性。

## 6. 数据与隐私原则

本地数据默认留在本机。发送给外部 Agent 或服务的上下文必须经过明确配置和权限控制。连接器声明访问范围和能力。日志避免记录密钥、敏感正文和不必要的个人数据。备份位置由用户知情选择。学习证据优先保存稳定引用和简要摘要，不默认复制完整源材料。

## 7. 版本范围

- **Alpha 0.1：** 本地工作台、Goal/Area/Project/Task、Dashboard、搜索、附件、备份恢复、审计、离线；学习域仅预留扩展点。
- **Alpha 0.2：** Agent Adapter 基础、KnowledgeRef、Learning Roadmap/Plan/Item/Session。
- **Beta 0.5：** Skill Matrix、Evidence、Assessment、项目学习反馈闭环、Content Studio 基础流程。
- **V1.0：** 核心工作域协同、学习复盘与可解释成长报告。
- **V1.1+：** 多设备同步、服务端和高级 Agent/Workflow 能力评估。

## 8. 发布阻断条件

任一目标平台无法安装或启动；离线核心流程失败；迁移或恢复导致数据丢失；存在未解决 P0 安全问题；Agent 可越权访问未授权资源；关键业务数据不持久化或 Dashboard 使用虚假数据。

## 9. 平台发布配置与待评审事项

Windows 最低支持版本、macOS 适配范围、凭据存储、微信接口和品牌权利核验，仍按平台发布门禁逐项确认。架构预留跨平台不等于已经完成跨平台构建或验证。