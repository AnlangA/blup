# AGENTS.md

## 1. 目标

`tools` 存放开发辅助工具、代码生成工具、schema 校验工具、插件打包工具、资产处理工具和本地运维脚本。

### 1.1 Phase 1 范围

**Phase 1 交付：**
- schema-validator：JSON Schema 校验工具

**Phase 1 不交付：**
- prompt-tester（Prompt 测试工具，Phase 2）
- plugin-builder（插件打包工具，Phase 3）
- asset-optimizer（资产优化工具，Phase 3）

## 2. 目标实现的路径

- 后续可加入 schema generation、plugin build、sandbox image build、asset optimization、prompt validation 等工具。
- 所有工具应明确输入、输出、副作用和安全限制。
- 工具应优先服务开发流程，不承载线上业务逻辑。

### 2.1 Phase 1 工具清单

| 工具名称 | 用途 | 输入 | 输出 | 使用场景 |
|---------|------|------|------|---------|
| **schema-validator** | 校验 JSON 数据是否符合 Schema | JSON 文件路径、Schema 文件路径 | 校验结果（通过/失败）、错误详情 | 开发时校验 LLM 输出、测试数据准备 |

**Phase 2+ 工具计划：**
- prompt-tester：测试 prompt 模板的输出质量
- sandbox-builder：构建沙箱 Docker 镜像
- asset-optimizer：压缩和优化图片、字体等资产

**Phase 3+ 工具计划：**
- plugin-builder：打包 WASM 插件
- schema-generator：从 Rust 结构体生成 JSON Schema

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

**全局约束请参考根文档第 6.1 节。**

**模块特有约束：**
- **[Module]** 不允许工具默认删除用户文件。
- **[Module]** 不允许工具默认上传本地数据。
- **[Module]** 不允许工具隐藏执行外部命令。
- **[Module]** 不允许在工具中硬编码本地绝对路径、密钥或个人配置。

## 5. 相关文档

- [根文档 AGENTS.md](../AGENTS.md) - 项目整体规划
- [schemas/AGENTS.md](../schemas/AGENTS.md) - 协议定义，schema-validator 的校验对象
