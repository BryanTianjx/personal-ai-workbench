# Architecture Decision Records

本目录记录影响产品边界、技术架构、数据、安全和集成方式的关键决策。

## 状态定义

- Proposed：提议中，尚未批准。
- Accepted：决策已批准并作为当前设计基线；不代表已经实现或验证。
- Superseded：已被后续 ADR 取代，保留历史记录。
- Deprecated：不再推荐，保留历史记录。

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

ADR-007～012 未在当前分支中发现，不因本次新增 ADR 而补造或重新编号；如后续需要，应先核对已有决策来源和历史记录。

## ADR 记录规则

每份 ADR 至少包含背景、决策驱动因素、候选方案（如适用）、决策、后果、显式风险与缓解措施、验证要求、决策状态、实施状态和验证状态。ADR 状态只表示决策状态；实现和验证状态必须单独记录。改变已接受的架构决策时，新增 ADR，不覆盖历史决策。