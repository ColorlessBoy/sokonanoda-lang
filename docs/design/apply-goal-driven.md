# apply：让目标先参与（apply-goal-driven）

- **状态**：批次 A 已落地（隔离树 `target/impl/src`），批次 B 待 spike · 2026-10-21。
- **范围**：只动前端（`crates/front/**`）；判定仍在内核；共享树零写入。
- **一句话**：`apply` 今天是「先定型、再对目标」，改成「先让目标参与、再定型」。
  用户 2026-10-20 拍板语义 A（对齐 Lean）：判不出角色的层 → **项级 mvar**，不造伪目标。

## 1. 问题（实测）

| 输入 | 今天（修前） | 修后 |
|---|---|---|
| `apply Eq.symm`（目标 `a = x`） | 类型文本读不回来（`?u.<id>`） | 子目标 `x = a` |
| `apply Eq.symm`（含 `h : x = a`） | 同上（调试版曾给错子目标 `x = x`） | 同上 |
| `apply Eq.refl` | 同上 | 闭合 |
| `apply Eq.trans h`（目标 `a = c`） | 报「结论是 `a = a`」（defaulting） | 子目标 `b = c` |
| `apply Eq.symm.{0}`（层不符） | 多一个 `⊢ Prop` | 目标集 `['x = a']`（闭合时内核仍拒 Sort0/Sort1） |
| `apply junk`（`α` 用不到） | `['Sort 1','P']` | **不变**（显式参数在 Lean 里就是目标 ✓，见 §3） |

**三条根因**
1. **顺序**：`apply_tactic` 先问类型（无期望类型，`by.rs`）；「带目标」的第二刀只在第一刀 Err 时才走
   ⇒ 目标不在场 ⇒ `c` 被 defaulting 成 `a`。
2. **剥层是文本剥层**：`peel_binders` 先 `parse_expr_text`，遇 `?u.<id>` 就**静默 break**，
   把整条上下文 telescope 交回 ⇒ `apply` 把上下文 binder 当成被应用项的层（σ 按名字撞车）。
3. **判角色靠名字猜**：`mentions(name, codomain|goal)` + `is_universe_domain` ⇒ 猜错就**静默造伪目标**。

## 2. 批次 A（已落地）

| 项 | 位置 | 读数 |
|---|---|---|
| 1a 失败边重试 | `by.rs::apply_tactic` + `apply_type_fits` + 开关 `SOKO_APPLY_GOAL_FIRST` | `Eq.trans h` 绿；开关=0 回退 |
| 1b 剥层前层归一 | `judge::normalize_level_mvars`（`peel_binders` 之前）+ 开关 `SOKO_APPLY_LEVEL_TEXT` | `Eq.symm`/`Eq.refl` 绿 |
| 1b 守卫 | `apply_type_fits`：`u_soko*` 不许进结论/子目标域 | 两例未触发 |
| 1c 可诊断 | `peel_binders` 的 `[peel-break]`（`SOKO_PEEL_PROBE=1`） | 项层面剥留后续 |
| 回归 | `lib/Set` 28/0、课程 5 代表文件两档一致、`--json` 逐字节 | ✅ |
| 性能 | 成功路径 **+0**（只在失败边多一次就地 elaborate） | ✅ |

**新增接受清单（红线：只增不减，逐条点名）**：`apply Eq.symm`（有无假设各一条）、`apply Eq.refl`、
`apply Eq.trans h`。**消息变化**：`apply Eq.subst`（零实参）从「读不回来」变成「目标不匹配：结果是 `p b`」
⇒ 已同步 D5 单测。

## 3. 批次 B（spike 结论：**不需要 `Pending`**）

- **真正的 bug 只有一处**：`{α : Sort u}` 这种**隐式**类型参数被误判成子目标（`apply Eq.symm.{0}` 多出
  `⊢ Prop`）。修法 = 判定信号加「名字出现在**后续任何一层的域**」⇒ 目标集 `['x = a']`；
  **闭合**的同类证明仍被内核拒（`期望 Sort(0)，实际是 Sort(1)`）。
- **误判更正**：`apply junk` 的 `['Sort 1','P']` **不是伪目标** —— `α` 是显式参数，Lean 同样把它当目标
  （`⊢ Type`）⇒ 「点名报错」的降级会拒绝合法写法，**已回退**（`SOKO_APPLY_PENDING` 删除）。
- **`ApplyArg::Pending` 不需要也做不到**：内核 `Expr::Meta` 是占位符（`add_declar` 硬拒、`eval` panic、
  `writer` 拒、`SyntheticOpaque` 明确「别拿类型约束填它」）⇒ 项级 mvar 是阶段 4 的地基，不是本批增量。

## 4. 判据（可执行）

```bash
bash scripts/apply-goal-driven-matrix.sh   # A 段三件必须全绿（exit 0）
cargo test -p sokonanoda-front --lib               # 含 D5 新期望 + 5 条新判据
python3 courses/set-theory/tools/check.py          # 课程门禁：258 目标 / 2190 checked / 0 判负（exit 0）
python3 scripts/gap.py check                       # 台账契约
python3 scripts/docs-lint.py                       # 文档预算
scripts/soko gate --fast                           # 环接收尾
```

- **结构计数**（禁绝对毫秒）：`JUDGE_INFER` 调用数、`prefix_bytes` 与基线相等；成功路径 elaborate +0。
- **反例锚**：关快路同一份解答 0.15s → 1.30s（≈8.5×）⇒ 禁止把「关快路」当修法。

## 5. 风险 / 回退 / 纪律

- **回退**：`SOKO_APPLY_GOAL_FIRST=0`（1a）· `SOKO_APPLY_LEVEL_TEXT=0`（1b）· `SOKO_APPLY_PENDING=0`（B）。
- **慢路**：`SOKO_JUDGE_INPLACE=off` 下 1a/1b **不生效**（实测）⇒ 本批取「写明边界」：逃生门保持今天行为，
  由阶段 4（内核合一）收口。
- **并行**：写入只在 `target/impl/src`；共享树有他人未提交改动。
- **连红 3 次**：降到真相层（front 单测断言子目标集合）再换招。

## 6. 协作项 C：strict `@` 的 level 收口（另排）

- 复现：`exact @Set.mem_univ Nat x` 在 strict 下报「期望 `Sort(1)`，实际是 `Sort(2)`」。
- 探针读数：`conv.rs` 的 Sort 比较两侧都是 `Zero` 基、差 **1 个 succ** ⇒ **不是未解层 mvar**，
  `level_solve` 不适用；项风格同样被拒 ⇒ 与 `apply` 无关（走 `exact`/项路径）。
- 下一步 P0-strict（30–60 min）定位「多一层」的构造点；No-go ⇒ 记已知限制。
