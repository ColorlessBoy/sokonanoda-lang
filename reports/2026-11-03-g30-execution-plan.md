# G-30 本体：期望类型传播 + `isDefEq` 核对 —— 执行计划（2026-11-03 用户选 A）

## 目标与验收

**要做成的事**：`Iff.intro (fun …) (fun …)`（省略前导 Prop 实参）在**已知期望类型**的位置能 elaborate ——
分支撑里的集合字面量 `{a}`/`{b}` 拿得到期望类型。方法是 Lean 的两段式：

1. **传播**：实参带着**形参域**去 elaborate（域可能是元变量）；
2. **核对**：elaborate 完用 `isDefEq(域, typeof(实参))` 判通过，不合再退回今天的路。

**验收（一条都不能省）**：

| # | 判据 | 现状 |
|---|---|---|
| 1 | `docs/gaps/repro/G30-min-eq-subst-half.sokonanoda` **转绿**（`grade` rc 1 → 0） | 红 |
| 2 | 同一文件在 strict 下的 `prefix_bytes` 与 `JUDGE_INFER fails` **降到 loose 水平**（0 / 0） | 4,640 / 2 |
| 3 | **整门课** `check.py` = 258 / 2191 / **0 判负** | 0 判负 |
| 4 | **loose 语料逐字节不变**（714 行里 loose 那 357 行） | 不变 |
| 5 | strict 侧**允许变**，但必须**逐条定性 + 重钉 golden**（不许"变就变"） | — |
| 6 | `gap.py check` 一致（G-30 的 `repro_expect` 与实测一致） | 一致 |
| 7 | 四门计分板 + `board`/`docs-lint`/`clippy`/测试 + CI 全绿 | 绿 |

**明确的非目标**：不为了让度量好看而放宽任何判据；不删允许清单来"变绿"。

## 已确立的事实（本会话实测，不再重复验证）

1. 提问来自 **IA-1 的 `arg_types` → `operand_type_expr` → `infer_type_text`**（`track_caller` + 探针实测）；**主候选**，不是第二候选（`SOKO_METAVAR=0` 读数一字未变）。
2. 内核**已经有**两处合一：`app_expected_def_eq`（期望 ≟ 结果类型）与 `app_arg_def_eq`（域 ≟ 实参类型）。
3. 失败现场（诊断打印）：`ArgMismatch{arg:0}` 于 `Iff.mp` 的 `(a ∈ {b})`（span 48:15）、`ArgMismatch{arg:1}` 于第二个分支 λ（span 50:6）。
4. **六条路已否掉**（都回滚了，见 `reports/2026-11-03-judge-infer-root-fix-plan.md` 末节）：全部推迟 / 只推迟 λ / 推迟集合字面量 / 缩小到有期望 / mismatch 改推迟重来 / span→类型 memo。
5. **关键判读**：③④ 之所以伤课程，是因为它们**替换**了源级那条路；⑤ 之所以无效，是因为 mismatch 那一位**本来就已解好**，把它改推迟只是让内核重新交域，而 `elab_ready` 交下去的是**内核 hint**、不是**源级 expected** —— 记法与集合字面量那条路**看不到 hint**。
6. `app_export_side` 导出的项**带松散变量**，直接 pp 会 panic（`eval: loose bvar`）⇒ 诊断必须先**闭包**（按 `binder_tys` 套 λ 或用 `with_pp_scoped`）。

## 步骤（每步一个判据，红就停）

### 步 0（半天）· 让 mismatch 的诊断能打出两边原文

- 现状：直接 pp 会 panic。改法：`app_arg_mismatch` 里把 `dom`/`at` **先 quote 成 `ExprPtr`**、用
  `with_pp_scoped(&scope_names, …)` 打印（就地路径 `infer_type_text_at_peeled` 用的就是这套）。
- 判据：`SOKO_APP_ARGDBG=1` 跑 G30 ⇒ 打出 `dom=… actual=…` 两行，**且进程不 panic**。
- 停止规则：仍 panic ⇒ 改用"套 `d` 层 λ 再 pp"的兜底；两版都不行 ⇒ 退回只打 `arg` + 布尔，不再花时间。

### 步 1（1 天）· `elab_ready` 给实参**源级**期望（这是缺的那一半）

- 改法：`postpone::elab_ready` 在给内核 hint 的同时，把域 **pp+parse 成源级 `Expr`**，作为
  `expected_src` 传给 `elab_expr`。pp 用 `infer_type_text_at_peeled` 那条（**不重编前缀**）。
- 只在**推迟位**上生效 ⇒ 今天能过的实参**一个都不碰**。
- 判据：G30 的 `prefix_bytes` **4,640 → 0**、`fails 2 → 0`；**整门课仍 0 判负**（这是硬门）。
- 停止规则：课程出现**任何**判负 ⇒ 立刻停，先定性（是"域给错了"还是"记法不吃这个 expected"），不许继续加条件绕过。

### 步 2（半天）· 失败后按位重试（源级优先，不合再用域）

- 改法：`run_once` 的 `AppOutcome::ArgMismatch { arg, .. }` 分支——**只对那一位**改用"带域重新 elaborate"，
  再跑一次会话；还 mismatch ⇒ 保持今天的行为（回落老路）。
- 与步 1 的区别：步 1 是"推迟位一律带域"，步 2 是"**已解好的位**在核对失败后**重来一次**"。
- 判据：步 1 之后仍红的用例（若有）转绿，且课程仍 0 判负。

### 步 3（半天）· 补丁收口：判卷输出变化逐条定性

- 用 714 行对拍列出 strict 侧变化的文件，逐条说明"为什么变"（预期只有集合字面量一族 + 三个 G30 片段）。
- 判据：每个变化文件**要么**解释清楚且课程仍绿，**要么**回滚该处改动（不许留下说不清的差异）。

### 步 4（半天）· 台账与 golden

- `docs/gaps/ledger.jsonl` 的 G-30：把 `repro` 指向能代表现在的片段、补 `repro_expect`（该片段转绿后按实测改），
  并在 `today` 写清"哪一支修好了 / 哪一支仍红"。
- `scripts/strict-invariant-baseline.json`：删掉 G30 那一条（ratchet 会要求）。
- 判据：`gap.py check` 一致、`strict-score` 绿、`docs-lint` 绿（注意 `docs/` 只剩 78 字节 ⇒ 台账改动要**等量压缩**别处）。

## 时间盒与停止规则

- **D1**：步 0 + 步 1（最可能出成果的一步）；D1 结束若步 1 仍伤课程 ⇒ **停，回报实测**，不硬推 D2。
- **D2**：步 2 + 步 3。
- **D3**：步 4 + 全量验收 + 推 CI（一批一次）。
- 任何一步出现"课程判负"且 30 分钟内定不了性 ⇒ 按仓库纪律**换招或降级**，把该步的读数和结论写进报告，不留半截代码。

## 风险

1. **判卷输出会变**（这是 A 的题中之义）：影响面按步 3 逐条核，必要时重新钉 golden。
2. **`pp+parse` 的往返可能丢信息**（记法、隐式实参）：若步 1 因此出现奇怪的失败，改用"域 → 源级期望"的
   另一条现成路径（`expected_spine::expected_to_inductive` 同族）先试。
3. **性能**：步 1 每推迟位多一次 pp（µs 级），但有 `JUDGE_INFER` 的下降对冲；步 3 会给出前后读数。

## 交付物

- 代码：`crates/front/src/compile/kernel_app_postpone.rs`（步 1）、`kernel_app.rs`（步 2）、`elab_app_holes.rs`（步 0 诊断，env 门控）
- 台账：`docs/gaps/ledger.jsonl`（G-30）、`scripts/strict-invariant-baseline.json`
- 报告：本文件追加"每步实测读数"一节

---

## 第 0/1 步实测与**计划更正**（2026-11-03，本轮）

### 步 0 ✅ 诊断已能安全打出（结构指纹，不 pp）

```
DBG_ARG arg=0 dom(kind=app,metas=2,nodes=65/89) actual(kind=sort,metas=0,nodes=1)
DBG_ARG arg=1 dom(kind=pi ,metas=0,nodes=18)   actual(kind=pi ,metas=0,nodes=12)
```
（`SOKO_APP_ARGDBG=1`，进程**不 panic**，`grade` rc 仍 1。）

### ⚠ **计划更正 1**：这个片段**本来就该判红**，不是"该开始通过"

实测：`grade --dialect loose` 与 `--dialect strict` **都 rc=1**。它的 `Iff.mp (a∈{b}) (a=b) h proof` 是
**旧写法**（显式实参在前，而预置里 `Iff.mp` 的 `a b` 是隐式）⇒ 两种方言都拒绝，差别**只在成本**
（loose `prefix` 0 / strict 4,640）。⇒**不变量要的只是"同样便宜地拒绝"**，不是让它通过。
（此前的"修好后它开始通过"是错的，据此把步 4 的台账改法改成"仍红、但记为已按 cost 收口"。）

### 步 1+2 实测：单用无效、合用爆炸

| 组合 | G30 读数 | 结论 |
|---|---|---|
| 步 1（`elab_ready` 同时给源级期望） | 4,640 / 8 / 2（**一字未变**） | 那两位是 `ArgMismatch`、**根本没走到 `elab_ready`**（没有域被交出来）⇒ 够不着 |
| 步 2（mismatch 后该位改推迟重来） | 一字未变 | 同上 |
| 步 1+2 合用 | prefix **51,040** · 推断 **30,452** · JSON 变 | 再入，**爆炸** ⇒ 回滚 |

（都已回滚：树干净、二进制 `3d63a53d`、714 行 0 差异。）

### 成本到底在哪：`report()` 的 `ArgMismatch ⇒ Fallback("argmismatch_text")`

`kernel_app_report::report()` 里只有一条**不花钱**的出口：`n_args > explicit_layers` 时直接
`Report::Error`（用老路那句 `strict_rejection_message`，**不碰判卷器**）。而 `Iff.intro` 是
**2 个实参 = 2 个显式层** ⇒ 走不到它，落到 `ArgMismatch => Report::Fallback` ⇒ **老路整条跑起来**
（含 `arg_types` 的两次判卷器提问）就是为了把正文打印成**逐字节相同**的那句。

### ⇒ 计划更正 2：真正的下一步是 **P2-13**（内核侧 def-eq 错误打印器）

- 现状注释写得很清楚：`ArgMismatch => Report::Fallback("argmismatch_text")` 的唯一理由是
  "正文要与今天逐字节相同 = 要内核那台打印器（`render_value_for_def_eq_error`）"。
- **做掉它** ⇒ `ArgMismatch` 也能走 `Report::Error` ⇒ **老路一行不跑**（那两次提问消失 ⇒ cost 降为 0）
  且正文来自内核（目标：与老路逐字节相同，用 714 行对拍钉住）。
- 这是"治本"的另一半：**不是把类型补上让它通过，而是让拒绝不必再问类型**。

---

## 步 3/4 实测：**成本是"载荷性"的**（本轮的决定性结论）

把 `ArgMismatch` 改成**直接报**（不再回落老路；正文用"第几位 + 签名"，不印内核类型，**无内部名**）：

| 判据 | 开关关（今天） | 开关开 |
|---|---|---|
| G30 `prefix_bytes` | 4,640 | **0**（= loose） |
| G30 `JUDGE_INFER fails` | 2 | **0** |
| G30 文件结论 | rc=1（判红） | rc=1 ✓（仍判红） |
| G30 正文 | `Iff.mp` 那条（隐式实参） | `类型不匹配：`Iff.intro` 的第 2 个显式实参…`（**无 `$5`/`Set.preimage.[]`，G3 干净**） |
| **整门课** | **258 / 2191 / 0 判负** | **258 / 2190 / 914 · 1 个被判负** ✗ |

那 1 个是 `units/solutions/I.4/unit12-solution.sokonanoda`：

```
✗ 解答 unit12  code=kernel-rejected  message=类型不匹配：`Set.ext` 的第 3 个显式实参，其类型与形参的要求不符。
```

⇒ **那条回落路径（问判卷器 + 拿不到类型也继续解）正是 `Set.ext` 这类应用"被接受"的原因**。
把回落掐掉，它们就被判死了。

**所以那 4,640 字节不是"浪费"，是"载荷"**：它换来的是老路更宽的行为。在「整门课 0 判负」这条硬门下，
**成本与接受面是耦合的**——要同时消成本，必须先把课程里依赖这份宽度的用法改掉（那是课程内容决定，
不是编译器能单方面做的）。

本轮所有实验均已回滚：树干净、二进制 `dd39645e`、714 行 0 差异、整门课 **258/2191/0**。
