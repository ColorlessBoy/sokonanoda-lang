# skills/ —— 为 code agent 封装的操作知识

本目录把"如何操作本项目"固化成 [Agent Skill](https://opencode.ai/docs/skills/)
格式（SKILL.md + frontmatter），任何支持该格式的 code agent（opencode、
Claude Code 等）都能一键加载。项目自身的 conformance 测试守护这些文件
不会与真实工具漂移（`crates/cli/tests/skill.rs`）。

| Skill | 给谁 | 内容 |
|---|---|---|
| `sokonanoda-teacher/` | 当老师的 agent | 教学循环、判卷事件决策表、出题规范与适配规则、解答钥匙使用守则；参考件：`references/events.md`（事件形状）、`references/curriculum.md`（题池地图） |
| `sokonanoda-dev/` | 接手开发的 agent | 接手清单、硬规则、TDD 三层与文档先行工作流、CI 门禁形态 |
| `sokonanoda-ci/` | 推代码/发布/查 CI 的 agent | 本地验证纪律（退出码、无 grep 掩膜）、GitHub Actions 陷阱台账、`gh` 排错三板斧、失败必录 |

## 安装

### opencode（项目级，零安装）

仓库根 `opencode.json` 已把本目录挂进 opencode（`skills.paths: ["./skills"]`）：
打开本项目时三者自动可加载，无需软链。同一配置还提供：

- LSP：`.sokonanoda` 文件自动启动 `sokonanoda-lsp` 并消费 kernel 判定的诊断；
- 命令：`/gate`（与 CI 一致的本地门禁）、`/check`（内核判卷并汇总事件）、
  `/round`（按本仓库流程启动一轮开发）；
- 主 agent `teacher`（Tab 切换）：在画布上充当 Lean 式证明老师；
- 权限：`lean`/`lake`/`elan`/`leanc` 命令 deny（硬规则落地为配置）。

### Claude Code / 其他 harness（软链）

把 skill 目录放进你的 agent harness 的 skills 目录（软链或复制均可）：

```bash
# Claude Code（用户级）
ln -s "$PWD/skills/sokonanoda-teacher" ~/.claude/skills/sokonanoda-teacher

# 其他读取 ~/.agents/skills 的 harness
ln -s "$PWD/skills/sokonanoda-teacher" ~/.agents/skills/sokonanoda-teacher
```

编辑器反馈通道无需额外配置：仓库根 `opencode.json` 把 `.sokonanoda` 挂到
仓库自带 launcher `.opencode/lsp/sokonanoda-lsp.sh`（opencode 打开教学文件时
自动启动并消费 kernel 判定的诊断）。launcher 优先复用已构建的 `target/`
二进制或 **VS Code 扩展自带的同版本 bin**，都没有才 `cargo build`——
装了扩展的机器不需要 cargo。

## 守护

- `cargo test -p sokonanoda-cli --test skill`：frontmatter 合法、引用的仓库
  路径真实存在、事件/方法词汇封闭且与 `docs/protocol.md` 一致；
- 协议词汇变更时先改 `docs/protocol.md` 与 `crates/cli/tests/common/mod.rs`，
  conformance 测试会指出 skill 侧需要跟改的位置。
