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

| 阶段 | 目标 | 涉及目录 | 预估时间 |
|------|------|----------|---------|
| **Phase 1: MVP** | 单用户 Web 对话学习助手：目标判断 → 用户画像 → 学习路径 → 章节对话教学 | `schemas/` `crates/agent-core` `prompts/` `apps/web-ui` | 4-6 周 |
| **Phase 2: 强化** | 练习/考核引擎、代码沙箱（Docker）、课程数据持久化、进度追踪 | `sandboxes/` `crates/assessment-engine` `crates/storage` `tests/` | 6-8 周 |
| **Phase 2.5: 桌面化** | Tauri 包裹已有 Web UI，验证桌面端可行性；Bevy 嵌入 PoC（共享纹理方案验证） | `apps/desktop` `crates/bevy-protocol` | 2-4 周 |
| **Phase 3: 扩展** | WASM 插件系统、Bevy 互动场景、多领域插件 | `plugins/` `apps/bevy-viewer` `tools/` `assets/` | 12-16 周 |

**迭代原则**：每一阶段必须交付一个可独立运行、可演示、可被用户使用的完整产品。不允许只交付 "框架" 或 "基础设施"。

> **Phase 2.5 说明**：Phase 2→3 跨度较大。Phase 2.5 作为承上启下的过渡阶段，先完成 Tauri 桌面化（不引入 Bevy）和 Bevy 嵌入技术验证，降低 Phase 3 的不确定性。
>
> **Bevy 嵌入方案**：采用**共享纹理方案**——Bevy 离屏渲染到 WGPU 纹理 → 通过共享内存传入 WebView → `<canvas>` 以 WebGL 展示。用户感知为同一窗口内的嵌入式画布。Phase 2.5 完成此方案的 PoC 验证，Phase 3 正式实现。备选方案为区域分割（Tauri 窗口内直接划分 WGPU Surface 区域）。
>
> **⚠️ WASM Component Model 风险**：WIT 和 WASM Component Model 规范仍在演进中（截至 2025 年尚未完全稳定）。建议 Phase 3 初期采用更成熟的插件通信方案（如 HTTP 微服务或 stdin/stdout 结构化协议），待 WASM Component Model 稳定后再迁移。`plugins/` 目录保留为 WASM 插件的最终目标形态。

## 4. Phase 1 MVP 精确定义

### 4.1 交付物

| 目录 | 交付内容 |
|------|---------|
| `schemas/` | JSON Schema 定义：LearningGoal、FeasibilityResult、UserProfile、CurriculumPlan、Chapter、Message、ChapterProgress |
| `crates/agent-core` | Rust HTTP 服务（Axum），包含：目标可行性判断、用户画像采集对话、学习路径规划、章节教学对话。LLM 通过 OpenAI-compatible API 调用，prompt 从独立模板文件加载 |
| `prompts/` | 每个核心步骤对应的 prompt 模板，要求结构化输出 |
| `apps/web-ui` | React 或 Svelte 单页应用，包含：聊天窗口、课程目录侧栏、章节内容区（Markdown + KaTeX + Monaco Editor）、简单状态路由 |

### 4.1.1 Phase 1 各模块交付清单

| 模块 | Phase 1 交付 | Phase 1 不交付 |
|------|-------------|---------------|
| `schemas/` | LearningGoal、FeasibilityResult、UserProfile、CurriculumPlan、Chapter、Message、ChapterProgress 的 JSON Schema | 插件协议（PluginManifest、PluginRequest）、沙箱协议（SandboxRequest、EvaluationResult）、Bevy 场景协议（SceneSpec） |
| `crates/` | `agent-core`（单一 crate，包含 HTTP 服务、状态机、LLM 调用、prompt 加载） | `storage`、`plugin-host`、`assessment-engine`、`tool-router`、`llm-gateway` |
| `prompts/` | feasibility_check、profile_collection、curriculum_planning、chapter_teaching、question_answering 模板 | 题目生成、答案评估、插件交互模板 |
| `apps/` | `web-ui`（React/Svelte SPA，聊天界面、课程目录、章节内容展示） | `desktop`（Tauri）、`bevy-viewer`（Bevy 渲染） |
| `sandboxes/` | 无（Phase 1 不运行用户代码） | Docker 沙箱、代码编译执行环境 |
| `tests/` | 核心流程集成测试（目标判断、画像采集、路径规划、章节教学） | 沙箱测试、插件契约测试、E2E 自动化测试 |
| `tools/` | schema-validator（JSON Schema 校验工具） | prompt-tester、代码生成工具 |
| `assets/` | 基础 UI 图标、字体（如需要） | 场景背景、3D 模型、音频 |
| `plugins/` | 无（Phase 3 功能） | 所有插件相关功能 |
| `docs-internal/` | 初始 ADR（架构决策记录） | 完整威胁模型、实验记录 |

### 4.2 技术栈

- **前端**：React 18+（或 Svelte 5），Vite，CodeMirror 6，KaTeX，react-markdown。
- **后端**：Rust，Axum，Tokio，Serde，reqwest（LLM HTTP client），tracing。
- **协议**：REST + SSE（流式），请求/响应体为 JSON Schema 校验。
- **运行方式**：`cargo run` 启动后端，`npm run dev` 启动前端，浏览器访问 `localhost:5173`。

#### 4.2.1 完整技术栈清单

**前端技术栈：**
- React 18+ 或 Svelte 5（UI 框架）
- Vite（构建工具）
- CodeMirror 6（代码编辑器，轻量 ~200KB。Phase 1 用于代码展示和语法高亮；Phase 2 引入代码执行后评估升级到 Monaco Editor）
- KaTeX（数学公式渲染）
- react-markdown（Markdown 渲染）
- TypeScript（类型安全）

**后端技术栈：**
- Rust（核心语言）
- Axum（HTTP 框架）
- Tokio（异步运行时）
- Serde（序列化/反序列化）
- reqwest（HTTP 客户端，用于 LLM 调用）
- tracing（日志和追踪）
- thiserror（错误处理）

**协议与数据格式：**
- REST API（HTTP 请求/响应）
- SSE（Server-Sent Events，流式输出）
- JSON Schema（数据校验）
- JSON（数据交换格式）

**开发工具（Phase 1）：**
- Cargo（Rust 包管理）
- npm/pnpm（前端包管理）
- JSON Schema 校验工具

**测试工具（Phase 1）：**
- Rust 内置测试框架
- 集成测试（HTTP API 测试）

**Phase 2+ 扩展技术栈：**
- Docker（沙箱隔离）
- SQLite/PostgreSQL（数据持久化，通过 SQLx）
- Playwright（E2E 测试）

**Phase 3+ 扩展技术栈：**
- Tauri 2（桌面应用框架）
- Bevy（游戏引擎，用于渲染层）
- Wasmtime（WASM 运行时）
- WASM Component Model（插件系统）

#### 4.2.2 LLM 输出校验失败处理

LLM 结构化输出必须先通过 JSON Schema 校验才能进入流程。校验失败时的处理策略：

1. **首次失败**：重试 LLM 调用（携带校验错误信息，要求 LLM 修正输出），最多重试 2 次
2. **重试耗尽**：返回友好错误给前端，提示用户"AI 暂时无法处理，请稍后重试"
3. **降级策略**：对于非关键字段（如 `estimated_duration`），缺失时使用默认值而不阻塞流程
4. **日志**：所有校验失败记录到 tracing（脱敏后），用于后续 prompt 调优

#### 4.2.3 状态机设计

学习流程的核心状态机：

```
IDLE → GOAL_INPUT → FEASIBILITY_CHECK → PROFILE_COLLECTION →
CURRICULUM_PLANNING → CHAPTER_LEARNING → COMPLETED

任意状态 → ERROR（LLM 失败 / 校验失败 / 网络错误）
ERROR → 上一状态（重试）或 IDLE（放弃）
```

| 状态 | 说明 | 可触发的转换 |
|------|------|-------------|
| IDLE | 初始/重置状态 | → GOAL_INPUT |
| GOAL_INPUT | 等待用户输入学习目标 | → FEASIBILITY_CHECK / ERROR |
| FEASIBILITY_CHECK | LLM 判断目标可行性 | → PROFILE_COLLECTION（可行）/ GOAL_INPUT（调整）/ ERROR |
| PROFILE_COLLECTION | 3-5 轮画像采集对话 | → CURRICULUM_PLANNING / ERROR |
| CURRICULUM_PLANNING | 生成学习路径 | → CHAPTER_LEARNING / ERROR |
| CHAPTER_LEARNING | 章节教学对话（可切换章节、提问） | → COMPLETED（全部章节完成）/ ERROR |
| COMPLETED | 全部章节完成 | → IDLE |
| ERROR | 错误状态 | → 上一状态（重试）/ IDLE（重置） |

**状态持久化**：Phase 1 将状态序列化为 JSON 文件；Phase 2 迁移至 SQLite。
**并发模型**：单会话串行流转，不支持同一会话并发状态切换。
**超时恢复**：用户断开连接后，通过 `session_id` 恢复至断点状态。

#### 4.2.4 SSE 事件规范

| 事件类型 | 方向 | 说明 | 数据结构 |
|---------|------|------|---------|
| `chunk` | Server→Client | LLM 流式输出的文本片段 | `{ "content": "string", "index": number }` |
| `status` | Server→Client | 状态变更通知 | `{ "state": "string", "message": "string" }` |
| `error` | Server→Client | 错误通知 | `{ "code": "string", "message": "string" }` |
| `done` | Server→Client | 当前步骤完成（携带结果） | `{ "result": <SchemaType> }` |
| `ping` | Server→Client | 心跳保活（每 15s） | `{}` |

**重连机制**：Client 通过 `Last-Event-ID` HTTP header 实现断点续传。Server 保留最近 50 条事件的回放缓冲。
**会话标识**：通过 URL query `?session_id=<uuid>` 关联 SSE 连接与会话状态。

#### 4.2.5 Phase 1 API 端点

| 方法 | 路径 | 说明 | 请求体 | 响应 |
|------|------|------|--------|------|
| POST | `/api/session` | 创建新学习会话 | — | `{ "session_id": "uuid", "state": "IDLE" }` |
| POST | `/api/session/{id}/goal` | 提交学习目标 | `LearningGoal` | SSE 流（状态变更 + FeasibilityResult） |
| POST | `/api/session/{id}/profile/answer` | 提交画像采集回答 | `{ "question_id": "...", "answer": "..." }` | SSE 流（下一问题 或 UserProfile 完成） |
| GET | `/api/session/{id}/curriculum` | 获取学习路径 | — | `CurriculumPlan` |
| GET | `/api/session/{id}/chapter/{ch_id}` | 获取/继续章节教学 | — | SSE 流（Chapter 内容） |
| POST | `/api/session/{id}/chapter/{ch_id}/ask` | 章节内提问 | `{ "question": "..." }` | SSE 流（Message） |
| POST | `/api/session/{id}/chapter/{ch_id}/complete` | 标记章节完成 | — | `ChapterProgress` |

所有端点返回 `application/json`，SSE 端点返回 `text/event-stream`。
错误响应格式：`{ "error": { "code": "string", "message": "string" } }`。

### 4.3 明确不包含的内容

- **不包含** Bevy、Tauri、WASM 插件、Docker 沙箱、数据库持久化（可用内存或 JSON 文件替代）。
- **不包含** 用户认证、多用户、支付、国际化。
- **不包含** 真实代码编译执行、数学计算引擎（LLM 输出数学解释文本即可，不假装计算）。
- **Phase 1 中代码示例仅供展示和阅读**，不提供在线执行。用户可查看语法高亮的代码块，但需自行在本地环境运行验证。代码执行能力在 Phase 2 通过沙箱提供。

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

#### 迭代构建策略

推荐在 3 个 Sprint 中逐步交付，避免严格瀑布：

| Sprint | 范围 | 交付 |
|--------|------|------|
| Sprint 1 | schemas/ 初版 + agent-core 骨架 + 目标判断端到端流程 | 可运行的 Axum 服务 + JSON Schema |
| Sprint 2 | 全状态机 + 5 个 prompt 模板 + 完整 HTTP API | 完整的后端服务 |
| Sprint 3 | web-ui 全部页面 + SSE 对接 | 可演示的完整产品 |

每个 Sprint 结束时进行集成测试，确保已交付部分可独立运行。

### 4.6 CI/CD

- **代码检查**：GitHub Actions，每次 PR 运行 `cargo fmt --check`、`cargo clippy`、`cargo test`、`npm run lint`、`npm run typecheck`
- **Schema 校验**：pre-commit hook 校验 JSON Schema 格式
- **构建产物**：Phase 2 起自动构建 Docker 沙箱镜像

### 4.7 LLM 成本与速率管理

| 策略 | 说明 |
|------|------|
| **Token 预算** | 单次学习会话上限 50K tokens（约 $0.05-$0.50，视模型而定） |
| **会话速率限制** | 单用户每秒最多 1 次 LLM 请求 |
| **响应缓存** | 相同学习目标 + 相同画像 → 复用已生成的 CurriculumPlan（1 小时 TTL） |
| **模型分层** | 简单对话用低成本模型（如 GPT-4o-mini），复杂规划用高能力模型 |
| **成本追踪** | 每次 LLM 调用记录 token 用量和预估成本到 tracing |

### 4.8 内容质量保证

LLM 生成的教学内容可能存在事实错误。Phase 1 采取以下策略：

1. **事实核查 Prompt**：`chapter_teaching.prompt.md` 中包含"请标注你不确定的内容"指令
2. **用户反馈**：每个章节底部提供 👍/👎 反馈按钮，收集内容质量信号
3. **来源引用**：Prompt 要求 LLM 标注关键概念的可查来源
4. **已知限制提示**：首次对话时告知用户"内容由 AI 生成，可能存在不准确之处，建议交叉验证"
5. **Phase 2+ 扩展**：引入知识库 RAG，将 LLM 生成内容与权威教材交叉验证

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

### 5.1 Crate 划分方案

**Phase 1：单一 Crate**
- `agent-core`：包含所有核心逻辑（HTTP 服务、状态机、LLM 调用、prompt 加载）
- 理由：Phase 1 聚焦快速验证核心流程，单一 crate 便于开发和调试

**Phase 2：功能拆分**
- `agent-core`：保留核心编排逻辑、状态机、HTTP API
- `storage`：数据持久化（SQLite/PostgreSQL）、用户数据、学习进度
- `assessment-engine`：练习题生成、答案评估、评分逻辑
- `llm-gateway`：LLM 调用封装、重试、缓存、多模型支持
- 理由：Phase 2 引入持久化和评估功能，拆分可提高模块独立性

**Phase 2.5：桌面化过渡**
- `bevy-protocol`：Bevy 场景协议实现，负责 Bevy 离屏渲染到 WGPU 纹理、通过共享内存传入 WebView 的纹理共享通道
- 理由：Phase 2.5 先做 Bevy 嵌入技术验证，降低 Phase 3 的不确定性

**Phase 3：进一步拆分**
- `plugin-host`：WASM 插件加载、权限管理、生命周期管理
- `tool-router`：工具调度、沙箱请求路由
- 理由：Phase 3 功能复杂度增加，细粒度拆分便于维护和测试

**Crate 依赖关系：**
```
schemas (无依赖)
  ↓
┌─ agent-core ─────────────────────┐
│      ↓              ↓            │
│  llm-gateway    storage          │
│      (Phase 2)   (Phase 2)       │
│      ↓              ↓            │
│  assessment-engine               │
│      (Phase 2)                   │
└──────────────────────────────────┘
       ↓
  plugin-host ─→ tool-router
  (Phase 3)      (Phase 3)
       ↓
  bevy-protocol (独立叶子 crate，仅依赖 schemas)
  (Phase 2.5-3)
```

- `agent-core` 依赖 `storage`（读写学习数据）和 `llm-gateway`（LLM 调用），三者构成核心层。
- `assessment-engine` 依赖 `agent-core` 的领域模型和 `storage` 存取题目/成绩。
- `plugin-host` 依赖 `agent-core` 的调度接口，通过 `tool-router` 路由沙箱请求。
- `bevy-protocol` 是独立叶子 crate，仅依赖 `schemas/`，实现 Bevy 场景渲染与 WebView 的纹理共享通道，不依赖其他业务 crate。

## 6. 约束与禁止事项

**约束类型说明：**
- **[Global]** 全局约束：适用于所有模块、所有阶段
- **[Phase X]** 阶段约束：特定阶段的约束
- **[Module]** 模块约束：特定模块的约束（详见各模块文档）

### 6.1 全阶段适用（硬约束）

- **[Global]** 不允许把所有逻辑塞进前端。
- **[Global]** 不允许让 LLM 伪造计算结果、编译结果或测试结果。
- **[Global]** 不允许把用户隐私、学习记录、API Key 写入日志或提交到仓库。
- **[Global]** 不允许在没有沙箱的情况下运行用户提交的代码。（Phase 1 不运行用户代码，Phase 2 起必须通过沙箱）
- **[Global]** 不允许在缺少权限边界的情况下加载第三方插件。（Phase 3 起适用）
- **[Global]** 不允许插件直接执行宿主系统命令。
- **[Global]** 不允许 Bevy 直接承担完整学习产品 UI。

### 6.2 Phase 1 放宽约束

Phase 1 以快速验证核心流程为目标，以下行为**允许**：

- **[Phase 1]** 允许创建探索性 demo、原型和验证代码（验证后归档到 `docs-internal/experiments/` 或删除）。
- **[Phase 1]** 允许使用内存存储或 JSON 文件替代数据库。
- **[Phase 1]** 允许 agent-core 和 web-ui 运行在同一台机器上，不做多租户隔离。
- **[Phase 1]** 允许 prompt 模板在调试阶段内联在代码中，但必须标注 `TODO: extract to prompts/`。

### 6.3 各阶段特有约束

| 阶段 | 禁止事项 |
|------|---------|
| **[Phase 1]** | 禁止引入 Bevy、Tauri、WASM、Docker 相关依赖。禁止在 UI 层直接调用 LLM API。 |
| **[Phase 2]** | 禁止在宿主机裸跑用户代码。禁止沙箱默认联网。禁止没有超时和资源限制地运行任务。 |
| **[Phase 3]** | 禁止插件绕过 Core 生成最终学习进度。禁止插件直接访问用户文件、网络、密钥或数据库。 |

## 7. 参考资源

### 7.1 Phase 1 参考

- OpenAI-compatible API：流式输出、工具调用、结构化输出文档。
- Rust async 生态：Tokio、Axum、Serde 文档。
- CodeMirror 6、KaTeX、Markdown 渲染资料。
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
