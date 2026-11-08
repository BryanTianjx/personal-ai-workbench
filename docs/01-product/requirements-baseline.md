# 需求基线与追踪规则

**状态：** Accepted planning baseline；实现状态另行追踪  
**版本：** 0.2.0

## 1. 优先级定义

- **P0：** Alpha 0.1 发布阻断需求。
- **P1：** 重要能力，允许在后续 Alpha/Beta 完成，但必须有明确计划；若为其他能力的安全前置条件，必须在依赖能力启用前完成。
- **P2：** 长期增强，不属于当前发布承诺。

## 2. P0 需求（保持现有基线）

| ID | 需求 | 验收依据 |
|---|---|---|
| PAW-F-001～008 | Workspace、Project、Task、Dashboard、搜索、附件、备份恢复、审计 | 按既有 E0.2.2.3 冻结基线验收 |
| PAW-N-001～004 | 双平台安装、离线核心可用、版本化迁移 | 真实环境测试 |
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

## 4. P2 需求

多设备同步和冲突解决、独立 Web Server、可视化工作流编辑器、多 Agent 团队协作、知识图谱、AI 自动学习建议和自动调整计划。

## 5. Learning 安全与质量约束

- 初始 Skill 不得预置未经用户确认的 proficiency。
- Session 时长、Item 完成率、课程完成状态不能作为能力等级的自动证明。
- Agent 生成的学习建议必须标记为建议，并经用户确认。
- 外部 KnowledgeRef 失效不得级联删除学习历史。
- 所有跨域引用必须验证 Workspace 一致性与访问权限。
- Assessment 历史保留；更正通过新记录或明确修订流程完成。

## 6. Agent 安全前置门禁

PAW-S-004 不属于 Alpha 0.1 的 Agent 功能交付承诺。Alpha 0.1 必须完成授权模型、接口边界和测试规范的设计；Agent 外部执行保持关闭。Alpha 0.2 中，只有在默认拒绝、最小权限、资源范围校验、人工审批绑定执行计划、审计记录及相应自动化测试均实现并通过验收后，才可启用 Agent 执行。

## 7. 追踪规则

每条需求应关联 PRD 章节、设计/架构文档、实现模块或任务 ID、自动化测试或人工验收用例、当前状态与证据。状态统一为 proposed、approved、implemented、verified、deferred、rejected。未经测试不得标记 verified。

## 8. 冻结规则

需求冻结后，变更必须记录原因、用户价值、影响模块、数据迁移、安全和平台影响、验收标准及目标版本。P0 变更需要重新评审发布范围。