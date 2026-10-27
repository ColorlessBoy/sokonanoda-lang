# 单元② 目标状态 / `@` 显示 / `apply False.elim` 缺陷分析

> **位置说明**：本报告放在 `reports/`（不在 `docs/**` 里），因为 `docs/` 的
> 活文档预算与过期登记是硬门禁（`scripts/docs-lint.py` ①②④，
> `scripts/docs-expiry-check.py`）。要把它收进 `docs/`，先按
> `docs-expiry-check.py --seed/--renew` 登记，并处理预算（归档/真删等量文档）。

**日期**：2026-10-27
**环境**：sokonanoda `0.97.10`（仓库 release 构建，`aarch64-apple-darwin`），
判卷走 `scripts/soko`（`grade` / `query state` / `query check`）。
**范围**：本次对话里出现的全部四类现象，外加“发布前夕”的可用性判断。
**性质**：分析 + 修复记录。**内核零改动**；B/C 两个修复落在 front/query/display，
E 是**未修**的真 bug（最小复现已固定）。

> 复现件都在 `/tmp`，命令用绝对路径（台账 G-12）。课程文件的模块根是
> `courses/set-theory`，所以带 `import` 的复现件一律加
> `--root "$PWD/courses/set-theory"`。

---

## 0. 结论摘要

| 编号 | 现象 | 判定 | 严重度 | 状态 |
|---|---|---|---|---|
| **A** | `apply Eq.subst h` 报错；随后 `apply subset_antisymm` 也报错 | **用户代码**（少 `Set.notMem_empty` 一步）；第二条是**连坐** | — | 已解释；三种修法均判绿 |
| **B** | `·` 块里每一行的 goal state 都是块入口的聚焦态（`intro x hx` 显示错） | **真 bug**：front 状态选择器取了最外层 `·` 的 span | **P0**（做题主循环） | **块内已修**；**块后（G）仍未对齐 Lean 4** |
| **C** | `hx : @x ∈ A` 多一个 `@` | **真显示 bug**：canonical pp 的 explicit spine + fold 不吃 `@` | P1（观感 / 正确性错觉） | **已修**；front 测试 + release 复验通过 |
| **D** | `exact False.elim (A x)` 反而编译通过 | **不是 bug**（默认方言）：`C` 是隐式，但默认方言保留“旧全参数写法”兼容 ⇒ `(A x)` 填 `C`，`h` 未给 ⇒ `False → A x`；strict 档要写 `(C := A x)` / `@` | — | 语义说明 |
| **E** | `apply False.elim` 报“被应用项没有通过内核检查” | **真 bug**：`apply` 在箭头目标上把目标域 `h : False` 带进了检查 | P1（apply / 诊断） | **未修**；最小复现已固定，根因已收窄 |
| **F** | 失败的 `by` 块后面的位置停在最后成功步 | B1 的既有设计（失败块保留成功步）；不是“前面的状态坏了” | P2（体验） | 建议改 UI / 错误恢复 |
| **G** | 块与块之间、块后的 gap state 仍是分支局部的空状态 | B 的**残留**：父层剩余目标没有记录；**块后不算对齐 Lean 4** | P2 | 待办（需加 after 状态） |
| **H** | `#check False.elim (A x) (A x)` 印 `A x`，但声明里同一项被内核拒绝 | `#check` 只推断、不核验实参 | P2（诊断误导） | 待确认/记录 |
| **I** | `  · intro h` 后回车，下一行缩进不对 | 编辑器 `language-configuration.json` 缺 Lean 4 的 `onEnterRules` | P1（打字手感） | **已修**（配置 + 静态判据；需 Reload Window） |
| **J** | `False.elim (A x)` 在默认档仍可用（用户以为“已禁止隐式”） | 严格档已实现，但**默认档仍是 loose**，课程清单也没 opt-in；翻默认被性能 14× 阻塞 | P1（政策落地） | **未落地**（决策/迁移问题，不是代码缺失） |

一句话：**A 和 D 不是 bug（D 只限默认方言，strict 档见 §5.3）；B、C、E 是真 bug，其中 B 是“goal states 全坏”的主因。**
B/C 已修，E 需要单独处理。`·` 的缩进/对齐实现本身没错，错的是**块入口状态被当成了块内每一行的状态**。

---

## 1. 复现与判据

### 1.1 用户的两段代码

第一段（**带 `·`**，第一次报“goal states 全坏”）：

```lean
theorem eq_empty_iff_forall_notMem (α : Type) (A : Set α) :
    (A = ∅) ↔ (∀ (x : α), ¬ (x ∈ A)) := by
  constructor
  · intro h
    intro x hx
    apply Eq.subst h
    exact hx
  · intro h
    apply subset_antisymm
    · intro x hx
      exact False.elim (x ∈ ∅) (h x hx)
    · exact empty_subset α A
```

第二段（**不带 `·`**，用户说“只输入这个就还正常一点点”，并问 `@`）：

```lean
theorem eq_empty_iff_forall_notMem (α : Type) (A : Set α) :
    (A = ∅) ↔ (∀ (x : α), ¬ (x ∈ A)) := by
  constructor
  · intro h
    intro x
    apply Eq.subst (Eq.symm h)
    intro hx
    exact hx
  intro h
  apply Set.ext
  intro x
  constructor
  exact h x
  exact False.elim (A x)
```

这两段的差别**恰好把几个 bug 分开**：第一段的 `intro x hx` 在 `·` 里 ⇒ B 发作；
第二段的**第二个分支**（`@` 与 `False.elim` 所在）**没有 `·`** ⇒ 那几行的状态
不受 B 影响，暴露出来的是 C（`@`）和 E（`apply False.elim`）。第一个分支仍有一个
`·`；如果光标落在它里面，B 同样会发作。

### 1.2 复现件与命令

```bash
# A：第一段（带 ·），在 unit02 的上下文里判
scripts/soko grade  --root "$PWD/courses/set-theory" /tmp/probe-A_user.sokonanoda --json
# B：无 import 的最小 bullet 夹具（绿）
scripts/soko query state --file /tmp/bullet-probe.sokonanoda --line 8 --col 7
# C：intro 出 hx 的那一格
scripts/soko query state --file /tmp/at-probe3.sokonanoda \
  --root "$PWD/courses/set-theory" --line 8 --col 5
# D：用户原样第二段
scripts/soko grade  --root "$PWD/courses/set-theory" /tmp/false-elim-exact.sokonanoda --json
# E：apply False.elim 的最小复现（无 import）
scripts/soko grade  /tmp/apply-elim-probe.sokonanoda --json
```

---

## 2. A —— `apply Eq.subst h` 的报错（用户代码，不是 bug）

### 2.1 事实

- 复现件 `/tmp/probe-A_user.sokonanoda`：`exit 1`。
- 诊断 1（第 102 行，`apply Eq.subst h`）：
  `elab-implicit-argument-unsolved` —— `Eq.subst` 的 4 个隐式形参
  （`α`、`p`、`a`、`b`）至少一个补不出来。
- 诊断 2（第 105 行，第二条 `·` 的 `apply subset_antisymm`）：
  `elab-tactic-failed` —— “`subset_antisymm` 的结果是 `A = B`，无法对齐当前目标
  `(x : α) → ¬ (x ∈ A)`”。

### 2.2 为什么是用户代码

`intro h` 之后是 `∀ x, ¬ (x ∈ A)`；`intro x` 之后是 `¬ (x ∈ A)`；
再 `intro hx` 会把 `Not` 展开成 `x ∈ A → False` ⇒ **手里是 `hx : x ∈ A`、目标是 `False`**。

`apply Eq.subst h` 此时：

- `Eq.subst` 的谓词 `p` 只能从「同一应用里后面的显式实参」和「当前目标」反解；
- 只给 `h` 时，`p` 没有来源；目标 `False` 只告诉它 `p ∅ = False`，定不出 `p`；
- 就算把 `hx` 补上（`apply Eq.subst h hx`），结果是 `x ∈ ∅`，**对不上 `False`**；
- 真正要做的是先**把目标从 `False` 退成 `x ∈ ∅`**（`Set.notMem_empty x`），再重写。

三种判绿写法（都跑过，`decl.checked`）：

```lean
-- 保住你的 intro x hx
· intro h
  intro x hx
  apply Set.notMem_empty x
  apply Eq.subst h
  exact hx

-- 练习里原有的骨架：不 intro hx
· intro h x
  apply Eq.subst (Eq.symm h)
  exact Set.notMem_empty x

-- 一步项风格
· intro h x hx
  exact Set.notMem_empty x (Eq.subst h hx)
```

### 2.3 第二条诊断是连坐

第一条失败 ⇒ 第一个 `·` 块整体回滚 ⇒ 第二个 `·` 拿到的是**第一个方向还没做完的
目标** (`(x : α) → ¬ (x ∈ A)`)，所以 `apply subset_antisymm` “对不上”。
把第一段单独留 `sorry`、第二段原样跑：`exit 0`、零诊断 ⇒ 第二段本身是对的。
修好第一段后拼回去，整条 `decl.checked`。

**判定**：用户代码问题；错误信息本身是准确的。练习的 hint 已经写了
“→ 用 `Eq.subst` 与 `Set.notMem_empty`”，缺的正是后者。

---

## 3. B —— `·` 块内的 goal state（真 bug，P0）

### 3.1 现象

一个**全绿**的 `·` 证明里，块内每一行（包括 `intro x hx` 这种第二条 tactic）
都显示**块入口的聚焦目标**：

最小夹具（`/tmp/bullet-probe.sokonanoda`，绿）：

```lean
theorem with_bullet (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  constructor
  · exact hp
  · exact hq
```

修复前：

```
L8 C5  （exact 起点）  step 1  goal P
L8 C7  （exact 内部）  step 1  goal P     ← 应为 step 2 / 无目标
L9 C5  （exact 起点）  step 3  goal Q
```

`·` 的 span 是**整块**（`[185,196]` = `· exact hp`），所以块里任何位置只要
不在某条内层 tactic 的**开区间**里，就被这块的 `step` 吞掉。

unit02 绿变体（`/tmp/probe-E_notMem_first.sokonanoda`）修复前：第 101–104 行
（`intro x hx`、`apply Set.notMem_empty x`、`apply Eq.subst h`、`exact hx`）
**全部** `step 1`，显示的是 `intro h` 之前的第一个子目标。修复后：

| 行 | 光标处 | step | 目标 |
|---|---|---|---|
| 101 | `intro x hx` 起点 | 2 | `(x : α) → ¬ x ∈ A`，`h : A = ∅` |
| 102 | `apply Set.notMem_empty x` 起点 | 3 | `False`，`h, x, hx` |
| 103 | `apply Eq.subst h` 起点 | 4 | `x ∈ ∅` |
| 104 | `exact hx` 起点 | 5 | `x ∈ A` |

### 3.2 根因

两处语义**没有对齐**：

1. `crates/front/src/by.rs` 的 `Tactic::Bullet` 分支在**跑块体之前**推一格：

   ```rust
   // `·` 的步进 = 进入聚焦（hover 看到聚焦目标，不是块后的外层剩余）。
   steps.push(ByStep {
       span: *span,          // ← 整个块的 span
       goals: vec![ByGoal {  // ← 聚焦目标（入口态，不是后态）
           ty: render_goal(...), binders: context_binders(...)
       }],
   });
   ```

   普通 tactic 的 `ByStep` 记的是**执行后**的状态；`·` 这一格记的是**入口**状态。
   两者在 `ByStepState` 里**没有标记区分**。

2. `crates/front/src/query/state.rs` 的 `select_state_at` 用
   `position`（**第一条** span 包含光标的 step）⇒ 外层的 `·` 永远排在
   内层兄弟前面，**遮住**了块内所有 tactic 的真实后态。
   光标恰在一条内层 tactic 起点时，开区间 `start < cursor` 又不成立
   ⇒ 仍然落到 `·` 的入口态。

3. **测试缺口**：`crates/front/src/query/tests.rs` 与
   `crates/lsp/src/tests/state.rs` 的状态夹具**没有一条带 `·`**
   （`grep` 只在注释里出现 `·`）。B2/2026-10-08 的判据全部是
   `intro/apply/sorry` 的平铺 `by` 块 ⇒ 这条路径没被覆盖。

### 3.3 修复

新增 `select_state_at_in_text(d, cursor, text)`（`crates/front/src/query/state.rs`）：

- **① 光标恰在一条 step 的起点**：
  - 这一步是 `·`（`text[span.start..]` 以 `·` 开头）⇒ 用**它自己的** goals（聚焦态）；
  - 否则 ⇒ 用**上一条**的后态（同分支兄弟的进入态）；`i == 0` ⇒ 根状态。
- **② 否则**：取**最后一条已经开始的** step（`rposition(start < cursor)`）的后态。
  源码顺序就是嵌套顺序 ⇒ 自动是最内层；块入口的 `·` 会被块里后面的兄弟盖住。
- **③ 第一条之前** ⇒ 根状态。

`is_bullet_step` 只看源码 `span.start` 处是不是 `·`；`have` 的 span 从 `have`
开始，不会误判。接入点：

- `QueryDoc::state_at_with`（`query state` 与 `soko/stateAt` 共用）；
- `QueryDoc::scope_binders_at`（补全的上下文）；
- LSP `tactic_goal_hover`（悬停 tactic 的 goal 块）。

旧的无文本 `select_state_at` 保留为 wrapper（`text = ""`），给只测语义的单测用。

### 3.4 验证

- 新增回归：`query::tests::state_at_inside_a_bullet_uses_the_inner_tactic_not_the_bullet_entry`
  —— 断言 `exact hp` 起点 = step 1 / `P`、内部 = 空目标、`exact hq` 起点 = `Q`。
- 13 条 `state_at*` 单测全绿（含 B2 的起点/内部/末尾边界）。
- release 复验（最终构建）：unit02 绿变体第 101–104 行分别为 step 2/3/4/5，
  目标如上表；`hx` 的 `@` 已消失（`x ∈ A`）。
- **失败变体的失败行起点**（`apply Eq.subst h`，第 102 行）现在答 step 3 /
  `False`（失败前最后一个成功步）；第 105 行（第二条 `·`）仍是 step 3 /
  `False` —— 这是失败截断（F），不是 B 复发。
- 判卷计数不变：unit02 画布仍是 `6 checked · 5 open · 0 failed`。

### 3.5 残留 G（本轮未修）

`·` 块**结束之后**、下一条 tactic 之前的 gap（空白/行尾），选择器拿到的是
**块内最后一条**的后态。那条后态是**分支局部**的（分支已闭合时是 `[]`），
不是父层剩下的目标。例：`with_bullet` 里第一条 `·` 结束后的换行处会显示
“已无目标”，而实际还剩 `Q`。下一条 `·` 的**起点**是对的（聚焦态 `Q`）。

最常见的残留不是“空白”，而是**块后直接写不加 `·` 的兄弟**：

```lean
theorem mixed (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  constructor
  · exact hp
  exact hq
```

在 `exact hq` 起点，Lean 4 显示 `Q`；我们显示 `goal: null`（选中的是
`exact hp` 的分支局部后态）。第一条 `·` 结束后的换行处也一样（`null`，不是 `Q`）。
⇒ **B 的块内语义已对齐，块后没有**；`mixed` 这种“第一条用 `·`、其余不带”的
写法很常见，所以发布前值得修。

要彻底修：在 `Tactic::Bullet` 跑完块体、`worklist.retain` 之后，把**父层的
剩余目标**记下来（给 `ByStep`/`ByStepState` 加一个 `after` 字段，或推一格
零宽的“块后”状态），选择器在“上一条属于一个已结束的 `·`”时改用它。
这会改 `ByStepState` 的序列化形状 ⇒ `REPORT_SHAPE` 要 +1，旧缓存要整体失效；
本轮没做。

### 3.6 `·` 的缩进 / 分组本身没有坏

本次所有带 `·` 的探针（绿的 `with_bullet` / `nested_bullet`、unit02 的两段）
都按预期解析并进入了对应分支；判卷结果与 Lean 的分支语义一致。
坏的是**块入口状态被当成块内每一行的状态**（选择器），**不是**缩进/对齐。
⇒ 这条 bug 不该修到 parser 里；用户“缩进功能漏洞百出”的直觉，来源是
goal state 的错位，不是分组错位。

---

## 4. C —— `hx : @x ∈ A` 的 `@`（真显示 bug）

### 4.1 现象与链路

`/tmp/at-probe3.sokonanoda`（无 `·`，open）在 `intro x hx` 之后：

```json
{ "name": "hx", "ty": "@x ∈ A" }
```

链路（全部是显示侧，**判定文本不动**）：

1. 目标是 `¬ (x ∈ A)`，`intro hx` 要剥 `Not`（def 头）⇒ 源码 AST 剥不动，
   走 `canonical_goal_with_spec` 的**内核 pp 规范形态**；规范形态的 binder 域是
   **全显式应用** `@Set.mem α x A`（`explicit_spine: true`）。
2. `by_step_states` 用 `render_expr` 渲染这个域 ⇒ 文本 `@Set.mem α x A`。
3. `display.fold` 解析它；`fold_spine` 把 `Set.mem` 折成 `∈`，替换范围是
   **表达式 span**。但 parser 的 `@` 是**先 bump 掉、再解析头**的
   （`parse_app`）⇒ `@` **不在 span 里** ⇒ `splice` 从 `Set.mem` 开始替换，
   把 `@` 留在原地 ⇒ `@x ∈ A`。

### 4.2 修复

- `display/spans.rs`（新）新增 `fold_span_including_explicit_marker`：折叠替换范围
  向左扩到紧邻的 `@`（允许中间有空白）。记法形态是源级写法，不需要 `@`。
- `fold_spine` 增加一条**部分应用**守卫：显式脊只在 `k == full`（操作数齐全）时折；
  `@Set.mem α a`（`k == full - prefix`）与闭项完全应用 `Set.mem a A` 的实参个数
  同形，折了会把类型参数当第一个操作数（`α ∈ a`）⇒ 不折。
- 测试：`display::tests::folding_an_explicit_application_drops_the_at_marker`
  （`@Set.mem α a A` → `a ∈ A`；`@Set.mem α a` 保持原样）。
- release 复验：`hx : x ∈ A`。

**注意**：`@` 是**真语义**（关闭隐式插入）；判卷/`--json` 的文本仍然带 `@`，
只有显示折叠把它去掉。这是有意的分工。

---

## 5. D —— `exact False.elim (A x)` 为什么能过（`C` 是隐式；默认方言的兼容规则）

用户追问：“`False.elim` 的 `C` 不是被声明成隐变量了吗？”——**是**。
prelude 的签名就是：

```
False.elim : {C : Prop} → (h : False) → C
```

所以 `exact False.elim (A x)` 能过，不是因为 `C` 是显式的，而是因为**默认教学方言**
保留了一条“旧全参数写法”的兼容规则。

### 5.1 那一刻的目标

`Set.ext` + `intro x` + `constructor` 之后，第二个子目标是：

```
x ∈ ∅ → x ∈ A
```

`Set.empty` 的定义是 `fun _ => False` ⇒ `x ∈ ∅` **定义上**就是 `False` ⇒
目标就是：

```
False → A x
```

### 5.2 兼容规则：显式类型实参可以填隐式前缀

`try_implicit_application_with`（`crates/front/src/compile/elab.rs`）在
`fits_old_style` 为真时，把位置实参按**整条望远镜**（含前导隐式层）逐位对齐：

```rust
let fits_old_style = !strict_at
    && args.len() <= layers.len()
    && args_fit_layers_in_order(&layers, &args, ...);
```

`False.elim` 的层是 `[{C : Prop}, (h : False)]`；`(A x) : Sort 0` 与第 0 层
`C : Prop`（也是 `Sort 0`）贴合 ⇒ `C := A x`，`h` 层没被应用 ⇒ 项的类型是
`False → A x`，正合目标。

**证据**（`/tmp/false-elim-check.sokonanoda`）：

```
#check False.elim          ⇒ {C : Prop} → (h : False) → C
#check False.elim (A x)    ⇒ False → A x
#check @False.elim (A x)   ⇒ False → A x
```

### 5.3 严格档（Lean 4 语义）下它会红

`--dialect strict`（或 VS Code 的 `sokonanoda.dialect.strictImplicitArgs`）关掉
`fits_old_style`：`False.elim (A x)` 的 `(A x)` 只能落到显式 `h : False` 上 ⇒

```
kernel-rejected：期望 False，实际是 Sort(0)
```

而两种 Lean 写法在**两档都绿**：

```
False.elim (C := A x)      -- 命名实参
@False.elim (A x)          -- 全显式
```

⇒ 课程默认方言里 `False.elim (A x)` 合法；严格档不是。D 的结论对**默认档**成立，
但原因是兼容规则，不是“`C` 被声明成显式”。画布里的 `False.elim (x ∈ A) hx`
依赖的是同一条规则。

### 5.4 没有 soundness 漏洞

`False.elim (A x) (A x)`（把第二个 `(A x)` 当 `False` 的证明）在声明里被内核拒绝：

```
kernel-rejected：期望 False，实际是 Sort(0)
```

⚠ 但 `#check False.elim (A x) (A x)` 在**默认档**会印 `A x` —— `#check` 只做推断、
不核验 h 实参 ⇒ 这是诊断层的另一个缺口（**H**），不要拿 `#check` 当“内核接受”的证据。
`exact` 路径没有这个问题：它走内核终审。

---

## 6. E —— `apply False.elim` 的内核检查错（真 bug，未修）

### 6.1 最小复现

```lean
theorem t1 (P : Prop) : False → P := by
  apply False.elim
```

`exit 1`，诊断（`elab-tactic-failed`）：

```
`apply` 的被应用项没有通过内核检查：期望
`∀(P : Sort(0)), ∀(C : Sort(0)), ∀( : False), C`，实际是
`∀(P : Sort(0)), ∀(h : False), ∀(C : Sort(0)), ∀( : False), C`。
```

`t2 (P) (h : P) : P → P := by apply False.elim` 与
`t3 (α) (A) (x) : x ∈ ∅ → x ∈ A := by apply False.elim` 报同形的错
（多出的 binder 分别是 `h : False` / `h : False`）⇒ **与 `Set`/`∅` 无关**，
是 `apply` 在**箭头目标**上的通病。

### 6.2 观察

- “actual”比“expected”**多一个目标域的 binder**：目标是 `False → …`，
  actual 就有 `h : False`；`SOKO_JUDGE_INPLACE=off` 照样复现 ⇒ **不是**
  `infer_type_text_inplace_expected` 的快路径。
- `exact False.elim (A x)` 不带这个 binder，过；`apply False.elim` 带，挂。
- `apply` 的完整检查在 `by.rs:2652`（`checked_apply_mismatch`，
  `apply_probe.rs:253`）。它把 `check_spec.ty = f_ty`（被应用项的类型，
  不含目标域的 binder），而内核报的 actual 含目标域的 binder ⇒ 两边不同源。
  嫌疑集中在 `apply_tactic` 用 `apply_goal` 做期望类型的那条路，以及
  spec 的 binder 集合是否把目标 Pi 域算了进去。

### 6.3 影响与判定

- **不是 soundness bug**：`exact` 路径正常；被拒绝的是一个合法的 `apply`
  （Lean 4 的 `apply False.elim` 在 `False → P` 上会留下子目标 `False`，
  不会报内核检查错）。
- 是**真 bug**（假拒绝 + 诊断把人引向“显式实参类型不对”的错误方向）。
  ⚠ 这里说“Lean 会留下子目标 False”是按 Lean 4 的 `apply` 语义推的
  （结论 `C` 与目标合一），**没有**跑官方 Lean 工具链验证（仓库硬规则 2）。
- **未修**：不要在没有回归判据（`t1` 修前红、修后绿）和反向验证前动它。
  下一步：在 `checked_apply_mismatch` 打印 `spec.binders` / `f_ty` / 目标的
  Pi 域，确认多出的 binder 是谁加的；如果是 spec 把目标域当成了上下文，
  修 spec；如果是 `apply` 的期望类型把项 eta 展开了，修那一段。

---

## 7. F —— 失败块后面的状态（B1 设计，不是“前面的状态坏了”）

无 `·` 的失败版本（`/tmp/nobullet-fail.sokonanoda`）：

| 行 | 位置 | step | 目标 | 说明 |
|---|---|---|---|---|
| 6 | `constructor` 起点 | -1 | `A = ∅ ↔ …` | 根状态 ✓ |
| 7 | `intro h` 起点 | 0 | `A = ∅ → …` | ✓ |
| 8 | `intro x hx` 起点 | 1 | `(x) → ¬ x ∈ A` | ✓ |
| 9 | `apply Eq.subst h` 起点 | 2 | `False` | ✓（失败那条**之前**的最后一个成功步） |
| 10+ | 失败之后 | 2 | `False` | 停在最后成功步（B1） |

**关键**：失败**不会**改坏它前面的状态；它只是让 `failure.steps` 截止在
失败那条之前。光标在失败行**之后**时，选择器只能拿到最后成功的那一格。

用户第一段的“前面全坏”是 **B（`·` 的 span 遮住块内步骤）** 造成的；
无 `·` 时前面的状态是好的。第一段 `apply subset_antisymm` 的报错则是
**A 的连坐**（第一个 `·` 失败回滚后，第二个 `·` 看到的是第一个方向的目标）。

建议（体验层）：失败块的光标落在最后成功步之后时，面板明确显示
“这条证明在第 N 行失败，状态停在失败前”，而不是静默复用最后一格；
或者像 Lean 那样按分支恢复（合成 sorry）。这是设计决策，不属于 B/C。

---

## 8. 发布判断

### 8.1 严重度与门槛

- **B（P0）**：做题主循环的 goal state 是核心；**块内**已修，front 单测 + release
  复验通过；**块后（G，`mixed`）仍未对齐 Lean 4**。发布前要么修 G（加 `after`
  状态、`REPORT_SHAPE` +1），要么在已知限制里写清。另应补一条 **LSP wire** 判据
  （`soko/stateAt` 在 `·` 内的目标）和一条 **CLI≡LSP** 契约判据。
- **C（P1）**：已修，低风险；顺手补一条 `#print`/binder 的显示判据即可。
- **E（P1）**：**未修**。它不影响 unit02 的 `exact` 写法，但 `apply False.elim`
  在箭头目标上是合法用法；发布前要么修 + 回归，要么在 `docs/gaps/ledger.jsonl`
  记一条带最小复现的已知限制（**不能**说“已修”）。
- **F/G（P2）**：不阻塞发布，但应在 CHANGELOG/已知限制里写清楚：失败块后的
  状态是“最后成功步”，块间 gap 可能显示分支局部状态。

### 8.2 零回归面

- 内核零改动；判定输入（`render_expr` 的判卷文本）零改动。
- `--json` 事件计数不变；unit02 画布仍是 `6 checked · 5 open · 0 failed`。
- B 的修复**没有**新增序列化字段 ⇒ `REPORT_SHAPE` 不变，旧缓存不失效。
  （G 的彻底修法会改 `ByStepState` ⇒ 那时才 bump。）
- 已跑：`cargo test -p sokonanoda-front --lib -- state_at`（13/13）、
  新增 display 判据（1/1）；全量 front 见 §9。

### 8.3 发布前建议顺序

1. 全量 `cargo test -p sokonanoda-front --lib` + LSP `state_at` 相关测试；
2. 用 release 构建重跑 §1.2 的四条命令，确认 B/C 的屏幕行为；
3. 补 LSP bullet 判据 + CLI≡LSP 契约；
4. 决定 E：修（带 `t1` 回归）或记账；
5. `python3 courses/set-theory/tools/check.py`（整门课门禁）与
   `scripts/soko gate --fast`（本批收尾）。

---

## 9. 测试缺口与过程教训

1. **状态选择器没有 `·` 夹具**：这是 B 漏网的直接原因。B2 的光标语义判据
   （起点/内部/末尾）都用了**平铺** `by` 块。以后凡是引擎里“某类 step 的
   goals 含义不同”的形状，都要有对应的选择器判据。
2. **“引擎记录的是哪一种状态”必须有标记**：普通 step 是后态，`·` 是入口态。
   两者靠猜（span 大小/顺序）就会出错；这次修法用源码的 `·` 来判定，
   但更稳的是在 `ByStep` 上加 `focus: bool`（会改 `REPORT_SHAPE`）。
3. **`@` 的显示折叠只测了点名/闭项文本**，没测 explicit spine；补上
   `@Set.mem …` 与部分应用两档。
4. **失败块的连坐**：第一个 `·` 失败后，第二个 `·` 的报错指向一个“过期状态”；
   诊断/UI 应说明“块在第一条失败处停止”，不要让用户以为第二条也独立错了。
5. **`apply` 的箭头目标**：没有“目标是箭头、被应用项的结论是元变量”的判据；
   `t1 : False → P := by apply False.elim` 是最小件，应进 `apply` 的测试矩阵。
6. **版本/构建身份**：本次所有读数来自 `0.97.10` 的**重建 release**；
   跨轮比较前先确认二进制时间戳（`target/release/sokonanoda`）。
7. **`#check` 只推断、不核验**：`#check False.elim (A x) (A x)` 默认档印 `A x`，
   而同样的项在声明里被内核拒绝。`#check` 不能当“内核接受”的证据；
   要么让它走一次内核检查，要么在文档里写清它只推断。

**本次实际跑过的判据**：`cargo test -p sokonanoda-front --lib -- state_at`
13/13；display 相关 41/41；**全量 front 1099/1099**；`python3 scripts/docs-lint.py` ✓；
release 复验见 §3.4/§4.2。

---

## 10. 附录：最小复现与原始 JSON

### 10.1 `·` 的最小绿夹具

```lean
theorem no_bullet (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  constructor
  exact hp
  exact hq

theorem with_bullet (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  constructor
  · exact hp
  · exact hq

theorem nested_bullet (P Q R : Prop) (hp : P) (hq : Q) (hr : R) : P ∧ (Q ∧ R) := by
  constructor
  · exact hp
  · constructor
    · exact hq
    · exact hr
```

修复后 `query state`（关键位置）：

```
with_bullet  L8 C5  （exact 起点）  step 1  goal P
with_bullet  L8 C7  （exact 内部）  step 2  goal null（无目标）
with_bullet  L9 C5  （exact 起点）  step 3  goal Q
```

### 10.2 `@` 的原始输出（修复前）

```json
{"name":"hx","ty":"@x ∈ A","ty_runs":[]}
```

修复后：

```json
{"name":"hx","ty":"x ∈ A","ty_runs":[]}
```

### 10.3 `apply False.elim` 的原始诊断

```
`apply` 的被应用项没有通过内核检查：期望
`∀(P : Sort(0)), ∀(C : Sort(0)), ∀( : False), C`，实际是
`∀(P : Sort(0)), ∀(h : False), ∀(C : Sort(0)), ∀( : False), C`。
```

### 10.4 修复涉及的文件（工作树，未提交）

- `crates/front/src/query/state.rs`：`select_state_at_in_text` + `is_bullet_step`；
- `crates/front/src/query/mod.rs`：`state_at_with` / `scope_binders_at` 接线；
- `crates/lsp/src/lib.rs`：`tactic_goal_hover` 接线；
- `crates/front/src/display.rs`：部分显式应用守卫 + display 判据；
- `crates/front/src/display/spans.rs`（新）：`fold_span_including_explicit_marker` +
  从 `display.rs` 迁出的 `arrow_token_between`（`display.rs` 净减，过 ⑨ 行数门禁）；
- `crates/front/src/query/tests.rs`：bullet 状态回归。

**未改**：`crates/kernel/**`、prelude、判卷事件流、`REPORT_SHAPE`。

---

## 11. I —— `· ` 行回车缩进（编辑器配置缺口，已修）

**现象**：在 `  · intro h` 行尾按回车，下一行停在 2 空格（与 `·` 同列），
不是 4 空格（`intro` 的列）。

**根因**：`editor/vscode/language-configuration.json` **没有 `onEnterRules`**。
Lean 4 的 `language-configuration.json` 恰好只有这一条（`leanprover.lean4-0.0.240`）：

```json
"onEnterRules": [
  { "beforeText": "^\\s*(·|\\.)\\s.*$", "action": { "indent": "indent" } }
]
```

`·` 是 2026-10-21 加的语言特性，编辑器侧这条规则没有同步 ⇒ 回车沿用上一行的缩进。
这不是 parser/缩进实现的问题（§3.6 已说 `·` 的分组是对的）。

**修复**：把 Lean 4 的同一条规则加进我们的配置；新增
`editor/vscode/test-language-config.js`（静态判据：规则存在且 `indent`；
匹配 `  · intro h` / `  · ` / `  · constructor`；不匹配 `intro h` /
`constructor` / `exact hx`），并接进 `npm run test:unit`。
判据：`node editor/vscode/test-language-config.js` ✓。

**注意**：语言配置在扩展激活时读取 ⇒ 已装的 VS Code 要 **Reload Window**
（或重装 VSIX）才生效；扩展版本 bump 才会随发布出去（发布触发器，别擅自 bump）。

---

## 12. J —— 严格隐式参数：决策已记录，默认档未翻（为什么 `False.elim (A x)` 还能过）

用户 2026-10-16 的原话（`REQUIREMENTS.md` §9.24）：

> **只有 `@` 形式接收隐式参数，正常形式不接受隐式参数**，并且**支持显式地指定参数名字**。

严格档**已经实现**（S1–S6）：CLI `--dialect strict` · 环境 `SOKO_STRICT_AT=1` ·
VS Code `sokonanoda.dialect.strictImplicitArgs` · 文件指令 · 清单；strict 判据 16/16。
strict 下 `False.elim (A x)` 被拒（`期望 False，实际是 Sort(0)`），
`False.elim (C := A x)` / `@False.elim (A x)` 通过 —— 正是用户要的语义。

**但默认档仍是 loose，课程也没有 opt-in**：

- `courses/set-theory/sokonanoda.toml` 只有 `name` / `requires`，**没有**
  `[dialect] strict_implicit = true`；`courses/set-theory/tools/check.py`
  也没有传 `--dialect strict` ⇒ 课程门禁跑的是 loose ⇒
  `elab.rs::try_implicit_application_with` 的 `fits_old_style` 仍把 `(A x)` 当 `C`。
- 默认不翻是**10-20 的后续决定**：用户原话（§9.26）「这个隐式参数和显式参数的
  实现方式太愚蠢了，几乎需要重构。**要么就先用普通模式发布一版，保持高速度高效率**」
  ⇒ 默认回到旧读法；当时实测严格档默认 **0.62 s → 8.8 s（14×）**、
  `gates-course` 10 → 30+ 分钟、LSP 测试超时；§9.27 又把它做成可配置、默认关；
  `docs/design/strict-mode-v2.md` §6 写「**默认不翻**（≥2 周 opt-in + 0 回归后再评估）」；
  `SUB-implicit-args-strict-default` 仍未完成。（v2 的 F5 把 strict 单文件代价降到
  4.4 s vs 3.25 s，debug；但“翻不翻默认”仍是独立决策，不是性能自动放行。）
- 语料没迁完：§9.24 实测 **269/294** 个文件会从接受变拒绝；课程里仍有 loose-only
  形式（`False.elim (x ∈ A) hx`、unit02 的 `False.elim (A x)`）。

⇒ **不是“没实现”，是“默认档没翻 + 课程没 opt-in”**。要现在落实，两个层次：

1. **只让课程严格**（最快）：给 `courses/set-theory/sokonanoda.toml` 加
   `[dialect] strict_implicit = true`，再把 loose-only 的写法迁成
   `False.elim (C := …)` / `@False.elim …`。代价：课程门禁吃 14×（先量）。
2. **翻全局默认**：先做 §9.26 的性能重构（让就地/缓存对全显式 pp 也成立），
   再翻 `Dialect::default`；这是 `SUB-implicit-args-strict-default` 的验收，
   不是一行 `default = Strict` 能收的。

---

## 13. K —— 严格档默认就绪测试（2026-10-27，回答“现在可以开了吗”）

**方法**：冷启动、隔离缓存、禁项目产物、静默机器（先杀掉残留扫描进程）：

```bash
SOKONANODA_CACHE_DIR=/tmp/perf-clean-$mode SOKONANODA_NO_PROJECT_ARTIFACTS=1   [SOKO_STRICT_AT=1] scripts/soko grade --root "$PWD/courses/set-theory" --json <file>
```

先跑 `unit01` 预热 lib，再跑 `unit12` / `unit12-solution`，只量后者。

| 文件 | loose | strict | 倍率 |
|---|---|---|---|
| `I.1/unit01-sets-membership` | 0.72 s | **8.27 s** | **11.5×** |
| `I.4/unit12-synthesis` | 2.51 s | **28.0 s（exit 1）** | **11.2×** |
| `I.4/unit12-solution` | 2.61 s | **>120 s（超时被杀）** | **>46×** |

- 冷启动 strict 是 **~11×**，不是 `strict-mode-v2` F5 的 1.35×（那是暖缓存/debug）。
  全量严格门禁在 15 分钟时被杀（卡在大文件）；按 11× 外推 loose 约 10 分钟 ⇒
  strict 约 110 分钟（**外推，未实测**），且 unit12 直接红。
- `unit12-synthesis` 的 strict 诊断是：

  ```
  elab-unknown-identifier：unknown identifier ` soko_m58`
  ```

  内部 fresh name 泄漏到用户可见的标识符解析 —— 这是 strict 路径的**真 bug**，
  不是“课程少写一个 `@`”。

**迁移清单（已确认）**：

1. `I.4/unit12-synthesis` + `I.4/unit12-solution`：strict 内部名 bug + 11× / >120 s；
   需要先修引擎（`operand_type_expr` 的 explicit pp / F5 未覆盖的冷路径），
   不是课程内容迁移。
2. `I.1/unit02-subsets-empty` 画布：strict 拒绝 loose 写法
   `False.elim (x ∈ A) hx`（出路 `@False.elim (x ∈ A) hx`）与位置式 `Eq.subst`
   （`apply Eq.subst (Eq.symm h)`，出路 `@Eq.subst …` / 命名 `p`）；
   练习骨架也要改。`unit02-solution` 在 strict 下 **0.05 s 通过**。
3. `strict-mode-v2` 当时列的 6 个 known limitations（unit06/08/09/10/11 及解答）
   现在 **0.07 s 通过 strict**，已不在清单里。

**边界**：243 个文件的 strict 逐文件扫描跑到 160/243 就被 unit12 卡死；上面的
清单是**已确认**的部分，剩余大部分是解答文件，需先修 unit12 再补齐全量。

**结论**：**不能开**。不是“只差迁移”：unit12 有 strict 正确性 bug + 11× 冷启动，
全量门禁跑不完；unit02 画布的课堂写法还要迁。先把这两类引擎问题修掉，再谈翻默认。

