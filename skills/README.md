# skills/ —— 为 code agent 封装的操作知识

本目录把"如何操作本项目"固化成 Agent Skill 格式（`SKILL.md` + frontmatter），
任何支持该格式的 code agent（DeepSeek Harness、opencode、Claude Code 等）都能
加载。本目录是**正文的唯一源**；各 harness 的入口见下方"安装"。
conformance 测试守护这些文件不与真实工具漂移：`crates/cli/tests/skill.rs`
（正文）与 `crates/cli/tests/dsh.rs`（DSH 入口 + 启动器 + LSP patch）。

| Skill | 给谁 | 内容 |
|---|---|---|
| `sokonanoda-teacher/` | 当老师的 agent | 角色定义与五条不可违反规则、教学循环、判卷事件决策表、出题规范与适配规则、解答钥匙使用守则；参考件：`references/events.md`（事件形状）、`references/curriculum.md`（题池地图）、`references/zh-style.md`（文风约束） |
| `sokonanoda-dev/` | 接手开发的 agent | 接手清单、硬规则、TDD 三层与文档先行工作流、CI 门禁形态 |
| `sokonanoda-ci/` | 推代码/发布/查 CI 的 agent | 本地验证纪律（退出码、无 grep 掩膜）、GitHub Actions 陷阱台账、`gh` 排错三板斧、失败必录 |
| `sokonanoda-update/` | **人工**运维命令 | 把缓存的 CLI + LSP 刷到仓库版本：何时需要、确切命令、**退出码语义**（`0` = 缓存写成了；`3` = `cache NOT refreshed` + `download:` 原因，即使有可用回退）、唯一可信判据（缓存 `marker` + 缓存二进制自述版本；`source` 不含 `STALE` **不算**）、"缓存写不进去"的三种处置 |
| `sokonanoda-doctor/` | **人工**运维命令 | 只读就绪诊断：`scripts/soko doctor --json`、退出码语义、要汇报的字段、下一步指向 `setup`/`update` |

前三个是**角色技能**（模型按任务加载）；后两个是**运维命令**（只给人用）：
DSH 入口带 `disable-model-invocation: true`，所以它们不进模型目录——模型侧的
等价能力写在 `AGENTS.md` 与三个角色技能里，不需要额外加载。

五个正文都只用 `name` + `description` frontmatter（加 `whenToUse` 供人工菜单），
这是所有 harness 的交集；harness 专属的调用策略只出现在各自的入口文件里。

## 环境：一条命令，所有 harness 通用

```bash
scripts/soko setup          # 版本锁定的 CLI + LSP → 缓存（幂等；零 cargo）
scripts/soko doctor --json  # 0=就绪 3=未就绪
scripts/soko grade playground.sokonanoda --json   # 判卷
scripts/soko gate           # 贡献者门禁（调用 cargo）
```

`scripts/soko` 是 harness 中立的启动器：解析"版本匹配"的仓库构建 → 缓存
（标记必须等于 `Cargo.toml` 版本）→ VS Code 扩展自带 → 版本锁定下载；其余
子命令原样转发给 `sokonanoda` CLI，**缓存过期直接拒绝运行**。`sokonanoda`
已在 PATH 时二者等价。设计见 `docs/design/deepseek-harness.md`、`docs/design/binary-cli.md`。

## 安装（按 harness）

### DeepSeek Harness（项目级，零安装）

DSH 自动发现 `<仓库根>/.agents/skills/`，而**技能名本身就是斜杠命令**：

- 打开本仓库即可用：`/sokonanoda-teacher`、`/sokonanoda-dev`、`/sokonanoda-ci`；
- 两个**人工**运维命令：`/sokonanoda-update`（刷新缓存）、
  `/sokonanoda-doctor`（就绪诊断）——入口带 `user-invocable: true` +
  `disable-model-invocation: true`，只在人的 `/` 菜单里出现；
- **DSH 的命令名文法不允许 `/`**（`COMMAND_NAME` 与技能名都是 kebab-case），
  所以 opencode 的 `/sokonanoda/update` 在 DSH 侧写作 `/sokonanoda-update`；
- `.agents/skills/<name>/SKILL.md` 是**薄入口**（正文在 `skills/<name>/SKILL.md`，
  入口里写明按仓库根解析；`dsh.rs` 守住两边不漂移）；
- LSP（hover / 跳定义 / 找引用）需显式启用：
  `dsh web --patch ./dsh/cordis.patch.yml`（**服务端诊断不会进 agent**，
  判卷一律走 CLI `--json`）：见 `dsh/README.md`；
- Lean 工具链 deny 用 `dsh/hooks/hooks.json` + hooks 桥（需在 profile 里插一行，
  因为 DSH 不做项目级 hook 发现）：见 `dsh/README.md`。

### opencode（项目级，零安装）

仓库根 `opencode.json` 已把本目录挂进 opencode（`skills.paths: ["./skills"]`）：
打开本项目时三者自动可加载，无需软链。同一配置还提供：

- LSP：`.sokonanoda` 文件自动启动 `sokonanoda-lsp` 并消费 kernel 判定的诊断；
- 命令：`/sokonanoda/setup|update|version|doctor|check|gate|round`；
- 主 agent `teacher`（Tab 切换）：加载同一份 `sokonanoda-teacher` 技能；
- 权限：`lean`/`lake`/`elan`/`leanc` 命令 deny（硬规则落地为配置）。

### Claude Code / 其他 harness

把 skill 目录链接进 harness 的技能根（`skills/` 本身不是标准位置，需要一次配置）：

```bash
# Claude Code（用户级）
ln -s "$PWD/skills/sokonanoda-teacher" ~/.claude/skills/sokonanoda-teacher

# 其他读取 ~/.agents/skills 的 harness（DSH 也扫这里，但与仓库内入口同源）
ln -s "$PWD/skills/sokonanoda-teacher" ~/.agents/skills/sokonanoda-teacher

# DeepSeek Harness：也可以不改仓库，直接把正文目录登记为额外技能根
#   $DSH_HOME/profiles/web/cordis.patch.yml
#   - id: skill-filesystem
#     config:
#       customSkillDirs: ["/abs/path/to/sokonanoda-lang/skills"]
```

其他 harness 若需要 LSP：`.opencode/lsp/sokonanoda-lsp.sh` shim →
`scripts/soko lsp`。

## 守护

- `cargo test -p sokonanoda-cli --test skill`：正文 frontmatter 合法、引用的仓库
  路径真实存在、事件/方法词汇封闭且与 `docs/protocol.md` 一致；
- `cargo test -p sokonanoda-cli --test dsh`：每个正文都有 `.agents/skills/` 入口
  （双向、无孤儿）、入口 frontmatter 只用 DSH 认的键、入口指向正文且路径存在、
  `dsh/cordis.patch.yml` 形状正确、`scripts/soko` 解析链完整且不写 `releases/latest`；
- 协议词汇变更时先改 `docs/protocol.md` 与 `crates/cli/tests/common/mod.rs`，
  conformance 测试会指出 skill 侧需要跟改的位置。
