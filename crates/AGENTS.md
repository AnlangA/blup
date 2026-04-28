# AGENTS.md

## 1. 目标

`crates` 存放 Rust 核心库，是项目的 Agent Runtime、业务状态机、插件宿主、数据模型、工具调度和协议实现所在地。

### 1.1 Phase 1 范围

**Phase 1 交付：**
- `agent-core`：单一 crate，包含所有核心逻辑
  - HTTP 服务（Axum）
  - 学习流程状态机（目标判断 → 画像采集 → 路径规划 → 章节教学）
  - LLM 调用封装（OpenAI-compatible API）
  - Prompt 模板加载
  - Schema 验证

**Phase 1 不交付：**
- `storage`（数据持久化）
- `plugin-host`（插件系统）
- `assessment-engine`（评估引擎）
- `tool-router`（工具路由）
- `llm-gateway`（LLM 网关）
- `bevy-protocol`（Bevy 协议）

## 2. 目标实现的路径

**Phase 1：单一 Crate**
- `agent-core`：包含所有核心逻辑（HTTP 服务、状态机、LLM 调用、prompt 加载）
- 使用 Rust 类型系统表达核心领域模型
- 使用状态机表达学习流程：目标输入 → 可行性判断 → 用户画像 → 路径规划 → 章节学习
- Core 只编排，不直接渲染，不直接执行不可信代码

**Phase 2/3：Crate 拆分计划**
- `storage`：数据持久化（SQLite/PostgreSQL）
- `assessment-engine`：练习题生成、答案评估
- `plugin-host`：WASM 插件加载、权限管理
- `tool-router`：工具调度、沙箱请求路由
- `llm-gateway`：LLM 调用封装、重试、缓存
- `bevy-protocol`：Bevy 场景协议实现

**实现原则：**
- 使用 trait 和 schema 定义模块边界
- 状态机驱动流程，确保可追踪和可回放
- 错误处理、权限检查、日志在 Core 层统一处理

## 3. 需要联网查找/参考的资料与核心思想

需要查找：

- Tokio、Serde、SQLx、thiserror、tracing 等 Rust 生态资料。
- async-openai 或 OpenAI-compatible client 文档。
- Rust 状态机建模实践。
- Rust trait object、dynamic dispatch、workspace crate 划分实践。
- Wasmtime host integration 文档。

核心思想：

- Rust Core 是系统的可信主控层。
- LLM、插件、沙箱、渲染器都应被视为外部能力，通过明确接口调用。
- 错误处理、权限检查、日志、安全边界必须在 Core 层统一处理。

## 4. 不允许做什么事情

**全局约束请参考根文档第 6.1 节。**

**模块特有约束：**
- **[Module]** 不允许 Core 直接包含 UI 组件代码。
- **[Module]** 不允许 Core 直接运行用户代码。
- **[Module]** 不允许绕过权限系统调用插件或沙箱。
- **[Module]** 不允许把 LLM 返回内容当作可信事实直接写入最终结果。
- **[Module]** 不允许使用未验证的字符串协议替代结构化类型。

## 5. 相关文档

- [根文档 AGENTS.md](../AGENTS.md) - 项目整体规划和 crate 划分方案
- [schemas/AGENTS.md](../schemas/AGENTS.md) - 协议定义，本模块依赖的数据结构
- [prompts/AGENTS.md](../prompts/AGENTS.md) - Prompt 模板，由本模块加载和使用
