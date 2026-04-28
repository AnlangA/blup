# AGENTS.md

## 1. 目标

`docs-internal` 存放内部设计记录、架构决策、威胁模型、协议草案和研究笔记。该目录不是用户文档，也不是 README 替代品。

## 2. 目标实现的路径

- 记录关键架构决策和取舍。
- 记录插件协议、沙箱安全、Bevy SceneSpec、Agent 状态机等设计草案。
- 记录外部资料调研结论和链接。
- 保持文档服务于实现，不做营销化介绍。

### 2.1 文档组织结构

```
docs-internal/
├── adr/                    # Architecture Decision Records
│   ├── 0001-use-rust.md
│   ├── 0002-llm-gateway-design.md
│   ├── 0003-phase-1-single-crate.md
│   └── template.md
├── threat-models/          # 威胁模型
│   ├── plugin-isolation.md
│   ├── sandbox-security.md
│   └── template.md
├── experiments/            # 实验和原型
│   └── README.md
└── research/               # 调研结论
    └── README.md
```

**ADR 格式：**
- 标题：简短描述决策
- 状态：提议（Proposed）、接受（Accepted）、废弃（Deprecated）、被替代（Superseded）
- 背景：为什么需要做这个决策
- 决策：具体的决策内容
- 后果：决策的影响（正面和负面）
- 替代方案：考虑过但未采纳的方案

**威胁模型格式：**
- 资产：需要保护的资源（用户数据、系统权限等）
- 威胁：可能的攻击向量（插件逃逸、沙箱突破等）
- 攻击向量：具体的攻击方式
- 缓解措施：如何防御
- 残余风险：无法完全消除的风险

## 3. 需要联网查找/参考的资料与核心思想

需要查找：

- ADR 架构决策记录格式。
- STRIDE 威胁建模资料。
- 插件系统安全设计资料。
- AI Agent 评估与教育产品设计资料。

核心思想：

- 重要决策要留下原因。
- 安全边界和协议设计必须先记录再实现。
- 内部文档应该帮助开发者减少误解，而不是堆砌概念。

## 4. 不允许做什么事情

**全局约束请参考根文档第 6.1 节。**

**模块特有约束：**
- **[Module]** 不允许存放用户隐私数据。
- **[Module]** 不允许存放 API Key、token 或凭据。
- **[Module]** 不允许写面向营销的空泛内容。
- **[Module]** 不允许把未验证的外部资料当作事实结论。

## 5. 相关文档

- [根文档 AGENTS.md](../AGENTS.md) - 项目整体规划，ADR 的主要参考对象
