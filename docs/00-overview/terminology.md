# 术语表

| 术语 | 定义 |
|---|---|
| 自磨工坊 | 产品母品牌及产品中文名称 |
| Personal AI Workbench | 产品英文描述；产品定位扩展为 Personal Work & Growth Platform |
| Workspace | 用户组织目标、项目、任务、学习记录和配置的本地工作空间 |
| Goal | 用户希望达成的长期或阶段性结果 |
| Area | 需要持续维护的责任领域 |
| Project | 有目标、范围、状态和成果的工作单元 |
| Task | 可分配、可执行、可跟踪的工作项 |
| Learning & Growth | 管理学习路线、计划、活动、技能证据与评估的独立业务域 |
| Learning Roadmap | 长期学习方向或课程路线 |
| Learning Plan | Roadmap 下具有阶段目标的学习计划 |
| Learning Item | 学习计划中的主题、练习、实验、实现或检查点 |
| Learning Session | 一次实际发生的学习活动记录 |
| Skill | 可被定义、练习和评估的能力项 |
| Skill Evidence | 支持某项技能评估的可追溯证据 |
| Learning Assessment | 用户根据证据确认的技能评估记录 |
| Agent | 通过外部运行环境执行特定任务的智能体 |
| Agent Adapter | 将外部 Agent 能力映射到统一契约的集成组件 |
| Agent Execution | 一次 Agent 执行的状态、输入/输出引用和审计记录 |
| Workflow | 由步骤、依赖和条件组成的流程定义 |
| KnowledgeRef | 指向外部或本地知识资产的引用，不必拥有原始内容 |
| Artifact | 可追溯的工程、学习或内容产物元数据及其来源引用 |
| Content Studio | 管理内容生产与运营流程的模块 |
| Evidence Status | 标记内容或技术结论的证据成熟度 |
| Local-first | 本地数据和本地操作优先，远程服务为可选扩展 |
| Offline-first | 网络断开时，明确范围内的核心能力仍可使用 |
| Modular Monolith | 单一可部署应用内按领域边界模块化 |
| ADR | Architecture Decision Record，架构决策记录 |
| P0/P1/P2 | 发布阻断、高优先级和后续优先级分类 |

## 内容证据状态

- `verified`：已通过可追溯来源核验。
- `tested`：已在明确环境中实际测试。
- `in_progress`：正在研究或实施。
- `proposed`：设计建议或待评审方案。
- `unverified`：尚未核实。

证据状态应与内容版本和来源关联，不应把 proposed 或 unverified 描述为已交付事实。技能评估状态与内容证据状态是不同概念，不得混用。