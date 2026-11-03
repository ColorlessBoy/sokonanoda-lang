# `strict ≤ loose` 不变量：D4 重评估（第一步 = 可行性评估；读数已复现，判定代码零改动）

> 触发：用户 2026-11-03 任务单（D4「接受例外」重评估；验收 = 全语料逐文件违反数从 7 下降）。
> 二进制身份：`target/release/sokonanoda` sha256 前 16 = **`06f3eb09852fcf0f`**（树 `HEAD 4b5bb690`）。
> 所有读数 = 冷缓存（独立 `SOKONANODA_CACHE_DIR`、两个 `NO_*_ARTIFACTS`、`-u SOKO_STRICT_AT`）。

## 0. 先修量具：违反数现在**有门**了

§2ter 的发现「`strict-score.py` 的 `work_le_loose` 只量 3 个手挑文件 ⇒ 绿 ≠ 不变量成立」已落地：

| 新增 | 作用 | 命令 |
|---|---|---|
| `scripts/strict-score.py --corpus` | 全语料 357 文件 × {loose,strict} 冷跑；**逐文件**要求 `calls · misses · prefix_bytes` **三项都** `strict ≤ loose`；违反表 = `scripts/strict-invariant-baseline.json` 的 **allowlist ratchet**（新增违反判红；条目不再违反也判红——基线腐烂比没有基线更坏） | `python3 scripts/strict-score.py --bin <bin> --corpus --corpus-only --jobs 6` |
| `scripts/strict/exception-census.py` | 逐文件**例外**计数（`nofallback_kept_*` / `fallback_shape` / `nofallback_dead`）= 验收判据 ③ 的原料 | `python3 scripts/strict/exception-census.py <bin> --write /tmp/exc.json` |

读数与用户口径一致：**357 文件、违反 7 个、正是那 7 个**；三项合计
calls 21,015 → 21,305 · misses 3,078 → 3,108 · prefix 224.04 MB → 224.82 MB。

## 1. ⭐ 关键读数：**「例外」≠「违反」**

全语料 **8** 个文件有 `nofallback_kept_*`，但只有 **4** 个违反；反过来 **7 个违反文件里 3 个一个 `kept` 都没有**：

| 文件 | `nofallback_kept_*` | 违反？ | 机制（本轮实测） |
|---|---|---|---|
| `unit19-solution` | `prefix_meta` 30 · `input_contains_a_meta` 22 | ✓ | 184 次 `judge_infer("∅")` 全失败（loose 0 次）⇒ 每次整份前缀重编 |
| `unit19-cardinal-le` | `prefix_meta` 22 · `input_contains_a_meta` 10 | ✓ | 同族 |
| `G19-zero-ary-notation-applied` | `input_contains_a_meta` 9 | ✓ | 同族（`∅` = `Set.empty ?α`） |
| `G48-notation-nullary-sugar-operands` | `input_contains_a_meta` 3 | ✓ | 同族（两侧都是零元糖，G-48 的鸡生蛋） |
| `unit02-subsets-empty` | **0**（3 个 `universe_poly_ho_motive` 是**尝试前**闸门） | ✓ | 判卷缓存扰动：strict 多判一整份前缀 + 同一前缀判两遍 |
| `unit06-solution` | **0**（7 个 `fallback_shape` 是**尝试前**形状闸门） | ✓ | 同 `unit02` |
| `G30-min-eq-subst-half` | `argmismatch_text` 2（+8 个尝试前闸门） | ✓ | loose 根本不问判卷（prefix 0），strict 问一次且失败 |
| `unit12-solution` · `unit22-solution` · `G-apply-goal-driven` | 各 1（`argmismatch_text` / `expected_retry`） | ✗ | —— |

⇒ **把例外全消到 0，最多只能修掉 4 / 7**；`unit02`·`unit06`·`G30` 的违反与回落无关。
§2ter 的「违反量 100% 由回落解释」是**文件级相关**，不成立。

## 2. P2 试过了：内核「闭元变量可登记」这条路**走不通**（有证据）

实施 + 读数（两次都还原干净，`grep app_register_inputs` = 0）：

1. 在 `elab_app_session` 里把输入（`binder_tys` / 头 / 实参 / λ binder 类型）里**闭**的元变量按 P2-9 的
   `app_register_externals` 机制登记，不再一律 `Unsupported("input contains a meta")`。
   **结果**：`input_contains_a_meta` 22/10/9/3 → 3/0/0/0（真的进去了），但**全部转成 `argmismatch_text`
   19/10/9/3**，一次都没解出。
2. 探针读数（临时 `eprintln`，已删）：失败点是 `?β x1 x2 ≟ ?m0`（`dom_bvars=2 at_metas=[0]`）。
3. **根因**：`crates/kernel/src/mvar.rs` 的 `in_meta_scope` —— 本内核**赋值以闭项存储**
   （`quote(depth, v).num_loose_bvars() == 0` 才允许），`depth_of(id)` 读了但**不用**。
   于是「把调用方的 `?α` 赋成它的 binder `α`」被干净地拒掉 ⇒ `ArgMismatch`。
   ⇒ 这不是「少一个登记钩子」，是**元变量表示的硬限制**（Lean 对应 `MetavarDecl.lctx`）。

## 3. 第二个根因（更主要）：strict 向**文本判定**问了一个必然失败的题

`unit19-solution` 的 strict 读数（同一二进制，`SOKO_JUDGE_STATS=2`）：

| 量 | loose | strict |
|---|---:|---:|
| `JUDGE_INFER.calls` | 64 | **284** |
| `JUDGE_INFER.fails` | **0** | **184**（逐条全是 `judge_infer("∅")`） |
| `JUDGE_STATS.prefix_bytes` | 796,174 | **1,587,462** |

机制：记法重试（`retry_with_prefix` → `elab::notation_prefix_args`）只能拿操作数的**渲染文本**去问
`judge_infer`；`∅`（零元记法）**没有独立类型**，这问题必然失败，而每次失败的代价是**整份前缀重编**。

我试过在**未冻结**的 `kernel_app_notation.rs` 里跳过这次重试（两个变体都已还原，`git diff` 空）：
* 版本 A（**任一**操作数是裸隐式常量 ⇒ 不重试）：`unit19-solution` **rc 0 → 1**（3 条 `Set.Le` 诊断），
  `prefix_meta`/`input_contains_a_meta` 归零但**判卷变红** ⇒ 不可接受。原因：混着可定型操作数时
  （`∅ ⊆ A` 的 `A`）重试**会成功**，而它正是该文件 `rc=0` 的来源。
* 版本 B（**所有**操作数都是裸隐式常量才跳过）：**全语料读数逐字不变**（`unit19-solution` 30/22、
  `unit19-cardinal-le` 22/10、`G19` 9、`G48` 3、`G30` 2、`G-apply` 1 全部相同，strict JSON 哈希相同）
  ⇒ 谓词在真实形状上**从不命中**（`bare_implicit_const` 只认 `Ident` 与零元 `Notation`；
  `{b}` 是 `Expr::SetLiteral`）⇒ **咬不住的闸不留下**（"咬不住的守卫等于没有"），已删除。

## 4. 逐条技术结论（不是"接受"）

| 例外/机制 | 次数 · 文件 | 本轮判定的技术结论 |
|---|---|---|
| `prefix_meta` + `input_contains_a_meta` | 96 · 4 文件 | **本轮不可消**：消它要在 `judge_infer` 之前把操作数按**内核项** elaborate（`elab.rs`，冻结）**或**让内核元变量能带 scope（`mvar.rs` 表示升级，多日 + soundness 风险）。已证两条捷径都不成立（§2、§3）。 |
| `argmismatch_text` | 4 · 3 文件 | 正文必须由内核 def-eq 打印机出（P2-13）；其中 2 个文件本来**不违反** ⇒ 收益 0。 |
| `expected_retry` | 2 · 2 文件 | `ExpectedMismatch` 交 S4 重解；两个文件都不违反 ⇒ 收益 0。 |
| `shape`（尝试前闸门） | 7 · `unit06` | **零回落税**（在 elaborate 头之前就返回）；`unit06` 的违反不是它造成的（§1）。 |
| 判卷缓存扰动（无 `kept`） | 3 文件 | **本轮不可消**：要动 `judge.rs` 的缓存键/负缓存或让内核项与老路项**逐字节同形**；`judge.rs` 冻结。 |

## 5. ⚠ 施工环境风险（必须记录）

本轮期间工作树被**并发进程**改动/重置过：`unit02-subsets-empty.sokonanoda`（06:51）、
`crates/front/src/ast.rs` + `parser.rs`（08:04，新增 `Command::name_span` ⇒ 当时**编不过**）、
`check/walk.rs`；我在 02:35 写入的未跟踪产出（本报告 + 两个脚本）一度被清掉。
⇒ 凡是与本轮读数做比较的构建，**必须同时钉住二进制 sha 与树状态**；跨并发改动不可比。

## 6. 本轮交付 / 未达成（如实）

**已交付（`43d7c6b4`）**
1. `scripts/strict-score.py --corpus` = 全语料逐文件不变量判据 + allowlist ratchet；
   咬住证明（合成行，四向）：违反在 allowlist 内 ⇒ 绿；缺条目 ⇒ `NEW` 判红；条目不再违反 ⇒ `STALE` 判红；
   多一条不违反的 ⇒ 同样判红。
2. `scripts/strict/exception-census.py` = 逐文件例外计数（验收判据 ③ 的原料）。
3. `scripts/strict-invariant-baseline.json` = 7 条 allowlist，**每条写机制**（不是"接受"）。
4. 三条**被证否**的捷径（§2 / §3 / 两个重试变体），都带二进制身份与可复跑命令。

**违反数**
* `HEAD 4b5bb690`（基线所钉的树）：**7**。
* 当前工作树（含并发进程对 `unit02-subsets-empty` 的证明改写：`apply Eq.subst h` + `exact hx`
  → `apply Eq.subst h hx`）：**6**，ratchet 正确报 `STALE=unit02-subsets-empty`。
  ⚠ 这 −1 **不是**本轮的改动贡献，它恰好**印证** §1 对 unit02 的机制判断（那 3 个 `Eq.subst` 站点正是
  判卷扰动的来源）。它落地后应把该条目从 allowlist 删掉（ratchet 要求）。

### 追加（2026-11-03 晚，C 案施工后）

* **C1–C3 已落**（`02979b5a`）：元变量赋值带 scope（`lower` 存 / `lift` 读 / 缓存键 `(ExprPtr, u32)`）。
  判据：**714 行逐字节 0 差异** + 内核单测 130/130。它本身**不改变任何读数**（今天所有 meta 的 depth 都是 0），
  是 C4 的前置层。
* **C4 判为不可交付**：接上后 `G19` 从 `rc=0` 变 `rc=1`（`elab-anon-ctor-no-expected-type` @43:3）——
  内核给调用方元变量赋值**不回传**前端 ` soko_m{id}` 表（`meta.rs` 冻结）⇒ 同一 id 两份实例不一致。
  详见 `reports/2026-11-03-metavar-scope-assignments.md` §8。⇒ 下一轮的真任务 = **赋值回传**，
  它与记法侧重试（`prefix_meta` 52 次）是同一块基础设施，应同一批做。

### 追加 2（2026-11-03 深夜，read-back 做完之后 —— **结论变了**）

前端半（回传）做完后判卷**全部保住**（G19/G48/unit19×2 的 JSON 与基线逐字节相同），说明回传确实
治好了 G19 变红的根因。**但 kept 计数一格没少**：`input_contains_a_meta` 22 → 3 + **`argmismatch_text` 19**，
`G19`/`G48` 的 9/3 变成 **`panic` 9/3**（内核裸 `panic!` = 拒绝）。

⇒ **C4 的收益是 0**，而且根因不是"少一块基础设施"，是**产品判定**：老路靠
`implicit::fill_pending_by_shape`（借同形兄弟，Lean 没有的启发式）才过得去；strict 要消掉这 4 个文件
就必须**照抄那条猜法**，与「不新造猜法」的设计前提冲突。三条出路（接受偏差 / strict 也猜 / loose 也拒）
见 `reports/2026-11-03-metavar-scope-assignments.md` §10。C4 + read-back 已**还原**（判卷不变但收益 0）。

**未达成**：`nofallback_kept_*` 一个都没消；`unit19-solution`/`unit19-cardinal-le`/`G19`/`G48`/`unit06`/`G30`
仍在表里。要推进必须动本轮冻结的四文件或内核元变量表示 —— **下一轮请先就"解冻哪一处"拍板**：

| 选项 | 改哪 | 能修掉 | 代价/风险 |
|---|---|---|---|
| A | `elab.rs`（记法操作数的类型改走**内核项**而不是 `judge_infer` 文本） | 184 次失败提问 ⇒ 4 个 meta 族文件 | 前端主路径，回归面大 |
| B | `judge.rs`（负缓存：同一 term 的 infer 失败不重复整份重编） | 同样的 4 个文件（便宜、正交） | 判卷缓存语义，需全量 `--json` 对拍 |
| C | `kernel/src/mvar.rs`（赋值带 scope，Lean `MetavarDecl.lctx`） | `input_contains_a_meta` 一族（真解） | 内核表示升级，多日 + soundness |

## 7. 机制定论（2026-11-03 深夜）：违反量**全部**落在「文本键推断缓存」

### 7.1 `prefix_bytes` 到底在数什么

`judge.rs` 里 `PREFIX_BYTES` 有**两个**增量点：① 整份前缀判定（`judge_pairs_uncached`，每次 `prefix_src.len()`）；
② **推断未命中的重编**（`judge_infer` 的 miss 路径）。⇒ 它 = "整份判定 + 未命中重编"的**总字节**。
再对上 `JUDGE_INFER.calls/misses/fails`，两档的差就**只有**一项来源：

> **同一份源码、同一批 `by` 判定，strict 的项/前缀文本与 loose 不同 ⇒ 文本键的推断缓存未命中 ⇒ 每次未命中整份重编。**

### 7.2 三个文件的逐项对账（`SOKO_JUDGE_STATS=2`，同一二进制）

| 文件 | judge 调用 / pairs | `JUDGE_INFER` calls / fails | `prefix_bytes` |
|---|---|---|---|
| `unit06-solution` | 26 / 35 **= 26 / 35**（完全一样） | 129 / **0** → 139 / **2** | 1,413,779 → **1,512,484**（+7%） |
| `unit02`（改前） | 15 / 26 → **16** / 28 | 35 / 0 → 42 / **3** | 282,609 → 324,046（+15%） |
| `unit19-solution` | 14 / 23 → **16** / 26 | 64 / 0 → 284 / **184** | 796,174 → **1,587,462**（+99%） |

`unit06` 是最干净的证据：**问的问题、对数一字不差**，只多了 2 次推断失败 + 若干次因文本不同而**未命中** ⇒ 字节 +7%。

### 7.3 所以「strict 与 tactic 优化机制不兼容」是准确的

判卷器的优化是一张**以渲染文本为键**的推断缓存（`judge_infer_key`：`extra_prefix + prefix_src + binders + term_text`）。
strict 走内核路径解出的项，回读/渲染后与 loose 的项**不是同一段文本**（隐式实参的呈现不同）⇒
语义相同、**文本不同** ⇒ 缓存不共享 ⇒ 每次未命中把整份前缀重编一遍（O(N²) 放大）。

### 7.4 两条可能的修法（都要动"非内核"的前端文件）

| 修法 | 落点 | 代价 / 风险 | 预期收益 |
|---|---|---|---|
| **A. 缓存键改成"源级"**（用声明在源文件里的字节区间 + binders 作键，不用渲染文本） | `judge.rs`（**冻结四文件之一**） | 键语义变了 ⇒ 必须全量 `--json` 对拍 + `judge` 的 own tests | 4 个扰动文件（unit02/unit06/G30/unit19 的成本）**全部**回落；`kept` 计数不变 |
| **B. 让渲染两档同形**（隐式实参的呈现规范化，使 strict/loose 的项印出同一段文本） | `display.rs` / `pp_shape.rs`（**不在冻结四文件里** ✓） | 影响**用户可见**输出 ⇒ golden/`--json` 逐字节判据会咬；工程量比 A 大 | 同上，且不需要动判卷缓存 |

⚠ 两者都**不改判卷结论**（这正是它们安全的地方）：A 只换缓存键、B 只换呈现。
先做 A 更小：它不动任何用户可见文本。

## 8. A′ 落地：违反数 **7 → 4**（2026-11-03 深夜，用户「整体怎么修复」后）

### 8.1 改了什么（一处，`elab.rs` 净 0 行）

`operand_type_expr`（记法前导位求解时问操作数类型的地方）新增一条短路：

> **裸隐式常量没有独立类型**（前导参数只能由**期望类型**给）⇒ 拿它去问 `judge_infer` 是**必然失败**的
> 提问，而每次失败都把**整份前缀重编**一遍 ⇒ 直接答 `None`（= 它失败后本来返回的那个值）。

判据（`bare_implicit_const`，P2-2 (c) 的既有定义：`Ident` 或**零元记法** 且 `implicit_prefix > 0`）。
**行为等价**：这些站点今天本来就是 `None`（提问失败），只是先付了一次整份前缀重编的学费。

### 8.2 读数（同一二进制 `b63807a4`，冷缓存）

| 文件 | 改前 | 改后 | loose |
|---|---|---|---|
| `unit19-solution` | calls 284 · misses 14 · prefix 1,587,462 · fails 184 | **64 · 4 · 796,174 · 0** | 64 · 4 · 796,174 |
| `unit19-cardinal-le` | calls 157 · misses 12 · prefix 1,400,430 | **64 · 4 · 796,174 · 0** | 64 · 4 · 796,174 |

⇒ 与 loose **逐项相等**，离开违反表。

| 判据 | 结果 |
|---|---|
| 全语料逐文件违反数 | **7 → 4**（`unit02` 由用户的证明改写修掉，`unit19×2` 由 A′ 修掉） |
| 714 行逐字节 | **0 差异**（判卷结论一字未变） |
| 四门计分板 | 全绿；`work_le_loose` 三个文件**逐项相等** |
| 整门课 | **258 目标 / 2191 checked / 0 判负** |

### 8.3 同一轮试过但**实测惰性**的（都还原了，不留死代码）

| 试的 | 读数 | 为什么无效 |
|---|---|---|
| 把"解出来但带未解载体"的实参也**推迟**（Lean `elabAndAddNewArg` 顺序） | `defer_meta_arg=9` 真的触发，但 4 个文件读数一字未变 | `elab_ready` 拿内核给的域重解时走**老路记法路径**（不看期望类型）⇒ 又造占位符 |
| `elab_ready` 里把**零元记法**也送进 `try_hinted_const`（原来只认 `Ident`） | 无变化 | 下一行拦住了 |
| `try_hinted_const` 传声明的 `implicit_prefix`（原来写死 0） | 无变化 | 该分支根本没走到（零元记法在 `run_once` 第一趟就被推迟，走的是另一条路） |

⇒ 结论：`unit19` 这一类要彻底消掉，需要**声明级元变量上下文**（Lean `TermElabM` 的 mctx 生命周期）；
A′ 只是把其中最贵的"必然失败提问"消掉。这条留给产品判定（见 §6 的三选一）。

## 9. 剩余 4 个：下一步实验已经收窄到"一次构建能判"（2026-11-03 深夜）

A′ 之后（`b63807a4`）剩下 4 个，本轮已经把每个的**失败调用点**查到 `#[track_caller]` 级别：

| 文件 | 现在 | 失败点（实测） | 下一步（一步能判） |
|---|---|---|---|
| `G19` | `kept_input_contains_a_meta=9` · fails **2** | `fails` 的 term 是 `"∅"`，调用链经 `judge_infer_explicit` → `infer_type_text` 的**全显式**支 | 给 `infer_type_text` 加 `#[track_caller]` 印出它的**调用者**（本轮试到这一层停了）。已知**不是** `operand_type_expr`（A′ 已挡）、**也不是** `operand_type_expr_explicit`（A′b 试过，全语料逐字不变 ⇒ 惰性，已还原） |
| `G48` | 同上 · fails **8** | 同上（`"∅"`） | 同上；注意 `G48` 按设计必须判红，它的 `kept` 是"语言边界"的证据 |
| `unit06` | calls 139 vs 129 · prefix +98,705 · fails **2** | term 是一段**渲染出来的证明项**（`And.intro (fun (a b) …) …`，长得像把 λ 的 binder 注解丢了） | 把那段文本与**源级**证明项对一遍：若 pp 丢了 binder 注解 ⇒ 修 pp 的该项；否则走 B（缓存键去文本化） |
| `G30` | calls 8 vs 4 · prefix 4,640 vs **0** | 同族：`fun (h : ({a}) = ({b})) => Iff.mp … (Eq.subst.…)` | 同上；另 `argmismatch_text` 要 P2-13 的内核打印机 |

**共同的根**（也是 B 刀的目标）：判卷器的推断缓存**以渲染文本为键**，而 strict 解出的项与 loose
的项在**呈现**上不同形（隐式实参写多/写少）⇒ 同一语义问题变成两个键 ⇒ 未命中/失败 ⇒ 整份前缀重编。
B 刀（键改"源级区间 + binders"）能一次覆盖 `unit06`/`G30` 这一类，且**不改判卷结论、不改可见文本**。

## 10. A′c：违反数 **4 → 2**（2026-11-03 深夜）

A′ 之后 `G19`/`G48` 的 `fails` 还各剩 2/8 次，term 都是 `"∅"`。用 `#[track_caller]` 追到
`judge_infer_explicit` → `infer_type_text` 的**全显式支**；把守卫从 `operand_type_expr` 下沉到
`infer_type_text`（带 `known` 版）**反而改了 `unit19-solution` 的判卷输出**（hash `790765e3` ≠ 基线
`47260191`）⇒ 立刻回滚（那一步只试了 1 次、有 `--json` 对拍照着，没扩散）。

真正的落点：**不需要 `known` 的窄判据** —— `infer_type_text` 入口只认**零元记法**
（`Expr::Notation { lhs: None, rhs: None, .. }`，不碰 `Ident`，因为裸常量的类型有时是可推的）：

| 文件 | A′ 之后 | **A′c 之后** | loose |
|---|---|---|---|
| `G19` | calls 12 · prefix 6,350 · fails 2 | **8 · 4,628 · 0** | 8 · 4,628 |
| `G48` | calls 12 · prefix 1,956 · fails 8 | **2 · 0 · 0** | 2 · 0 |

⇒ 两个文件都**逐项等于 loose**，离开违反表。

| 判据 | 结果 |
|---|---|
| 全语料逐文件违反 | **7 → 4（A′）→ 2（A′c）** |
| 714 行逐字节 | **0 差异** |
| 四门计分板 | 全绿（`work_le_loose` 三文件逐项相等） |
| 整门课 | **258 / 2191 / 0 判负** |

### ⚠ 一个必须说清的差别：「成本达标」≠「例外清零」

`unit19-solution`（`kept_prefix_meta=10` / `input_contains_a_meta=14`）、`unit19-cardinal-le`（9/6）、
`G19`（`input_contains_a_meta=8`）、`G48`（`prefix_meta=3`）**仍在回落**，只是**回落不再贵**
（浪费的"必然失败提问"被消掉）⇒ `strict ≤ loose` 成立、但它们不是"消干净"。
要真清零需**声明级元变量上下文**（Lean `TermElabM` 的 mctx），即把 `∅` 的 `α` 留成待解元变量、
由另一操作数在**声明末**统一 —— 这是下一个大件，不是补丁。

剩 2 个（`unit06`/`G30`）**都不含能力缺口**：失败 term 是**渲染出来的证明项**
（`And.intro (fun (a b) …)` / `fun (h : …) => Iff.mp … (Eq.subst …)`）⇒ 属"文本键缓存"那一族（B 刀）。

## 11. 最后两个（`unit06`/`G30`）：失败原因已定，但**修法受限**（2026-11-03 深夜）

A′c 之后只剩这两个。用 `SOKO_DBG_INFER` 把失败原因与"问不问"都量出来了：

| 量 | loose | strict |
|---|---|---|
| `fails` | **0** | **2** |
| 失败**错误码** | —— | `elab-implicit-argument-unsolved` |
| 失败 term | —— | `And.intro (fun (a b) …) …`（`unit06`）· `fun (h : …) => Iff.mp … (Eq.subst …)`（`G30`） |
| loose 有没有问同样形状的项 | **0 次** | 2 次 |

⇒ 两个结论：

1. **这些提问是 strict-only**（loose 一次都不问）⇒ 不是"缓存没命中"，是**多出来的提问**。
2. 失败原因是**渲染文本丢了前导隐式实参**（`And.intro` 的 `a b`、`Iff.mp` 的 `A B`），
   而这次提问**没有期望类型**可反解 ⇒ 必然 `elab-implicit-argument-unsolved`。

### 为什么"改用全显式 pp"不能修（重要）

看起来 `EXPLICIT_PP`（G-71 的 `judge_infer_explicit`）能让文本带上隐式实参、从而成功 —— 但**度量不降**：
成功走慢路**同样**要合成 `#check` 文档、重跑整份前缀（`PREFIX_BYTES` 的第二个增量点）。
⇒ 在这把尺子下，**唯一**的收益来自**不问**。而"能不能不问"要求回答
"这个头的前导隐式实参能不能由**显式实参**推出来"（`And.intro` 的 `a b` 不能、`Set.singleton` 的 `α` 能）
—— 那正是 `notation_prefix_args` 那台求解器的活。

### 所以本轮的落点

`unit06`/`G30` 属于**缓存/提问策略**问题（无能力缺口），要修得先有"前导隐式可推性"的判定；
在那之前它们留在 allowlist 里，每条都写清了机制/次数/失败码。**这不是"接受"，是"已知边界 + 已知修法方向"**。

### 11.1 补：调用点定位到**第二候选**（`judge_infer_explicit`）

`#[track_caller]`（`Location::caller()` 在宏外取）钉到 `judge.rs:2945` = **`judge_infer_explicit`**
⇒ 这些提问走的是**第二候选**（`operand_type_expr_explicit` → `infer_type_text(explicit=true)`）。
**因果链闭合**：strict 的主候选（`operand_type_expr`）在这些站点失败得更频繁 ⇒ 触发第二候选
⇒ 第二候选把**丢了隐式实参的渲染文本**当 `#check` 的查询项 ⇒ 必然
`elab-implicit-argument-unsolved` ⇒ 每次失败重编整份前缀。

⚠ 也确认了"改用全显式 pp"这条路**是死路**：`judge_infer_explicit` 的 `EXPLICIT_PP` 只影响
**返回的类型文本**（"完整可回读"），**不影响送进去的查询文本**（`let term = render_expr(operand)`
在进档前就渲染好了；而那个 `render_expr` 是**故意的**——判定查询必须用显示渲染，否则记法折叠会改判定）。

⇒ 要消这两个，只有一条路：**让第二候选不问不可答的问题**，即先判定
"这个头的前导隐式实参能否由**显式实参**推出"（`And.intro` 的 `a b` 不能；`Set.singleton` 的 `α` 能）
—— 那台求解器就是 `notation_prefix_args`。这是一件独立的小刀（估 100–200 行 + 音界判据），
不是补丁；本轮时间盒内不冒险动它。

## 12. 负结果：A′e（"写全参数 ⇒ 第二候选帮不上"）**回滚**（2026-11-03 深夜）

想消最后两个（`unit06`/`G30`）时试的第三条路：把"**写全参数**的应用（位置实参 ≥ 声明显式层数）、
头是裸隐式常量"也归入 `no_standalone_type` —— 理由看起来成立（前导隐式只能由已有实参的类型推，
而主候选已经试过）。

**实测：全语料 714 行对拍 512 行差异**（loose/strict 两侧 256 个文件都变）⇒ 立刻回滚（`git checkout`，
二进制重建回 `a13ffc0f` = 已提交状态）。

**为什么错**（值得记）：主候选（`operand_type_expr`）**不走内核** —— 它用 `KnownName::Decl` 里的
**源级类型文本**（`implicit` 签名表 + `explicit_arity`，零内核调用）答"这个操作数的类型"。
所以"写全参数 ⇒ 不可答"是**假命题**：绝大多数写全参数的应用主候选答得出，根本不需要第二候选。

⇒ **不能再用谓词消**（这已是第三次同类失败：`any` 操作数、`all` 操作数、写全参数）。
最后两个要真消，只有一条路：**真做"前导隐式实参能否由显式实参推出"的判定**（= `notation_prefix_args`
那台求解器的 `unify_extract`），并**只在该判定为否时**才跳过第二候选 —— 不是形状启发式。

另外：`unit06` 单独看**确实**能被 A′e 修好（calls 139→26、prefix 1,512,484→1,413,779=f=loose、fails 2→0），
它那两问正是"写全参数的证明项"；但 A′e 的判据无法只命中它而不伤别人 ⇒ 放弃。
