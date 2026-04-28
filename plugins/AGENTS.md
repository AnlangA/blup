# AGENTS.md

## 1. 目标

`plugins` 存放领域学习插件。每个插件提供一个特定学习环境或领域能力，例如语言学习、数学学习、编程学习、物理模拟、艺术创作等。

### 1.1 Phase 1 范围

**Phase 1 交付：**
- 无（插件系统属于 Phase 3）

**Phase 1 不交付：**
- 所有插件相关功能
- WASM 插件系统
- 插件权限管理

## 2. 目标实现的路径

- 插件优先采用 WASM Component Model。
- 每个插件声明 metadata、capabilities、permissions、input schema、output schema。
- 插件可生成 LessonSpec、AssessmentSpec、SceneSpec 和 ToolRequest。
- 插件不能直接访问系统资源，必须通过宿主提供的能力调用工具。
- 插件可以请求数学计算、代码运行、场景渲染、语音练习等能力，但由 Core 和 Sandbox 决定是否执行。

### 2.1 Phase 3 插件权限模型

**权限列表：**
- `read:curriculum`：读取学习路径信息
- `read:user_profile`：读取用户画像（脱敏后）
- `generate:content`：生成学习内容（LessonSpec）
- `generate:assessment`：生成评估内容（AssessmentSpec）
- `generate:scene`：生成场景描述（SceneSpec）
- `request:tool`：请求工具调用（需 Core 审批）
  - `tool:math`：数学计算
  - `tool:code_run`：代码运行
  - `tool:render`：场景渲染

**禁止权限：**
- 直接文件系统访问
- 直接网络访问
- 直接系统命令执行
- 直接数据库访问
- 直接访问其他插件

**插件生命周期：**
1. **加载（Load）**：验证插件签名、检查依赖
2. **初始化（Init）**：分配资源、注册能力
3. **激活（Activate）**：开始接收请求
4. **执行（Execute）**：处理学习内容生成请求
5. **暂停（Pause）**：暂停插件（保留状态）
6. **卸载（Unload）**：释放资源、清理状态

每个阶段都有权限检查点，确保插件不能越权操作。

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

**全局约束请参考根文档第 6.1 节。**

**模块特有约束：**
- **[Module]** 不允许插件直接执行 shell 命令。
- **[Module]** 不允许插件直接访问用户文件、网络、密钥或数据库。
- **[Module]** 不允许插件直接操作 Bevy ECS。
- **[Module]** 不允许插件绕过 Core 生成最终学习进度。
- **[Module]** 不允许插件把 LLM 输出当作确定性计算结果。

## 5. 相关文档

- [根文档 AGENTS.md](../AGENTS.md) - 项目整体规划和阶段划分
- [schemas/AGENTS.md](../schemas/AGENTS.md) - 插件协议定义（Phase 3）
- [crates/AGENTS.md](../crates/AGENTS.md) - 核心库，包含插件宿主（Phase 3）
