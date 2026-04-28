# AGENTS.md

## 1. 目标

`plugins` 存放领域学习插件。每个插件提供一个特定学习环境或领域能力，例如语言学习、数学学习、编程学习、物理模拟、艺术创作等。

## 2. 目标实现的路径

- 插件优先采用 WASM Component Model。
- 每个插件声明 metadata、capabilities、permissions、input schema、output schema。
- 插件可生成 LessonSpec、AssessmentSpec、SceneSpec 和 ToolRequest。
- 插件不能直接访问系统资源，必须通过宿主提供的能力调用工具。
- 插件可以请求数学计算、代码运行、场景渲染、语音练习等能力，但由 Core 和 Sandbox 决定是否执行。

## 3. 需要联网查找/参考的资料与核心思想

需要查找：

- WASM Component Model。
- WIT 接口定义。
- Wasmtime plugin host。
- 插件权限模型设计资料。
- 教育领域插件设计案例。

核心思想：

- 插件是领域能力扩展，不是系统主控。
- 插件输出结构化学习内容和工具请求。
- 插件必须可隔离、可卸载、可审计、可限制权限。

## 4. 不允许做什么事情

- 不允许插件直接执行 shell 命令。
- 不允许插件直接访问用户文件、网络、密钥或数据库。
- 不允许插件直接操作 Bevy ECS。
- 不允许插件绕过 Core 生成最终学习进度。
- 不允许插件把 LLM 输出当作确定性计算结果。
