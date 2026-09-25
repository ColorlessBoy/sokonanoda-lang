---
name: sokonanoda-dev
description: Develop and extend the sokonanoda-lang teaching compiler stack (Rust workspace kernel/front/cli/lsp) safely - the frozen kernel, TDD three-layer testing, modularization limits, docs-first workflow and CI gates. Use when taking over development, adding syntax or compiler features, or touching CI/docs in this repo.
---

# sokonanoda-dev：接手 sokonanoda-lang 开发

## 0. 接手清单（按序读完再动手）

1. `AGENTS.md` —— 仓库根入口（硬规则速记 + 命令 + 收尾义务）；
2. `REQUIREMENTS.md` —— 用户全部要求的权威总账（含硬规则）；
3. `docs/HANDOVER.md` —— 交接汇总（现在在哪、还剩什么、怎么继续）；
4. `STATUS.md` —— 当前进度日志（最新一轮在最上）；
5. `ROADMAP.md` —— 里程碑与 §10 待办（I 系列编号）；
6. `docs/README.md` —— 文档地图（根入口 / `docs/` 顶层参考 / `design/` / `notes/`）；
7. `docs/architecture.md` —— 流水线、内核机制与 §8 gotchas；
8. `docs/design/` 设计文档 —— 已确认方案的 as-built 记录；近期实现见
   `goal-rendering` / `highlighting` / `compile-cache` / `match-patterns` /
   `indexed-inductives` / `by-tactics`（§11 换行分隔 tactic） /
   `elaborator-let-match` / `notation-subset`（用户自定义记法，0.59.0）。

## 1. 不可动摇的硬规则（REQUIREMENTS §2，违者返工）

1. **kernel 可以改**（2026-09-21 用户解冻，含热路径与内部表示，目的可以是提速）。
   唯一红线是**判定正确性不变**：同一批输入接受/拒绝不变、事件计数不变、
   golden 与 `--json` 逐字节不变。每次内核改动必须带三层回归
   （kernel `tests/` + front 单测 + CLI e2e）+ 语料对拍 + 课程门禁计数逐项不变；
   已改过的部分在 `docs/architecture.md` §6，**改之前先读它 + §8 gotchas**；
2. 无官方 Lean 工具链依赖（lean/lake/lean4export 一律不调用）；
3. 教学语法是真实 Lean 4 的子集；新增语法 = 课程 + 测试 + 白名单三件套；
   加新语法点照 `docs/TESTING.md` §4 的七步 checklist（lex → parse → elab/kernel
   → ErrorKind/`docs/protocol.md` → CLI e2e → 语料 → 白名单）。**记法是例外面**：
   `infix`/`infixl`/`infixr`/`notation`（0.59.0，设计 `docs/design/notation-subset.md`）
   不引入新语义、只做源到源重写，所以它的"课程"是使用者自己写的声明行；
4. 判定永远走 kernel：新增功能禁止文本比对（`front::judge` 是合成声明走
   完整流水线的范例）；
5. 反馈即功能：类型/化简/打印/错误都要结构化输出，人和模型都能无文档驱动。
6. 多文件项目（0.57.0，I16）：闭包编译在 `crates/front/src/project/`，错误
   **按命令下标**归属到文件（`CompileOutput.error_cmds`）——绝不能按 span，不同
   文件的偏移会互相命中；**无 `import` 的文件必须逐字节走原单文件路径**
   （A1，`crates/cli/tests/imports.rs` 守住）；内核一行未改（一个 arena 顺序
   跑完拓扑序的 unit）。设计：`docs/design/imports-and-projects.md`；架构
   §4.5；测试地图 `docs/TESTING.md` 的三行「多文件 …」。

## 2. 工作流（TDD 三层 + 文档先行）

- **先写设计**：新功能先出设计方案落 `docs/`（含取舍与验收标准），再动手；
- **测试三层**：front 单元测试（`crates/front/src/compile/tests.rs` 等模块内
  `#[cfg(test)]`）→ CLI e2e（`crates/cli/tests/`）→ 语料/协议/golden 守护
  （`examples.rs` / `protocol.rs` / `course.rs` / `skill.rs`）；多文件特性再加一层
  `crates/cli/tests/imports.rs`（真 CLI 跑两文件项目 + 缓存失效 + A1 字节一致）
  与 `course_project.rs`（`course` 对有 `import` 的单元走闭包：`failed` 与 `grade`
  退出码同判、计数与 `query check` 同源、模块根回退、闭包摘要与 `build` 共用）；
- **「签名/命名类」bug 的三层回归范例**（G-01 / WO-004 的形状，照抄即可）：
  ① front 单测一对**通过/失败边界**（合法签名仍 `exercise.open` + 0 诊断；
  坏签名 1 条诊断 + 声明 `Failed` + **没有** `exercise.open`）+ 一条 span 断言
  （诊断必须落在**签名**的源范围，见 G-15）；② CLI e2e 在 `protocol.rs` 里用
  **同一个来源**的两种写法做对照（`example : Prop -> Prop := sorry` exit 0 vs
  `theorem t : 3 := sorry` exit 1 + code + 无 `exercise.open`），
  并在 `query.rs` 钉「`query check` 的 `failed`/计数与 `--json` 事件流一致」；
  ③ 语料层跑真二进制**全仓扫一遍**（`python3 courses/set-theory/tools/check.py`
  + 入门课 `course.rs`/`course_status.rs` 的双 GOLDEN）——**预测「新增诊断 0 条」
  必须实测**，逐条判「真腐烂」还是「超出子集边界」；
- **多用 subagent**：探索/调研/机械重构派出去并行，主会话做核心设计编码，
  产出后主会话验证（编译 + 全量测试）;
- **模块化**：任何文件接近 ~500 行即拆分；公开 API 用 re-export 保持稳定；
- **交接友好**：落 commit 前先更新 `STATUS.md`；用户新要求追加进
  `REQUIREMENTS.md` §9 并注明日期，冲突时以该文件为准。
- **性能例行化**：动编译/项目/LSP 路径后跑 `scripts/perf-ledger.sh`
  （分阶段 `PERFJSON` → `docs/perf/ledger.jsonl`，提交这份记录）；口径与阈值
  原则见 `docs/PERF.md`「项目层与编辑器宿主」。扩展有独立的宿主层行为测试
  `editor/vscode/test-extension-host.js`（在 `npm run test:unit` 里）。
- **VS Code + skills 同步**：用户可见改动必须**同一轮**改 `editor/vscode/` **与**
  `skills/` 三个技能 + `AGENTS.md` + `docs/vscode-dev-guide.md`（测试 `crates/cli/tests/skill.rs`）。
- **code agent 适配是一等公民**：计划里先定「agent 怎么用/验证」——`--json` 结构化输出、
  写进 skills、命令可直接执行、`docs/HANDOVER.md` 同步。
- **skill 写法**：给**确切可执行的一条命令**，少 token，不写"建议/可以考虑"式散文。

## 3. 质量门禁（CI 与本地一致）

**两档**（2026-09-23 起，用户报"gate 太慢、严重阻碍迭代"）：

```bash
scripts/soko gate --fast   # **迭代内环 ~30s**：fmt + clippy + 改动过的 crate 的单测
                           #   + 锚点 + 课程门禁（持久缓存 ⇒ 秒级）；跳过缺口台账与集成测试
scripts/soko gate          # **提交/推送前**（~5 分钟）：上面那些 + 全量 test + 缺口台账
```

`--fast` 不是"更弱的判据"而是"更小的范围"（课程门禁照跑）；推送前必须跑完整那条。
细则与账见 `docs/vscode-dev-guide.md`「迭代速度」一节。

手动等价（需要单跑某一步时）：

```bash
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
# 禁止 `cargo fmt --all`：会重排**冻结内核**（kernel 快照不得改动）；
# 只 fmt 教学 crates，或直接 `scripts/soko gate`。
cargo clippy --workspace --all-targets      # 教学 crates 经 [lints] deny；kernel 只 warning
cargo test --workspace --locked             # 4 个 lib test target + 13 个集成测试文件
cd editor/vscode && npm run test:unit       # Infoview webview/server 纯 Node 行为测试
python3 courses/set-theory/tools/check.py --selftest   # 课程门禁（卷 I，判据 G1–G6；gate 已含，单跑用这条）
python3 scripts/gap.py selftest             # 台账判据自检（judge() 的期望推导 / repro_expect / 非法值）
python3 scripts/gap.py check                # 台账契约：缺口复现必须与 status 一致（gate 已含，单跑用这条）
```

- 教学 crates 的严格度来自各自 `Cargo.toml` 的 `[lints.rust] warnings = "deny"`；
- kernel 是冻结快照：其 lint 保持 warning 级，fmt 门禁不覆盖（rustfmt.toml
  需要 nightly）；
- `scripts/soko gate` 的 playground 锚点用**运行中二进制的内嵌编译器**；若它与
  仓库 `Cargo.toml` 版本不一致（旧下载缓存 / `target/` 旧构建），gate 会
  直接 exit 3 —— 先 `scripts/soko update`，或直接跑
  `cargo run -q -p sokonanoda-cli --bin sokonanoda -- playground.sokonanoda`；
- 编辑器测试三层：静态契约 `crates/cli/tests/extension.rs`（进 `cargo test`）、
  webview/stub 宿主纯 Node 行为（`cd editor/vscode && npm run test:unit`）、
  **真 VS Code 集成** `SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh`
  （自动构建 release + stage + 跑 + 记 `docs/e2e/ledger.jsonl`；手册 `docs/E2E.md`。
  改了 `editor/vscode/` 就跑它——被 ignore 的 `bin/` 会让测试静默测旧二进制）；
- 协议防漂移：改事件/输出格式必须同步 `docs/protocol.md`
  （`protocol.rs` / `skill.rs` conformance 测试会抓漂移）；
- **真相层不得绕过**（`docs/design/agent-query-channel.md`）：任何"问内核"的
  新能力都加在 `crates/front/src/query/`（`QueryDoc`），LSP / CLI `query` /
  MCP 只做**映射与传输**；禁止在适配器里重算目标/洞的位置（那会产生第二份
  真相，违反 §2.4）。门槛测试：`crates/cli/tests/query.rs` 的
  `query_check_counts_match_the_json_event_stream` 钉死 `query` ≡ `--json`；
  `crates/cli/tests/dsh.rs` 钉死 MCP 工具 ↔ `query` op 一一对应。

## 4. 环境搭建（新机器）

```bash
git clone https://github.com/ColorlessBoy/sokonanoda-lang.git && cd sokonanoda-lang

# 方式 ①（agent / headless，与 VS Code 解耦）：一条命令
scripts/soko setup    # 按版本钉下载 CLI + LSP（幂等；零 cargo）
scripts/soko doctor   # 0=就绪 3=未就绪；--json 机器可读
scripts/soko version --json  # version / version_source（env|version.txt|manifest.requires|Cargo.toml）
# 版本钉源链同上；课程仓（无 Cargo.toml）靠仓根 sokonanoda-version.txt；
# 解析不出期望版本时启动器拒绝 exec（G-11/G-16），别加 SOKONANODA_BIN 绕过。
# 网络受限时先 `export HTTPS_PROXY=…`（启动器经 curl 下载，会用它）。

# 方式 ②（开发必需）：源码编译（CLI + LSP + 全量测试）
cargo build --release --locked -p sokonanoda-cli -p sokonanoda-lsp
export PATH="$PWD/target/release:$PATH"

# 门禁（= CI：fmt + clippy + test + playground 锚点 **+ 课程门禁 + 缺口台账门禁**；
# 后两步要 python3，探不到即 exit 3——无法判定 ≠ 绿）
scripts/soko gate
```

Release 资产：**26 个**——`sokonanoda-lsp-<rust-triple>.tar.gz` ×8 +
`sokonanoda-cli-<rust-triple>.tar.gz` ×8（后者就是可直接执行的 CLI）+ VSIX ×9
（8 平台包内嵌两者 + universal 回退包）+ `SHA256SUMS`，每资产另附 SLSA
provenance。不要用 `releases/latest`——下载 URL 按仓库版本锁定，版本错配是
明确要避免的故障。**发版已全自动**：bump 两处版本（`Cargo.toml` +
`editor/vscode/package.json`）→ push main → `ci.yml` auto-tag 自动打 tag 并
dispatch `release.yml`（手动推 tag 仅应急，见 `docs/RELEASE.md`）。
完整入门设计见 `docs/design/onboarding.md`。

## 5. VS Code 扩展开发规范

`editor/vscode/` 的改动有独立开发规范：`docs/vscode-dev-guide.md`。
版本纪律（feature→minor / fix→patch）、测试三层（静态契约→集成→手动）、
常见坑（node_modules 打包/didOpen 通知/LSP 帧格式/代理）全在里面。
近期呈现/缓存设计（改编辑器/Infoview 前先读）：`docs/design/goal-rendering.md`
（goal 同源 + Infoview 落右侧辅助侧栏）、`docs/design/highlighting.md`
（高亮单一起源 + Infoview 自研固定色板）、`docs/design/compile-cache.md`
（共享缓存）；webview 行为测试在 `editor/vscode/test-webview.js`
（`npm run test:unit`）。**加一个用户可见命令 = 同一轮改五处**：`package.json`
（`contributes.commands` + 键位/菜单）、`extension.js`（注册 + 实现，子进程要有
超时 kill）、`crates/cli/tests/extension.rs`（静态契约：声明↔注册一致、键位指向
已声明命令、子进程纪律）、`editor/vscode/src/test/extension.test.js`（真宿主冒烟）、
`README.md`/`CHANGELOG.md` + 本技能/teacher 技能 + `AGENTS.md`（版本号随 feature
minor bump）。

## 6. 常用命令

```bash
cargo test -p sokonanoda-front compile::tests::       # 前端单测
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
cargo run -q -p sokonanoda-lsp                        # 编辑器反馈通道
```

- 编译缓存（dev aid）：`sokonanoda build [--json] [--clean] [<file>|<dir>…]`
  预热/清理共享落盘缓存（key = 编译器版本 + 二进制构建指纹 + prelude 模式 +
  源文本；内核仍是唯一判定者）；**项目**闭包产物落**模块根**
  `<模块根>/.sokonanoda/compiled/`（同格式同键、自忽略、上限 32 条按 mtime 淘汰、
  `--clean` 两处都清、逃生门 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`；
  设计 `docs/design/project-artifacts.md`，判据 `crates/cli/tests/artifacts.rs`）；
  `SOKONANODA_CACHE_DIR` 改缓存根、
  `SOKONANODA_NO_CACHE=1` 关闭；测试用临时 cache dir 隔离。设计
  `docs/design/compile-cache.md`。

内核/前端机制细节（arena 生命周期、prelude 与原生 Nat 技巧、conv 缓存、
EnvBuilder 语义）见 `docs/architecture.md`——不要凭直觉猜内核行为。
