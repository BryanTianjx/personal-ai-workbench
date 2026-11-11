# ADR-023～029 R1 跨 ADR 一致性审查报告

- **项目：** Personal AI Workbench
- **审查批次：** ADR-023～029 R1
- **日期：** 2026-11-11
- **审查结论：** 有条件通过文档一致性审查；提交状态仍为 Proposed
- **决策状态：** Proposed / Not Accepted
- **实现状态：** Not Implemented
- **验证状态：** Not Verified
- **审查范围：** ADR-001～006、ADR-013～022、产品 PRD、领域架构、跨领域数据契约、模块边界、Agent Adapter Architecture、Agent Authorization Model、E0.3/E1 现有工程基线

## 1. 执行摘要

R1 将 Domain Plugin、Role Profile、WorkspaceComposition、Capability Registry、Agent invocation security 作为一组相互约束的架构决策提出。总体方向与既有 Modular Monolith、四 crate 边界、单 SQLite、本地优先、Learning & Growth 一等领域、Application Coordination、Operation 可靠性和 Workbench/Hermes 边界保持一致。

R1 的关键收敛点是：

- Domain 继续拥有业务事实、实体、不变量、持久化与领域事件；Plugin 只是静态注册和组合单元。
- Role Profile 是体验模板，不是安全角色或权限主体。
- WorkspaceComposition 是展示/编排配置，不是 Data Workspace，不构成安全隔离。
- Capability Registry 只维护描述符和版本解析，不是 Service Locator 或执行容器。
- Agent/Provider 输出均是不可信输入；授权由后端执行，默认拒绝，并在每个副作用前重新检查。
- Approval 绑定不可变规范化计划摘要、主体、数据工作区、资源范围、目标、能力、工具、有效期和尝试策略。
- 插件停用、升级或移除不得删除领域数据、篡改历史或静默取消 durable Operation。
- Learning & Growth 继续保持 ADR-013～016 的领域边界，不因插件化扩张 Alpha 范围。

本报告不批准任何实现，也不把文档审查等同于设计决策批准。七份 ADR 均保持 Proposed / Not Implemented / Not Verified，须由产品/架构负责人逐份审批。

## 2. 基线与审查方法

审查按以下维度进行：领域所有权、数据/Workspace 安全边界、模块依赖、能力契约、Agent 授权、Operation 可靠性、版本兼容、产品阶段范围、迁移和验收可执行性。

主要对照基线：

| 基线 | 本次约束 |
|---|---|
| ADR-002 Modular Monolith | 不引入第五 crate、微服务或动态插件进程 |
| ADR-003 / ADR-006 数据所有权与单 SQLite | 领域数据仍由领域模块拥有；不新增第二套数据拓扑 |
| ADR-004 Agent Adapter Boundary | Provider 适配与业务能力、授权分离 |
| ADR-005 Offline-First | 内置插件与核心能力需定义离线行为；不得隐含在线依赖 |
| ADR-013～016 Learning & Growth | Learning 是一等领域；遵循证据、评估和 Knowledge 边界 |
| ADR-017 Coordination Layer | 跨领域协调仍由 Application Coordination 负责 |
| ADR-018 Rust Workspace Boundaries | 维持四 crate 与可执行模块边界 |
| ADR-019 Operation Reliability | 持久化 Operation、幂等、Outbox、OutcomeUnknown 和恢复语义继续有效 |
| ADR-020 Contract Versioning | 跨域契约显式版本化，兼容性不得依赖隐式约定 |
| ADR-022 Workbench ↔ Hermes | Workbench 不复制外部 Agent Runtime；外部执行受 Workbench 治理 |
| Agent Authorization Model | 默认拒绝、最小权限、显式资源范围、后端执行、审批与审计 |

## 3. 七份 ADR 的逐项审查

| ADR | 核心决策 | 与既有基线的一致性 | R1 主要风险控制 | 审查结果 |
|---|---|---|---|---|
| 023 | Domain Plugin + 静态注册 | 与领域模块和模块化单体一致 | 区分注册、可用性、组合偏好、UI 可见性；停用不删数据、不取消 Operation | 通过，需人工批准 |
| 024 | Role Profile 仅为体验模板 | 不改变 System & Security 权限语义 | 不携带 grant、secret、approval；导入校验与安全回退 | 通过，需人工批准 |
| 025 | WorkspaceComposition 与 Data Workspace 分离 | 保留 ADR-003/006 数据边界 | 组合配置只存引用；每次使用仍由后端授权 | 通过，需人工批准 |
| 026 | 五层能力与 Agent 调用治理 | 对齐 ADR-004/017/018/019/022 | Registry 仅元数据；执行经 Domain Use Case 和 Coordination | 通过，需人工批准 |
| 027 | 一方领域插件采用策略 | 保留 ADR-013～016 与现有领域所有权 | Learning 一等领域；插件化不扩 Alpha 范围 | 通过，需人工批准 |
| 028 | 依赖 DAG 与兼容治理 | 对齐 ADR-020 和领域迁移所有权 | 版本维度分离；破坏性/安全变更 fail closed；Operation drain/reconcile | 通过，需人工批准 |
| 029 | Agent invocation 与精确审批绑定 | 对齐授权模型、ADR-019/022 | 默认拒绝；digest/target/scope/expiry 绑定；每个副作用前重新授权 | 通过，需人工批准 |

## 4. 跨 ADR 不变量矩阵

| 不变量 | 023 | 024 | 025 | 026 | 027 | 028 | 029 |
|---|---|---|---|---|---|---|---|
| Domain owns business data and invariants | ✓ | — | ✓ | ✓ | ✓ | ✓ | ✓ |
| Composition/profile never grants authority | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Data Workspace is the security boundary | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Registry is metadata-only | ✓ | — | — | ✓ | ✓ | ✓ | ✓ |
| First-party static registration only in initial scope | ✓ | — | — | ✓ | ✓ | ✓ | — |
| Versioned contracts and fail-closed compatibility | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| ADR-019 Operation semantics preserved | ✓ | — | — | ✓ | — | ✓ | ✓ |
| Agent output is untrusted | — | — | — | ✓ | — | — | ✓ |
| Human approval does not expand grants | — | ✓ | ✓ | ✓ | ✓ | — | ✓ |
| Learning & Growth remains first-class | — | ✓ | ✓ | — | ✓ | — | — |

## 5. 已消除的关键歧义

### 5.1 Plugin 的多个状态被混为一谈

R1 将 registration、availability、composition preference、presentation visibility 分开。否则 UI 隐藏可能被误解为权限撤销，或插件停用被误解为删除数据。

### 5.2 Role Profile 被误用为安全角色

R1 明确 Profile 不承载授权、密钥或审批。即使 Profile 配置某插件，命令调用仍必须经过当前后端授权。

### 5.3 Workspace 概念冲突

R1 使用 `WorkspaceComposition` 表示体验布局，保留既有 Data Workspace 的数据和安全语义。组合配置不得被用作隔离控制。

### 5.4 Capability Registry 演变为 Service Locator

R1 限定 Registry 只解析 descriptor/version；实际执行必须进入领域拥有的 use case，并经 Application Governance & Coordination。

### 5.5 Agent 自报结果、审批过期和计划漂移

R1 把 Agent 输出视为不可信输入；审批绑定规范化计划摘要和精确目标/范围；每个副作用前重新授权；不确定结果进入 OutcomeUnknown，不盲目重试。

### 5.6 插件停用/升级造成 Operation 悬挂

R1 明确必须遵循 ADR-019 的 drain/reconcile 语义，不能静默取消或删除历史记录。

### 5.7 Learning 被工具化或挤入 Core

R1 明确保留 Learning & Growth 的实体所有权、证据和 Assessment 语义，并禁止因插件化扩张 Alpha 交付范围。

## 6. 残余问题与后续规格工作

以下项目刻意不在 ADR 层面过早冻结，需在人工批准后由实现规格任务处理：

1. Plugin descriptor 的序列化格式、schema registry 和字段兼容工具。
2. 具体 Rust trait/type、crate 内模块目录、贡献点 API 和静态注册宏/机制。
3. Capability canonicalization 的确定性编码规则、摘要算法和跨版本测试向量。
4. Profile 与 WorkspaceComposition 的 SQLite 表、索引、迁移、备份与恢复设计。
5. Plugin 依赖状态传播、可用性诊断和启动故障隔离策略。
6. Approval 一次性/多次执行策略、撤销传播和并发审批语义。
7. 细化风险分类、人工审批 UI、审计保留期与敏感字段分级。
8. Profile/Composition/Plugin 具体进入哪个产品里程碑，需由 roadmap owner 明确，不由本组 ADR 自动决定。
9. 第三方插件、动态加载、团队共享 Profile、组织级 RBAC 均属于后续独立决策。

## 7. 冲突与阻断项

本轮未发现需要推翻现有 ADR-001～022 的结构性冲突。以下情况属于实现阻断条件：

- 任一实现把 Role Profile 或 WorkspaceComposition 作为授权来源。
- 任一插件/Registry/Core 模块直接写入其他领域的持久化表。
- Agent 能绕过 Capability Governance 直接访问数据库、文件系统或任意命令。
- 审批不绑定精确计划/目标/范围，或只在 Operation 创建时做一次授权检查。
- 不确定的非幂等外部副作用被自动盲重试。
- 插件 disable/remove 直接删除领域数据或取消未完成 Operation。
- 插件化导致 Learning & Growth 领域范围或 Alpha 里程碑未经审批扩张。

## 8. 结论与审批建议

**结论：R1 文档包可进入人工决策审查，不可直接进入代码实施。** 建议按 023→024→025→026→027→028→029 的顺序逐份批准；ADR-028/029 的审批应以 023/026 的边界先获得认可为前提。若人工审批修改任何核心不变量，应先修订本组 ADR 和本报告，再创建实施任务。

人工审批必须明确：每份 ADR 的 Accepted/Rejected/Revise 状态、决策人、决策日期、附加条件，以及是否授权进入下一阶段实现规格。未明确批准的 ADR 继续保持 Proposed。

## 9. 决策记录（2026-11-12 追加）

仓库 owner 于 2026-11-12 作出 H0 决策：**ADR-023/024/025/027/028 与 ADR-026/029 全部批准为 Accepted**，条件为本报告 §3/§7 的边界与不变量为规范性约束（逐份决策记录见 [owner-review-checklist](../architecture/adr-023-029-hermes-task-package/owner-review-checklist.md)）。

随之生效的门禁状态：

1. **E5 实现规格**：获授权将 [E5 实现规格草案 R0](../05-architecture-evolution/E5-implementation-specification.md) 定稿为正式实现规格（ADR-026/029 为其治理基线）；PAW-INF-005/006 转 approved。
2. **E7 实现规格**：未授权；E7（阶段归属见 roadmap v0.5.0）的实现规格与实现开工另行开闸。
3. **实现开工**：未授权；E5 实现开工是独立 owner gate。
4. 各 ADR 的 Implementation / Verification 状态保持 Not Implemented / Not Verified；Accepted 不改变实现与验证事实。

> **后续状态更新（2026-11-12 之后）：** E5 交付并合入 `main` 后，ADR-026 / ADR-029 转为 **Implemented / Partially Verified**（自动证据与缺口见 [E5 工程证据报告](E5-engineering-evidence-report.md)）；ADR-023 / 024 / 025 / 027 / 028 仍为 Not Implemented / Not Verified（E7 范围）。本节 H0 决策记录保持原样，不追溯改写。
