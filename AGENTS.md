# AGENTS.md — sokonanoda-lang

给任何 code agent 的项目入口（opencode / Claude Code 等原生读取本文件）。
按序读完再动手：

1. `docs/REQUIREMENTS.md` —— 用户全部要求的**权威总账**（硬规则、新要求追加到 §9）；
2. `docs/STATUS.md` —— 当前进度（最新一轮在最上）；
3. `ROADMAP.md` §10 —— 待办与验收标准；
4. `docs/architecture.md` —— 流水线与内核 gotchas（§8 必读）。

## 角色技能（Agent Skills）

- **当老师（产品主循环）**：加载 `skills/sokonanoda-teacher`——画布
  `playground.sokonanoda` 出题/判卷/决策的完整操作手册。
- **做开发**：加载 `skills/sokonanoda-dev`——冻结内核、TDD 三层、文档先行。
- **推代码/发布/查 CI**：加载 `skills/sokonanoda-ci`——本地验证纪律
  （退出码、无 grep 掩膜）、workflow 陷阱、`gh` 排错三板斧、失败必录。

## 硬规则速记（全文见 REQUIREMENTS §2/§3）

1. kernel 冻结快照：不改语义、不动热路径；bugfix 带三层回归测试；
2. 不调用官方 Lean 工具链（lean/lake/lean4export/elan）；
3. 教学语法是真实 Lean 4 的子集；新增语法 = 课程 + 测试 + 白名单三件套；
4. 判定永远走 kernel——**禁止文本比对**（tactic 判定范例：`front::judge`）；
5. 模块化：文件接近 ~500 行即拆分；公开 API 用 re-export 保持稳定。

## 命令（改动落盘前全绿）

```bash
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
```

编辑器/agent 反馈通道：`.sokonanoda` 文件的 LSP 诊断由完整 kernel 判定
（仓库根 `opencode.json` 已接线：`skills/` 自动加载、`/gate` `/check` `/round`
命令、`teacher` 主 agent、Lean 工具链命令 deny）；goal 视图走自定义请求
`soko/goals` / `soko/hints` / `soko/nextHole` / `soko/stateAt`
（`docs/protocol.md`）。

## VS Code 扩展改动

改 `editor/vscode/` 下的任何文件前，先读 `docs/vscode-dev-guide.md`
（版本纪律 / 测试三层 / 常见坑）。版本号必须随功能改动同步 bump。

## CI 失败记录

每次 CI 红了，在 `docs/CI-FAILURES.md` 追加一条（原因/修复/预防）。
同一类失败不犯第二次。

## 收尾义务

- 落 commit 前更新 `docs/STATUS.md`；
- 用户新要求追加进 `docs/REQUIREMENTS.md` §9 并注明日期（冲突以该文件为准）；
- 设计先行：新功能先写设计进 `docs/`，再动手；多用 subagent 并行调研。
