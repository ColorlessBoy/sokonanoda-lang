---
description: 按本仓库的开发流程启动一轮开发（文档先行 + TDD 三层 + 收尾义务），参数为任务描述
agent: build
---

启动一轮开发。任务：$ARGUMENTS

按 `skills/sokonanoda-dev` 与 `AGENTS.md` 执行：

1. **接手清单**：先读 `REQUIREMENTS.md`（用户要求权威总账）、`STATUS.md`（最新轮在最上）、`ROADMAP.md` §10、`docs/architecture.md` §8（gotchas）。
2. **设计先行**：动手前把方案写进 `docs/design/*.md`（取舍 + 验收标准 + as-built 更新）。
3. **TDD 三层**：front 单测 → CLI e2e → 语料/协议/golden 守护；新增语法必须配课程 + 测试 + 白名单三件套。
4. **硬规则**：kernel 冻结（`docs/architecture.md` §6，不改语义/热路径）；不调用官方 Lean 工具链（`lean`/`lake`/`lean4export`/`leanc`/`elan` 等已在 opencode 权限里 deny）；判定永远走 kernel（`front::judge` 合成声明模式），禁止文本比对；用户/agent 路径零工具链依赖（REQUIREMENTS §2 第 9 条）。
5. **并行 subagent**：探索/机械重构可派发，任务书必须文件集互斥；主会话验证其产出（编译 + 全量测试）。
6. **收尾**：`/sokonanoda/gate` 全绿；更新 `STATUS.md`（新轮置顶）与新要求（追加 `REQUIREMENTS.md` §9 并注明日期）；commit + push 并监控 CI 到终态（`skills/sokonanoda-ci`），失败必录 `docs/CI-FAILURES.md`。
