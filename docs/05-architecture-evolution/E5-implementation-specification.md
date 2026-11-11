# E5 — Implementation Specification: Governed Agent Adapter, Capability Invocation and Approval Binding

- **Status:** Finalized R1 (2026-11-12，由 Draft R0 定稿；owner 授权记录见 [跨 ADR 评审报告 §9](../04-quality/ADR-023-029-R1-cross-adr-review.md))
- **Decision status:** 治理 ADR [ADR-026](../03-adr/ADR-026-capability-registry-and-ai-invocation.md) / [ADR-029](../03-adr/ADR-029-agent-invocation-security-and-approval.md) 已于 2026-11-12 批准为 Accepted（R1 条件为规范性约束）；本规格为其实现规格
- **Implementation status:** Not Implemented（实现开工是独立 owner gate，本规格本身不授权任何代码、schema 或配置变更）
- **Verification status:** Not Verified
- **Baseline:** roadmap v0.5.0（E5 设计基线 = ADR-026/029）；`agent-authorization-model.md`；`agent-adapter-architecture.md`；ADR-004/017/018/019/020/022；`E0.3.2-engineering-specification.md` §2/§3；requirements baseline §4 PAW-INF-005/006、PAW-S-004

## 1. Purpose

将 ADR-026/029 的治理要求落到四 crate 工程契约：能力描述符与注册表、授权决策 v2、规范化调用计划与摘要、审批绑定、Agent 注册/执行生命周期、不可信输出处理与审计脱敏。E5 的退出证据沿用 E0.3 §4：**fake-provider conformance、approval binding、unknown-outcome reconciliation**。外部 Agent（Hermes 等）的真实启用另以 [ADR-022](../03-adr/ADR-022-workbench-hermes-boundary.md) 批准为前置，不属于 E5 交付。

## 2. Stage scope and work packages

| WP | Deliverable | 主要落点 | 退出证据 |
|---|---|---|---|
| E5-01 | Capability descriptor 模型 + 静态注册表（仅元数据） | paw-application（描述符类型、注册表、契约版本） | 描述符 schema 校验、重复 ID/未知版本拒绝、registry 不返回可调用实现的架构测试 |
| E5-02 | 授权 v2：显式 grants、范围求交、风险分级、规范化计划与摘要 | paw-application/ports::authorization 扩展 + paw-infrastructure/security + migration 0009（agent_grants） | 默认拒绝、范围求交拒绝、digest 确定性/突变敏感测试 |
| E5-03 | Agent 注册 + Agent Execution 生命周期 + fake adapter | paw-domain（ai_collaboration 模块）+ ports + infra 仓储 + migration 0009（agent_registrations、agent_executions） | 生命周期迁移校验、与 Operation 关联不互相覆盖状态、fake adapter 契约 conformance |
| E5-04 | 审批绑定服务 | paw-application + infra + migration 0009（approvals） | digest/主体/工作区/范围/目标/能力/工具/有效期绑定校验；过期/撤销/不匹配拒绝 |
| E5-05 | 调用管线 + 每副作用前重新授权 + OutcomeUnknown 对账 + 审计脱敏 | paw-application（协调路径）+ operations 集成 | 授权撤销后第二个副作用被阻断；非幂等不确定结果不盲重试；脱敏测试 |
| E5-06 | Desktop bridge + UI（Agent 区、审批对话框）+ 集成测试 + 证据报告 | paw-desktop commands/UI + docs/04-quality/E5-engineering-evidence-report.md | 全部新命令 gated+audited；UI 审批显示目标/动作/范围/数据流/风险/有效期 |

## 3. Normative principles

1. 描述符、RoleProfile、组合偏好、Provider 侧权限都是**元数据或建议，不是 grant**；有效授权只由后端策略在每次受保护副作用前求值（ADR-026 R1）。
2. Agent 计划、工具参数、provider 消息与结果均为不可信输入：先 schema 校验，再解析主体/工作区/目标，再授权（ADR-029 §4.1）。缺失/畸形/过期上下文一律拒绝。
3. 授权、审批策略、Operation 生命周期、correlation/causation、Outbox、恢复归 Application Coordination；能力执行归属主域 use case；provider 协议翻译归 AI Adapter（五层权属，ADR-026 R1）。
4. Registry 只解析描述符/版本，返回解析结果而非任意 callable；不成为 Service Locator 或执行容器。
5. 非幂等动作在结果不确定时不得重试；`OutcomeUnknown` 保持可见直到显式对账（ADR-019，E3 已实现的状态机原样复用）。
6. 秘密值不进入 prompt、审批展示、普通日志或审计行（脱敏由构造保证 + 测试断言）。
7. 四 crate 边界与 ADR-018 allowlist 不变；不引入第五 crate、不引入独立 Agent Runtime、不复制 provider 语义。

## 4. Capability descriptors and registry (E5-01)

描述符是**静态一方声明**，在应用启动时于 paw-application 内注册，E5 不建 SQL 表、不做动态加载：

```rust
pub struct CapabilityDescriptor {
    pub id: &'static str,                 // 如 "work_management.task.create"
    pub owner_domain: DomainId,           // 枚举，不是字符串自由量
    pub contract_version: ContractVersion,// 语义化版本，ADR-020 口径
    pub input_schema: SchemaRef,          // 静态 schema 引用（类型级，非运行时 JSON Schema）
    pub output_schema: SchemaRef,
    pub required_scope: ScopeRequirement, // 声明性元数据：需要何种 grant 维度
    pub risk_class: RiskClass,            // Low / Medium / High（High ⇒ 审批必选）
    pub idempotency: Idempotency,         // Idempotent / NonIdempotent / Unknown
    pub offline_behavior: OfflinePolicy,  // LocalOnly / Deferred / Unavailable
    pub approval_hint: ApprovalHint,      // None / Required / Conditional(rule)
}
```

- Registry：`HashMap<CapabilityId, VersionedDescriptor>` + `resolve(id, version_range) -> Resolution`（Supported / UnsupportedVersion / UnknownCapability / DuplicateRegistration 阻断启动）。返回 `Resolution`，**不返回可调用对象**；派发始终走属主域 use case 经协调层。
- 兼容规则按 ADR-028 §4.3 编码为测试：加性可选字段可兼容；删除/改名/收紧取值/改授权范围/改幂等语义 = breaking（fail closed）。
- 初期描述符集来自既有已 gate 的 Api 面（Work Management、Learning、Knowledge、Operations 命令），不新增业务能力。

## 5. Authorization v2 (E5-02)

现状（E1）：`ProtectedAction` 枚举 + `LocalSingleUserAuthorizer` 的 (actor, action) 允许对。E5 扩展为授权模型 §2 的 grant 模型，**语义不变：默认拒绝、范围求交、无隐式继承**。

- **Grant**（migration 0009 `agent_grants` 草图）：`id, subject_id, capabilities, workspace_id, project_id?, resource_roots, operation_set, tool_set, valid_from, valid_until?, created_by`。缺失/冲突/含糊 scope 一律拒绝；subject=`local-user` 或 `agent:<registration_id>`。
- **ProtectedAction 演进**：新增 `AgentInvoke { capability, workspace_id }` 等变体；`AuthorizationError` 保持非泄漏（仍只有 `Denied`，原因码走审计）。
- **决策管线**（授权模型 §3 顺序不变）：能力校验 → 主体/工作区解析 → 资源范围 → 策略求值（默认拒绝）→ 分派 Allowed / ApprovalRequired / Denied。风险类来自描述符（High ⇒ ApprovalRequired，即便 grant 允许）；四类"系统策略禁止"永远不可被 grant 或审批覆盖。
- **规范化调用计划与摘要**：确定性序列化 = 固定字段顺序 + map 键排序 + UTF-8 + 无空白 JSON 编码，`canonicalization_version` 字段随计划落库；摘要算法 SHA-256。测试必须对每个 material 字段做突变敏感断言（任一变化 ⇒ digest 变化），并锁定向量防算法漂移。

## 6. Approvals (E5-04)

`approvals` 表草图（字段即 ADR-029 §4.3 绑定集，一列不多一列不少）：

`id, canonical_digest, actor_id, agent_registration_id?, data_workspace_id, resource_scope_json, capability_id, capability_contract_version, tool_adapter, target_identity, material_params_digest, risk_class, side_effect_set_json, expires_at, attempt_policy(single|bounded(n)), consumed_attempts, status, created_at, decided_at?`

- **状态机**：`Active → Consumed | Expired | Revoked`（时间驱动过期；撤销为显式命令）。校验失败（digest/范围/目标不匹配）不改变状态，只拒绝当次执行。
- 幂等重试仅在 canonical action/target/scope/attempt policy 不变且 approval 仍 Active 时可复用（attempt_policy 消费计数校验）；任何 material 变更要求新授权 + 新审批。
- 审批展示必须含：目标、动作、受影响资源、material 数据流摘要、风险、有效期；秘密值脱敏，凭据仅显示 opaque 引用。
- 审批记录存引用与摘要，不存秘密或完整敏感 payload。

## 7. Agent registration and Agent Execution lifecycle (E5-03)

新领域模块 `paw_domain::ai_collaboration`（不触 provider SDK；纯类型 + 状态机，风格同 `operation_machine.rs`）：

- **AgentRegistration**（`agent_registrations`）：`id, display_name, adapter_kind(fake|cli_hermes|api_…), declared_capabilities, status(Active|Suspended|Retired), workspace_id`。注册/变更本身是受 gate 的命令。
- **AgentExecution 生命周期**（adapter-architecture §3 原样采纳，状态机进 domain）：
  `Created → Validating → AwaitingApproval → Approved → Submitted → Running → Completed | Failed | Cancelled | TimedOut | OutcomeUnknown`
- **与 Operation 的关联**（ADR-019 语义，不混用状态机）：每次治理调用开 `Operation`，`owner_domain='ai_collaboration'`, `owner_type='agent_execution'`, `owner_id=execution_id`；`agent_executions.operation_id` 反向引用。Operation 完成 ≠ Agent Execution 完成；两事实独立持久，互不推导。
- **Adapter 契约 port**（`ports/agent_adapter.rs`）：`capabilities() / health() / submit(req) / status(external_id) / cancel(external_id) / result(external_id)`，not-supported 返回显式枚举而非错误。E5 只交付 **FakeAgentAdapter**（进程内、可编排超时/断连/重复提交），Hermes/Codex/Cursor 适配器在 ADR-022 批准 + 版本化兼容测试之后另立任务。
- 重试分类沿用 operations.rs 既有 rejected/accepted/indeterminate 分类与退避；fake adapter 的断连场景映射 `OutcomeUnknown`。

## 8. Invocation pipeline (E5-05)

```text
Agent proposal(不可信)
 → parse + schema 校验                       [paw-desktop command 入口]
 → 解析 actor / Agent / Data Workspace / 目标  [paw-application]
 → resolve CapabilityDescriptor + 版本        [registry]
 → 授权求值（默认拒绝；grant ∩ scope ∩ 时间）   [authorization v2]
 → 风险分级 + 审批判定                        [descriptor risk_class]
 → 规范化计划 + digest                        [§5]
 → 校验审批绑定（如需）                       [approvals]
 → 创建/关联 durable Operation + attempt      [operations.rs]
 → 每个副作用前重新授权（撤销/过期/禁用 ⇒ 中断）
 → 经受治理的 adapter 派发                    [ports/agent_adapter]
 → 落 outcome / OutcomeUnknown                [operations + agent_executions]
 → 对账、审计、有界返回                       [reconciliation + audit]
```

- **每副作用前重新授权**实现在 attempt 循环内部：任何一次 recheck Denied ⇒ 当前 attempt 以 `Failed(process-category=authorization_revoked)` 终止，Operation 保持可查询，不回滚已完成的前序副作用。
- 输出校验：adapter 返回先过 schema/资源引用校验再持久化；workspace 谓词在每条回读查询上；工具结果不得注入指令进策略/审批系统（提示注入面 = 数据，永远不进决策路径）。

## 9. Desktop bridge and UI (E5-06)

- 新命令（全部经 `gate_and_audit`，denial 审计 + Allowed 记录）：`register_agent / suspend_agent`、`list_agent_executions`、`propose_agent_invocation`、`decide_approval`（approve/deny）、`revoke_approval`、`reconcile_agent_execution`。
- UI 新增 **Agent 协作**区：注册列表、执行列表（含 Operation 状态列）、审批对话框（§6 展示要素）、`OutcomeUnknown` 行的"对账"入口。审批对话框是唯一审批入口；UI 可见性永远不是授权。
- CommandError 新增 `agent_registration_not_found / approval_not_found / approval_not_active / capability_unsupported` 变体。

## 10. Migration 0009 sketch

四张表：`agent_grants`、`agent_registrations`、`agent_executions`、`approvals`。约束沿用既有纪律：workspace 一致复合键；执行历史对 registration 删除 `RESTRICT`（历史不静默级联）；approvals 对 executions 无级联；CHECK 约束字符串与 domain 状态机逐字一致（0002 先例）。实际 SQL 与 checksum 纪律（行尾不变式）在实现期定稿。

## 11. Required invariants and conformance tests

映射来源：ADR-026 R1 Validation additions、ADR-029 §7、授权模型 §9、adapter §7。实现期逐条落为 `#[test]`/集成测试：

1. 默认拒绝：缺失/畸形/过期上下文；未知能力；不支持版本。
2. 跨工作区/越范围调用经后端直接调用被拒（不是仅 UI 过滤）。
3. RoleProfile/组合/插件状态/provider 权限变化不改变有效授权（本阶段无 profile/composition 表，以 grant 与 policy 单测固化语义）。
4. digest 突变敏感：目标、工具、范围、material 参数、副作用集任一变化 ⇒ 拒绝复用审批。
5. 授权撤销发生在两个副作用之间 ⇒ 第二个被阻断。
6. 非幂等 + OutcomeUnknown ⇒ 不自动重试；对账命令显式落结果。
7. Agent Execution 状态不能覆盖 Operation 状态；反之亦然。
8. 审批不能提升 grant、不能覆盖系统策略禁止类。
9. 秘密脱敏：审批展示、日志、audit 行三处构造性断言。
10. fake adapter conformance：capabilities 精确性、malformed 输入、取消支持声明、超时/断连 ⇒ OutcomeUnknown、重复提交防护、部分结果。

## 12. Open questions carried to implementation

跨 ADR 审查 §6 明确留给实现规格的项，本规格的取舍如下（定稿复核于 2026-11-12，维持原取舍）：

1. **审批并发与撤销传播时点**（审查 §6.6）：E5 取"消费时校验 + 状态惰性过期"，撤销即时生效于下次 recheck；不做分布式撤销广播。
2. **风险目录枚举**：初版 RiskClass 仅三档，规则表随描述符静态声明；细化分级延后。
3. **审批记录保留期**：随 audit_days 策略，不短于其关联 Operation 的保留期。
4. **canonicalization 向量**：随 E5-02 测试固定，算法变更须 bump `canonicalization_version` 并提供迁移。

## 13. Preconditions and gates

| 门禁 | 内容 |
|---|---|
| 规格定稿前置 | ✅ 已满足：ADR-026 / ADR-029 于 2026-11-12 批准为 Accepted（[决策记录](../architecture/adr-023-029-hermes-task-package/owner-review-checklist.md)），本规格同日定稿 |
| 实现开工前置 | PAW-INF-005/006 已转 approved；**待 owner 实现开工授权**（独立于 ADR 批准的 gate） |
| 外部 Agent 启用前置 | ADR-022 批准 + 真实 provider 版本化兼容测试与操作手册（Gate Alpha 0.2 项） |
| E5 退出证据 | fake-provider conformance、approval binding、unknown-outcome reconciliation（E0.3 §4），证据报告入 docs/04-quality |

## 14. Traceability

- PAW-INF-005（ADR-026）→ §4/§8；PAW-INF-006（ADR-029）→ §5/§6/§8/§11；PAW-S-004（授权/审批/审计门禁）→ §5/§6/§9；ADR-019 语义复用 → §7/§8；ADR-020 版本化 → §4/§5；ADR-022 → §13。
