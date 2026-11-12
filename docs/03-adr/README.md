# Architecture Decision Records

本目录记录影响产品边界、技术架构、数据、安全和集成方式的关键决策。

## 状态定义

- Proposed：提议中，尚未批准。
- Accepted：决策已批准并作为当前设计基线；不代表已经实现或验证。
- Superseded：已被后续 ADR 取代，保留历史记录。
- Deprecated：不再推荐，保留历史记录。

ADR 状态、实现状态和验证状态必须分开记录。Accepted 不代表已实现或已验证。

## 决策索引

| ADR | 标题 | 状态 |
|---|---|---|
| [ADR-001](ADR-001-product-identity.md) | 品牌、产品与仓库身份分离 | Accepted |
| [ADR-002](ADR-002-modular-monolith.md) | Modular Monolith | Accepted |
| [ADR-003](ADR-003-local-data-ownership.md) | Local-First Data Ownership | Accepted |
| [ADR-004](ADR-004-agent-adapter-boundary.md) | Isolate Agent Providers Behind Adapters | Accepted |
| [ADR-005](ADR-005-offline-first-runtime.md) | Offline-First Desktop Runtime | Accepted |
| [ADR-006](ADR-006-sqlite-storage-topology.md) | Single SQLite Storage Topology | Accepted |
| [ADR-013](ADR-013-learning-growth-domain.md) | Learning & Growth as a First-Class Domain | Accepted |
| [ADR-014](ADR-014-learning-project-integration.md) | Learning and Project Integration Contract | Accepted |
| [ADR-015](ADR-015-skill-evidence-and-assessment.md) | Skill, Evidence, and Assessment Model | Accepted |
| [ADR-016](ADR-016-learning-knowledge-boundary.md) | Learning and Knowledge Ownership Boundary | Accepted |
| [ADR-017](ADR-017-coordination-layer.md) | Coordination Layer for Cross-Domain Operations | Accepted |
| [ADR-018](ADR-018-rust-workspace-boundaries.md) | Rust Workspace and Enforceable Module Boundaries | Accepted |
| [ADR-019](ADR-019-operation-lifecycle-reliability.md) | Durable Operation Lifecycle and Reliability Semantics | Accepted |
| [ADR-020](ADR-020-cross-domain-contract-versioning.md) | Cross-Domain Contract and Event Versioning | Accepted |
| [ADR-021](ADR-021-stage-version-mapping-windows-only-alpha.md) | Stage-to-Version Mapping and Windows-Only Alpha 0.1 Delivery | Accepted |
| [ADR-022](ADR-022-workbench-hermes-boundary.md) | Workbench ↔ Hermes Execution Boundary | Accepted |
| [ADR-023](ADR-023-domain-plugin-model.md) | Domain Plugin Model and Static Registration First | Accepted (R1) |
| [ADR-024](ADR-024-role-profile-model.md) | Role Profile Is a User Experience Profile, Not an Authorization Role | Accepted (R1) |
| [ADR-025](ADR-025-workspace-composition.md) | Workspace Composition Is Separate from the Data Workspace Boundary | Accepted (R1) |
| [ADR-026](ADR-026-capability-registry-and-ai-invocation.md) | Capability Registry and Governed AI Invocation | Accepted (R1) |
| [ADR-027](ADR-027-first-party-domain-plugin-policy.md) | First-Party Domain Plugin Policy and Learning & Growth Boundary | Accepted (R1) |
| [ADR-028](ADR-028-plugin-dependency-compatibility-governance.md) | Plugin Dependency and Compatibility Governance | Accepted (R1) |
| [ADR-029](ADR-029-agent-invocation-security-and-approval.md) | Agent Invocation Security and Human Approval Binding | Accepted (R1) |

ADR-007～012 未在当前基线中发现，不因新增 ADR 而补造或重新编号；如后续需要，应先核对历史决策来源和记录。

## ADR 记录规则

每份 ADR 至少包含背景、决策驱动因素、候选方案（如适用）、决策、后果、显式风险与缓解措施、验证要求、决策状态、实施状态和验证状态。改变已接受的架构决策时，新增 ADR，不覆盖历史记录。

E0.3.2 已于 2026-11-08 批准 ADR-017～020。ADR-022 已于 2026-11-12 由 owner 批准为 Accepted（条件：真实 provider 启用仍以版本化兼容测试与操作手册为前置，批准不等于实现）。**ADR-023～029 R1 已于 2026-11-12 由仓库 owner 逐份批准为 Accepted**（决策记录见 [owner-review-checklist](../architecture/adr-023-029-hermes-task-package/owner-review-checklist.md) 与 [跨 ADR 评审报告 §9](../04-quality/ADR-023-029-R1-cross-adr-review.md)；条件为 R1 评审报告 §3/§7 的边界与不变量约束）。Accepted 仅表示设计决策批准，实现与验证状态逐份记录在各 ADR 头部，不因批准而改变。E5 交付后：ADR-026 / ADR-029 为 **Implemented / Partially Verified**（自动证据见 [E5 工程证据报告](../04-quality/E5-engineering-evidence-report.md)；Gate E 窗口化运行与人工验收尚未完成）；ADR-023 / ADR-024 / ADR-025 / ADR-027 / ADR-028 仍为 **Not Implemented / Not Verified**（E7 阶段范围）。

## Review artifacts

- [ADR-023～029 R1 跨 ADR 一致性审查报告](../04-quality/ADR-023-029-R1-cross-adr-review.md)
- [Hermes 受控审查任务包](../architecture/adr-023-029-hermes-task-package/README.md)
