# Blup

AI 交互式学习 Agent 平台。用户输入学习目标后，系统自动判断可行性、采集用户画像、生成个性化学习路径，并按章节提供资料、互动内容、练习、考核和反馈。

## 架构

```
Tauri 桌面应用
├── Web UI          — 聊天、课程目录、章节内容、Markdown、代码编辑器
├── Rust Agent Core — 目标判断、路径规划、章节编排、考核评估、插件调度
├── WASM 插件系统   — 每个插件负责一个学习领域或一种学习环境
├── Bevy 渲染层     — 2D/3D、像素风、模拟实验、游戏化互动
└── 沙箱环境        — Docker/WASI/Firecracker 提供真实编译与运行能力
```

## 技术栈

- [Tauri 2](https://v2.tauri.app/) — 桌面应用容器
- [Bevy](https://bevyengine.org/) — ECS 游戏引擎，用于渲染互动内容
- [Wasmtime](https://wasmtime.dev/) — WASM Component Model 插件隔离
- Rust (Tokio / Axum / Serde / SQLx) — 异步后端
- [CodeMirror 6](https://codemirror.net/) / [KaTeX](https://katex.org/) — 编辑与公式渲染

## 当前阶段

**Phase 1: MVP** —— 单用户 Web 对话学习助手。

| Phase | 目标 | 状态 |
|-------|------|------|
| 1: MVP | Web 对话学习助手（目标判断 → 画像 → 路径 → 章节教学） | 🚧 进行中 |
| 2: 强化 | 练习/考核引擎、代码沙箱、数据持久化 | ⏳ 计划 |
| 2.5: 桌面化 | Tauri 包裹 Web UI，Bevy 嵌入 PoC（共享纹理） | ⏳ 计划 |
| 3: 扩展 | WASM 插件系统、Bevy 互动场景 | ⏳ 计划 |

详见 [AGENTS.md](./AGENTS.md)。

## 核心原则

- LLM 负责解释、规划和对话，不伪造确定性任务结果
- 数学计算交给计算引擎，代码执行交给沙箱
- 插件通过结构化协议与宿主通信，不直接穿透系统边界
- 所有学习内容和评估结果尽量结构化，便于验证、复用和回放

## 项目结构

```
blup/
├── apps/           — 前端应用 (Web UI)
├── assets/         — 静态资源
├── crates/         — Rust crate 模块
├── docs-internal/  — 内部文档
├── plugins/        — WASM 插件
├── prompts/        — LLM prompt 模板
├── sandboxes/      — 沙箱环境配置
├── schemas/        — 结构化协议定义 (LessonSpec, SceneSpec 等)
├── tests/          — 测试
└── tools/          — 开发工具与脚本
```

## License

TBD