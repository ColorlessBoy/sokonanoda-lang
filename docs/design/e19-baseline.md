# E19 基线（刀 0）：甲案（元变量）开工前冻结的读数

> **一句话**：E19 甲案（给求解器加元变量）**开工前**把"今天判成什么样"钉成数字 —— 每条都带
> **可复跑的命令**；三刀（§6/§7）只许在**红线**（§1）上逐字节不变，课程计数会**有意**变化。
> 评估与三刀计划 ⇒ `docs/design/e19-evaluation.md` §4；缺口本体 ⇒ 台账 **G-48**。

## 0. 冻结对象

| 项 | 值 |
|---|---|
| 冻结日期 | 2026-09-30 |
| `git rev-parse HEAD` | `2c20be09e6dbb9ef339f5c2457550634dedde05c` |
| 二进制 | `target/release/sokonanoda`（`cargo build --release -p sokonanoda-cli --bin sokonanoda`，1m32s） |
| 二进制 sha256 | `7ae7782e7abc86fc771e12cb2a0ad6642dd5023aa9765ea6e57f8474b74bb963` |
| 副本 | `/tmp/e19/sokonanoda-baseline`（**易失** ⇒ 丢了就按上两行重建 ✓） |

## 1. 红线（三刀 **必须**逐字节不变）

| # | 判据 | 命令 | 基线读数 |
|---|---|---|---|
| **R1** | 非课程语料逐字节对拍 | `bash scripts/kernel-diff.sh --non-course <前> <后>` | **172 组 0 差异**（86 文件 × 2 op：`grade --json` + `query check`）· 19.27s |
| **R2** | 非课程 `--json` 摘要 | `bash scripts/kernel-diff.sh --digest <bin>` | `43581e0650e217f69a81f6dbe928f4c82b583ba144c7eb344649ecadf68ae5e5` |
| **R3** | 课程 `--json` 摘要 | 同上 | `d0375577b5999b5b60ceb8606e16c5945115d35e02e47ed33e751eaee064454d` |
| **R4** | 全语料 `--json` 摘要 | 同上 | `0231dcc44fbc795882287d9872083c24e1fd12c1bdf69bdd99fc399e806ff68e` |

* **口径**：`--non-course` = `course/` + `examples/` + `docs/gaps/repro/`（**86 文件**）。
  ⚠ **画布 `playground.sokonanoda` 不算非课程**（它是"当前练习"、随教学变；算进去是 87×2=174，
  与台账里写的 172 对不上 ✗）—— 它只进 R4 的全量口径（129 文件）。
* **摘要**= 逐文件 `grade --json` + `query check` 的 stdout（含 stderr 与退出码）拼起来取 sha256，
  **心跳（`build.tick`/`build.progress`）不进口径** ✓。
* 三刀的**开关关**态：R1–R4 **四条全等**；**开**态：R1/R2 **仍必须全等**（非课程一个字不许动，
  **G-48 复现件本身除外** —— 它就是要变绿的那一份），R3/R4 允许变 ⇒ 变了几条要**逐条记账** ✓。

## 2. 三层判据（真相 / 契约 / 课程）

| 层 | 命令 | 基线读数 | 耗时（同机） |
|---|---|---|---|
| 真相（front） | `cargo test -p sokonanoda-front` | **769 passed / 0 failed**（lib）· 15 个 target 合计 802/0（2 ignored） | 执行段 1.53s · 墙钟 72.18s（含编译） |
| 契约（notation） | `cargo test -p sokonanoda-cli --test notation` | **49 passed / 0 failed** | 执行段 6.46s · 墙钟 11.44s |
| 课程门禁 | `SOKONANODA_BIN=<bin> python3 courses/set-theory/tools/check.py --json` | **43 目标 · 377 checked · 99 open · 0 判负**（canvas_open 96 · lib_open 0 · solutions_open 0） | 69.70s |

## 3. 目标缺口（G-48）

`docs/gaps/repro/G48-notation-nullary-sugar-operands.sokonanoda`：基线 **exit 1**
（`∅ ≈ {b}` 两侧都补不出论域）；刀1 开态 **exit 0** ✓。同轮复核 `python3 scripts/gap.py check`
⇒ 全部与台账一致 ✓（默认关 ⇒ G-48 仍 `open` + "仍有失败" ✓）。

## 4. 性能基线（**先计数后墙钟**；墙钟只兜数量级）

| 读数 | 基线 |
|---|---|
| 全课程冷 `build` 墙钟 | **56.67s**（`SOKONANODA_BUILD_JOBS=1 SOKO_STAGE_STATS=1 <bin> build --json courses/set-theory`，先 `build --clean`）· `compiled 42 / failed 0 / hit 0` ✓ |
| `JUDGE_PREFIX runs` / `bytes` | **905** / **47,866,553**（52,891 B/run）—— 结构计数，噪声免疫 ⇒ **退化先看这两个数** |
| `passes` / `by_calls` / `judge_ms` | **1362** / 21,551 / 23,826（`pass_total_ms=114,109` · `by_total_ms=37,084`） |
| `JUDGE_INFER` | calls **87,507** · hits/misses **86,602/905** |
| front lib / notation / 门禁 / 172 组对拍 | 1.53s / 6.46s / 69.70s / 19.27s |

⚠ 与 `docs/ONBOARDING.md` §1 的 47.8s / `runs=791` 不同：那组在第 518–519 轮语料变化**之前**
⇒ E19 的对照口径**只认本文件** ✓。⚠ **不许用绝对毫秒当判据**（同一份二进制两次跑差过 15%）✓。

## 5. 记法面守卫（刀1/刀2 都碰 `elab.rs` ⇒ 同轮复核）

`notation-lint` OK（91 文件零旧写法 · **587 处显式豁免**全带标记）·
`audit-notation-paths` OK（无新增绕过 · 命中 50 处 · 基线 46 条指纹是地板）✓。

## 6. 刀1 as-built（2026-09-30 **收口 ✓**）：记法**操作数位**的待定参数

**开关** `SOKO_NOTATION_METAVAR=1`（**默认关**）；回退 = `implicit::metavar_enabled()` 一处改回 `false`。
**实现**：记法前导参数在**最大候选**（操作数对齐到**最后** `operands.len()` 层 —— 语义上正确的
那一读）上失败时走**待定档**（`solve_prefix_args_pending`）：解不出的位先记待定（`?α`），走完由
`fill_pending_by_shape` 与**同形的已解兄弟**合一（`spine::same_shape`，忽略 span）。
⚠ 只放宽最大候选：更小的候选是**错位读法**（实测 `∅ ≈ {b}` 会掉到 `missing=1`，把 `{b}` 对到
`A : Set α` 上、解出 `α := β`，再让 `∅` 拿到期望类型 `Type` ⇒ 报"补不出参数" ✗）。
**语义**：开态 `∅ ≈ {b}` 读作 **`Set.Equiv β β ∅ {b}`**（论域**跟同形的已解兄弟**，是**选出来的**）；
`∅ ≈ ∅`（无兄弟可依）**仍判红** ✓ —— **用户 2026-09-30 拍板接受** ✓。

| 判据 | 关态（默认） | 开态 |
|---|---|---|
| G-48 复现件 | **exit 1**（逐字节不变） | **exit 0**（两条声明都 checked） |
| 非课程 172 组 / §1 摘要 | **0 差异** ✓ · 三个 sha256 **逐字节等于刀0** ✓ | **2/172**（恰好 G-48 那一份）· 课程摘要**不变** |
| front / notation / 门禁 | **770/0** · **49/0** · **43/377/99/0** ✓ | 门禁 **43/377/99/0** ✓ |
| 新判据 | `crates/cli/tests/notation_metavar.rs`（开绿 / **反向**关红 / **不猜**）✓ | — |

## 7. 刀2 as-built（2026-09-30 **收口 ✓**）：推广到 `solve_prefix` 一般路径

**开关**：与刀1 **共用** `SOKO_NOTATION_METAVAR=1`（**默认关**，用户拍板"沿用同一个开关" ✓）。
**实现**：机制**上移一层**到 `implicit::solve_prefix`（应用 / 裸常量 / `by` 块的 `apply` 共用它）——
先跑**严格档**（既有行为），失败且开关开 ⇒ 再跑**待定档**（`solve_prefix_pending`）+ 同一份
`fill_pending_by_shape`；刀1 的 `fill_pending_by_shape`/`metavar_enabled` **移进 `implicit.rs` 共用** ✓
（`solve_prefix` 的返回形状与三个调用方**都没动** ✓）。
**病根**（与 G-48 同形、入口不同）：`Set.Equiv {α β : Type} (A : Set α) (B : Set β)` 写成
`Set.Equiv ∅ {b}` ⇒ `α` 只能从 `∅` 的类型解，而 `∅` 又要靠期望类型才定论域 ⇒ 鸡生蛋。

| 判据 | 关态（默认） | 开态 |
|---|---|---|
| 一般路径夹具 `Set.Equiv ∅ {b}` | **判红**（`elab-implicit-argument-unsolved`） | **绿**（类型 `∀ β b, ¬ Set.Equiv β ∅ {b}`，`α := β`） |
| `Set.Equiv ∅ ∅`（无兄弟可依） | 判红 | **仍判红** ✓（不猜） |
| 非课程 172 组 / §1 摘要 | **0 差异** ✓ · **三个 sha256 等于刀0** ✓ | **2/172**（**仍是** G-48 那一份 ⇒ 刀2 没再改任何语料）· 与刀1 开态摘要**逐字节相同** |
| front / notation / 门禁 | **771/0**（+1 真值层判据）· **49/0** · **43/377/99/0**（80.60s） | 门禁 **43/377/99/0**（79.93s） |
| 冷 `build` 结构计数 | `runs=905` / `bytes=47,866,553` · `passes=1362` · `by_calls=21,551` · `JUDGE_INFER 87,507` —— **逐项等于刀0** ✓ | `runs=886`（−19）· `bytes=47,437,669` · `passes=1343` · `by_calls=21,268` · `JUDGE_INFER 86,440` ⇒ **判定结果不变**（计数/摘要逐项相同）而**工作量略降** ✓ |

**既有判据逐条重审**（`ElabImplicitArgumentUnsolved` 的**全部**判据面 —— 刀2 只该"多解出能解的"，
**不许**把"解不出"也放过去 ✗）：

| 判据 | 关态 | 开态 | 结论 |
|---|---|---|---|
| `crates/cli/tests/notation.rs::an_unsolvable_implicit_argument_reports_its_own_code` | 报码 ✓ | **仍报码** ✓（写进刀2 判据④，夹具逐字相同） | 不动 ✓ |
| 同文件 `implicit_arguments_are_inserted_from_the_first_explicit_argument`（正向） | 绿 ✓ | 绿（接受面只会更宽） | 不动 ✓ |
| 同文件 `the_at_marker_disables_implicit_insertion` | 绿 ✓ | 绿（`@` 那条路**不进**求解器） | 不动 ✓ |
| `docs/protocol.md` 的码 / hint 契约 | 不变 | 不变（**不加码、不改文本**） | 不动 ✓ |

**新判据**：`crates/cli/tests/implicit_metavar.rs`（一个 `#[test]`、四条断言：开绿 · **反向**关红 ·
**不猜** · **既有判据开态仍红**）+ 真值层
`implicit.rs::pending_solver_unifies_same_shape_siblings_and_never_guesses` ✓。

## 8. 复跑

```bash
cargo build --release -p sokonanoda-cli --bin sokonanoda
bash scripts/kernel-diff.sh --digest ./target/release/sokonanoda
bash scripts/kernel-diff.sh --non-course ./target/release/sokonanoda ./target/release/sokonanoda
SOKONANODA_BIN=./target/release/sokonanoda python3 courses/set-theory/tools/check.py --json
SOKO_NOTATION_METAVAR=1 ./target/release/sokonanoda docs/gaps/repro/G48-notation-nullary-sugar-operands.sokonanoda
```
