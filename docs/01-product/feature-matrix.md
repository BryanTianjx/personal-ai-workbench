# 功能矩阵

**状态：** Accepted planning baseline · **版本：** 0.3.1（0.3.0 基础上记录 ADR-023～029 全部 Accepted（owner 2026-11-12）；实现/验证仍 Not Implemented / Not Verified）

| 功能域 | Alpha 0.1 | Alpha 0.2 | Beta 0.5 | V1.0+ |
|---|---|---|---|---|
| Dashboard | 项目/任务真实数据 | Agent 与学习计划摘要 | 学习/内容待办与证据摘要 | 可解释的工作与成长洞察 |
| Work Management | Goal/Area/Project/Task 基础闭环 | 关联与视图增强 | 复盘增强 | 个人工作管理增强 |
| Learning & Growth | 领域边界和关联扩展点；不交付完整学习中心 | Roadmap、Plan、Item、Session | Skill Matrix、Evidence、Assessment、项目学习闭环 | 差距分析、成长报告、AI 辅助建议 |
| AI Collaboration | 模块边界和状态模型；不开放 Agent 执行 | Agent 注册、Adapter、受控执行；调用经能力注册与审批绑定（ADR-026/029 方向） | 多适配器与协作基础 | 多 Agent 协作 |
| Knowledge & Context | 本地文件/附件管理与本地搜索；预留引用入口 | KnowledgeRef、外部知识引用、Lingshu/Hindsight 连接器 | 关联与沉淀增强 | 高级知识组织 |
| Engineering | 项目入口和关联预留 | Git 信息集成、Artifact 引用契约 | 工程活动与测试证据 | DevOps 增强 |
| Creator | PRD/领域设计 | 内容模型预留 | 选题、草稿、版本、审核 | 内容运营闭环 |
| Automation | 不提供可视化编排 | 基础触发/任务模型 | 简单工作流 | 可视化与编排 |
| System & Security | 设置、审计、备份恢复；定义权限边界 | Agent 执行前置授权/审批/审计 | 安全策略增强 | 高级治理 |
| 平台组合（插件/档案/组合） | — | 能力注册表与治理式调用基座（随 E5，ADR-026/029 批准后） | —（不扩张既定内容范围） | 内置一方插件静态注册、RoleProfile、WorkspaceComposition、依赖治理（随 E7，ADR-023–025/027/028 批准后） |

## 说明

1. 矩阵表示版本规划，不代表功能已经实现。
2. Learning Item 与 Task 是不同领域对象；Task 可作为学习行动步骤。
3. 技能等级必须由用户根据证据确认，不因完成课程或学习任务自动升级。
4. 微信公众号自动发布取决于官方接口权限和真实账号验证。
5. 完全离线能力属于 Alpha 0.1 发布阻断要求。
6. 所有版本均需保留数据可导出、备份和迁移能力。
7. "平台组合"行对应 [ADR-023～029](../03-adr/README.md)（已于 2026-11-12 全部批准为 Accepted，实现与验证状态仍为 Not Implemented / Not Verified）：各单元格为已批准设计基线下的版本规划；实现规格与实现开工按阶段独立授权（E5 规格已定稿，E7 规格未授权）；插件化不改变 Learning/Creator/Automation 的既定交付范围（ADR-027）。
