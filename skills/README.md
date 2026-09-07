# skills/ —— 为 code agent 封装的操作知识

本目录把"如何操作本项目"固化成 [Agent Skill](https://opencode.ai/docs/skills/)
格式（SKILL.md + frontmatter），任何支持该格式的 code agent（opencode、
Claude Code 等）都能一键加载。项目自身的 conformance 测试守护这些文件
不会与真实工具漂移（`crates/cli/tests/skill.rs`）。

| Skill | 给谁 | 内容 |
|---|---|---|
| `sokonanoda-teacher/` | 当老师的 agent | 教学循环、判卷事件决策表、出题规范与适配规则、解答钥匙使用守则；参考件：`references/events.md`（事件形状）、`references/curriculum.md`（题池地图） |
| `sokonanoda-dev/` | 接手开发的 agent | 接手清单、硬规则、TDD 三层与文档先行工作流、CI 门禁形态 |

## 安装

把 skill 目录放进你的 agent harness 的 skills 目录（软链或复制均可）：

```bash
# Claude Code / opencode（用户级）
ln -s "$PWD/skills/sokonanoda-teacher" ~/.claude/skills/sokonanoda-teacher
ln -s "$PWD/skills/sokonanoda-teacher" ~/.agents/skills/sokonanoda-teacher

# opencode 项目级：仓库根已带 opencode.json（配置了 sokonanoda LSP），
# skills 放进项目根的 .opencode/skill/ 或用户级目录均可
```

编辑器反馈通道无需额外配置：仓库根 `opencode.json` 已把
`sokonanoda-lsp` 挂到 `.sokonanoda` 扩展名上（opencode 会在打开教学文件时
自动启动它并消费 kernel 判定的诊断）。

## 守护

- `cargo test -p sokonanoda-cli --test skill`：frontmatter 合法、引用的仓库
  路径真实存在、事件/方法词汇封闭且与 `docs/protocol.md` 一致；
- 协议词汇变更时先改 `docs/protocol.md` 与 `crates/cli/tests/common/mod.rs`，
  conformance 测试会指出 skill 侧需要跟改的位置。
