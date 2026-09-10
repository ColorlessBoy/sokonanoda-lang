# AGENTS.md — sokonanoda-lang

给任何 code agent 的项目入口（opencode / Claude Code 等原生读取本文件）。
按序读完再动手：

1. `docs/REQUIREMENTS.md` —— 用户全部要求的**权威总账**（硬规则、新要求追加到 §9）；
2. `docs/STATUS.md` —— 当前进度（最新一轮在最上）；
3. `ROADMAP.md` §10 —— 待办与验收标准；
4. `docs/architecture.md` —— 流水线与内核 gotchas（§8 必读）。

## Setup（30 秒，用户/agent 零 cargo；设计见 `docs/design-onboarding.md`）

```bash
bash scripts/soko.sh setup    # 幂等下载版本锁定的 CLI + LSP 到缓存
bash scripts/soko.sh doctor   # 就绪诊断；--json 机器可读，0=就绪 3=未就绪
bash scripts/soko.sh grade playground.sokonanoda   # 判卷（CLI --json）
```

- opencode 里等价命令：`/sokonanoda/setup` `/sokonanoda/doctor` `/sokonanoda/check`；
  启动插件会自动跑一次 setup 并把缓存目录注入 PATH；
- 贡献者（需要 Rust）：`cargo build/test` 或 `bash scripts/soko.sh gate`
  （见 `skills/sokonanoda-dev`）；
- 禁止：`releases/latest`、为使用仓库安装 Rust/cargo（REQUIREMENTS §2 第 9 条）。

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
5. 模块化：文件接近 ~500 行即拆分；公开 API 用 re-export 保持稳定；
6. **用户/agent 路径零工具链依赖**：获取与运行只用 Release 二进制或平台
   插件，不把 cargo/Rust 当使用前提（cargo 仅贡献者开发需要；见
   REQUIREMENTS §2 第 9 条）。

## 命令（贡献者：需要 Rust；用户/agent 用 `scripts/soko.sh`）

```bash
bash scripts/soko.sh gate     # = CI 门禁：fmt + clippy + test + playground 锚点
# 或手动：
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
```

编辑器/agent 反馈通道：`.sokonanoda` 文件的 LSP 诊断由完整 kernel 判定
（opencode 由启动插件自动接线：解析原生 `sokonanoda-lsp`——仓库构建 / VS Code
扩展自带 / 缓存 / 版本锁定下载（`fetch`+`tar`，跨平台、零 bash）——并改写
`lsp.command`；`shell.env` 注入 PATH；`opencode.json` 不再含 lsp 命令。
非 opencode harness 可用 `.opencode/lsp/sokonanoda-lsp.sh` shim →
`scripts/soko.sh lsp`。`skills/` 自动加载、`/sokonanoda/*` 命令、`teacher`
主 agent、Lean 工具链命令 deny）；goal 视图走自定义请求
`soko/goals` / `soko/hints` / `soko/nextHole` / `soko/stateAt`
（`docs/protocol.md`）。每个 release 仍正常产出各平台
`sokonanoda-lsp-<triple>.tar.gz` 与 `sokonanoda-cli-<triple>.tar.gz`
（各 8 个；CLI tarball 里就是可直接执行的二进制，agent 无需 cargo）与
VSIX（9 个，平台包内嵌 LSP 与 CLI），供自动下载与 headless 手动安装；
**下载一律按仓库版本锁定，禁用 `latest`**。

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
