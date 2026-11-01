# P3 **第一刀**：strict 专属文本通道盘点（2026-11-01）

> 实施单：[`reports/2026-11-01-p3-spec.md`](2026-11-01-p3-spec.md) · 上游 `replan §4` 的 P3 行 ·
> `REQUIREMENTS.md` §9.39（禁止 print→parse）。
> 分支 `strict/2026-10-29-kernel-app`，读数时 HEAD = **`df7cb01b`**（S4d `88b99539` + 发版实施单已并入；
> S4d 只动 `kernel_app_fallback.rs` / `kernel_app_report.rs` / `cli/tests/strict_diagnostics.rs`，
> **没碰**本表 30 个点所在的 5 个文件 ⇒ 行号与计数不受影响，脚本复跑同值 4/25/1 佐证）。
> 产物：[`scripts/audit-strict-paths.py`](../scripts/audit-strict-paths.py)（489 行，已进 `gate --fast`）。

## 0. 一句话

**strict 专属项 = 0 ⇒ P3 第二刀要删的 strict 专属文本调用 = 0 个。**
30 个盘点点逐条归类：**21 条共用**（其中 `elab.rs:4853` 在 strict 档实测 0 次、只在 loose 跑到）·
**4 条定义** · **1 条 `use` 导入** · **4 条兄弟函数的内部接线**；**strict 专属 0 条**（脚本与归类表同结论）。

⇒ 本刀把 P3 的"删"对象**改写**了：不是因为"strict 专属还剩几个"，而是**一个都没有** ——
30 个点里 **10 个是真调用点**（`operand_type_expr_explicit(` 2 · `infer_type_text(` 7 · `pp=explicit` 1），
**10 个全部落在 loose 也在跑的路径上**。所以第二刀若按原计划"删 strict 专属"，删除列表是空的；
真要削文本通道，对象是**老路本身**（`try_implicit_application_with` + 兄弟函数），
顺序与风险见 §5 —— 那已经不是"strict 清理"，是 P4 的老路拆除。

## 1. 先把两个数字对齐：**提及 4/25/1（= 30）** vs **调用 2/7/1（= 10）**

实施单 §1 的 30 = **裸名字出现在代码行**（跳过 `//` / `///`）；旧脚本的 `2/6/1` 与
本分支实测的 `2/7/1` = **`name(` 的精确调用语法**。两者都能复现（`--all` / `--json` 同时给两把尺）：

| 入口 | 提及（代码行） | 调用（`name(`） | 差 = 什么 |
|---|---:|---:|---|
| `operand_type_expr_explicit` | **4** | **2** | 1 条定义（`elab.rs:4116`）+ 1 条 `use`（`named_arg.rs:36`） |
| `infer_type_text` | **25** | **7** | 1 条定义 + 3 条兄弟函数定义（`_inplace` / `_inplace_expected`）+ 14 条兄弟函数/内部调用（`_inplace` / `_inplace_expected` / `_at` / `_at_peeled` 都含这个子串） |
| `pp=explicit` | **1** | **1** | 同一条（`judge.rs:2190` 的缓存键字面量） |
| **合计** | **30** | **10** | 4 定义 + 1 `use` + 4 内部接线 + **21 共用（10 个真调用全在这 21 条里）** |

⚠ **棘轮按"提及"计基线**（30 条全在表里，新增一条就红）；`--json` 同时报"调用"数（10）便于对旧脚本的账。
本分支比旧分支多 1 条调用（`2/7/1` vs `2/6/1`）：两棵树的分文件记账不同
（旧 `elab.rs 4 + named_arg.rs 1 + type_oracle/level.rs 1`；本 `elab.rs 6 + named_arg.rs 1`，
逐条见 §3）—— 无论归因是搬家还是新调用点，**没有一条是 strict 专属**（判据见 §4）。

## 2. 判据与方法（可复现、可复核）

### 2.1 静态判据（三条，实施单 §2.2）

1. **调用链上有没有 dialect 守卫**：`dialect.strict()` / `kernel_app::effective(dialect)` /
   `implicit_msg::strict_at_enabled(options)`；
2. **是不是老路共用函数**：`try_implicit_application_with`（唯一调用方 `elab.rs:5719` 的 `App` 臂，
   **无条件调用**）、`by.rs` 的 tactic 机械、`named_arg.rs` 的具名实参、`judge_infer*`；
3. **往上走到 dialect 判断**（`git grep` 调用者逐个复盘）。

### 2.2 运行时读数（**临时探针，已按纪律删除**）

结构推断不够，用读数钉住"这个点到底在哪个档跑"：

* 探针形状：在 4 个函数（`infer_type_text` / `infer_type_text_inplace` / `infer_type_text_inplace_expected` /
  `operand_type_expr_explicit`）、`judge.rs` 的显式 pp 两处入口 + 缓存键、`implicit.rs` 的 alt 求解器入口
  各加 `#[track_caller]` + `SOKO_P3_PROBE` 开关的 `eprintln`（打印**原始调用方** `file:line` 与 `dialect`）；
* 语料：**10 个课程文件**（`units/I.1/unit01` · `I.2/unit05/06/07` · `I.3/unit08/10` ·
  `I.4/unit55` · `I.5/unit13` · `I.10/unit24` · `lib/Logic`），每个文件跑 **loose（`SOKO_STRICT_AT=0`）**
  与 **strict（`=1`）** 两档，`SOKONANODA_NO_CACHE=1`（防磁盘缓存把第二档的调用吃掉）；
* 读数规模：探针行 **loose 76,650 / strict 24,649**（strict 档依赖文本通道的强度约为 loose 的 32%）；
  每个文件的 `.probe` 里 `strict=` 字段与档位逐行一致（抽查 unit01：loose 6,955 行全 `strict=0`，
  strict 1,945 行全 `strict=1`）。

### 2.3 构建身份与还原证据（探针纪律）

| 项 | 值 |
|---|---|
| 探针版构建（**dev profile**） | `4490ec249ef52ed6`（`cargo build -p sokonanoda-cli`） |
| 正式 release（**未被触碰**） | `b213488c786075d7`（= S6 读数用的那份；探针只进 debug，`target/release` 没重编） |
| 还原后的 debug 构建 | `f82215630a981bd8` |
| 探针残留 | `grep -rn 'SOKO_P3_PROBE\|p3probe' crates/` = **0** |
| `docs-lint ⑨` | **绿**：`elab.rs 8841` · `implicit.rs 1218` · `judge.rs 4748`（`python3 scripts/docs-lint.py` exit 0） |

> 探针只用于本表的"loose/strict 次数"一列；**没有留在树里**，也不进任何判据。

## 3. 30 个点的归类表

`loose/strict` = 上述 10 文件语料里的**命中次数**（`0/0` = 语料未触达，判据为静态）；
"真调用"= 是否 `name(` 语法（棘轮的硬口径）。

| # | 文件:行 | 所在函数 | 入口 | 归类 | 判据（静态 + 读数；真调用标 ✓） |
|---:|---|---|---|---|---|
| 1 | `elab.rs:4116` | `operand_type_expr_explicit` | operand | **定义** | 通道本体（全显式 pp 第二候选） |
| 2 | `elab.rs:4853` | `try_implicit_application_with` | operand ✓ | **共用** | 老路 `alt_type` 第二候选，门是 `second_candidate_enabled()`（`SOKO_METAVAR≠0`），**与 dialect 无关**；loose **654** / strict **0**（strict 只在回落后才可能进这里，且本语料主候选全解出） |
| 3 | `named_arg.rs:36` | `<top>` | operand | **导入** | `use`（无运行时） |
| 4 | `named_arg.rs:259` | `apply` | operand ✓ | **共用** | 具名实参 `apply` 的第二候选；具名实参两档都收（`implicit_named_args.rs` 的正对照含 `@g Nat 5`）⇒ 不按 dialect 分叉；语料 0/0（课程无具名实参），静态判据 = `apply` 由 `elab.rs:4462` 无条件调用 |
| 5 | `apply_probe.rs:181` | `level_hint_of_inplace` | infer ✓ | **共用** | tactic `by` 的层级提示（`by.rs:353`）；**54/54** |
| 6 | `apply_probe.rs:198` | `level_hint_of_inplace` | infer ✓ | **共用** | 同上第二问法；**52/52** |
| 7 | `by.rs:386` | `restore_universe_levels` | infer ✓ | **共用** | tactic 重建脊的层级恢复；**52/52** |
| 8 | `by.rs:2523` | `apply_tactic` | infer ✓ | **共用** | `apply`/`cases` 的期望类型就地路；**266/266** |
| 9 | `by.rs:2533` | `apply_tactic` | infer ✓ | **共用** | tactic 另一分支（`cases` 被消去项）；语料 0/0（静态：`by.rs` 整条 tactic 机械不看 dialect） |
| 10 | `by.rs:2569` | `apply_tactic` | infer ✓ | **共用** | 同上（另一处 `_expected`）；语料 0/0 |
| 11 | `elab.rs:2240` | `application_arg_expected` | infer ✓ | **共用** | 应用实参期望类型（`elab_expr` 的 `App` 臂，`try_implicit_application_with` 返回 `None` 后**两档都继续**）；**5035/4937** |
| 12 | `elab.rs:2271` | `universe_level_text_of_operands` | infer ✓ | **共用** | 记法层级（`notation_levels.rs:26` + recursion）；**4304/3857** |
| 13 | `elab.rs:2300` | `universe_level_text_of_operands` | infer ✓ | **共用** | 同上的就地第二问；**4304/3857** |
| 14 | `elab.rs:3499` | `inplace_render_type` | infer ✓ | **共用** | `judge_render_type_inplace` → tactic（`by.rs:857/925` 的 `cases`）；语料 0/0（静态） |
| 15 | `elab.rs:3518` | `infer_type_text_inplace` | infer | **定义** | 就地通道本体 |
| 16 | `elab.rs:3530` | `infer_type_text_inplace` | infer ✓ | **内部接线** | 就地通道内部（带期望类型版），由上面各入口驱动 |
| 17 | `elab.rs:3546` | `infer_type_text_inplace_expected` | infer | **定义** | 就地通道（期望类型版）本体 |
| 18 | `elab.rs:3655` | `infer_type_text_inplace_expected` | infer ✓ | **内部接线** | 通道内部的 `at` 变体 |
| 19 | `elab.rs:3675` | `infer_type_text` | infer | **定义** | 慢路通道本体 |
| 20 | `elab.rs:3740` | `infer_type_text` | infer ✓ | **内部接线** | 慢路内部的就地分支（`InplaceMode::On`） |
| 21 | `elab.rs:3850` | `infer_type_text` | infer ✓ | **内部接线** | 慢路内部的影子分支（`InplaceMode::Shadow`，只比对） |
| 22 | `elab.rs:4070` | `operand_type_expr` | infer ✓ | **共用** | 操作数类型（应用/记法/pp 共用；两档都问）；**29262/101** —— loose 的最大宗 |
| 23 | `elab.rs:4122` | `operand_type_expr_explicit` | infer ✓ | **共用** | 全显式 pp 通道本体，只由 #2/#4 驱动；**654/0**（同 #2） |
| 24 | `elab.rs:7528` | `infer_expected_level` | infer ✓ | **共用** | match/期望类型层级（两档）；**5/5** |
| 25 | `elab.rs:7566` | `infer_expected_level` | infer ✓ | **共用** | 同上（另一分支）；**5/5** |
| 26 | `elab.rs:7651` | `infer_recursor_universes` | infer ✓ | **共用** | 递归子消去层级（`Nat.rec`/`And.rec` 一类）；**6/6** |
| 27 | `elab.rs:7845` | `infer_const_universes_with` | infer ✓ | **共用** | 常量宇宙层（应用臂两档都进）；**62/5** |
| 28 | `named_arg.rs:144` | `next_arg_expected` | infer ✓ | **共用** | `@` 脊与 `apply` 共用的"下一实参期望类型"（全显式 pp 档，逃生门 `SOKO_KERNEL_ARG_PP=default`）；**1094/906** |
| 29 | `judge.rs:2190` | `judge_infer_cache_key` | pp ✓ | **共用** | 键由 `explicit_pp_active()` 选（线程局部），**不由 dialect 选**：loose 也走全显式 pp（#2/#4/#28）；**13401/1578**（= 全显式 pp 键的构建次数） |
| 30 | `judge.rs:2964` | `judge_infer_inplace_with_explicit` | infer ✓ | **共用** | 全显式就地判定的受控入口（由 #19/#28 调用）；随调用者，**654/0**（strict 0 与 #2 同因） |

## 4. 为什么"strict 专属 = 0"（每个"不是 strict 专属"的判据）

1. **老路不是 strict 专属的**：`try_implicit_application_with` 的**唯一**调用点是 `elab.rs:5719`（`App` 臂），
   **无 dialect 守卫**；strict 档只是先跑 `kernel_app::try_app`（`effective = dialect.strict()`），
   解出就早退、**回落**（`Report::Fallback` 例外表 + `Unsupported`/`NeedArgs`/`ExpectedMismatch`）就继续跑老路。
   ⇒ 老路上的文本调用（#2/#11/#12/#13/#22/#23/#26/#27）**两档都会到**；
   读数也证实：strict 档 `application_arg_expected` 仍问 **4,937** 次文本、`operand_type_expr` 仍问 **101** 次；
   再往下一层，**老路的隐式前缀求解器**在 strict 档也真的跑（额外一次入口探针，loose/strict）：
   unit01 **3,049/28** · unit07 **1,548/11** · unit55 **3,290/28** —— strict 只是「先让内核路径答」（答出就早退），
   **回落之后同一段老路照跑**，不是「strict 不进老路」。
2. **兄弟函数不是 strict 专属的**：`infer_type_text_inplace(_expected)`（#13/#16/#18）由 `by.rs`（tactic）、
   `apply_probe.rs`（层级提示）、`infer_expected_level`（match）驱动 —— 这些机械与 dialect 无关，
   读数里 **loose = strict**（54/54 · 52/52 · 266/266 · 5/5 · 6/6）。
3. **`pp=explicit` 不是 strict 专属的**：它是 `explicit_pp_active()` 的键标记；`next_arg_expected`（#28）
   与第二候选（#2/#4）**在 loose 也请求全显式 pp** ⇒ loose 13,401 次、strict 1,578 次。
4. **唯一的"档位不对称"点是 #2/#23（`operand_type_expr_explicit`）**：loose 654 / strict 0。
   但这是 **loose 专属**（老路的第二候选），不是 strict 专属 —— 删它**会改 loose**（`Eq.symm`/`Eq.subst`
   一类靠它解前导 `α`），风险方向正好相反。
5. **`kernel_app*.rs` 才是 strict 专属模块**（`effective(dialect) == dialect.strict()`），
   而它们**一条三个入口的调用都没有**（脚本的 `STRICT_ONLY_FILES` 规则今天命中 0 条）——
   这就是"strict 专属 = 0"的结构性原因。

## 5. 第二刀的删除顺序建议（若仍要删文本通道）

**前提**：strict 专属 = 0 ⇒ **没有"删了只影响 strict"的项**。下面按"删掉后谁受影响"排序，
目标是"先削 loose 依赖最小、再削最大"（不是本刀的执行项；本刀**不删任何判定代码**）：

| 顺序 | 对象（#） | 删掉会动谁 | 前置条件 / 风险 |
|---:|---|---|---|
| 1 | #14/#18/#30 的 `cases`/渲染就地路（tactic 显示与 cases 期望类型） | **loose + strict 的 tactic**（`by.rs`） | 必须先让 `judge_render_type_inplace` 有非文本替代；否则 `cases` 回退慢路（只慢，不改判定，但 `shadow` 判据会红） |
| 2 | #5/#6/#7/#8/#9/#10（tactic 的层级提示/脊恢复） | **loose + strict 的 tactic** | `restore_universe_levels` 重建脊靠层级文本；替代 = 内核 telescope 直接给层（P2 的 typed telescope 已有一半） |
| 3 | #26/#27（recursor/const 宇宙层） | loose + strict（`And.rec`/`Eq.refl` 一类） | 内核路径已经给出 `Solved` 的层；老路那半边要等"老路整体退休" |
| 4 | #11（`application_arg_expected`） | loose + strict（`Set.empty α` 类 227 处） | 依赖 G-21（`needs_expected_type`）的未决设计，**风险最高** |
| 5 | #22/#12/#13（`operand_type_expr` + 记法层级） | **loose 最大宗**（29,262 次/10 文件） | 删了 loose 大面积回退 ⇒ 只能随 P4 老路拆除一起走 |
| 6 | #2/#4/#23（`operand_type_expr_explicit` 第二候选） | **loose 654 次/10 文件**（strict 0 次） | 需要把"全显式 pp 第二候选"移植进内核路径（typed telescope + metavariable），**否则 loose 回归**（`Eq.symm hx` / `Eq.subst …` 解不出前导 `α`） |
| 7 | #28（`next_arg_expected`） | loose + strict 的 `@` 脊/`apply` | 与 #2 同源（全显式 pp），一起移植 |
| 8 | #29（`pp=explicit` 缓存键） | 全部显式 pp 查询 | 最后一个退休：它是上面各条的缓存键，**上面都删完了它自然没有调用者** |
| 9 | #1/#3/#15/#17/#19（定义）与 #16/#18/#20/#21（内部接线） | —— | 定义随最后一个调用者一起消失；内部接线随 #15/#17 一起消失 |

**一句话**：这 9 组**没有一组是 strict 专属**；第 1–4 组会动 tactic（loose 也动），
第 5–7 组会动 loose 的判卷面，第 8–9 组是尾巴。**P3 的"删"必须等 P4 的老路退休，不能按 strict 清理做。**

## 6. 棘轮（`scripts/audit-strict-paths.py`）

### 6.1 三个模式（本分支实测）

```text
$ python3 scripts/audit-strict-paths.py --self-test          # exit 0
strict-paths self-test ✓（strict 专属点必红 · 未分类必红 · 注释不计 · 删掉即绿）

$ python3 scripts/audit-strict-paths.py --all                # exit 0
audit-strict-paths: 提及 4/25/1（基线 4/25/1） · 调用 2/7/1（基线 2/7/1） · strict 专属 0
✓ 文本通道只减不增（strict 专属 = 0）

$ python3 scripts/audit-strict-paths.py --strict-only        # exit 0
（同上；`--strict-only` 与 `--all` 的差别只在"红不红"的判据：前者只数 strict + 未分类）
```

`--all --json` 出 30 条逐点清单（`sites[]` 带 `file/line/fn/fam/cls/call/why`），可直接对账本表。

两把判据**故意分开**：`--all` 红于「提及/调用越基线 **或** 未分类 **或** strict 专属 > 0」；
`--strict-only` **只**红于「strict 专属 > 0 **或** 未分类」（fail-closed —— 新点证不出共用就按 strict 算；
所以「往共享点加一行」在 `--strict-only` 下也会红，但**理由**是未分类，不是越基线）。

### 6.2 **咬住证明**（必须；四次读数）

做法：把 `crates/front/src` **逐字节复制**到 `/tmp/p3bite/src`（`diff -r` 空 = identical），
在副本里做实验（**不动工作区**；脚本新增 `--root` 正是为这条证明与夹具留的）：

| 读数 | 树 | 命令 | 结果 |
|---:|---|---|---|
| ① | 副本未改 | `--strict-only --root /tmp/p3bite/src` | **exit 0**：提及 4/25/1 · 调用 2/7/1 · strict 专属 **0** |
| ② | 往 **`compile/kernel_app.rs`（strict 专属模块）** 加一行 `let _ = infer_type_text(ctx, scope, x, None, None, false);` | `--strict-only --root …`（`--all` 同树也红） | **exit 1**：提及 **4/26/1** · 调用 **2/8/1** · strict 专属 **1**；stderr：`✗ strict 专属 1 条 · 未分类 0 条（模式 --strict-only）` |
| ③ | 删掉那一行（`cp` 回原件） | `--strict-only --root …` | **exit 0**：回到 4/25/1 · 2/7/1 · strict 专属 0 |
| ④ | **只降方向**：把 `elab.rs:4853` 那行注释掉（= 按本表"loose 专属"删一条） | `--all --root …` | **exit 0**：提及 **3/25/1** · 调用 **1/7/1** · strict 专属 0（**只许降**：低于基线不判红） |

脚本 `sha256-16` = `b1098374f7577b8f`（钉住这四次读数用的是同一份脚本）。

⚠ **为什么"加一行"不是加在真树的某个 strict 专属调用点**：本分支**没有 strict 专属调用点**（= 本报告的结论），
所以演示加在 **`kernel_app.rs`（唯一一类 strict 专属模块）** 里 —— 脚本按
`STRICT_ONLY_FILES = kernel_app*.rs` 结构性判 `strict`，这正是"第二刀真出现 strict 专属项时"的形态。
**未改真树**：`git status` 里本刀只有 `scripts/audit-strict-paths.py`（新文件）与 `scripts/soko`（gate 接线）。

### 6.3 进 `gate --fast`

`scripts/soko` 的 gate 段（`audit-notation-paths` 之后、`audit-wire-fields` 之前）加了三次调用：
`[]`（= `--all`）· `--strict-only` · `--self-test`，毫秒级 ⇒ `--fast` 与完整 gate 都跑。
**咬不住的守卫等于没有**：所以 `--self-test` 与两条棘轮一起进 gate。

## 7. 一处前提纠正（**主会话请确认**）

任务前提写"S6 之后 **strict 现在是默认档**"。读数不支持这个说法：

* `Dialect` 的 `#[default]` 仍是 **`Loose`**（`crates/front/src/compile/dialect.rs:39-40`），
  `CompileOptions::default()` = `dialect_from_env().unwrap_or_default()` ⇒ **不设 `SOKO_STRICT_AT` = loose**；
* 实测（release `b213488c`，不设任何 `SOKO_*`）：

  ```text
  $ ./target/release/sokonanoda grade --json /tmp/p3probe/default_dialect.sokonanoda
  {"human":"checked declaration id2","name":"id2","type":"decl.checked"}
  {"human":"checked example","type":"example.checked"}       ← `example : Nat := id2 Nat 3` 在默认档是 checked（loose 读法）
  $ SOKO_STRICT_AT=1 … 同一文件
  {"code":"elab-implicit-argument-unsolved", …}               ← strict 档才拒
  ```

* S6 自己的读数也写：`check.py` 默认档（= **loose**）全过（`reports/2026-11-01-p212-s6-readings.md` §0）。

**S6 真正翻的是"strict 档内的默认"**：`kernel_app::effective(dialect) = dialect.strict()`、
`no_fallback = dialect.strict()`（两道环境变量闸门消失）⇒ **选了 strict 就默认走内核路径，例外表命中才回老路**。
这条对 P3 的结论**更有利**：strict 是 opt-in，老路上的文本调用只在"strict 且回落后"才可能进
（#2 全语料 strict 0 次即证），所以"strict 专属"更不可能存在。

## 8. 复现（逐条可执行）

```bash
# 棘轮三模式（本分支应为 0 / 0 / 0）
python3 scripts/audit-strict-paths.py --self-test
python3 scripts/audit-strict-paths.py --all
python3 scripts/audit-strict-paths.py --strict-only

# 30 条逐点清单（给对账用）
python3 scripts/audit-strict-paths.py --all --json

# 咬住证明（副本里加一行 strict 专属调用 ⇒ 红；删掉 ⇒ 绿）
rm -rf /tmp/p3bite && mkdir -p /tmp/p3bite && cp -R crates/front/src /tmp/p3bite/src
python3 scripts/audit-strict-paths.py --strict-only --root /tmp/p3bite/src      # 0
printf '\nfn p() {\n  let _ = infer_type_text(x);\n}\n' >> /tmp/p3bite/src/compile/kernel_app.rs
python3 scripts/audit-strict-paths.py --strict-only --root /tmp/p3bite/src      # 1（strict 专属 1）
rm -rf /tmp/p3bite

# 文档预算（探针已删后必须绿）
python3 scripts/docs-lint.py
```

## 9. 本刀边界（如实）

* **不删任何判定代码** ✅（`git diff` 里没有 `crates/` 的改动，S4d 的那几个文件不是本刀的）。
* **不碰 `crates/kernel/**`** ✅（0 行）。
* **探针已删** ✅（`grep` = 0 · `docs-lint ⑨` 绿 · 三个文件回到 8841 / 1218 / 4748）。
* **未 commit / 未 push** ✅。
* **未跑全量测试** ✅（只用 10 个课程文件做运行时读数，`SOKONANODA_NO_CACHE=1`）。
* 已知不确定：`named_arg.rs:259`、`by.rs:2533/2569`、`infer.rs` 的 #14 在本语料里 0 次
  （课程没有具名实参 / 未触发那两条 tactic 分支）⇒ 归类是**静态判据**（调用链与 dialect 无关），
  不是运行时读数 —— 表里已逐条标注"语料 0/0（静态）"。
