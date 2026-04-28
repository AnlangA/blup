# AGENTS.md

## 1. 目标

`schemas` 存放跨模块通信协议和结构化数据定义，例如 LessonSpec、SceneSpec、AssessmentSpec、ToolRequest、EvaluationResult、UserProfile、CurriculumPlan。

## 2. 目标实现的路径

- 使用 JSON Schema、TypeScript types、Rust structs 或 WIT 定义协议。
- 所有 Agent、插件、UI、Bevy、沙箱之间的重要数据必须结构化。
- SceneSpec 用于描述 Bevy 可渲染场景。
- AssessmentSpec 用于描述题目、评分规则和通过条件。
- ToolRequest 用于描述真实计算、编译、运行、检索等请求。

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

- 不允许无版本字段的长期协议。
- 不允许直接传递未经校验的 LLM 原始输出。
- 不允许在多个模块中重复定义同一份协议而不统一来源。
- 不允许在协议中包含 API Key、系统路径或敏感实现细节。
