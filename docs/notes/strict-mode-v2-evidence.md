# strict-mode-v2：实测证据与失败方案（2026-10-20）

> 配套 `docs/design/strict-mode-v2.md`。全部读数来自隔离 worktree
> `/tmp/sokonanoda-strict-v2-draft`（分支 `draft/strict-mode-v2`，基线 `f6a27097`，
> 即 S3 `--dialect` 之后），**主仓未改一行代码**。墙钟是 debug 构建，只作比值参考。

## 1. 性能：13× 就是 `operand_type_expr` 那一行

同一文件 `courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda`、同一构建：

| 构建 | 档 | `JUDGE_PREFIX runs` | `passes` | 墙钟 | `--json` vs loose |
|---|---|---|---|---|---|
| 基线 | loose | 5 | 23 | 3.25 s | — |
| 基线 | strict | **16** | **34** | **46.50 s** | 逐字节相同 |
| 只改 R3 | strict | **5** | **23** | **4.40 s** | 逐字节相同 |

R3 的草稿 diff（worktree 里保留）：

```diff
-    let explicit = super::implicit_msg::strict_at_enabled(ctx.options);
+    let explicit = false; // R3 draft: engine-read types always use default pp
```

⇒ §1 的「慢在 explicit pp 支路」成立；但下节说明它**单独不可上线**。

## 2. 判定：356 文件两档对拍，27 差异 / 25 个 `0→1`

条件：release 冷缓存、`SOKONANODA_NO_PROJECT_ARTIFACTS=1`、`SOKONANODA_NO_CACHE=1`、
`SOKO_STRICT_AT=0|1`，比较 stdout 字节与退出码。

- **27 个文件输出不同**；其中 **25 个是 `loose 0 → strict 1`**；
- 8 个 strict 的 `PREFIX_RUNS` 比 loose 多（最多 4→10）；
- 2 个 `docs/gaps/repro/` 文件两档都 exit 1、但错误文案不同（说明“0 差异”会把文案也算进去）。

最小根因（反向验证：只差 R3 这一行、同一个 unit05）：

| 构建 | `units/I.2/unit05-pairs-products` strict |
|---|---|
| 基线 | **exit 0** |
| 只改 R3 | **exit 1**（`lib.Prod` 先红，再连带） |

`lib/Prod` 单独跑 R3：8 条声明**全部 checked**，却仍出 **2 条**诊断、exit 1；
诊断 span 都是 **line 1**（合成文本坐标），message 是 `elab-implicit-argument-unsolved`。
⇒ 严格档拒的是**引擎自己产出的文本被重新 elaborate**，不是用户源。

**F5 后（最终）**：同一 356 快照两档 sweep **18 diffs / 16 个 `0→1`**，与 HEAD 逐行
**0 mismatch**（`same`/`loose_exit`/`strict_exit`）；耗时 **117.7s vs HEAD 537.9s**。
`lib/Prod` strict exit 0 / 10 checked / 0 diag；`b2` strict 仍红；loose 24/24 逐字节不变。

## 3. 调用链（`SOKO_STRICT_TRACE=1` 打出的真实栈）

两类入口都命中（158 次；`head=Prod.prod_mk whole=Prod.prod_mk A B a b`）：

```
strict_rejection_message
  ← try_implicit_application_with (elab.rs:4738，唯一拒绝点)
  ← elab_expr(operand)                         # 主路径：build_theorem
  ← … build_theorem ← Walk::theorem ← run_pass_with
```

```
strict_rejection_message
  ← try_implicit_application_with
  ← infer_type_text_inplace_expected           # 就地类型探测
  ← infer_type_text ← operand_type_expr ← pp_shape::arg_types
```

⇒ provenance 必须覆盖「**抽出的值稍后在主路径再 elaborate**」这条，
线程局部的“解析窗口”不够。

## 4. 两条被实测否掉的捷径

### 4.1 探测期宽松（thread-local，只包 `infer_type_text_inplace` 的 `elab_expr`）

- unit01 strict：`PREFIX_RUNS 16 → 5`、`--json` 与 loose 相同（性能看起来完美）；
- **但** `lib/Prod` 从 baseline 的 **0 诊断** 变成 **2 条拒绝**（不同列号）；
- 原因：探测在宽松档下“成功”了，于是**不再回落到 explicit 慢路**，交出的类型/值
  与慢路不同 —— 正是「两条路必须同源」那条纪律。
- ⇒ 结论：**不许**用“探测期宽松”换性能；除非先有 shadow 对拍证明两条路逐字节一致。

### 4.2 `@` 编码（把 pp 文本回读出的所有 App 脊标 `explicit_spine=true`）

- `lib/Prod` 不再报 `prod_mk`，但 `lib.Set` 当场红：
  `` `exact` 类型不匹配：期望 `Set.mem x A`，实际是 `Set.mem` ``；
- 原因：pp 文本可能是**部分应用**（默认 pp 丢第一个隐式实参）；
  `@` 会关掉隐式插入 ⇒ 部分应用停在函数上。
- ⇒ 结论：`explicit_spine` 与 provenance **语义不同**，不能复用。

### 4.3 被否掉的还有

- 文本指纹（`span=line 1`）推断来源：违反「判定不许靠文本」，且 span 会跨变换漂移；
- 只修缓存（计划 A）：16 个键两两不同，冷编无命中；
- 只修就地装配（计划 B）：证据同上 4.1，改的是判定而非纯性能，需全语料对拍。

## 5. 课程未迁完（HEAD 16 USER + R3 10 ENGINE）

```
courses/set-theory/gaps/C-112-cb-step5.sokonanoda
courses/set-theory/lib/Demo.sokonanoda
courses/set-theory/lib/Prod.sokonanoda
courses/set-theory/units/I.2/unit05-pairs-products.sokonanoda
courses/set-theory/units/I.2/unit75-prod-distrib.sokonanoda
courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda
courses/set-theory/units/I.8/unit20-cardinal-arithmetic.sokonanoda
courses/set-theory/units/I.9/unit23-products.sokonanoda
courses/set-theory/units/notation-cheatsheet.sokonanoda
courses/set-theory/units/solutions/I.1/unit04-solution.sokonanoda
courses/set-theory/units/solutions/I.10/unit78-solution.sokonanoda
courses/set-theory/units/solutions/I.11/unit46-solution.sokonanoda
courses/set-theory/units/solutions/I.12/unit51-solution.sokonanoda
courses/set-theory/units/solutions/I.13/unit39-solution.sokonanoda
courses/set-theory/units/solutions/I.2/unit05-solution.sokonanoda
courses/set-theory/units/solutions/I.2/unit06-solution.sokonanoda
courses/set-theory/units/solutions/I.2/unit75-solution.sokonanoda
courses/set-theory/units/solutions/I.3/unit08-solution.sokonanoda
courses/set-theory/units/solutions/I.3/unit09-solution.sokonanoda
courses/set-theory/units/solutions/I.3/unit10-solution.sokonanoda
courses/set-theory/units/solutions/I.4/unit11-solution.sokonanoda
courses/set-theory/units/solutions/I.4/unit12-solution.sokonanoda
courses/set-theory/units/solutions/I.8/unit20-solution.sokonanoda
courses/set-theory/units/solutions/I.9/unit23-solution.sokonanoda
courses/set-theory/units/solutions/notation-cheatsheet-solution.sokonanoda
```

典型形态：`And.intro a b ha hb`、`Prod.prod_mk A B a b`、`Or.elim a b c f g h`
——都是 §4 要删的多余隐变量。**F5 已修好 9/10 ENGINE**；`unit05-solution` 是真实证明错误（基线也红）。

## 6. 行数预算（`docs-lint` ⑨，实测）

| 文件 | 当前 | 基线上限 | 余量 |
|---|---|---|---|
| `crates/front/src/compile/elab.rs` | 8822 | 8826 | **4 行** |
| `crates/front/src/judge.rs` | 4669 | 4670 | **1 行** |
| `crates/front/src/by.rs` | 4352 | 4353 | **1 行** |

在 `judge.rs` 加 3 行即判红（实测：`⑨ ... 变长 4672 > 4670`）。
⇒ provenance 代码放**新模块**，`elab/judge/by` 调用点净零行。

## 7. 既有判据（R3 构建上仍绿）

- `cargo test -p sokonanoda-front --lib`：**1006 passed / 0 failed**；
- `cargo test -p sokonanoda-cli --test implicit_at_alignment --test dialect_flag`：
  **5 + 7 passed**（`b2` 拒绝、逃生门、`@` 正对照都在）；
- `courses/set-theory/lib/Prod.sokonanoda`：baseline strict **0 诊断**；
- `unit01` 两档 `--json` 逐字节相同。

## 9. F5（最终采用，2026-10-20）

- **根因**：R3 让内核 pp 文本（`Prod.prod_mk A B a b`）成为严格档非法源文本，而它在
  `elab.rs:5008` 被当源码 elaborate ⇒ `lib/Prod` 的 `Prod.fst_snd_mk` /
  `Set.mem_prod_mk` 带 line-1 span **真实失败**（不是诊断泄漏）。
- **修法**：保留 R3；`elab.rs:4617` / `:5008` 两处「解出值 elaborate」改用
  `pp_shape::elab_engine_value`（严格档下换默认档 `ElabCtx`；非严格档原样直调）。
  用户写的实参（`:5024`/`:4641`）不动；`elab.rs` 行数净增 0，helper 在未冻结的
  `pp_shape.rs`（+36 行）。
- **判据**：`lib/Prod` strict exit 0 / 10 checked / 0 diag；25/25 strict-diff 文件
  baseline ≡ F5；356 sweep 18 diffs / 0 row mismatch；`front --lib` 1007；
  canary 3；`b2` strict 红；loose 24/24 逐字节。
- **AST 原型（已放弃）**：`Expr::Engine` 编译面只要 14 match / 68 行，但包在 pp 模板根
  会新增 `lib.Rel` 37 条诊断（模板/合一失效）⇒ 不采用。F5 用 2 个调用点解决了同一问题。

## 8. 复现命令

```bash
cd /tmp/sokonanoda-strict-v2-draft
cargo build -p sokonanoda-cli --bin sokonanoda
F=courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda
SOKO_STAGE_STATS=1 SOKONANODA_NO_PROJECT_ARTIFACTS=1 target/debug/sokonanoda grade --json $F
SOKO_STRICT_AT=1 SOKO_STAGE_STATS=1 SOKONANODA_NO_PROJECT_ARTIFACTS=1 target/debug/sokonanoda grade --json $F
SOKO_STRICT_TRACE=1 SOKO_STRICT_AT=1 target/debug/sokonanoda grade --json <file> 2>trace.err
python3 /tmp/sweep_corpus.py          # 356 文件两档对拍（release；结果 /tmp/sweep_corpus_results.json）
python3 scripts/docs-lint.py --check-src-lines
```
