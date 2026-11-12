# E7 — Implementation Specification: Platform Composition (Domain Plugins, Role Profiles, Workspace Compositions)

- **Status:** Finalized R0 (2026-11-12)
- **Decision status:** 治理 ADR [ADR-023](../03-adr/ADR-023-domain-plugin-model.md) / [ADR-024](../03-adr/ADR-024-role-profile-model.md) / [ADR-025](../03-adr/ADR-025-workspace-composition.md) / [ADR-027](../03-adr/ADR-027-first-party-domain-plugin-policy.md) / [ADR-028](../03-adr/ADR-028-plugin-dependency-compatibility-governance.md) 已于 2026-11-12 批准为 Accepted（R1 条件为规范性约束）；本规格为其实现规格
- **Implementation status:** Not Implemented（规格定稿与实现按 owner"按计划推进"授权合并执行；各工作包以提交为证据）
- **Verification status:** Not Verified → 每工作包以自动化测试交付
- **Baseline:** roadmap v0.5.1（E7 = V1.0 前置）；ADR-017～020、ADR-026/029（E5 已交付的治理管线）；requirements baseline §4 PAW-INF-007～009

## 1. Purpose

将五份组合轴 ADR 落到四 crate 工程契约：插件描述符与注册表（静态一方）、四独立生命周期维度、依赖 DAG 治理、RoleProfile 体验模板、WorkspaceComposition 展示组合。硬边界：**组合/档案永不授权**——Data Workspace 与后端策略保持唯一安全边界（ADR-027 §R1；跨 ADR 评审 §7 阻断项）。

## 2. Stage scope and work packages

| WP | Deliverable | 主要落点 | 退出证据 |
|---|---|---|---|
| E7-01 | 插件描述符模型 + 静态注册表 + 依赖 DAG 校验 | paw-application（`ports::platform`：描述符、依赖图、四状态维度枚举、确定性拓扑排序、环拒绝） | 环拒绝、缺失必需依赖阻断、可选依赖降级声明、启动校验测试 |
| E7-02 | 插件生命周期持久化 + 停用语义 | migration 0013（plugin_states、role_profiles、workspace_compositions + 链接表）+ infra 仓储 | 停用阻断新调用（能力门禁接入）、数据/历史/进行中 Operation 保留（ADR-019 drain）、跨工作区拒绝 |
| E7-03 | RoleProfile 体验模板 | 领域类型 + 仓储 + 导入剥离安全字段 | 模板不含 grant/凭据/审批；切换不改授权；损坏/过期模板安全回退 |
| E7-04 | WorkspaceComposition + 桥 + UI + e2e | 组合配置 CRUD + desktop 命令 + UI 设置区 + e2e | 组合仅存引用/展示配置；按 Data Workspace 解析；损坏 schema 回退不改业务数据 |

## 3. Normative principles

1. **四状态维度独立**（ADR-023 R1）：registration（启动时静态注册）/ availability（依赖+兼容+策略）/ composition preference（某 WorkspaceComposition 是否选用）/ presentation visibility（当前 UI 是否展示）——禁止单一 `enabled` 布尔。
2. **Registry 仅元数据**：解析描述符/版本，不返回可调用对象；派发仍走 E5 治理管线（Capability Registry + Application Coordination）。
3. **版本维度分离**（ADR-028 §4.2）：plugin_version / descriptor_schema_version / capability_contract_version / data_migration_version / minimum_app_version 独立校验，fail closed。
4. **Profile/Composition 不承载授权**：不含 grant、凭据、审批令牌、可执行代码；导入按不可信配置校验并剥离安全字段；UI 可见性不是权限。
5. **停用不破坏**：停用插件隐藏贡献、阻断新调用，但不得删数据、不改历史、不静默取消进行中 Operation（ADR-019 drain/reconcile 语义由 E5 管线承担）。
6. 四 crate 边界与 ADR-018 allowlist 不变；无第五 crate、无动态加载。

## 4. Capability plugins — descriptor and lifecycle (E7-01/E7-02)

```rust
pub struct PluginDescriptor {
    pub id: &'static str,                      // "core.work_management" 等
    pub plugin_version: (u32, u32, u32),
    pub descriptor_schema_version: u32,        // 本规格 = 1
    pub required_dependencies: Vec<PluginDependency>,
    pub optional_dependencies: Vec<PluginDependency>,
    pub owned_capabilities: Vec<&'static str>, // E5 registry 的 capability id
    pub migration_namespace: &'static str,
}
pub struct PluginDependency { pub id: &'static str, pub min_version: (u32, u32, u32) }
```

- 静态一方插件集（ADR-027 §R1 映射）：core.work_management、core.learning_growth、core.knowledge_context、core.ai_collaboration、core.system_security——增量注册，无大爆炸重写。
- `PluginRegistry::register_all` 启动时校验：id 冲突、依赖存在、版本满足、**环拒绝**（确定性拓扑排序，按 id 稳定打破平局）；失败组件隔离并报告精确环。
- 运行时状态（migration 0013 `plugin_states`，用户偏好持久化）：`plugin_id PK, disabled INTEGER, disabled_at, updated_at`——availability 由 registry 校验结果推导，composition preference 存于组合表，visibility 由 UI 推导。
- **停用语义**：`disabled` 插件的 owned capabilities 在 E5 描述符解析处拒绝（`PluginGate` 检查加在 `propose_agent_invocation` 的 registry 解析前）；领域数据、审计、进行中 Operation 原样保留（drain/reconcile 按 ADR-019）。

## 5. RoleProfile — experience template only (E7-03)

```rust
pub struct RoleProfile {
    pub id: ProfileId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub plugin_preferences: Vec<String>,   // 插件 id 引用，非授权
    pub dashboard_layout: Option<String>,  // 展示配置（声明式，非代码）
    pub agent_profile_refs: Vec<String>,   // Agent 注册引用
}
```

- 无 grant、无凭据、无审批令牌、无可执行代码（导入校验剥离 + 测试断言）。
- 切换 profile 不改变有效授权（直接后端调用拒绝测试）；损坏/未知 schema/缺失引用 → 安全回退默认组合，不改业务数据。
- migration 0013 `role_profiles`（UNIQUE (id, workspace_id) 复合键纪律）。

## 6. WorkspaceComposition — presentation aggregate (E7-04)

```rust
pub struct WorkspaceComposition {
    pub id: CompositionId,
    pub workspace_id: WorkspaceId,        // 限定单一 Data Workspace
    pub actor_id: String,
    pub name: String,
    pub profile_ref: Option<ProfileId>,   // 可选模板引用
    pub plugin_selection: Vec<String>,    // 偏好顺序，非授权
    pub navigation_config: Option<String>, // 声明式导航引用
}
```

- 使用时后端仍按 Data Workspace + 当前策略授权（E5 管线不变）；组合过滤不是隔离控制。
- 损坏/未知/未授权组合 → 安全默认回退，不改业务数据（测试覆盖）。
- migration 0013 `workspace_compositions`（UNIQUE (id, workspace_id)）。

## 7. Required invariants and conformance tests

映射 ADR-023/024/025/027/028 Validation required 与跨 ADR 评审 §7：

1. 依赖缺失 → 依赖插件 blocked，能力不可调用；可选依赖缺失 → 仅声明过的安全降级。
2. 环 → 确定性拒绝 + 精确报告；不激活部分不安全组件。
3. 四维度独立测试（注册≠可用≠偏好≠可见）。
4. 停用插件：新调用拒绝；数据/审计/Operation 历史保留且可对账。
5. Profile/Composition 携带 grant/凭据字段的导入被剥离或拒绝；切换后直接后端调用授权不变。
6. 跨工作区组合读写拒绝；损坏 schema 安全回退。
7. Registry/Core 无属主域数据表写入（架构测试）。
8. Learning 一等领域不被吸收（ADR-027）；插件化不改变 Alpha/Beta 既定交付范围。

## 8. Preconditions and gates

| 门禁 | 内容 |
|---|---|
| 规格前置 | ✅ ADR-023/024/025/027/028 Accepted（2026-11-12）；本规格 R0 定稿 |
| 实现前置 | owner"按计划推进"授权（2026-11-12 会话指示，决策采用推荐项） |
| E7 退出证据 | §7 一致性矩阵全绿 + 证据报告入 docs/04-quality；Gate V1.0（E7 + 收口）为最终门禁 |
| Profile/Composition 进入 UI | 组合/档案持久化交付后即呈现（设置区），不新增产品范围（ADR-027 不扩张原则） |

## 9. Traceability

- PAW-INF-007（ADR-023/027/028）→ §4/§7；PAW-INF-008（ADR-024）→ §5；PAW-INF-009（ADR-025）→ §6；ADR-017/019 → §4 停用语义；ADR-026/029（E5 管线）→ §4 能力门禁接入。
