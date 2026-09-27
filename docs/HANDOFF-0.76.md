# 交接单 —— **v0.76.0「prelude 显式化 + 两跳」（E09–E11）起点**

> 写于 2026-09-27 深夜（第三个会话收尾；本会话已完成 v0.74.0 与 v0.75.0 两个发版点 ✓）。
> **唯一真相 = `docs/PLAN-0.74-0.79.md` 第 563 行那一节**（「🚀 v0.76.0」）——
> 动手前**逐字读它** ✓，本文件只是"起点快照 + 已钉好的坐标 + 陷阱"，**不取代它** ✗。

## 0. 交接时的状态（接手先认这几个数）

- `HEAD` = `63b4c6a` 之后（v0.75.0 发布收尾）· **工作树干净** ✓ · `origin/main` 与 HEAD **0/0** ✓；
- 已发布：**v0.75.0**（Latest ✓）· 上一版 v0.74.0 ✓ · 下一版就是 **0.76.0**（**不跳版、不动旧 tag** ✗）；
- 课程门禁 **36 目标 · 327 checked · 99 open · 0 判负** ✓；
- 本地全量 `scripts/ci-local.sh` 全绿 ✓；e2e 三平台 **35/35** ✓（`docs/e2e/ledger.jsonl`）；
- 相关台账：**G-53**（Infoview 记法符号点不动 · open）· **G-54**（跨模块目标跳不到 · wo-filed）·
  **G-55**（`documentHighlight` 目标名 · open）· G-48/G-49（`≈` 零元糖 / 裸 de Bruijn 编号）。

## 1. 三个环节与**已钉好的代码坐标**（省掉一轮 grep）

### E09 —— prelude 落成仓库内真实文件 + **字节相等守卫**

- 真相在**编译期常量**：`crates/front/src/compile/prelude.rs`
  · `PRELUDE_EQ_SRC`（**L164**）· `PRELUDE_L1_SRC`（**L199**）
  · `pub fn prelude_source()`（**L247**，`OnceLock` 缓存 `format!("{EQ}\n{L1}")` ✓）
  · `pub fn prelude_source_path()`（**L291**，能**物化**成临时文件 ✓ —— F12 到 prelude 就靠它）
  · 文件 839 行；`-- sokonanoda:prelude none|bare` ⇒ `PreludeMode::Bare` 的说明在 **L40** ✓
- **要做**：新增 `prelude/Prelude.sokonanoda`（或等价路径）**作为可读文本**，
  并加**守卫**：`prelude_source()` 的内容与那份文件**逐字节相等**，进 `scripts/soko gate` / CI ✓。
- ⚠ **红线**：**单一真相仍留在编译期常量** ✗✓ —— **别**改成运行时读文件（会碰 `PreludeMode::Bare` ✗）。
  守卫的方向是"常量 ⇒ 文件"（文件是**产物/镜子**，常量是源 ✓），不是反过来 ✓。
- ⚠ 先读设计：`docs/design/prelude-l1-proposal.md`（367 行）——
  重点 §2「让位规则」（核心机制）· §2.3「两个实测边界」· §3「三件套落地计划」· §4「影响面（两处 GOLDEN）」✓。
- **判红怎么取**：现在 `prelude/Prelude.sokonanoda` **不存在** ⇒ 直接贴 `ls`/`git ls-files` 的实测
  （或"学生点不到、只能看到临时物化文件"的实测路径 ✓）；守卫要先在**没有**那份文件时判红 ✓。

### E10 —— 内建记法 `∧ ∨ ↔ ¬ →` 写进 prelude 当**声明点**（双向）

- 现在内建记法是**硬编码表**：`crates/front/src/parser.rs::builtin_notations()`
  → `crates/front/src/notation.rs::builtin_notation_decls()`（**L71**）——
  它给每条 `NotationDecl` 填 `span: Span::default()`、`module: None`，
  注释原话：**「没有声明点（T-D10）：内建记法不在任何源文本里」** ✓✓（这就是 E10 要消掉的那句 ✗）。
- 消费方（别漏）：`crates/front/src/display.rs` **L925 / L1507 / L1531 / L1545** 都 `splice/extend`
  这份表；`semantic.rs` 的 `merge_known`（内建符号进词法表）✓。
- **要做**：把 `∧ ∨ ↔ ¬ →` 等**写进 prelude 源**（`infixr:35 " ∧ " => And` 这类），
  接通**双向**：名字 → 记法行 → 定义 → **回记法** ✓（E10 的原话：双向 ✓）。
- ⚠ **不许新增"记法形状"** ✗（会把记法概念分叉 ✗，与 E11 的第 ③ 条同一条纪律 ✓）。

### E11 —— 内建糖登记区（复用**已有** `-- sokonanoda:<指令>` 约定，**零新增语法**）

- 硬编码糖的**真相**在 `crates/front/src/compile/elab.rs`：
  · **L1330**：集合字面量 `{a}` / `{a, b}` → `Set.singleton` / `Set.pair` ✓
  · **L1562**：`Set.singleton`/`Set.pair` 的 `α := α₀`（元素类型）✓
  · **L2836**：`⟨a, b⟩` 的目标构造子**由期望类型的头决定**（"路线 C"）⇒ **没有单一目标** ✓✓
- **要做**：在 prelude/库里用 `-- sokonanoda:builtin-sugar "…" => …` **登记**这些糖
  （`"{a}" => Set.singleton` · `"{a, b}" => Set.pair` · `"⟨a, b⟩" => 期望类型决定` ✓）。
- ⚠ 三件事（PLAN 原话）：① `⟨a, b⟩` **无单一目标** ⇒ 登记要**如实**、**不许编** ✗；
  ② 与 `elab.rs` 的硬编码目标**逐字一致**，**加守卫** ✓；③ **不许为此新增"记法形状"** ✗。
- 指令解析：`-- sokonanoda:` 的匹配代码在 `session.rs` / `compile/mod.rs` 一带
  （`PreludeMode` 的消费点：`crates/front/src/{session,judge}.rs` + `compile/{mod,cache,goals}.rs`）；
  精确定位一条命令：`grep -rn 'sokonanoda:' crates/front/src --include=*.rs | grep -v '///'` ✓。

## 2. 每环节的**四件事**（用户口径，缺一不可）

① **判红**：贴实测原文（命令 + 输出），证明现在确实是坏的/不存在的；
② **一处一 commit**；
③ **判据与测试同步更新**（改了行为却没动测试 ⇒ **不算做完** ✗）——
   三层各司其职：front/CLI 单测 = **真相** · LSP 单测 = **wire 字段存在性** ·
   e2e = **用户看到的东西**；接缝（A 产出 / B 消费）要有 A∖B 对账守卫 ✓；
④ **反向验证**：撤掉改动 ⇒ 判据必须重新变红，且**报错与判红时逐字一致** ✓。

## 3. 环境与流程纪律（**都是本轮实测踩出来的**，别再交学费 ✗）

| 事项 | 做法 |
|---|---|
| **bump 后 pre-push 会红** ✗ | 判卷二进制与仓库版本不一致 ⇒ 课程门禁 **exit 2** ⇒ 先 `cargo build -p sokonanoda-cli --bin sokonanoda`，再 `SOKONANODA_BIN=$PWD/target/debug/sokonanoda git push`（不带它走 debug 全量：课程门禁实测 **626s** ✗） |
| **CI 的 `e2e ledger` 会往 main 回写** ⇒ 每次推都非 fast-forward ✗ | `git fetch origin && git rebase origin/main` 再推（**禁止 force push** ✗）；e2e 台账冲突：`ledger.jsonl` **取并集**（按 `date` 排序去重）、`latest.json` **取 theirs** |
| 轮询 | **≤90 秒**，**逐 job 判 `conclusion`**（matrix 是 `fail-fast: false`，盯整轮 = 白等 ✗）；`skipped` **不算绿** ✗（`fast-fail`、纯 docs 推的 `changes` 条件 job 除外 ✓） |
| 判真绿 | `python3 scripts/ci-green.py --run <id>` ⇒ **exit 0**（它会把"重活被 skip"判成 **假绿 exit 2** ✗✓ —— 纯 docs/editor 推常见 ✓） |
| 取 CI 日志 | 沙箱里 `gh` 要写 `~/.cache/gh` ✗ ⇒ `XDG_CACHE_HOME=/tmp/ghcache gh run view --job=<id> --log-failed`；失败详情在 artifact：`gh run download <run-id>` ⇒ `e2e-<os>-vscode-<ver>/logs/<date>-<sha>-*.log` 的「## 失败详情」✓ |
| e2e 写判据 | **"等 A 断 B"是经典假红且只在慢 runner 上现形** ✗✓ ⇒ 把"后面要读的**每一个**异步状态"都列进 `waitFor` ✓；快机器绿、慢机器红 ⇒ **先怀疑判据，别叫 flake** ✗；**rerun 只能证伪 flake、不能消红** ✓ |
| 动共享缓存的用例 | 清空项目缓存后**必须预热回去**（否则邻居用例前置判红，实测踩过 ✗） |
| 文档预算 | 放宽**可以**（用户 2026-09-27 拍板），但要在**同一个 commit** 记：哪个文件 · 从→到 · 为什么 · 「后续 refactor 时清理」✓；`docs/CI-FAILURES.md` 与 `docs/protocol.md` 有**冻结预算**（只许减不许增）⇒ 优先**压缩**而不是抬上限 ✓ |
| 长命令 | `timeout <s> <cmd> > /tmp/<名字>.log 2>&1 &` + tail 轮询（**禁止**干等 ✗）；编译只付一次，想 grep 几次就 grep 几次 ✓ |
| fmt | 只 `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp`（**绝不 `cargo fmt --all`** ✗ —— 会重排整个内核） |

## 4. 收尾（批次制）

整组 **E09–E11 全 ✅** + 本地 `scripts/ci-local.sh` 全绿 + **CI 逐 job 真绿**（`ci-green.py` exit 0）
⇒ `python3 scripts/bump.py 0.76.0`（**手写** `editor/vscode/CHANGELOG.md` 的 `## [0.76.0]` 一节 ✓）
⇒ 重建 CLI ⇒ 推 ⇒ 等 CI 真绿 ⇒ auto-tag ⇒ release ⇒ `gh release list` 核对 **v0.76.0 / Latest** ✓
⇒ 把版本号写进 `STATUS.md` 的「已发布」行与 PLAN 的「发版点 🚀 v0.76.0」行 ✓
⇒ 写 `docs/HANDOFF-0.77.md`（v0.77.0 的详情已拆到 `docs/design/v077-set-theory.md` ✓，先读它）✓。
