# AGENTS.md

## 1. 目标

`apps` 存放面向用户的应用入口，包括 Tauri 桌面应用、Web UI、Bevy 渲染宿主或 Bevy WASM 嵌入入口。

## 2. 目标实现的路径

- 后续可拆分为 `apps/desktop`、`apps/web-ui`、`apps/bevy-viewer`。
- Tauri 负责桌面窗口、系统权限、文件访问、IPC、安全配置。
- Web UI 负责学习界面、聊天、章节、题目、Markdown、公式和代码编辑器。
- Bevy Viewer 负责展示由 SceneSpec 描述的 2D、3D、像素风和模拟场景。
- 应用层只能调用 Rust Core 暴露的命令或 API，不直接实现 Agent 业务逻辑。

## 3. 需要联网查找/参考的资料与核心思想

需要查找：

- Tauri 2 frontend/backend 通信方式。
- Tauri permission 和 command 文档。
- React 或 Svelte 与 Tauri 集成资料。
- Bevy WASM 嵌入 WebView 的实践资料。
- Monaco Editor、KaTeX、Markdown 渲染最佳实践。

核心思想：

- 应用层关注用户体验，不拥有核心学习状态。
- Web UI 和 Bevy 是两个不同渲染层：Web UI 渲染信息，Bevy 渲染互动场景。
- 所有跨层数据交互应通过明确的结构化事件或命令完成。

## 4. 不允许做什么事情

- 不允许在 UI 层直接调用 LLM。
- 不允许在 UI 层直接执行代码、编译代码或运行系统命令。
- 不允许把业务状态分散存放在多个前端组件中而不经过 Core。
- 不允许让 Bevy 替代传统表单、长文本、聊天和代码编辑器 UI。
