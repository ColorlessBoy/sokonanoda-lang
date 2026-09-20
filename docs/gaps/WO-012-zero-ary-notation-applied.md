# WO-012 零元记法（`∅`）落在「被应用」的位置 + `by` 块（G-19）

> 台账：`docs/gaps/ledger.jsonl` → `G-19`（`kind=language` · `severity=painful` ·
> **`status=fixed` · `fixed_in=0.61.0`**）
> 复现件：`docs/gaps/repro/G19-zero-ary-notation-applied.sokonanoda`（自足：自己声明 `Set`/`∅`/`⊆`）
> 实测环境：本树 0.61.0（未发布）· `scripts/soko grade <绝对路径> --json`
> 证据纪律：下面每个「今天」都是**亲手跑出来的输出**；只读源码没跑到的判断一律标「待确认」。

## 用户可见症状 / 最小复现

```bash
scripts/soko grade "$PWD/docs/gaps/repro/G19-zero-ary-notation-applied.sokonanoda" --json
```

**今天的实际输出**（0.61.0 实测）

```text
decl.checked  Set / Set.mem / Set.empty / Set.subset
decl.checked  g19_mem_empty      ← 对照①：`x ∈ ∅ -> False := by intro h; exact h`
decl.checked  g19_pointful       ← 对照②：`Set.subset α (Set.empty α) A := by …`
diagnostic    elab-tactic-failed  `exact` 判定失败：记法 `∅` 展开成 `Set.empty` 时补不出
                                  前面的类型参数：请写出点名形式（例如 Set.empty α …）
# 退出码 1
```

同一句的 **term 模式**（`:= fun (x : α) => fun (hx : Set.mem α x (Set.empty α)) => …`）
**通过**（实测，隔离模块根）。

## 根因（比 S9 底稿更精确）

`∅` 是**零元**记法 ⇒ 源 AST 里是 `Notation { target: "Set.empty", operands: [] }`。

| 位置 | 为什么 | 结果 |
|---|---|---|
| `∈` 的操作数（`x ∈ ∅`） | `∈` 指向 `Set.mem (α) (a) (A : Set α)`，第三参数位给了 `∅` **期望类型 `Set α`** ⇒ 补得出 `α` | ✅ 通过 |
| `⊆` 的操作数（`∅ ⊆ A`） | `intro` 先要把 `Set.subset`（**def**）delta 展开成 `forall (x : α), A x -> B x`（`spine.rs::unfold_head_once`），`∅` 于是落进**被应用**的位置（`∅ x`）——**期望类型没了** | ❌ 判红 |

回读路径（render→re-read）拿到 `Set.empty` 的**零实参应用**，`α` 无从解出 ⇒
`elab-notation-argument-unsolved`。与 X11/X12 同族（render→re-read 丢信息）。

## 期望行为（Lean 4）

`∅ ⊆ A := by …` 与 term 模式、与点名形式 `Set.subset α (Set.empty α) A` **同判**。
记法是糖：`∅` 只是 `Set.empty` 的另一种写法，不该因为出现在被应用的位置就判红。

## 影响面

- 课程 **227 处 `Set.empty`** 几乎全在此形状（`∅ ⊆ A`、`A ∪ ∅`、`∅ ∈ 𝒫 A` …）；
- 隐式实参课程批 **B0 的前置**：不先修，改完 lib 签名后分不清是哪个改动闯的祸。

## 实际修法（**已落，0.61.0**）

设计时列的两条方向（改 `render_expr`）**都不是最终解**——真正的落点在 **`intro`**：
问题的源头不是渲染，而是**源级 delta 展开把记法操作数搬进了「被应用」的位置**。

`crates/front/src/by.rs` 的 `Tactic::Intro` 分支改成两级：

1. 先按**源 AST** 剥一层（`spine::peel_pi`）——零额外开销，绝大多数目标在这儿解决；
2. 剥不动（说明头是 **def**：`A ⊆ B` 的 `Set.subset`、`¬ A` 的 `Not`）⇒ **先换用内核 pp
   的规范形态再剥**（`canonical_goal_with_spec`：点名 + 参数写全），源级 delta 展开
   （`peel_pi_delta`）只作退路。

pp 形态下 `∅ ⊆ A` 是 `Set.subset α (Set.empty α) A`，展开成 `(Set.empty α) x` ⇒ 回读无碍。
`keep_if_lossless` 继续挡住「pp 丢隐式实参」那一档（`rfl` 在 `Eq.{1} (Set α) (Aᶜ) …` 上
报错的那次教训）；归一化失败一律退回原 AST，绝不因为归一化把好文件判红。

**与既有机械的关系**：`apply` 早就有同款失败重试（L1.2），`cases` 有
`canonicalize_binder_type`——这一刀把 `intro` 补齐，三条路的判据一致。

**代价**：只有「目标头是 def」的 `intro` 多一次 `judge_render_type`（有缓存），
普通 `∀`/`->` 目标零开销。

## 验收（同轮实测，全绿）

1. 复现件**干净判卷**：`scripts/soko grade docs/gaps/repro/G19-zero-ary-notation-applied.sokonanoda`
   ⇒ **exit 0**、7 条声明全 `decl.checked`；
2. `python3 scripts/gap.py close G-19 --version 0.61.0` ⇒ 「已关账（复现判定：sokonanoda/0 clean）」；
   `gap.py check` + `selftest` exit 0；
3. 回归测试 `crates/cli/tests/notation.rs::a_zero_ary_notation_survives_delta_unfolding_in_a_by_block`
   ——**改前 FAILED、改后 ok**（含两条改前就过的对照组，钉住"没顺手改坏"）；
4. `cargo test --workspace --locked` **1180 passed / 0 failed**；课程门禁
   **36 目标 · 329 checked · 99 open · 0 判负** 逐项不变、`--selftest` exit 0；
   `git diff --stat -- crates/kernel/` **空**。
