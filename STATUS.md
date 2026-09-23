# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-21（第一百二十九轮：**逐 surface 的判别性** —— 线 C 收口并发版；
> 折叠开关 `SOKO_NO_NOTATION_FOLD=1` 实测 **3 红 3 绿**（与设计逐格一致）；
> 课程门禁 36 目标 · 328 checked · 99 open · 0 判负**逐项不变**；版本 **0.65.0**）
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

## 本轮进度（2026-09-23，第一百三十六轮：**T-D40 三层测试补齐 + `gate --fast` 修缺陷**）

1. **T-D40 三层测试补齐**（矩阵用例 #7/#8）。e2e 两条早在 T-D02/T-D10..T-D13
   就落地了，但三层里**缺两层**，这轮补上：
   * **LSP**：`goto_definition_on_a_notation_symbol_lands_on_its_declaration`
     —— `definition` 在 `∈` 上跳到**声明它的模块**那一行；
   * **front**：`notation_folding_does_not_clobber_a_use_points_resolution`
     —— 线 C 的折叠只动**显示副本**，点名使用点的 `resolution` 仍在。
   * **判据实跑**：`vscode-e2e.sh --grep "notation symbol" --profile debug`
     ⇒ **2 passed / 0 failed**（46s），台账进 `docs/e2e/ledger.jsonl`。
   * **一处如实记录**：front 那条最初想断言"记法符号自己的 hover 行没有
     resolution"，实测**这个夹具里没有正好落在 `⊆` 上的 hover 行** ⇒ 那半条是
     **空断言**，删掉换成"前提守卫 + resolution 仍在"两条真能失败的前提。
     **不凑数。**
2. **发现并修掉 `gate --fast` 的一个真缺陷**：它只看**未提交**的改动
   （`git diff HEAD`）⇒ **提交之后**跑就打印"crates/ 下没有改动"、**静默跳过所有
   单测**——而 `--fast` 恰恰最常在提交后跑。改成取三段并集（`origin/main...HEAD`
   ∪ 工作区 ∪ 未跟踪）；无远端时退回 `HEAD~1`。**造了一个"已提交未推送的 crates
   改动"验证过**：修后确实跑 `cargo test -p sokonanoda-front --lib`。
3. **下一环**：T-D14（parser 保留记法符号 token 的 span——AST 变更，为"表达式内
   跳转"铺路）。

## 本轮进度（2026-09-23，第一百三十五轮：**两条独立缺口收口（T-D30 / T-D31）**）

1. **T-D30 修掉一个正确性 bug**：`documentHighlight`/`references`/`rename` 在**记法
   符号**上会误解析到**外层 binder**——binder 的 span 覆盖**整段类型标注**
   （`(h : a ∈ A)`）⇒ 光标在 `∈` 上被当成 `h`，`rename` 会去改 `h`。
   两处回退（LSP 的 `highlight_uses`、front 的 `resolve_at`）都加了同一条守卫：
   **光标落在记法符号上就直接答"没有名字"**（判据走词法 `symbol_at`）。
   * 判据**两侧都带对照**：`∈` 上答 `None`、`highlight` 空、`rename` 被拒，
     而**同一个 binder 的 `h` 本身仍解析得到**（别把定义点那一支修坏）。
   * 踩到的坑：`character` 是**字符**计数，而 Rust 的 `str::find` 给的是**字节**
     下标——行里有 `α`/`∈` 时两者不等，第一版 `offset_of` 按字节算 ⇒ 守卫不触发。
2. **T-D31 按计划只做"立台账 + 判定实验"**：`docs/gaps/ledger.jsonl` 新增 **G-36**
   （`position_to_offset` 按 `char` 计数而非 LSP 的 UTF-16 码元 ⇒ `𝒫` 之后整行
   偏一格）。判定实验 `docs/gaps/repro/G36-utf16-position-mapping.sh` 起**真 LSP**
   证明：`𝒫 A` 的 `A` 在 UTF-16 列 44 时 hover 给的是**外层表达式**，而列 43 才给
   `A : Set α`。**两条纪律都是踩出来的**：夹具必须**编译干净**（否则红的原因是错误
   卡片不是位置映射）；判据**不能只看 `range`**（落偏时会退化成整行表达式，range
   照样覆盖光标 ⇒ 恒真），要看文本且**必须带对照**。真修单独立项。
3. **顺带修掉一条静默假红**：`editing_a_dependency_refreshes_the_open_entry` 在
   **全量** LSP 套件里失败且**没有任何 panic 文本**。三步排除法（单跑 5/5 过 ·
   只跑 `project` 组过 · `--skip perf_course` 全绿 · 只跑两组也过）⇒ 需要全量争抢
   才复现 ⇒ **资源饿死**（`perf_course` 整门课编一遍）。修法：重课程编译与
   时序敏感的跨文件刷新**共用一把锁**（`testutil::HEAVY_LOCK`），**断言一条没动**。
   修后全量 **156 通过 / 0 失败**。
4. **下一环**：T-D40（三层测试，矩阵用例 #7/#8）。

## 本轮进度（2026-09-23，第一百三十四轮：**线 D 的 hover 收口 + 发 0.65.2**）

1. **T-D03 hover 的"原始类型"三种形态齐了**：本文件声明（`⊗`）/ 语言内建（`∧`）/
   `import` 来的（`∈`）各一条测试。
   **"解析不出就不显示"钉在函数层**（`notation_input::target_resolution_tests`），
   不是 hover 层——因为**在能编译的文件里这条不可达**（认得出来的符号必有 target）。
   我试着加 LSP 级反向用例时构造不出"能编译 + 符号无 target"的文件，所以如实钉在
   函数层、**不硬凑假用例**；hover 那侧靠"那一行写在 `if let Some(target)` 里"
   结构性保证。
2. **⬆ BUMP patch → 0.65.2**（§0.2："用户可感知的能力落地"）。CHANGELOG 另记了
   本版包含的**记法跳转**（随 0.65.1 发布的 T-D10..T-D13）与**门禁提速**。
3. **发版闭环（用户新要求）**：推 main → CI → auto-tag → release →
   `gh release list` 核对。**上一版 v0.65.1 已确认上线**（26 资产、Latest、
   `Cargo.toml` 与之相等）；0.65.2 已推送，等 CI 与 release 产出后核对。
4. **bump 的已知代价实测**：bump 会让**编译缓存全失效**（缓存键含
   `CARGO_PKG_VERSION`）⇒ bump 后第一次完整 gate 从 5.25 分钟变成 **37.7 分钟**
   （课程门禁与测试套件都从头编一遍）。这是文档里记过的代价，不是回归；
   `gate --fast` 仍然 ~30s。
5. **下一环**：T-D14（parser 保留记法符号 token 的 span——AST 变更，为"表达式内
   跳转"铺路）。

