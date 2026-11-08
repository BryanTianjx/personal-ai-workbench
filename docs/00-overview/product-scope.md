# 产品范围与边界

**状态：** Accepted planning baseline · **版本：** 0.2.0

## 产品定位

自磨工坊是面向个人的**工作与成长平台（Personal Work & Growth Platform）**。它统一组织目标、领域责任、项目、任务、学习、知识上下文、AI 协作、工程活动与内容创作。专业工具负责专业执行；工作台负责组织、关联、授权、追踪和复盘。

## 业务工作域

| 工作域 | 职责 | 首期范围 |
|---|---|---|
| Work Management | Goal、Area、Project、Task | Alpha 0.1 核心闭环 |
| Learning & Growth | Roadmap、Plan、Item、Session、Skill、Evidence、Assessment | Alpha 0.1 预留；Alpha 0.2 起分阶段交付 |
| Knowledge & Context | 本地笔记、知识引用、上下文关联 | Alpha 0.1 本地文件/搜索；外部引用 Alpha 0.2 |
| AI Collaboration | Agent 注册、能力、执行记录 | Alpha 0.1 定义边界；Alpha 0.2 受控启用 |
| Engineering | 仓库、工程活动、Artifact 引用 | 首期项目入口与扩展预留 |
| Creator | 内容选题、草稿、版本、审核、发布记录 | Beta 0.5 |
| Automation | 触发器、Workflow、执行记录 | 后续分阶段 |
| System & Security | 设置、策略、审计、备份恢复 | Alpha 0.1 基础能力 |

Dashboard 是跨域只读汇总，不作为独立业务数据所有者。

## Alpha 0.1 必须交付

Windows 和 macOS 桌面应用可构建、安装、启动；本地 Workspace 初始化；Goal/Area/Project/Task 管理；真实数据 Dashboard；项目/任务本地搜索；附件元数据和受控文件访问；SQLite 版本化迁移；数据库与附件备份恢复；关键操作审计；完全断网下核心工作闭环。Learning & Growth 只保留领域边界、稳定 ID/关联扩展点，不交付完整学习中心。

## 后续扩展

Alpha 0.2：Agent 注册/受控连接、KnowledgeRef、Learning Roadmap/Plan/Item/Session。Beta 0.5：Skill Matrix、Evidence、Assessment、项目驱动学习闭环及 Creator 基础。V1.0：跨域工作与成长闭环。后续版本：Web 服务、多设备同步、可视化工作流和多 Agent 协作。

## 非目标

不要求 Docker 或 PostgreSQL 才能运行桌面端；不将远程 AI API 作为核心数据管理依赖；不自动上传本地文件；不默认授予 Agent 任意文件系统或命令执行权限；不复制整个 Lingshu/Hindsight 知识库；不在缺少官方接口验证时宣称支持公众号自动发布；不把 Workbench 定位成完整 LMS。

## 范围变更

新增功能必须说明用户价值、依赖、数据影响、安全影响、平台影响、版本归属和验收标准。跨领域架构变化应新增 ADR。
