# AGENTS.md

## 1. 目标

`schemas` 存放跨模块通信协议和结构化数据定义，例如 LessonSpec、SceneSpec、AssessmentSpec、ToolRequest、EvaluationResult、UserProfile、CurriculumPlan。

### 1.1 Phase 1 范围

**Phase 1 交付：**
- LearningGoal、FeasibilityResult、UserProfile、CurriculumPlan、Chapter、Message、ChapterProgress 的 JSON Schema 定义
- 用于核心学习流程的基础协议

**Phase 1 不交付：**
- 插件协议（PluginManifest、PluginRequest、PluginResponse）
- 沙箱协议（SandboxRequest、EvaluationResult、ToolRequest）
- Bevy 场景协议（SceneSpec、RenderCommand）

## 2. 目标实现的路径

- 使用 JSON Schema、TypeScript types、Rust structs 或 WIT 定义协议。
- 所有 Agent、插件、UI、Bevy、沙箱之间的重要数据必须结构化。
- SceneSpec 用于描述 Bevy 可渲染场景。
- AssessmentSpec 用于描述题目、评分规则和通过条件。
- ToolRequest 用于描述真实计算、编译、运行、检索等请求。

### 2.1 Phase 1 Schema 清单

| Schema 名称 | 用途 | 关键字段 | 依赖关系 |
|------------|------|---------|---------|
| **LearningGoal** | 用户输入的学习目标 | `goal_text`（目标描述）、`domain`（领域，如编程/数学）、`user_context`（用户背景） | 无 |
| **FeasibilityResult** | 目标可行性判断结果 | `is_feasible`（是否可行）、`reason`（原因）、`suggestions`（调整建议）、`estimated_duration`（预计学习时长） | 依赖 LearningGoal |
| **UserProfile** | 用户画像 | `experience_level`（经验水平）、`background`（背景知识）、`available_time`（可用时间）、`learning_style`（学习风格）、`preferences`（偏好） | 无 |
| **CurriculumPlan** | 学习路径规划 | `chapters`（章节列表）、`total_duration`（总时长）、`prerequisites`（前置知识）、`learning_objectives`（学习目标） | 依赖 LearningGoal、UserProfile |
| **Chapter** | 单个章节 | `id`（章节 ID）、`title`（标题）、`description`（描述）、`content`（内容，Markdown）、`duration`（预计时长）、`order`（顺序） | 属于 CurriculumPlan |
| **Message** | 对话消息 | `role`（角色：user/assistant/system）、`content`（内容）、`timestamp`（时间戳）、`chapter_id`（所属章节） | 可选依赖 Chapter |
| **ChapterProgress** | 章节学习进度 | `chapter_id`（章节 ID）、`status`（状态：not_started/in_progress/completed）、`completion_percentage`（完成百分比）、`last_updated`（最后更新时间） | 依赖 Chapter |

**Schema 版本管理策略：**
- 每个 schema 包含 `version` 字段（如 "1.0"）
- 向后兼容的修改（新增可选字段）：小版本号递增（1.0 → 1.1）
- 破坏性修改（删除字段、修改字段类型）：大版本号递增（1.0 → 2.0）
- 所有 schema 文件命名格式：`{schema_name}.v{version}.schema.json`

## 3. 需要联网查找/参考的资料与核心思想

需要查找：

- JSON Schema 规范。
- OpenAPI 规范。
- WIT 与 WASM Component Model 类型映射。
- Serde schema 生成实践。
- 结构化输出和 schema validation 最佳实践。

核心思想：

- 协议优先，模块解耦。
- LLM 输出必须经过 schema 验证。
- UI、插件、Core、Bevy、沙箱之间不能依赖随意字符串格式。

## 4. 不允许做什么事情

**全局约束请参考根文档第 6.1 节。**

**模块特有约束：**
- **[Module]** 不允许无版本字段的长期协议。
- **[Module]** 不允许直接传递未经校验的 LLM 原始输出。
- **[Module]** 不允许在多个模块中重复定义同一份协议而不统一来源。
- **[Module]** 不允许在协议中包含 API Key、系统路径或敏感实现细节。

## 5. 相关文档

- [根文档 AGENTS.md](../AGENTS.md) - 项目整体规划和约束
- [crates/AGENTS.md](../crates/AGENTS.md) - 核心库实现，依赖本模块的 schema 定义
- [prompts/AGENTS.md](../prompts/AGENTS.md) - Prompt 模板，输出结构需符合本模块的 schema
