# 设计：`match` 依赖 motive（2026-09-14）

> 现状：`match` 的 motive 恒为 `fun (_ : Ind) => R`（非依赖），分支期望类型都是
> 常量 `R`。本设计支持**依赖 motive**：结果类型随 scrutinee 取值变化，例如
> `theorem foo (n : Nat) : P n := match n with | Nat.zero => … | Nat.succ k => …`。
> kernel 冻结；复用既有 `goals::substitute_names`（shadow-aware AST 替换）。

## 1. 触发条件（v1）

- scrutinee 是**裸局部变量** `x`（`Expr::Ident`），且期望类型 `R`（`expected_src`）
  中**出现** `x` → 依赖 motive；
- 否则保持**常量 motive**（当前行为，完全兼容）。
- 不做：scrutinee 为非变量表达式的一般 dependent elimination（需把整个表达式的
  值抽象出来）；带索引归纳的 dependent motive（v1 之外）。

## 2. 构造

- 取新鲜 motives 名 `t`（避让作用域名），
  **motive = `fun (t : Ind params) => R[x := t]`**（`substitute_names(R, {x ↦ t})`）。
- 第 i 分支（构造子 `C`，模式变量 `v…`）：
  **期望类型 = `R[x := C params v…]`**（把 scrutinee 变量替换成该分支的构造子项）；
  branch body 以此为 expected（`expected_src`），其 kernel 形式由 elaborate 得到。
- 递归字段的**归纳假设 IH** 类型 = `motive <field>` = `R[x := field]`（依赖 IH，
  不再是常量 R）。
- **宇宙 level**：与现状一致，由 `judge_infer(R)` 映射 Sort（R 在声明作用域里，
  x 是已绑定的局部量；依赖类型要求各分支同 Sort）。

## 3. 与既有特性

- **参数化归纳**：params 部分不变（从 scrutinee 书写源类型取）；motive 域为
  `Ind params`；dependent + params 可组合。
- **`sorry` 洞 / goal 视图**：`goals.rs` 的 match-arm 走查须同样把 `x` 换成
  构造子项，使 branch 洞的期望类型正确（`R[x := C …]`）；非依赖路径不变。
- **判定的 sound 性**：全部交给完整内核；前端只做结构替换 + 组装。

## 4. 测试三层

- **front**：依赖结果（`P n`）的 `match` 通过内核；`zero`/`succ` 分支各自期望
  `P Nat.zero` / `P (Nat.succ k)`；IH 依赖（IH : `P k`）；非依赖回归（常量 R）；
  非变量 scrutinee 仍走常量 motive（不报错）；branch `sorry` 的洞期望为
  `P <ctor>`。
- **CLI e2e**：依赖 `match` 文档 `decl.checked`；开放练习 `exercise.open`。
- **课程**：unit5 加一个依赖 `match` 演示（如 `P : Nat -> Prop` 的分情况）+ 练习。
- 文档：`docs/design/match.md`（Phase 3 记录）、`architecture.md`、`TESTING.md`。

## 5. 验收

- 依赖 `match` 被完整内核接受；分支/IH 期望类型正确；非依赖无回归；
- kernel 零改动；`gate` 全绿；版本 minor；STATUS/REQUIREMENTS §9 同步。

## 6. 风险

| 风险 | 缓解 |
|---|---|
| 替换不 shadow-aware，误替换被遮蔽的 `x` | 用 `goals::substitute_names`（已 shadow-aware） |
| 分支期望 kernel 形式与 motive 应用不一致 | 两端都用同一替换结果 elaborate；内核 def_eq 抓 |
| goal 走查未同步 → 洞期望错 | §3 专门改 `goals.rs` match-arm 走查 + 单测 |
| level 取错 | 与现状同法；测试覆盖 Prop 结果 |

---

## 7. as-built（2026-09-14，0.39.0）

- **触发**：scrutinee 是裸局部变量 `x` 且 `R` 含 `x` → 依赖；否则常量 motive
  （完全兼容）。
- **构造**：motive `fun (t : Ind params) => R[x:=t]`（`substitute_names`，
  shadow-aware）；分支期望 `R[x:=C params v…]`；IH 类型 `R[x:=field]`；motive/
  分支/IH 类型都在其 binder 存活的 scope 里 elaborate（否则 de Bruijn 错位）。
- **level**：`infer_expected_level` 改为只把 `R` **依赖到**的 binder 纳入
  `judge_infer` 望远镜（`ElabScope::judge_binders_for`），修掉「无关的函数型
  binder 破坏 telescope 渲染」导致的 level 查询失败（声明 binder 形式的
  `nat_induction` 因此可用）。
- **goal 视图**：`goals.rs` 的 match-arm 走查同样做 `x := C params v…` 替换，
  分支 `sorry` 期望类型为 `R[x:=ctor]`。
- **测试**：front `match_dependent_*`（含 `match_dependent_motive_sees_declaration_binders`
  ——`nat_induction` 声明 binder 形式）；CLI `cli_match_dependent_motive_checks_via_kernel`；
  课程 unit5 加依赖 match 节（`nat_induction` + 练习 8）；golden
  `(9,7,4)→(10,8,4)`、汇总 `checked 53→54 / open 40→41`。
- **版本** 0.38.0 → **0.39.0**（新能力 minor）。
- **已知限制**：motive 引用「自身类型是以箭头结尾的依赖函数」的 binder 时，
  `judge_infer` 的 render→parse 往返仍可能腐蚀 telescope（完整修需 `judge.rs`
  一次性解析内核类型、或 `proof::render_expr` 给 domain 位的 `Forall` 加括号）。

---

## 8. 已修：`judge_infer` 往返健壮性（2026-09-14，0.39.1）

§7 的「已知限制」（motive 引用类型为依赖函数的 binder 时 telescope 被腐蚀）已修：
`proof::render_expr` 的 **Arrow domain 位**改用 `render_fun_position`（Lambda/
Forall/Arrow/Plus/Let/Match 一律补括号），因此内核类型文本 → 前端 AST 的往返不再
右结合误读。回归：`render_expr_round_trips` 增「Forall 作 domain」用例 +
`match_dependent_motive_with_function_typed_binder_round_trips_safely`（结果类型
`Q hs n`，`hs` 为依赖函数 binder）→ 内核通过。该修复同时保护 `judge_infer` 的
其他消费方（建议/半表达式 hover/level 查询）。
