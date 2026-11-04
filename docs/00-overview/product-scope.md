# 产品范围与边界

**状态：** Draft baseline · **版本：** 0.1.0

## 产品组成
自磨工坊由桌面工作台、领域模块、外部集成适配器和本地数据设施组成。桌面应用是首期主要交付形态；独立 Web 服务属于后续版本，不是桌面端运行前提。

## 业务工作域
| 工作域 | 职责 | 首期范围 |
|---|---|---|
| Dashboard | 汇总重点工作和状态 | 真实项目、任务和系统状态 |
| Personal Workspace | Workspace、项目、任务 | 核心 CRUD、关联、归档 |
| AI Workspace | Agent 连接与执行管理 | Adapter 契约、连接状态、执行记录基础 |
| Knowledge Workspace | 知识入口与引用 | 引用元数据、外部服务状态 |
| Engineering Workspace | 工程项目入口 | 项目关联、Git 信息入口设计预留 |
| Creator Workspace | 公众号内容运营 | 选题、草稿、版本、审核基础模型 |
| Automation Workspace | 自动化 | 首期以可控任务记录和接口预留为主 |
| System & Security | 设置、安全、审计、数据维护 | 权限边界、日志、备份恢复 |

## Alpha 0.1 必须交付
Windows 和 macOS 桌面应用可构建、安装、启动；本地 Workspace 初始化；Project 和 Task 创建、读取、更新、归档/完成；真实数据 Dashboard；本地搜索；附件元数据和受控文件访问；SQLite 版本化迁移；数据库与附件备份恢复；关键操作审计；完全断网下核心工作闭环。

## 不作为 Alpha 0.1 阻断项
多设备同步、独立 Web Server、完整 Agent 团队编排、可视化工作流编辑器、自动化微信公众号发布、完整 DevOps 控制面、知识图谱和自主知识推理。

## 后续扩展
Alpha 0.2：Hermes 基础连接、Agent 执行状态、知识连接器。Beta 0.5：Content Studio 内容生产与审核、经验证的微信平台适配。V1.0：核心工作域协同。后续版本：Web 服务、多设备同步、可视化工作流和多 Agent 协作。

## 非目标
不要求 Docker 或 PostgreSQL 才能运行桌面端；不将远程 AI API 作为核心数据管理依赖；不自动上传本地文件；不默认授予 Agent 任意文件系统或命令执行权限；不在缺少官方接口验证时宣称支持公众号自动发布。

## 范围变更
新增功能必须说明用户价值、依赖、数据影响、安全影响、平台影响、版本归属和验收标准。跨领域架构变化应新增 ADR。