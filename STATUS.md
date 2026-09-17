# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-17（第九十一轮：DSH 人工运维命令 `/sokonanoda-update` / `-doctor` 上架；
> 第九十一轮**续**：`redundant-sorry`（多余的 `sorry`）落地 = 5 分钟实验定位 + 内核
> 显式限界 + 会话 warning 快照 + 洞级 `redundant` 标记，**版本已 bump 到 0.56.2
> 待 push**；上一轮 0.56.1 = 清 LSP 测试文件债）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`；**harness 适配 = `docs/design/deepseek-harness.md`**；
> **agent 查询通道 = `docs/design/agent-query-channel.md`**（ROADMAP I15）。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-17，第九十一轮续：`redundant-sorry` 落地 —— 5 分钟实验定位 + 内核显式限界 + 会话快照）

> 用户：「`docs/design/redundant-sorry.md` §8 直接做那个 5 分钟实验」→ 实验一次定位
> 真根因（旧假说被推翻）；用户：「继续」→ 按修正后的修法落地并补三层验收。
> **设计到实现的全过程在 `docs/design/redundant-sorry.md`（§8.1 根因 / §8.2 实验 /
> §8.3 修法 / §8.4 验收）。**

1. **实验（决定性，三行证据）**：同一 env、同一份探针，`as_is` 报的未知常量地址
   **正好等于**环境里 `A` 的规范节点地址（身份没问题）；只把探针 `info.name` 换成
   下一条真实声明 `g`（`decl_idx = 15` = 该练习的 `env_before`）、`ty`/`val` 一字未动
   → `Ok(())`。**旧假说"`NamePtr` 身份不对"作废**。
2. **真根因**：`check_simple_declar` 用 `EnvLimit::ByName(d.info().name)` 定可见前缀，
   而 `EnvLimit::ByName` 对没进过环境的名字取 **0**（`env.rs:257-260`）；探针**故意
   不入环境** ⇒ 空环境 ⇒ 连 `A` 都 `unknown const`。真实声明没事是因为它的名字有
   `decl_idx`。
3. **kernel（只加不改语义，已进 `docs/architecture.md` §6 适配表）**：新增
   `ExportFile::check_declar_at(d, EnvLimit)` / `try_check_declar_at(d, EnvLimit)`；
   `check_declar` 保持原行为，批量路径（`run_session_inner`）显式传同一个 `ByName`
   ⇒ 行为逐字节不变、热路径零改动。回归：`crates/kernel/tests/memory_api.rs`
   `synthetic_declaration_needs_an_explicit_environment_limit`（同时钉住"名字定限界
   = 空环境"这个坑与 `ByIndex`/`ByName` 等价）。
4. **front**：`PendingOp::OpenExercise` 带上 pre-pass 已有的 `env_before`，终审改
   `try_check_declar_at(_, EnvLimit::ByIndex(env_before))`（与真实声明的 cutoff
   **同一个值** ⇒ sound：前瞻引用照样不可见），探针检查计入 `stats.kernel_checks`；
   两条验收测试摘掉 `#[ignore]`，另加一条**可见前缀护栏**（同一形状只把 `g` 挪到
   练习后面 → 前瞻引用不可见 ⇒ 不报；经验配对实测 1 条 vs 0 条）。
5. **会话/LSP 的真实缺口**：`session.rs` 每轮只用 `collect_warnings` 重算语法级
   warning，**把内核终审过的 warning 丢了**（LSP 因此看不到 `redundant-sorry`）。
   修法：`CmdSnapshot.warnings` 按 span 归属命令、随快照跨版本复用并做坐标重映射，
   `WarningKind::is_kernel_verified()` 明确区分两类；LSP 侧该声明**不再**叠
   "not yet solved"（洞 span 与 warning span 形状不同 → 用包含判定），真缺口照旧报。
6. **验收**：`cargo test --workspace --locked` **全绿**（kernel 8 / front 413 /
   lsp 118 / cli …，0 failed；6 ignored = 缺 fixture 的 kernel 老用例）；
   `scripts/soko gate` **PASS**。用户 playground 326–328 的形状现在产出
   `warning[redundant-sorry]`（span 收窄到那个 `sorry`），语义不变（仍
   `exercise.open`、退出码 0）。测试账：**新增 6 条**（kernel 1 / front 1 新 +
   2 条摘 `#[ignore]` / CLI 2 / LSP 1 / session 1）。
7. **同步 + 发版准备**：`docs/architecture.md` §6、`docs/protocol.md`（warning 码
   清单 + `holes[].redundant` 字段 + `query` 两张表）、`docs/TESTING.md`（新行 +
   总量 772）、`skills/sokonanoda-teacher/{SKILL.md,references/events.md}`、
   `dsh/mcp/server.js`（工具描述教 agent 读 `redundant`）、`dsh/README.md`、
   `editor/vscode/README.md`（"Honest warnings"）+ `extension.js` 注释。
   **版本已 bump 到 0.56.2（patch）**：`Cargo.toml` + `editor/vscode/package.json`
   两处 + `Cargo.lock`（`cargo check` 跟上）+ `editor/vscode/CHANGELOG.md`
   `## [0.56.2]`，并**重建 `target/`**（`sokonanoda 0.56.2`，启动器解析回
   `repo-build`；否则会撞 `docs/vscode-dev-guide.md` 陷阱 13）。**只剩 commit + push**
   （auto-tag `v0.56.2` → release）。
8. **洞级标记（`query` 层）**：`HoleInfo`/`LocatedHole` 加 `redundant`（判定来自
   **同一份**报告的 kernel 终审 warning，用与 LSP 相同的包含规则），`soko/goals`
   wire 同字段；`crates/cli/tests/query.rs` 的**两视图契约**逐字段对拍
   `query goals`/`holes` ≡ `soko/goals`（真缺口为对照组）。
9. **本轮产物**：`crates/kernel/src/{tc,util}.rs`、`crates/kernel/tests/memory_api.rs`、
   `crates/front/src/compile/{check,goals,tests,warning}.rs`、`crates/front/src/session.rs`、
   `crates/front/src/query/{mod,types,tests}.rs`、`crates/lsp/src/{lib,protocol,query_map,by_sorry_range_tests}.rs`、
   `crates/cli/tests/{protocol,cli,query}.rs`、`docs/design/redundant-sorry.md`、
   `docs/{architecture,protocol,TESTING}.md`、`skills/sokonanoda-teacher/*`、
   `dsh/{README.md,mcp/server.js}`、`editor/vscode/{README.md,CHANGELOG.md,package.json,extension.js}`、
   `Cargo.toml`/`Cargo.lock`、`REQUIREMENTS.md` §9、`STATUS.md`
   （+ 第八十九轮归档进 `docs/STATUS-ARCHIVE.md`）。

## 本轮进度（2026-09-17，第九十一轮：DSH 侧两个人工运维命令上架 + 真用出来的两个启动器 bug 修复）

> 用户：「deepseek harness 没有类似 opencode 一样的 command 机制吗？比如我这边输入
> `/sokonanoda/update` 就能执行升级命令。」→ 上架两个人工命令；随后用户连续真敲了
> `/sokonanoda-doctor` 与 `/sokonanoda-update`，**第一次真用就抓到启动器两个 bug**；
> 用户拍板「1 + 2」= 改手册 + 改启动器，同轮修完。**不动 Rust 语义代码、不 bump 版本。**

1. **先答机制，再动手**（核对当前 checkout 源码，不引旧笔记）：DSH **有**真命令
   注册表 `ctx.commands.register`，但**没有文件发现**（无 `.opencode/command/*.md`
   等价物、无 `.dsh/commands` 根），项目仓库零配置能自带的只有 **skills**；
   而两条命名文法**都禁止 `/`**——`COMMAND_NAME = /^[a-z][a-z0-9_-]*$/u`
   （`packages/interaction/commands/src/index.ts:32`）、
   `SKILL_NAME = /^[a-z0-9]+(?:-[a-z0-9]+)*$/`（`packages/skill/skill/src/index.ts:21`）、
   手势 `SKILL_GESTURE`（`packages/skill/tool-skill/src/index.ts:409`）。
   结论：`/sokonanoda/update` 在 DSH 里**物理上拼不出来**，等价物是
   **`/sokonanoda-update`**（平铺）。
2. **人工通道已实测核实**（这是本轮唯一的"行为假设"，逐行查过）：
   `tool-skill` 的手势边界只查 `isUserInvocable`（`src/index.ts:195`）再注入
   `renderSkillContent`，**与 `modelInvocable` 无关**；`/` 菜单用 `description`
   作标签、`!modelInvocable` 时前缀 `menu.userOnly`
   （`packages/client/ui-skill/src/client/index.ts:156`），行数据同时带 `whenToUse`
   （`packages/client/connection/src/client/fixture.ts:3894`）。
3. **上架两个人工命令**（用户三选一里选了最小集）：正文
   `skills/sokonanoda-update/SKILL.md`（何时需要 / 确切命令 / 判据"无 `STALE`、
   `marker` 等于 `Cargo.toml`" / 纪律）+ `skills/sokonanoda-doctor/SKILL.md`
   （只读诊断 / 退出码 0·3·其他 / 要汇报的字段 / 不擅自 `setup`）；
   薄入口 `.agents/skills/<name>/SKILL.md` 带 `user-invocable: true` +
   `disable-model-invocation: true`——**人可见、模型目录不可见**（模型侧等价能力
   已在 `AGENTS.md` 与三个角色技能里，不需要重复占目录）。
4. **刻意不改**：`.opencode/command/sokonanoda/{update,doctor}.md` 一个字没动
   （`opencode.rs` 守卫要求它们存在且 cargo-free）；opencode 用户行为不变。
   `setup`/`version`/`check`/`gate` 四个暂不铺 DSH 命令（需要时按同一模式增补），
   `docs/design/deepseek-harness.md` H3 as-built 已写明边界。
5. **文档同步**：`skills/README.md`（表格 + 人工命令小节 + DSH 命名文法说明）、
   `AGENTS.md`（Setup 的 DSH 条目 + 角色技能节）、`dsh/README.md`（能力表 +
   一分钟接入）、`docs/design/deepseek-harness.md` H3 as-built 与 §9 表。
6. **第一次真用就抓到启动器两个 bug（用户敲 `/sokonanoda-update` 时暴露）**：
   命令报 `[repo-build]`、**exit 0**、看起来正常，**实际一个字节都没写进缓存**。
   - **①静默成功**：`ensure(force)` 强制下载失败后 `resolve()` 兜底到"版本匹配的
     仓库构建"，两者都非空又不是 `cache(STALE…)` → 只打 `[repo-build]` 就 exit 0，
     而 `lastDownloadError` 只在"结果缺失或 STALE"分支才打印 ⇒ 断网/磁盘满/
     缓存只读**全都长得像成功**。修法：`ensure()` 回传 `refreshed`，`update` 在任一
     目标未刷新时打 `cache NOT refreshed` + 每个 `download: <原因>` + 实际回退并
     **exit 3**；`setup`（只承诺就绪）不变。
   - **②崩栈顶掉可行动消息**：`[cli, lsp].filter(r => r.source…)` 在 `cli === undefined`
     时抛 `TypeError`、exit 1 崩栈，使下面那段 "could not provide matching binaries …
     Next: allow network access" **永远不可达**（死代码）。修法：`r?.source`。
   - **根因实证**（修好后启动器自己吐出来的）：`download: EPERM: operation not
     permitted, copyfile '/tmp/sokonanoda-XXXX/sokonanoda' ->
     '~/.local/share/sokonanoda/bin/sokonanoda'`——**curl 下载与 tar 解包都成功，
     只有最后写缓存被沙箱拒绝**（DSH 会话 workspace-write 不管 `~/.local/share`）。
   - **红先测试**：新增 `crates/cli/tests/launcher.rs` = **首个真跑 Node 的行为契约**
     （`dsh.rs` 只断言文件形状，覆盖不到行为），3 条；红态实测 exit `0` / exit `1`，
     修后全绿。CI 缺 `node` 硬失败、本地缺则打印 skip（避免埋掉唯一的行为钉子）。
   - **手册同步**：`skills/sokonanoda-update/SKILL.md` 判据改为"退出码 + 缓存 `marker`
     + 缓存二进制自述版本"，**明确否掉** `source` 不含 `STALE` 这个会被任何回退满足的
     弱证据；新增"常见失败：缓存写不进去（EPERM）"与三种处置；DSH 薄入口、
     `AGENTS.md`、`skills/README.md`、`dsh/README.md`、`docs/TESTING.md`、
     `docs/LESSONS.md`（"静默成功是最坏的失败"）同步。
7. **验收**：`cargo test -p sokonanoda-cli --test dsh --test skill` = **8 + 4 全绿**；
   `--test launcher` = **3 全绿**（红先已留档：红态 exit `0` / exit `1`）；
   `cargo test --workspace --locked` = **759 passed / 0 failed**（= 上轮 756 + 新增 3）；
   `scripts/soko gate` **PASS**；`python3 scripts/check-site.py` ok（9 页、链接与版本
   干净）。**未改任何 Rust 语义代码、未 bump 版本**（接线层 + 启动器脚本改动）。
8. **本轮产物**：`skills/sokonanoda-{update,doctor}/SKILL.md`、
   `.agents/skills/sokonanoda-{update,doctor}/SKILL.md`、`crates/cli/tests/launcher.rs`、
   `scripts/soko`、`AGENTS.md`、`skills/README.md`、`dsh/README.md`、
   `docs/design/deepseek-harness.md`、`docs/TESTING.md`、`docs/LESSONS.md`、
   `REQUIREMENTS.md` §9、`STATUS.md`（+ 第八十八轮归档进 `docs/STATUS-ARCHIVE.md`）。

## 本轮进度（2026-09-17，第九十轮：清掉 HANDOVER §4 的 LSP 测试文件债 + 0.56.1 发布）

> 用户：「继续 handover 吧，完成之后再 bump」。本轮 = 清第八十九轮登记的两笔结构债
> 里剩下的那笔（`crates/lsp/src/tests.rs` 2567 行），然后 bump 发 **0.56.1**。

1. **测试文件拆分（零语义改动）**：`crates/lsp/src/tests.rs` 2567 行 →
   `crates/lsp/src/tests/` 一目录：`mod.rs` **399**（31 个共享 const/fixture +
   `pub(crate) use` 再导出，子模块靠 `use super::*;` 取用）+ 9 个特性文件
   （`hover` 392 / `lenses` 366 / `navigation` 280 / `state` 262 / `goals` 252 /
   `lifecycle` 242 / `hover_brackets` 167 / `tokens` 122 / `perf` 117），
   `by_sorry_range_tests.rs`（60）原地保留。`lib.rs` 仍 **1105 行**——
   `#[cfg(test)] mod tests;` 自动解析到 `tests/mod.rs`，一行未改。
2. **"移动而非改写"的证据**（这次也按上轮的标准自证）：HEAD 的 `tests.rs` 里
   **107/107 顶层 item 逐字出现在新文件**、8/8 banner 注释保留、规范化代码行
   多重集 **2394 == 2394**（only-in-old 0 / only-in-new 0）；函数名 **95/95 一致**、
   测试名各出现一次（76 个测试：72 `#[tokio::test]` + 4 `#[test]`）、
   assert 记账 **214（tests/）+ 6（by_sorry）== HEAD 的 214 + 6 = 220**。
   新增行只有 plumbing：模块 doc 4 行、`use super::*;` ×10、`pub(crate) use` 再
   导出块、`mod …;` ×9；编译期唯一被迫改动是去掉再导出里没人用的 `Value`。
3. **两轮验证**：拆分中途（全部子文件首次编译通过）与冻结最终态各跑一遍
   `cargo test -p sokonanoda-lsp --locked` = **117 passed / 0 failed**；
   `cargo fmt --check` exit 0（**首次 fmt 没有改动任何文件**）；
   `cargo clippy -p sokonanoda-lsp --all-targets` exit 0、`crates/lsp/**` 零 warning。
4. **HANDOVER §4 的债清零**：`docs/HANDOVER.md` 该条从"已知债 + 拆分方案"改为
   "0.56.1 已清 + 最终布局"；`docs/TESTING.md` 的 LSP 行、设计文档 §3.2 文件表、
   `ROADMAP.md` I15 的备注同步到 `tests/` 新路径与新行数。
5. **版本 0.56.0 → 0.56.1**（内部重构，无用户可见变更）：`Cargo.toml` +
   `editor/vscode/package.json` 两处同步、`editor/vscode/CHANGELOG.md` 记
   "内部重构（测试文件拆分），扩展行为不变"。按仓库流程 push main → `ci.yml`
   auto-tag `v0.56.1` → `release.yml` 出八平台产物 + VSIX + marketplace。
6. **验收**：`cargo test --workspace --locked` 全绿（756）、fmt 零 diff、
   clippy 教学 crates 零 warning、`scripts/soko gate` **PASS**；发布 job 全绿后
   用发布产物复验（同第八十九轮的做法）。
7. **本轮产物**：`crates/lsp/src/tests/`（10 个文件）、`docs/HANDOVER.md`、
   `docs/TESTING.md`、`docs/design/agent-query-channel.md`、`ROADMAP.md`、
   `REQUIREMENTS.md` §9、`editor/vscode/CHANGELOG.md`、两处版本号。
