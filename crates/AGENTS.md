# AGENTS.md

## 1. 目标

`crates` 存放 Rust 核心库，是项目的 Agent Runtime、业务状态机、插件宿主、数据模型、工具调度和协议实现所在地。

## 2. 目标实现的路径

- 后续可拆分为 `agent-core`、`planner`、`lesson-engine`、`assessment-engine`、`plugin-host`、`tool-router`、`storage`、`llm-gateway`、`bevy-protocol`。
- 使用 Rust 类型系统表达核心领域模型。
- 使用状态机表达学习流程：目标输入、可行性判断、用户画像、路径规划、章节学习、练习、考核、补救、完成。
- 使用 trait 和 schema 定义插件、工具、存储、LLM Gateway 等边界。
- Core 只编排，不直接渲染，不直接执行不可信代码。

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

- 不允许 Core 直接包含 UI 组件代码。
- 不允许 Core 直接运行用户代码。
- 不允许绕过权限系统调用插件或沙箱。
- 不允许把 LLM 返回内容当作可信事实直接写入最终结果。
- 不允许使用未验证的字符串协议替代结构化类型。
