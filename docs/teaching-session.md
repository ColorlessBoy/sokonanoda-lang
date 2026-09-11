# 教学循环：playground.sokonanoda（agents 开课前必读）

> 画布：仓库根 `playground.sokonanoda`（学习者与 agent 共同编辑）。
> 依据：`REQUIREMENTS.md` §6（教学工作流）；事件协议：`docs/protocol.md`。
> 本文档 = 开课手册 + 事件决策表 + 全部练习的解答钥匙（agent 专用，勿直接给学习者）。

## 0. 课程层与执行层的关系（用户原则，2026-09-07）

**`course/` 只是大模型的路线图/素材库；执行层必须按用户灵活适配。**

- course/ 的 5 个单元 = 知识点顺序、题池、可解性守护（CI 验证解答）；
- **真正教学时，agent 必须依据当前用户的反馈历史动态重组**：
  - 用户在某概念反复出错 → 追加同概念的变式练习，或先放一个"填好
    的演示声明"再重新出题；
  - 用户进度快 → 跳过热身、合并单元；进度慢 → 拆小步、增加提示层；
  - 用户的兴趣/背景（程序员/学生/纯新手）→ 调整比喻与例子；
- 用户面对的**永远只是当前画布**（playground.sokonanoda 或其分支），
  不是 course/ 文件本身；course/ 文件永不直接丢给用户当"课程"读。

## 1. 开课手册（3 步循环）

1. **讲课**：agent 往画布里写 `--` 讲解 + 定义（演示）+ 练习（`sorry` 洞声明）。
   语法点永远先出现在讲解注释里，再出现在练习里（白名单即课程）。
2. **作答**：学习者编辑画布填洞（支持部分作答：先写几层 fun，最后一层留
   `sorry`，剩余目标会显示在 hover/诊断里）。
3. **判卷**：agent 跑
   ```bash
   SOKO="$HOME/.local/share/sokonanoda/bin/sokonanoda"   # Release 二进制，零 cargo
   "$SOKO" --json playground.sokonanoda
   ```
   （二进制的获取见 `skills/sokonanoda-teacher` §1。）
   读结构化事件（不是 exit code）决定反馈；一个练习红了不影响其他练习
   （逐声明容错）。然后再回到第 1 步追加内容。

## 2. 事件决策表（已用真实内核验证）

| 事件 / code | agent 解读 | agent 动作 |
|---|---|---|
| `decl.checked`（原练习名） | 练习解出 | 肯定 + 追加下一个概念/练习 |
| `exercise.open` 持续 | 未做/卡住 | 给一层提示（画布注释里有），永不直接给答案 |
| `elab-unknown-identifier` | 拼写错 **或** 引用了还没解出的练习（open 声明不进环境） | 先查 open 列表再判拼写；「先做练习 N」 |
| `elab-duplicate-declaration` | 重名 | 讲“单赋值世界”，换名 |
| `elab-hole-misplaced` | 洞不在可恢复位置（嵌套洞/非直接实参，如 `n + sorry`；答案尾巴、构造子 spine 与已知函数直接实参都合法） | 讲“洞只能放答案末尾，或已知函数/构造子的直接实参位” |
| `kernel-rejected`（def_eq failed） | 填了类型而不是证明项 / 方向反了 / 宇宙忘了 `.{1}` / 忘了 Not 会展开 | 对比声明类型与所填项的形状，让学习者逐参数预言类型 |
| `kernel-rejected`（app arg def_eq failed） | 部分应用 / 参数顺序错 | 一起数构造子签名参数 |
| 无诊断但语义不对（如 `double := fun n => n`） | 内核只判类型不判意图 | 设计“证明形状”的需求（见 two_def 模式） |

## 3. 第一课练习清单与解答钥匙（全部经完整内核验证；逻辑先行排序）

> 逻辑骨架由画布内 axiom 提供（True/False/And/Or/Not，与官方 Lean 同构）；
> Eq 三件套由 prelude 提供（`Eq`/`Eq.refl`/`Eq.subst`，签名与官方 Lean 一致）。
> 排序哲学（REQUIREMENTS §6）：单元①直接证明命题，Sort 等"函数类型的类型"
> 问题自然出现时（单元④）才揭晓。

| # | 练习 | 目标误解 | 钥匙（kernel 验证） |
|---|---|---|---|
| 1 | `theorem true_is_true : True := sorry` | 证明=项；True.intro 已经存在 | `True.intro` |
| 2 | `and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := sorry` | 类型箭头 ↔ 值 fun 一一对应 | `fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro a b ha hb` |
| 3 | `and_swap : ... And a b -> And b a := sorry` | 先消去再构造 | `fun (a : Prop) => fun (b : Prop) => fun (h : And a b) => And.intro b a (And.right a b h) (And.left a b h)` |
| 4 | `ex_falso : (P : Prop) -> False -> P := sorry` | False 只能“用”不能“证” | `fun (P : Prop) => fun (h : False) => False.rec P h` |
| 5 ★ | `and_not_absurd : (a : Prop) -> And a (Not a) -> False := sorry` | Not 是黑盒；部分应用陷阱 | `fun (a : Prop) => fun (h : And a (Not a)) => And.right a (Not a) h (And.left a (Not a) h)` |
| 6 | `def two : Nat := sorry` | “数字就是数字”——1+1 是会被内核计算的表达式 | `2`（或 `1 + 1`；`two_def` 闭环回判此值） |
| 7 | `one_plus_one_eq_two : Eq.{1} Nat (1 + 1) 2 := sorry` | rfl 不是咒语，是函数；conv 会计算 | `Eq.refl.{1} Nat (1 + 1)`（`Eq.refl.{1} Nat 2` 也过：conv 双向计算） |
| 8 | `eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a := sorry` | 谓词 p 要自己设计（本场最深的一步） | `fun (a : Nat) => fun (b : Nat) => fun (h : Eq.{1} Nat a b) => Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat x a) a b h (Eq.refl.{1} Nat a)` |
| 9 | `def double : Nat -> Nat := fun (n : Nat) => sorry` | 程序不必一次写完；洞下剩余目标 = Nat | `fun (n : Nat) => n + n` |
| 10 | `def twice : (Nat -> Nat) -> Nat -> Nat := sorry` | 函数是一等公民；括号是分组不是应用 | `fun (f : Nat -> Nat) => fun (x : Nat) => f (f x)` |
| 11 | `example : Sort 1 := sorry` | “类型没有类型”；Prop=Sort 0，Type=Sort 1 | `Nat` |
| 12 ★ | `Eq.symm {u} : {α : Sort u} -> (a : α) -> (b : α) -> Eq.{u} α a b -> Eq.{u} α b a := sorry` | 宇宙不可怕：就是 8 换成 α/.{u}，隐式 binder ↔ fun {..} | `fun {α : Sort u} => fun (a : α) => fun (b : α) => fun (h : Eq.{u} α a b) => Eq.subst.{u} α (fun (x : α) => Eq.{u} α x a) a b h (Eq.refl.{u} α a)` |
| 13 | `and_left_by : (a : Prop) -> (b : Prop) -> And a b -> a := by sorry` | by 块里先 intro 拆箭头再 exact；`sorry` 是占位 | `by intro a; intro b; intro h; exact And.left a b h` |
| 14 ★ | `or_inl_by : (a : Prop) -> (b : Prop) -> a -> Or a b := by sorry` | apply 把目标套上构造子、拆子目标 | `by intro a; intro b; intro ha; apply Or.inl; exact ha` |

收尾：`two_def : Eq.{1} Nat two (1 + 1) := Eq.refl.{1} Nat two` —— 画布里先
注释着，练习 6 解出后放开；变绿 = 内核回判了练习 6 的值。

## 4. Gotchas（全部验证过，别踩）

1. **裸 `Eq` 默认 u=0**（Prop 层）；Nat 级必须 `Eq.{1}`/`Eq.refl.{1}`/`Eq.subst.{1}`。
2. **无隐式参数补全**：隐式 binder 也按位置显式给全；`@` 只是语法糖（等价不带 @）。
   教学画布的 axiom 全用显式 binder（`And.intro a b ha hb` 风格），只有 prelude 的
   Eq 与毕业题 12 用隐式。
3. **open 声明不进环境**：后面的代码引用它会得到 `elab-unknown-identifier`
   （两_def 初期报错即此——所以 two_def 先注释）。
4. **kernel 只判类型**：`double := fun n => n` 也能过；语义要求用证明形状表达。
5. **`Type 1` 不是合法输入**：教 `Sort n`；`Type` = `Sort 1` 只出现在讲解里。
6. **命名要防撞 prelude**：若画布自己声明 `Eq`/`Eq.refl`/`Eq.subst` 任一，
   整个 Eq prelude 跳过（all-or-nothing，与显式 `Nat` 块行为一致）。
7. **本课不含排中律/or_comm**（Or 没有 rec；那是单元⑤的内容——别许诺）。

## 5. 下一课预告（单元⑤，待 elaborator 支持后）

`Or.rec` 与 `or_comm`、`eq_trans`、显式 `inductive Nat` 块 + `Nat.rec`/iota 归纳。
语料参照 `examples/py-nat.sokonanoda` 与 `examples/fol-basics.sokonanoda`。
