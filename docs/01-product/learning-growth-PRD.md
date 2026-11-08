# Learning & Growth — 产品需求文档

**产品：** Personal AI Workbench / 自磨工坊  
**领域：** Learning & Growth  
**状态：** Accepted domain scope; phased delivery  
**版本：** 0.1  
**决策依据：** PAW-ADR-013～016  
**实现状态：** Not Implemented

## 1. 产品目标

为个人持续学习和自研项目开发建立可追踪闭环，使学习计划能够服务真实项目，项目实践能够反向暴露学习差距，并通过可审查的证据沉淀技能成长。

核心循环：

**Learn → Build → Verify → Deliver → Reflect**

Learning & Growth 是个人工作与成长平台中的独立业务域，不是 LMS 的完整替代品，也不负责托管全部知识资料。

## 2. 目标用户与场景

目标用户为长期开展软件工程、AI Agent、协议工程及系统研发的个人开发者。

典型场景：
1. 制定跨季度技术路线，并拆分为阶段性学习计划。
2. 将 Rust、Agent Runtime、Durable Execution 等学习项关联 FEMRIX 项目实践。
3. 将 C/C++、Wi-Fi、3GPP NAS、ASN.1 等学习项关联 ProCodec 工程任务。
4. 从项目复盘中记录新的能力缺口，形成待确认的学习建议。
5. 将实验、代码实现、测试报告、技术文档或项目交付物关联为技能证据。
6. 定期回顾学习投入、实践结果和待补齐能力。

## 3. 功能范围

### 3.1 Learning Roadmap
管理长期学习方向和主题分组。Roadmap 包含名称、目标描述、状态、排序、时间范围（可选）及关联技能/项目。一个 Roadmap 可包含多个 Learning Plan。

### 3.2 Learning Plan
管理阶段性目标、范围、里程碑和关联项目。计划可处于 `draft`、`active`、`paused`、`completed`、`archived`。计划完成表示计划目标已由用户确认完成，不代表技能等级自动提升。

### 3.3 Learning Item
表示一个可学习、可实践或可验证的目标。类型建议包括 `concept`、`reading`、`exercise`、`experiment`、`implementation`、`review`。支持优先级、预期结果、验收标准、前置项和外部知识引用。Learning Item 可以关联 Work Management Task，但两者不是同一个实体。

### 3.4 Learning Session
记录实际发生的学习活动，包括开始/结束时间（可选）、学习方式、时长（可选）、学习摘要、困难、收获、下一步和关联资料/项目/任务/产物。Session 是历史记录，不因计划调整而被覆盖。

### 3.5 Skill Matrix（Beta 0.5）
展示用户定义的技能及最近一次确认评估。支持技能分类、评估尺度版本、证据列表、评估日期、置信度和说明。不得在无证据或未经用户确认时推断等级。

### 3.6 Skill Evidence（Beta 0.5）
支持概念解释、练习、实验、实现、测试验证、项目交付、技术评审、教学/文档等证据类型。证据指向 Artifact、Note、KnowledgeRef、Task 或 Project，不默认复制源内容。

### 3.7 Assessment（Beta 0.5）
用户可根据证据创建评估记录。保留历史；Agent 只能给建议，不能直接修改正式技能等级。

### 3.8 Project Learning Loop
Project 可提出待确认的学习需求；Learning 可关联项目、创建学习项或请求生成 Task；学习结果可回流项目复盘或 Dashboard。跨域交互遵循 PAW-ADR-014 和跨领域数据契约。

## 4. 项目结合示例

| 学习路线 | 关联项目 | 示例学习主题 | 可形成的证据 |
|---|---|---|---|
| Agent & Runtime Engineering | FEMRIX | Agent Harness、Durable Execution、Actor Model、Event Sourcing、Rust Async、Capability/Policy | 最小实现、故障恢复测试、ADR、基准测试 |
| Protocol & Systems Engineering | ProCodec | C/C++/Rust、内存与二进制协议、Wi-Fi 6/7、3GPP NAS、ASN.1/PER | Codec 实现、互操作测试、协议解析报告 |
| AI Engineering & Tooling | Personal AI Workbench / Lingshu | Agent Adapter、MCP、RAG、检索、Hindsight、自动化 | 集成测试、检索评估、运行记录 |
| Software Architecture & Product Engineering | 跨项目 | DDD、模块化单体、ADR、测试、CI/CD、AI 辅助开发 | 架构决策、CI 结果、发布记录、复盘 |

上述路线仅为初始模板，不代表用户当前技能等级。技能和评估须由用户确认。

## 5. 核心用户流程

### 5.1 计划学习
创建 Roadmap → 创建 Plan → 添加 Learning Items → 关联 Project / KnowledgeRef → 可选生成 Task。

### 5.2 记录学习
打开 Learning Item → 创建 Session → 记录内容和结果 → 关联 Note / Artifact / Task → 更新 Item 状态。

### 5.3 项目驱动学习
Project 复盘提出学习缺口 → 形成建议事件/Inbox 项 → 用户确认 → 创建或关联 Learning Item → 计划练习和验证 → 将结果回流项目。

### 5.4 技能评估
选择 Skill → 查看证据 → 用户创建 Assessment → 保留评估历史和尺度版本。

## 6. 非功能与数据要求

- 所有实体按 Workspace 隔离；跨 Workspace 引用必须拒绝。
- 离线时可创建和编辑本地 Roadmap、Plan、Item、Session（对应版本交付后）。
- 时间、时长和进度不能被误读为能力证明。
- 学习历史和评估历史具备审计与导出能力。
- 外部资料只保存引用和必要快照，不复制整个知识库。
- 删除、归档和解除关联语义必须明确。
- AI 生成内容默认标记为建议，需用户确认后成为正式计划或评估。

## 7. 版本规划

| 版本 | 范围 |
|---|---|
| Alpha 0.1 | 只预留领域边界、稳定 ID、关联扩展点；可用通用 Task 类型/标签表达学习任务，但不宣称已有完整学习域 |
| Alpha 0.2 | Roadmap、Plan、Item、Session 基础 CRUD 与项目关联；基础学习中心视图 |
| Beta 0.5 | Skill、SkillEvidence、Assessment、项目驱动学习闭环 |
| V1.0 | 学习复盘、跨项目成长报告、可解释的差距分析 |
| V1.1+ | 经用户授权的 AI 学习建议、计划调整辅助和自动化报告 |

## 8. Alpha 0.2 验收标准

- 可创建、编辑、归档 Roadmap 和 Plan。
- Plan 可包含多个 Learning Item。
- Item 可关联一个或多个 Project，并可通过 Work Management 命令关联 Task。
- 可创建多条不可互相覆盖的 Learning Session。
- 所有跨域关联通过稳定 ID 和 Workspace 校验。
- 外部知识来源失效不删除学习记录。
- Dashboard 能区分计划进度、Session 活动和正式技能评估。
- 断网可完成本地学习记录流程。
- 数据导出和备份恢复覆盖学习数据。

## 9. 明确非目标

Alpha 0.2 不实现完整课程市场、考试系统、团队 LMS、自动技能评级、自动抓取付费课程、未经确认的 AI 学习计划改写或全量复制 Lingshu/Hindsight 内容。
