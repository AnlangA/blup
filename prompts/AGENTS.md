# AGENTS.md

## 1. 目标

`prompts` 存放 Agent 使用的提示词、系统角色、规划模板、评估模板、结构化输出模板和安全约束模板。

### 1.1 Phase 1 范围

**Phase 1 交付：**
- 核心学习流程的 5 个 prompt 模板
- 每个模板要求 LLM 输出结构化结果（符合 schemas 定义）

**Phase 1 不交付：**
- 题目生成模板（Phase 2）
- 答案评估模板（Phase 2）
- 插件交互模板（Phase 3）

## 2. 目标实现的路径

- 将目标可行性判断、用户画像采集、学习路径规划、章节教学、题目生成、答案评估分别设计为独立 prompt 模板。
- Prompt 应尽量要求模型输出结构化结果。
- Prompt 应明确禁止模型伪造工具执行结果。
- 与 schema 配合使用，LLM 输出必须校验后才能进入 Core 流程。

### 2.1 Phase 1 Prompt 模板清单

| 模板名称 | 用途 | 输入参数 | 输出格式 | 对应 Schema |
|---------|------|---------|---------|------------|
| **feasibility_check.txt** | 判断学习目标是否可行 | `goal_text`（目标描述）、`domain`（领域） | JSON 结构化输出 | FeasibilityResult |
| **profile_collection.txt** | 采集用户画像（3-5 轮对话） | `conversation_history`（对话历史）、`current_question`（当前问题） | JSON 结构化输出 | UserProfile |
| **curriculum_planning.txt** | 生成个性化学习路径 | `learning_goal`（学习目标）、`user_profile`（用户画像） | JSON 结构化输出 | CurriculumPlan |
| **chapter_teaching.txt** | 生成章节教学内容 | `chapter`（章节信息）、`user_profile`（用户画像）、`previous_chapters`（前置章节） | Markdown + 结构化元数据 | Chapter（content 字段） |
| **question_answering.txt** | 回答用户在学习过程中的提问 | `question`（用户问题）、`chapter_context`（章节上下文）、`conversation_history`（对话历史） | Markdown 文本 | Message |

**Prompt 模板结构：**
- 系统角色定义（System Prompt）
- 任务描述和目标
- 输入参数说明
- 输出格式要求（JSON Schema 引用）
- 安全约束（禁止伪造结果、禁止输出敏感信息）
- 示例（Few-shot examples）

**Prompt 版本管理：**
- 文件命名格式：`{template_name}.v{version}.txt`
- 每个模板包含版本号和最后更新日期
- 重大修改时递增版本号

## 3. 需要联网查找/参考的资料与核心思想

需要查找：

- OpenAI-compatible structured output 文档。
- Prompt injection 防护资料。
- 教育测评 prompt 设计资料。
- RAG grounded generation 最佳实践。

核心思想：

- Prompt 是可版本化的业务规则，不是随手写在代码中的字符串。
- LLM 只负责生成建议、解释和草案，最终结果必须由 Core、schema 和工具校验。
- 教学 Prompt 应关注个性化、可评估、可追踪和可纠错。

## 4. 不允许做什么事情

**全局约束请参考根文档第 6.1 节。**

**模块特有约束：**
- **[Module]** 不允许在 prompt 中硬编码 API Key 或隐私数据。
- **[Module]** 不允许让 prompt 要求模型伪造运行、编译、计算、检索结果。
- **[Module]** 不允许把 prompt 散落在 UI 组件中。
- **[Module]** 不允许没有版本和用途说明的 prompt 模板。

## 5. 相关文档

- [根文档 AGENTS.md](../AGENTS.md) - 项目整体规划和技术栈
- [schemas/AGENTS.md](../schemas/AGENTS.md) - 协议定义，prompt 输出需符合 schema
- [crates/AGENTS.md](../crates/AGENTS.md) - 核心库，负责加载和使用 prompt 模板
