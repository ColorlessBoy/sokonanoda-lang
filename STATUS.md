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

## 本轮进度（2026-09-23，第一百三十三轮：**发版闭环修通 + 迭代提速 30×**）

> 用户两条新要求都落到了实处：**BUMP 必须闭环（确认线上发版生效）** 与
> **gate 太慢严重阻碍迭代**。

1. **查出"发版停了三版"的根因并修通闭环**。`origin/main` 已经推到 0.65.1，
   而线上最新发布**还停在 v0.63.0** —— 中间三次推送的 CI 全红（e2e 的
   known-red 用例），而 `ci.yml` 的 **auto-tag 只在 CI 绿时发版** ⇒ 一个 tag
   都没打。**bump 在本地"完成"了、线上一步没动**，正是用户那条要求要防的事。
   * 修法：把线 D 的导航链落地（见下）⇒ e2e 从 **23/2** 变 **25/25**；
   * **闭环实测**：CI 绿 → auto-tag `v0.65.1` → release workflow success →
     **`gh release list` 第一行 = v0.65.1（Latest）**、**26 个资产**、
     `Cargo.toml` 版本与之**相等** ✓。
2. **线 D 导航链（T-D10..T-D13）**：`NotationDecl` 加 `span`/`module`
   （记法跨 `import` 传播，只有 target 不够）· 入口记法表进 `ProjectReport`
   （以前算完就丢）· `QueryDoc::notation_at` + `module_path` ·
   `goto_definition` 在声明表之前先试记法分支。
   **G-23 关账**（用户第 6 条反馈"记法不能跳转 / hover 无原始类型"整条修好）。
3. **迭代提速（用户报"gate 太慢"）——先量再改**：
   | 阶段 | 改前 | 改后 |
   |---|---|---|
   | 课程门禁 `check.py` | **164s** | **0s**（持久编译缓存） |
   | `cargo test --workspace` | ~250s | `--fast` 只跑改动过的 crate |
   | 缺口台账 `gap.py check` | 94s | `--fast` 跳过（提交前跑） |
   | **完整 gate** | ~15 分钟 | **5.25 分钟** |
   | **`gate --fast`** | — | **30 秒** |
4. **CI 的三条假红全是测试自身的时延假设**（不是产品回归，修它们时一行产品代码
   没改）：跨文件刷新的两条只等"被改的那份"、而下游是**异步**重发的 ⇒ 慢 runner
   上落到排水窗口外（改用 `did_change_at_drained_expecting` 等两份）；
   性能哨兵是**绝对秒数** ⇒ 同一用例本机 8.8s / CI 62s（150+ 用例并行抢 CPU）
   ⇒ 改成**机器无关的相对判据** `unit12/unit01 < 12`（本机 4.5×、CI 4.1×）。
   全部记进 `docs/CI-FAILURES.md`。
5. **文档**：`REQUIREMENTS.md` §9（新要求 + 背景）· `AGENTS.md` 命令区两档 gate ·
   `docs/vscode-dev-guide.md`「迭代速度」一节（含两档纪律与持久缓存的安全性）·
   `skills/sokonanoda-dev` 门禁一节。
6. **下一环**：**T-D03**（hover 的原始类型只对能解析出 target 的符号显示）
   ——它带 ⬆ BUMP(patch) 点，做完发 0.65.2 并**按新要求闭环确认**。

