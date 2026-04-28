# AGENTS.md

## 1. 目标

`prompts` 存放 Agent 使用的提示词、系统角色、规划模板、评估模板、结构化输出模板和安全约束模板。

## 2. 目标实现的路径

- 将目标可行性判断、用户画像采集、学习路径规划、章节教学、题目生成、答案评估分别设计为独立 prompt 模板。
- Prompt 应尽量要求模型输出结构化结果。
- Prompt 应明确禁止模型伪造工具执行结果。
- 与 schema 配合使用，LLM 输出必须校验后才能进入 Core 流程。

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

- 不允许在 prompt 中硬编码 API Key 或隐私数据。
- 不允许让 prompt 要求模型伪造运行、编译、计算、检索结果。
- 不允许把 prompt 散落在 UI 组件中。
- 不允许没有版本和用途说明的 prompt 模板。
