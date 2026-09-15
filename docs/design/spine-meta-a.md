# 设计：spine meta 方案 A —— refine 子洞的 kernel 级 expected type（2026-09-14）

> 触发：ROADMAP §10 I9 余项「refine 的子洞 kernel 级 expected type（spine
> meta）」、`docs/notes/gap-analysis.md` 远期设计项「spine meta 方案 A」。
> 依据：`docs/design/round14.md` §0.4（A/B′/C 取舍）、
> `docs/design/goal-func-spine.md` D3/D4、`docs/protocol.md`（半表达式
> hover 的 `judge_infer` + 有界缓存纪律）。**本文只定方案，不改码。**

## 1. 现状与限制（B′ 的天花板）

子洞期望类型现在由前端**语法级 walk** 生成，全部在
`crates/front/src/compile/goals.rs`：

- 模板来自源内 `axiom`/`def`/`theorem`/归纳构造子，或 Eq prelude 文本
  （`GoalTemplates`，`goals.rs:48`）；
- 构造子 spine（`ctor_spine_case`，`goals.rs:553`）取「结果头对齐」的字段；
- 函数实参洞（`func_spine_case`，`goals.rs:596`）；
- 字段类型实例化是**纯 AST 替换**（`field_type_text`，`goals.rs:516`；
  `instantiate_binder_type`，`goals.rs:731`），B′ 已升级为深替换 + 遮蔽守卫。

由此产生的硬限制（`goals.rs` 注释与 round14 §0.4 已记）：

1. **无 def_eq**：类型是语法渲染，不做 delta/β/iota。别名或定义包裹的
   类型必须手写 `unfold_def_once`（`goals.rs:703`）这类启发式，覆盖不到
   泛化情形；`Not a`、`Eq α x y` 之外的复合形状容易漏。
2. **依赖/超量应用靠猜**：`func_spine_case` 的 over-applied 分支只展开
   一层 def 体，`unfold_def_once` 有 8 次上限；失败即 `None`。
3. **前置洞即断链**：前一个实参是洞 → 后一个洞 `ty = None`（面板 `?`）；
   无法用「假设该洞满足其期望类型」继续穿透。
4. **嵌套洞不可能**：`f (g sorry)`、`fun (x : T) => sorry` 直接判
   `elab-hole-misplaced`（`goal-func-spine.md` D4），没有 expected 穿透。
5. **局部假设是覆盖层近似**：`local_func_templates`（`goals.rs:251`）只按
   语法收集带类型 binder，类型不经内核。

结论：B′ 是「建议生成的廉价近似」，其边界正是**没有 kernel 参与**。

## 2. 方案 A：elab 实参级 expected 穿透（kernel 驱动，前端落地）

核心认识：我们不需要内核的 metavariable / unknown 概念（方案 C 的障碍），
因为「第 i 个实参的期望类型」= **部分应用 `<head> a₁ … aᵢ₋₁` 的类型剥掉
最外层 Pi 的 domain**。部分应用本身就是有类型的开放项，现有内核原语
`infer_under_binders(&[binder_ty], e)`（architecture §5.3，hover 已用）可
在给定局部上下文下推断并 quote 回来。前端已有同形的探针：`front::judge`
的 `judge_infer`（`by-tactics.md` §4）合成 `#check fun <binders> => <term>`
走完整内核。

具体机制（全部在 front，内核语义零改动）：

1. **头类型而非模板**：第 i 洞的期望类型 = 探针
   `fun <intros> => <head> a₁ … aᵢ₋₁` 的结果类型文本 → `peel_pi` 取
   domain。内核负责 delta/defeq，别名/定义/依赖实例化自然正确。
2. **穿透（前置洞）**：第 j < i 个实参若是洞，用**递归算得的期望类型**
   把它提升为局部 binder `h_j : A_j`，代入 `<head> a₁ … aᵢ₋₁` 后继续；
   即双向检查的 expected 传播。算不出（非直接洞/无界）则退回 `None`
   （与 B′ 同语义，不倒退）。
3. **嵌套洞**：对实参表达式做 expected 驱动的递归——外层得到 `A_i`，
   再以 `A_i` 为期望对 `g ?` 求其头部的实参期望，直到洞位。v1 可只做
   **直接实参 + 一层嵌套**，深层按 `None` 回退。
4. **消费时机**：与半表达式 hover 同纪律——**请求期惰性计算**
   （`soko/goals` / inlay / hover），有界指纹缓存（128），绝不进
   keystroke / didChange 路径。`open_goal` 仍用 AST 走查只做「是否 Open +
   洞 span」（廉价，不能丢）；`sub_goals[i].ty` 改为按需 kernel 探针填充。

若实现中发现确需一个**新的显示层内核原语**（如批量「部分应用类型」），
按 architecture §6 的冻结改动规则办：仅冷路径、纯显示、三层回归、
`docs/architecture.md` §6 记账；默认假设是**不需要**。

## 3. 备选与取舍

| 方案 | 结论 |
|---|---|
| B′（已实现） | 保留为**快路径/回退**：模板命中且无洞穿透时零探针；A 不可得不倒退 |
| C（judge 合成 fresh axiom / metavariable） | **否决**：内核无 unknown 项概念（round14 §0.4）；metavariable 会侵入内核 |
| 改内核为完整双向 elaborator | **否决**：违反内核冻结（REQUIREMENTS §2 第 1 条），爆炸半径过大 |
| 移植 Lean 求解器 / 文本比对 | **禁止**：REQUIREMENTS §2 第 2/4 条 |
| 命令期就全量 kernel 渲染 | **否决**：需把子洞类型计算挪到 ops 期，pipe 重构大；用请求期惰性探针替代 |

## 4. 测试计划（三层）

- **front**（`crates/front/src/compile/tests.rs`）：B′ 返回 `None` 的
  形状给出正确文本——defeq 别名（定义包裹的字段类型）、依赖字段
  （`p a` 经替换）、一层嵌套洞（`f (g sorry)` 取内层期望）、前置洞穿透
  （`f (sorry) sorry` 后者用前者的期望）；**B′ 既有断言逐条不变**
  （值相等回归）。
- **CLI e2e**（`crates/cli/tests/cli.rs`）：`--json` 事件形状**零变化**
  （不新增事件、不改 payload），open 练习仍 exit 0、无 diagnostic。
- **LSP**（`crates/lsp/src/{lib,inlay}.rs`）：上列形状下 `sub_goals[i].ty`
  与 inlay 非空；契约测试保持「客户端不得文本扫洞」；有界缓存不放大
  （请求期只算一次）。
- **kernel**：默认不新增测试；仅当新增显示原语时按 §6 三层（memory_api +
  front + CLI）。

## 5. 验收

- round14 §0.4 / `goal-func-spine.md` D4 列出的缺口关闭：嵌套洞与
  defeq 别名两条从「`elab-hole-misplaced` / `?`」变为可用期望类型；
- `soko/goals` 的 `sub_goals` 字段形状不变（仅更多非 null `ty`），
  `docs/protocol.md` 措辞若需澄清只补一句；
- 内核语义零改动（除非 §2 末的显示原语，且记账 + 三层测试齐备）；
- `sokonanoda gate` 全绿；`STATUS.md` / `REQUIREMENTS.md §9` 同步；
  版本按 `docs/vscode-dev-guide.md` §2 判断（信息更正确 = patch；若引入
  新的公开 front API 则 minor）。

---

## 6. as-built（2026-09-14，0.32.0）

- **front**（`goals.rs`）：新增公开 `probe_sub_goal_types(doc_src, options,
  decl_span) -> Vec<(hole_offset, ty)>`——请求期重解析 → 定位声明 → 带
  `judge_infer` 重跑走查。第 i 实参期望 = 部分应用 `<head> a₁…aᵢ₋₁` 的类型剥
  最外层 Pi domain；前置洞提升为 `_h0/_h1…` 局部 binder 并递归取期望
  （穿透）；一层嵌套洞（`f (g sorry)`）取内层期望。`open_goal` 仍以
  `probe=None` 运行 → **键路径零内核调用**。
- **LSP**（`lib.rs`/`inlay.rs`）：`probed_report(doc)` 仅在 `soko/goals`、
  hover、inlay 请求期把 `None` 的 `sub_goals[i].ty` 补上；`stateAt`/`nextHole`
  不探测。协议形状与洞数不变（仅更多非空 `ty`）。
- **测试**：front 4 条（defeq 别名 + 前置洞 → `Not _h0`、依赖字段替换
  `_h1 a`、一层嵌套、深于一层回退）+ LSP 4 条；B′ 既有断言逐条不变；perf
  无回退（goals/hover/completion 0ms、didChange 1ms、缩放比 6.8×）。
- **版本** 0.31.0 → **0.32.0**（新增公开 front API → minor）。
- **未闭环（留档）**：超量应用下「def 包裹的结果类型」的 whnf 展开需要
  内核/pp 暴露 whnf（违反冻结）→ 仍走 B′；更深嵌套/非 spine 实参仍 `None`。
