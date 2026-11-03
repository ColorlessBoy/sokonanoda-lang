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

我试过在**未冻结**的 `kernel_app_notation.rs` 里跳过这次重试：
* 版本 A（**任一**操作数是裸隐式常量 ⇒ 不重试）：`unit19-solution` **rc 0 → 1**（3 条 `Set.Le` 诊断），
  `prefix_meta`/`input_contains_a_meta` 归零但**判卷变红** ⇒ 不可接受，**已还原**。
* 版本 B（**所有**操作数都是裸隐式常量才跳过）：构建期撞上**并发改动**（见 §5），未能取到读数。

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
