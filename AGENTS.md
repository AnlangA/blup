# AGENTS.md

## 1. 目标

本项目是一个 AI 交互式学习 Agent 平台。用户输入学习目标后，系统先判断目标是否可实现；如果不可实现，则给出原因和调整建议；如果可实现，则采集用户画像，生成个性化学习路径，并按章节提供资料、互动内容、练习、考核和反馈。

长期架构：Tauri 桌面应用 + Web UI + Rust Agent Core + WASM 插件系统 + Bevy 渲染层 + 沙箱化真实计算/编译环境。

## 2. 核心思想

- LLM 负责解释、规划和对话，不负责假装执行确定性任务。
- 确定性任务必须交给真实工具：数学由计算引擎完成，代码由编译/运行沙箱完成。
- Bevy 只做渲染和交互场景，不承担传统 UI 或 Agent 主逻辑。
- 插件通过结构化协议和宿主通信，不能直接穿透系统边界。
- 所有学习内容、题目、场景、评估结果都应尽量结构化，便于验证、复用和回放。
- 协议优先，模块解耦。`schemas/` 是一切模块的通信基础。

## 3. 分阶段路线图

| 阶段 | 目标 | 涉及目录 | 交付时间参考 |
|------|------|----------|-------------|
| **Phase 1: MVP** | 单用户 Web 对话学习助手：目标判断 → 用户画像 → 学习路径 → 章节对话教学 | `schemas/` `crates/agent-core` `prompts/` `apps/web-ui` | 优先 |
| **Phase 2: 强化** | 练习/考核引擎、代码沙箱（Docker）、课程数据持久化、进度追踪 | `sandboxes/` `crates/assessment-engine` `crates/storage` `tests/` | Phase 1 完成后 |
| **Phase 3: 扩展** | Tauri 桌面端、WASM 插件系统、Bevy 互动场景、多领域插件 | `apps/desktop` `plugins/` `apps/bevy-viewer` `tools/` `assets/` | Phase 2 稳定后 |

**迭代原则**：每一阶段必须交付一个可独立运行、可演示、可被用户使用的完整产品。不允许只交付 "框架" 或 "基础设施"。

## 4. Phase 1 MVP 精确定义

### 4.1 交付物

| 目录 | 交付内容 |
|------|---------|
| `schemas/` | JSON Schema 定义：LearningGoal、FeasibilityResult、UserProfile、CurriculumPlan、Chapter、Message、ChapterProgress |
| `crates/agent-core` | Rust HTTP 服务（Axum），包含：目标可行性判断、用户画像采集对话、学习路径规划、章节教学对话。LLM 通过 OpenAI-compatible API 调用，prompt 从独立模板文件加载 |
| `prompts/` | 每个核心步骤对应的 prompt 模板，要求结构化输出 |
| `apps/web-ui` | React 或 Svelte 单页应用，包含：聊天窗口、课程目录侧栏、章节内容区（Markdown + KaTeX + Monaco Editor）、简单状态路由 |

### 4.2 技术栈

- **前端**：React 18+（或 Svelte 5），Vite，Monaco Editor，KaTeX，react-markdown。
- **后端**：Rust，Axum，Tokio，Serde，reqwest（LLM HTTP client），tracing。
- **协议**：REST + SSE（流式），请求/响应体为 JSON Schema 校验。
- **运行方式**：`cargo run` 启动后端，`npm run dev` 启动前端，浏览器访问 `localhost:5173`。

### 4.3 明确不包含的内容

- **不包含** Bevy、Tauri、WASM 插件、Docker 沙箱、数据库持久化（可用内存或 JSON 文件替代）。
- **不包含** 用户认证、多用户、支付、国际化。
- **不包含** 真实代码编译执行、数学计算引擎（LLM 输出数学解释文本即可，不假装计算）。

### 4.4 端到端用户故事

> 用户打开 Web 页面 → 在聊天框输入 "我想学 Python 数据分析" → 系统调用可行性判断 prompt，LLM 返回可行性结论和调整建议 → 用户确认后，系统进入画像采集对话（3-5 轮问答：编程经验、数学基础、可用时间、学习风格）→ 系统生成个性化学习路径（如 5 章：Python 基础、NumPy、Pandas、数据可视化、实战项目）→ 用户在左侧目录看到章节列表 → 点击第一章进入学习 → 系统按章节输出教学内容（Markdown + 代码示例）→ 用户可与 Agent 对话提问 → 完成第一章后标记进度。

### 4.5 Phase 1 构建顺序

```
schemas/  →  crates/agent-core  →  prompts/  →  apps/web-ui
```

1. 先定义所有协议 schema（JSON Schema 文件），作为各模块的契约。
2. 实现 agent-core（Rust 结构体、状态机、LLM 调用、HTTP API），schema 和 prompt 模板通过文件路径引用。
3. 编写 prompt 模板，配置到 agent-core 中。
4. 实现 Web UI，通过 HTTP + SSE 与 agent-core 通信。

## 5. 模块依赖关系图

```
                    schemas/         ← 协议定义层（所有模块的公共依赖）
                 ┌──────┴──────┐
          crates/agent-core     ← 核心编排层
          ┌────────┼────────┐
     prompts/   apps/web-ui  ← Phase 1 交付
                            
[Phase 2 扩展]                  
          ┌────────┼────────┐
  crates/storage  sandboxes/  crates/assessment-engine  tests/

[Phase 3 扩展]
          ┌────────┼────────┬────────┬────────┐
   apps/desktop  plugins/  apps/bevy-viewer  tools/  assets/
```

- **schemas/** ：无依赖，最先完成。
- **crates/agent-core**：仅依赖 schemas。
- **prompts/**：逻辑上依赖 schemas 的输出结构定义。
- **apps/web-ui**：依赖 agent-core 的 HTTP API 契约（非代码依赖）。
- **sandboxes/**：依赖 schemas 中的 ToolRequest/EvaluationResult 和 agent-core 的工具调度接口。
- **plugins/**：依赖 schemas 和 agent-core 的插件宿主接口。

## 6. 约束与禁止事项

### 6.1 全阶段适用（硬约束）

- 不允许把所有逻辑塞进前端。
- 不允许让 LLM 伪造计算结果、编译结果或测试结果。
- 不允许把用户隐私、学习记录、API Key 写入日志或提交到仓库。
- 不允许在没有沙箱的情况下运行用户提交的代码。（Phase 1 不运行用户代码，Phase 2 起必须通过沙箱）
- 不允许在缺少权限边界的情况下加载第三方插件。（Phase 3 起适用）
- 不允许插件直接执行宿主系统命令。
- 不允许 Bevy 直接承担完整学习产品 UI。

### 6.2 Phase 1 放宽约束

Phase 1 以快速验证核心流程为目标，以下行为**允许**：

- 允许创建探索性 demo、原型和验证代码（验证后归档到 `docs-internal/experiments/` 或删除）。
- 允许使用内存存储或 JSON 文件替代数据库。
- 允许 agent-core 和 web-ui 运行在同一台机器上，不做多租户隔离。
- 允许 prompt 模板在调试阶段内联在代码中，但必须标注 `TODO: extract to prompts/`。

### 6.3 各阶段特有约束

| 阶段 | 禁止事项 |
|------|---------|
| Phase 1 | 禁止引入 Bevy、Tauri、WASM、Docker 相关依赖。禁止在 UI 层直接调用 LLM API。 |
| Phase 2 | 禁止在宿主机裸跑用户代码。禁止沙箱默认联网。禁止没有超时和资源限制地运行任务。 |
| Phase 3 | 禁止插件绕过 Core 生成最终学习进度。禁止插件直接访问用户文件、网络、密钥或数据库。 |

## 7. 参考资源

### 7.1 Phase 1 参考

- OpenAI-compatible API：流式输出、工具调用、结构化输出文档。
- Rust async 生态：Tokio、Axum、Serde 文档。
- Monaco Editor、KaTeX、Markdown 渲染资料。
- 教育学：掌握学习、形成性评价、脚手架式教学。

### 7.2 Phase 2 参考

- Docker sandbox 安全实践。
- SQLx 文档。
- Judge0 或在线评测系统架构。
- Python SymPy、NumPy、SciPy 资料。

### 7.3 Phase 3 参考

- Tauri 2 官方文档：窗口、命令、权限、IPC、安全模型。
- Bevy 官方文档：ECS、渲染、WASM、窗口管理、输入事件。
- Wasmtime 与 WASM Component Model 文档：插件隔离、接口定义、权限控制。
- WIT 接口定义。
- WASI 权限模型和 Firecracker microVM 文档。
- 像素风素材制作、glTF/PNG/WebP 资产格式。
