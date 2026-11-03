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
