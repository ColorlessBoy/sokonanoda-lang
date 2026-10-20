# 严格档 v2：把 `@` 规则**关在用户语法里**（一条：不许影响编译）

> 用户 2026-10-20 拍板两条：①「重新规划严格档，**这次不能影响编译**」；
> ②「课程正文**从加 `@` 改成删多余的隐变量**」。本文是这两条的唯一执行依据。
> 前置：`REQUIREMENTS.md` §9.26（翻默认 → 翻回 opt-in 的止损记录）。

## §0 结论

严格档本身**不慢**；慢的是它逼出来的一条**内部表示支路**（全显式 pp）。v2 不打算
给那条支路提速，而是**让它不再需要存在**：`@` 规则只约束**用户写的源码**，
引擎自己合成/回读的文本按旧绑定规则走（一个 provenance 位）。

## §1 as-built：慢在哪（同一份 `unit01-sets-membership`，同一构建）

| 档位 | 就地判定 **开** | 就地判定 **关** |
|---|---|---|
| 旧读法 | **0.69 s** | 5.61 s |
| 严格档 | **8.84 s** | 4.91 s |

⇒ **关掉就地两档几乎一样**（≈5 s）⇒ 13× 全在「就地/缓存这条路能不能用」，
**不在解析、不在 `@` 本身**。另一个独立读数（CI 失败 artifact）：
`didOpen unit01 = 20280ms（修前基线 1.8s）`。

**调用链**（`crates/front/src/compile/`）：

```
pp_shape::arg_types
  └ elab::operand_type_expr
      let explicit = implicit_msg::strict_at_enabled();      // ← 严格档 ⇒ 1
      └ elab::infer_type_text(…, explicit)
          (On, Some(env)) if explicit =>
              judge::judge_infer_inplace_with_explicit(…)     // 就地：失败
                .or_else(|| judge::judge_infer_explicit(…))   // ⇒ 造 #check + 整段前缀重编
          (On, Some(env)) =>
              judge::judge_infer_lookup(…)                    // 这里有廉价缓存命中
```

三个已核实的事实，缺一条都不足以解释 12×：

1. **`pp=explicit` 支路不查缓存**（源码注释原文「不查普通缓存槽」）——它有自己的键是对的，
   但**连查都不查**；
2. **就地那条答不出**：`infer_type_text_inplace` 失败在 `elab_expr(operand)`
   （代码注释：「实测就地失败 100% 在这一步」）—— operand 是**引擎自己合成的 AST**；
3. **未命中 = 整段前缀重编**：同一文件 `judge_infer` 未命中 **16 次**（旧读法 **5 次**），
   其中 **12 次**来自 `pp_shape::arg_types`；前缀 15 669 字节。

## §2 三条候选与取舍

| | 做法 | 冷编收益 | 判定风险 | 结论 |
|---|---|---|---|---|
| **A** | 给 `pp=explicit` 补廉价缓存 | **0**（实测 16 个键两两不同 ⇒ 冷编无重复可命中）；只救 LSP 连续键入 | 无 | 不够：不满足「不影响编译」 |
| **B** | 让就地答得出（修 `elab_expr(operand)`：引擎重建 AST 时 `explicit_spine` 保真族） | 大（回到 0.69 s 量级） | 中（动引擎装配，需全语料对拍） | **兜底**：C 做不完时用它 |
| **C** | **provenance**：`@` 规则只约束用户源；引擎合成/回读文本按旧规则 | 大（那条支路**整条消失**） | 中（边界要盘干净） | **主选** |

**为什么 C 是结构解**：A/B 都是给那条路**提速**，C 是把它**删掉** ⇒ strict 与 loose
走**同一条**热路径 ⇒ 「不影响编译」由构造保证，而不是靠调优。
**与 Lean 4 对照**：`Lean/Elab/App.lean:706` 的 `processImplicitArg`
（`if (← read).explicit then processExplicitArg else addImplicitArg`）约束的是**用户语法树**；
Lean 的 elaborator **不把自己的项转成文本再按用户规则重解析**（我们用文本往返是为了复用判卷通道）。
⇒ 我们这条往返文本**本来就不该受用户语法规则约束**。

## §3 C 的最小改动面（要盘的**全部**，不许漏）

1. **产生合成文本的入口**（都要打 provenance）：`implicit_msg::{refl_text, at_prefix}` ·
   `judge::judge_infer{,_explicit}` 的 `#check` 合成 · `elab::operand_type_expr` ·
   `by.rs` 的 `rfl_candidate` · `goals.rs::substitute_names` · `expr.rs::with_root_span`。
2. **回读入口**（读 provenance）：`compile::parse_fragment` / `elab_expr` 的顶层；
   provenance 打开时**不**走 `implicit_msg::strict_rejection_message`。
3. **边界要写死**：provenance **只**由上述入口置位，**不许**由"文本看起来像引擎产物"推断 ❌
   （那就是文本启发式，与「判定不许靠文本」冲突）。
4. `strict_at_enabled()` 的消费者**只剩**用户源路径 ⇒ `operand_type_expr` 里那行
   `explicit = strict_at_enabled()` **删除**（这就是 12 × 前缀重编的来源）。

## §4 课程正文的迁移规则（用户 2026-10-20 拍板，**取代**「加 `@`」）

**规则**：正常形式只会按**显式形参**逐位绑定 ⇒ 多余的隐式实参**删掉**，不是加 `@`。

| 现在（`@` 写法） | 改成（删多余隐变量） |
|---|---|
| `@Set.univ α` | `Set.univ` |
| `@And.intro a b ha hb` | `And.intro ha hb` |
| `@Eq.refl.{1} α a` | `Eq.refl a` |
| `@Exists.intro α p w h` | `Exists.intro w h` |

* `@` **只在**「确实要显式指定某个隐式位」时才写（例如要喂 `{α}` 而后续实参给不出线索）。
* 为什么改：① 两档**都合法**（旧读法也接受短写）⇒ 迁移一次不用再迁；② 更短、更接近 Lean 4 习惯；
  ③ 与严格档的**语义**一致（正常形式本来就不该喂隐式位）。
* ⚠ 已知边界（台账里踩过）：一族**不能**只加 `@` 或只删——`Iff.mpr h ha` 合法而
  `@Iff.mpr h ha` 会 `kernel-rejected` ⇒ **逐个按诊断的「出路 ②」改，跑两档对拍**。

## §5 验收（**禁绝对毫秒**；`AGENTS.md` 2026-09-29 第 ② 条）

1. **结构计数**：同一文件、同一构建，`SOKO_STRICT_AT=1` 的 `PREFIX_RUNS` ≤ 不设时的；
   `PREFIX_BYTES` 比值 ≤ **1.05**；`judge_infer` 未命中数 strict ≤ loose。
   （量具已存在：`stats::prefix_runs()` / `SOKO_INFER_TRACE` / `SOKO_PREFIX_MISS_PROBE`。）
2. **判定不变**：全语料 356 个 `.sokonanoda` 两档 `--json` **逐字节**对拍 0 差异
   （现成脚本形状：`/tmp/sweep2.py` 那套 4 线程，冷缓存、`SOKONANODA_NO_PROJECT_ARTIFACTS=1`）。
3. **墙钟只做比值兜底**：同 run 内 strict/loose ≤ **1.1**（数量级兜底 20×，不当主判据）。
4. **课程门禁**：`courses/set-theory/tools/check.py --json` 在两档下都 `failed 0`。

## §6 四棒排期（每棒独立可回退）

| 棒 | 交付 | 回退点 |
|---|---|---|
| 1 | **读数先接上**：`On` 档也把 `INPLACE_FAIL_REASONS` 落文件（现在只在 shadow 档打）＋ 一个复现件（strict/loose 结构计数比） | 纯诊断，删掉即可 |
| 2 | provenance 贯通（§3 的 1–3），**默认仍关**，只用 `SOKO_STRICT_AT=1` 验 | 开关关着 ⇒ 行为零变化 |
| 3 | 删 `operand_type_expr` 的 `explicit = strict_at_enabled()`；跑 §5 的四条 | 恢复那一行 |
| 4 | 语料按 §4 迁移（按诊断「出路 ②」）+ 两档对拍 + **单独一笔**把默认翻回严格并连续盯两轮 CI | 翻回 opt-in（一笔） |

**第 4 棒才发版**，且**不与第 2/3 棒同一次推送**（默认值那一笔必须自己独占一轮 CI）。

## §7 边界（不做）

* 不改用户可见的 `@` 语义（`@f α a` 仍逐位吃望远镜）；
* 不新增「按名字/按位置混搭」之类的语法；
* 不引入绝对毫秒判据；不靠"调宽超时/加缓存键"达标；
* 不在同一版里既改判定又改默认（`487f9a77` 的教训：翻默认那次自己写着「未收口」却没量墙钟）。
