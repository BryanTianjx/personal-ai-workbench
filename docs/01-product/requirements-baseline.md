# 需求基线与追踪规则

**状态：** Accepted planning baseline；实现状态另行追踪
**版本：** 0.4.0（0.3.0 基础上按 ADR-023～029 R1 方向新增 §4 PAW-INF-005～009；调整由 owner 于 2026-11-12 指示，对应 ADR 仍为 Proposed，批准前不排实现）

## 1. 优先级定义

- **P0：** Alpha 0.1 发布阻断需求。
- **P1：** 重要能力，允许在后续 Alpha/Beta 完成，但必须有明确计划；若为其他能力的安全前置条件，必须在依赖能力启用前完成。
- **P2：** 长期增强，不属于当前发布承诺。

## 2. P0 需求（保持现有基线）

| ID | 需求 | 验收依据 |
|---|---|---|
| PAW-F-001～008 | Workspace、Project、Task、Dashboard、搜索、附件、备份恢复、审计 | 按既有 E0.2.2.3 冻结基线验收 |
| PAW-N-001～004 | Windows 安装（原双平台安装经 ADR-021 调整为 Windows-only，macOS 移至后续版本 macOS readiness 里程碑；此 P0 变更的重新评审记录即 ADR-021）、离线核心可用、版本化迁移 | 真实环境测试（Windows） |
| PAW-S-001～003 | 默认无公网监听、敏感日志脱敏、文件访问限制 | 安全测试 |

## 3. 新增 P1 需求

| ID | 需求 | 目标版本 | 验收要点 |
|---|---|---|---|
| PAW-LG-001 | Learning Roadmap 生命周期 | Alpha 0.2 | 创建、编辑、归档、恢复 |
| PAW-LG-002 | Learning Plan 与 Roadmap 关系 | Alpha 0.2 | 计划归属与状态迁移正确 |
| PAW-LG-003 | Learning Item 生命周期 | Alpha 0.2 | 类型、验收标准、前置项和状态持久化 |
| PAW-LG-004 | Learning Session 记录 | Alpha 0.2 | 多次记录不可覆盖；可关联项目/任务/资料 |
| PAW-LG-005 | Learning 与 Project 关联 | Alpha 0.2 | 多对多关联、Workspace 隔离 |
| PAW-LG-006 | Learning Item 请求创建 Task | Alpha 0.2 | 通过 Work Management 命令，Task 仍由其拥有 |
| PAW-LG-007 | 项目学习缺口提议 | Beta 0.5 | 提议需用户确认，事件消费幂等 |
| PAW-LG-008 | Skill 与 SkillEvidence | Beta 0.5 | 证据类型、来源和 provenance 可追溯 |
| PAW-LG-009 | LearningAssessment 历史 | Beta 0.5 | 用户确认、尺度版本化、历史保留 |
| PAW-LG-010 | 学习/项目成长报告 | V1.0 | 数据来源可解释，不伪造能力分数 |
| PAW-K-001～002 | KnowledgeRef、Lingshu/Hindsight 连接器 | Alpha 0.2+ | 引用权限、来源失效和降级正确 |
| PAW-S-004 | Agent 授权、审批与审计机制 | Alpha 0.2 前置门禁 | 默认拒绝、最小权限、审批绑定、审计测试 |

## 4. 基础设施需求（ADR 派生）

以下需求由 E0.3 批准后的治理补全引入，用于把 ADR-017～020 的验证要求纳入可追踪基线。它们是架构派生的基础设施需求，不是产品功能需求：不扩大 Alpha 0.1 的产品范围，目标版本按 E 阶段（ADR-021）归属，验收依据为对应 ADR 的 Validation required 小节，ADR 始终是治理源头。

| ID | 需求 | 治理 ADR | 目标版本 | 验收依据 |
|---|---|---|---|---|
| PAW-INF-001 | 跨域协调边界：仅命名用例协调、不暴露私有仓储、业务不变量留在属主域、每次受保护副作用前重新授权 | ADR-017 | E2/E3 | ADR-017 Validation required |
| PAW-INF-002 | Rust workspace 依赖方向向内、禁止边由自动化架构检查强制（目录约定不充分） | ADR-018 | E1 | ADR-018 Validation required |
| PAW-INF-003 | Operation/OperationAttempt 持久化、进程重启恢复、OutcomeUnknown 阻断非幂等重试、状态机不与 Task/Workflow Run/Agent Execution 混用 | ADR-019 | E1/E3 | ADR-019 Validation required |
| PAW-INF-004 | 跨域契约/事件版本化、at-least-once 投递与消费幂等、未知版本隔离、毒消息有界重试 | ADR-020 | E3/E4 | ADR-020 Validation required |
| PAW-INF-005 | 能力注册表与治理式调用：描述符版本化（ID/属主域/契约版本/schema/risk class/idempotency/offline/approval hint）、Registry 仅元数据、调用经属主域 use case 与 Coordination | ADR-026 | E5 | ADR-026 Validation required |
| PAW-INF-006 | Agent 调用安全与审批绑定：不可信输入默认拒绝、审批绑定规范化计划摘要/主体/工作区/范围/目标/有效期、每个副作用前重新授权、非幂等不确定结果不盲重试、审计证据完整且脱敏 | ADR-029 | E5 | ADR-029 Validation required |
| PAW-INF-007 | 一方插件静态注册与依赖治理：描述符校验、依赖 DAG 与环拒绝、四状态维度独立（注册/可用/组合偏好/可见性）、版本维度分离 fail-closed、停用/移除不删数据且 Operation 先对账、Learning 一等领域不被 Core 吸收、插件化不扩张 Alpha/Beta 既定范围 | ADR-023/027/028 | E7 | ADR-023/027/028 Validation required |
| PAW-INF-008 | RoleProfile 仅为体验模板：不携带 grant/凭据/审批；切换不改授权；导入校验剥离安全字段；失效/损坏模板安全回退 | ADR-024 | E7 | ADR-024 Validation required |
| PAW-INF-009 | WorkspaceComposition 仅存引用与展示配置、限定单一 Data Workspace、使用时由后端授权解析、损坏/未知 schema 安全回退且不改业务数据 | ADR-025 | E7 | ADR-025 Validation required |

PAW-INF-005～009 于 2026-11-12 按 [ADR-023～029 R1 方向](../04-quality/ADR-023-029-R1-cross-adr-review.md)引入：目标版本按本路线图 owner 决策归属（005/006→E5，007～009→E7）。对应 ADR 当前均为 Proposed / Not Accepted / Not Implemented / Not Verified；**各 ADR 批准为 Accepted 前，对应需求状态保持 proposed，不进入实现规格排期**。

## 5. P2 需求

多设备同步和冲突解决、独立 Web Server、可视化工作流编辑器、多 Agent 团队协作、知识图谱、AI 自动学习建议和自动调整计划。

## 6. Learning 安全与质量约束

- 初始 Skill 不得预置未经用户确认的 proficiency。
- Session 时长、Item 完成率、课程完成状态不能作为能力等级的自动证明。
- Agent 生成的学习建议必须标记为建议，并经用户确认。
- 外部 KnowledgeRef 失效不得级联删除学习历史。
- 所有跨域引用必须验证 Workspace 一致性与访问权限。
- Assessment 历史保留；更正通过新记录或明确修订流程完成。

## 7. Agent 安全前置门禁

PAW-S-004 不属于 Alpha 0.1 的 Agent 功能交付承诺。Alpha 0.1 必须完成授权模型、接口边界和测试规范的设计；Agent 外部执行保持关闭。Alpha 0.2 中，只有在默认拒绝、最小权限、资源范围校验、人工审批绑定执行计划、审计记录及相应自动化测试均实现并通过验收后，才可启用 Agent 执行。

## 8. 追踪规则

每条需求应关联 PRD 章节、设计/架构文档、实现模块或任务 ID、自动化测试或人工验收用例、当前状态与证据。状态统一为 proposed、approved、implemented、verified、deferred、rejected。未经测试不得标记 verified。

PAW-INF 系列需求的追踪对象是对应 ADR 及其 Validation required 小节；实现与验证状态随 E 阶段门禁（ADR-021）记录，与 `requirements-architecture-traceability.md` 的架构追踪表一一对应。

## 9. 冻结规则

需求冻结后，变更必须记录原因、用户价值、影响模块、数据迁移、安全和平台影响、验收标准及目标版本。P0 变更需要重新评审发布范围。
