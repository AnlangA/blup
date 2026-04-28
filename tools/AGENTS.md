# AGENTS.md

## 1. 目标

`tools` 存放开发辅助工具、代码生成工具、schema 校验工具、插件打包工具、资产处理工具和本地运维脚本。

## 2. 目标实现的路径

- 后续可加入 schema generation、plugin build、sandbox image build、asset optimization、prompt validation 等工具。
- 所有工具应明确输入、输出、副作用和安全限制。
- 工具应优先服务开发流程，不承载线上业务逻辑。

## 3. 需要联网查找/参考的资料与核心思想

需要查找：

- Rust CLI 工具开发资料，如 clap、xshell、duct。
- JSON Schema validation 工具。
- WASM plugin build pipeline。
- Docker image build 最佳实践。
- Asset optimization 工具资料。

核心思想：

- 工具链应可重复、可审计、可在 CI 中运行。
- 生成物和源文件边界必须清晰。
- 工具不能绕过 Core 的安全模型。

## 4. 不允许做什么事情

- 不允许工具默认删除用户文件。
- 不允许工具默认上传本地数据。
- 不允许工具隐藏执行外部命令。
- 不允许在工具中硬编码本地绝对路径、密钥或个人配置。
