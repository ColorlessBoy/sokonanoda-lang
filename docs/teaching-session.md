# 教学循环：playground.sokonanoda（agents 开课前必读）

> 画布：仓库根 `playground.sokonanoda`（学习者与 agent 共同编辑）。
> 依据：`REQUIREMENTS.md` §6（教学工作流）；事件协议：`docs/protocol.md`。
> 本文档 = 开课手册 + 事件决策表 + 全部练习的解答钥匙（agent 专用，勿直接给学习者）。

## 0. 课程层与执行层的关系（用户原则，2026-09-07）

**`course/` 只是大模型的路线图/素材库；执行层必须按用户灵活适配。**

- course/ 的 8 个单元 = 知识点顺序、题池、可解性守护（CI 验证解答）；
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
> 排序哲学（REQUIREMENTS §6）：单元①直接证明命题，Sort 等“函数类型的类型”
> 问题自然出现时（单元⑤）才揭晓；`by` 写法提前到单元④（即时反馈加速器），
> 归纳拆成单元⑥（Ⅰ）/⑦（Ⅱ），量词在单元⑧（见本节末尾“第二课”），
> 关系与联结词在单元⑨、读证明与综合在单元⑩（见本节末尾“第三课”）。
>
> 下表按**真实课程单元·题号**编号（`U1·1` = `course/unit1-*.sokonanoda`
> 练习 1），不再用旧的 playground 顺序号。

| 单元·题 | 练习 | 目标误解 | 钥匙（kernel 验证） |
|---|---|---|---|
| U1·1 | `theorem true_is_true : True := sorry` | 证明=项；True.intro 已经存在 | `True.intro` |
| U1·2 | `theorem and_intro_rule : (a : Prop) -> (b : Prop) -> a -> b -> And a b := sorry` | 类型箭头 ↔ 值 fun 一一对应 | `fun (a : Prop) => fun (b : Prop) => fun (ha : a) => fun (hb : b) => And.intro a b ha hb` |
| U1·3 | `theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a := sorry` | 先消去再构造 | `fun (a : Prop) => fun (b : Prop) => fun (h : And a b) => And.intro b a (And.right a b h) (And.left a b h)` |
| U1·4 | `theorem ex_falso : (P : Prop) -> False -> P := sorry` | False 只能“用”不能“证” | `fun (P : Prop) => fun (h : False) => False.rec P h` |
| U1·5 ★ | `theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False := sorry` | Not 是黑盒；部分应用陷阱 | `fun (a : Prop) => fun (h : And a (Not a)) => And.right a (Not a) h (And.left a (Not a) h)` |
| U1·6 | `theorem and_intro_rule2 (a : Prop) (b : Prop) (ha : a) (hb : b) : And a b := sorry` | 声明级 binder：同样一件事不用写 fun | `And.intro a b ha hb` |
| U2·1 | `def two : Nat := sorry` | “数字就是数字”——1+1 是会被内核计算的表达式 | `2`（或 `1 + 1`；`two_def` 闭环回判此值） |
| U2·2 | `theorem one_plus_one_eq_two : Eq.{1} Nat (1 + 1) 2 := sorry` | rfl 不是咒语，是函数；conv 会计算 | `Eq.refl.{1} Nat (1 + 1)`（`Eq.refl.{1} Nat 2` 也过：conv 双向计算） |
| U2·3 | `theorem eq_symm_nat : (a : Nat) -> (b : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b a := sorry` | 谓词 p 要自己设计（本场最深的一步） | `fun (a : Nat) => fun (b : Nat) => fun (h : Eq.{1} Nat a b) => Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat x a) a b h (Eq.refl.{1} Nat a)` |
| U2·4 | `theorem eq_trans_nat : (a : Nat) -> (b : Nat) -> (c : Nat) -> Eq.{1} Nat a b -> Eq.{1} Nat b c -> Eq.{1} Nat a c := sorry` | 配方同 U2·3，只换谓词 | `fun (a : Nat) => fun (b : Nat) => fun (c : Nat) => fun (h1 : Eq.{1} Nat a b) => fun (h2 : Eq.{1} Nat b c) => Eq.subst.{1} Nat (fun (x : Nat) => Eq.{1} Nat a x) b c h2 h1` |
| U2·5 | `theorem eq_refl_prop : {a : Prop} -> Eq.{1} Prop a a := sorry` | 等式对任何类型都成立，命题也一样 | `fun {a : Prop} => Eq.refl.{1} Prop a` |
| U3·1 | `def double : Nat -> Nat := fun (n : Nat) => sorry` | 程序不必一次写完；洞下剩余目标 = Nat | `fun (n : Nat) => n + n` |
| U3·2 | `def twice : (Nat -> Nat) -> Nat -> Nat := sorry` | 函数是一等公民；括号是分组不是应用 | `fun (f : Nat -> Nat) => fun (x : Nat) => f (f x)` |
| U3·3 | `def three_args : Nat -> Nat -> Nat -> Nat := sorry` | 三层 fun；本题要求交回第一个参数 a | `fun (a : Nat) => fun (b : Nat) => fun (c : Nat) => a` |
| U3·4 | `def apply_twice : (Nat -> Nat) -> (Nat -> Nat) := sorry` | 返回类型本身还是箭头，要再剥一层 | `fun (f : Nat -> Nat) => fun (n : Nat) => f (f n)` |
| U3·5 | `def double_let : Nat -> Nat := fun (n : Nat) => let m : Nat := sorry; m` | let 给中间结果起名；洞在值位 | `fun (n : Nat) => let m : Nat := n + n; m` |
| U3·6 | `def body_uses_let : Nat -> Nat := fun (n : Nat) => let m : Nat := n + 5; sorry` | let 绑定后 body 里直接用 m | `fun (n : Nat) => let m : Nat := n + 5; m` |
| U4·1 | `theorem by_ex1 : (a : Prop) -> a -> a := by sorry` | by 块里先 intro 拆箭头再用 assumption | `by intro a; intro h; assumption` |
| U4·2 | `theorem by_ex2 : (a : Prop) -> (b : Prop) -> And a b -> a := by sorry` | intro 拆箭头后 exact 交项 | `by intro a; intro b; intro h; exact And.left a b h` |
| U4·3 | `theorem by_ex3 : (a : Prop) -> (b : Prop) -> a -> Or a b := by sorry` | apply 把目标套上构造子、拆子目标 | `by intro a; intro b; intro ha; apply Or.inl; exact ha` |
| U4·4 | `theorem by_ex4 : (a : Prop) -> (b : Prop) -> a -> b -> And a b := by sorry` | apply 出两个子目标，按序 exact | `by intro a; intro b; intro ha; intro hb; apply And.intro; exact ha; exact hb` |
| U4·6 | `theorem by_ex6 : (a : Prop) -> (b : Prop) -> And a b -> And b a := by sorry` | 综合：intro 拆三层，exact 一次性收尾 | `by intro a; intro b; intro h; exact And.intro b a (And.right a b h) (And.left a b h)` |
| U5·1 | `def function_type_universe : Sort 1 := sorry` | 读 `#check (Nat -> Nat)` 的输出判层 | `Nat -> Nat` |
| U5·2 | `def type0_universe : Sort 2 := sorry` | 读 `#check (Type 0)` 的输出判层 | `Type 0` |
| U5·3 | `example : Sort 1 := sorry` | “类型没有类型”；Prop=Sort 0，Type=Sort 1 | `Nat` |
| U5·4 | `example : Sort 0 := sorry` | 等式就是命题 | `Eq.{1} Nat (1 + 1) 2` |
| U5·5 ★ | `theorem Eq.symm {u} : {α : Sort u} -> (a : α) -> (b : α) -> Eq.{u} α a b -> Eq.{u} α b a := sorry` | 宇宙不可怕：Nat 换 α、.{1} 换 .{u}，隐式 binder ↔ fun {..} | `fun {α : Sort u} => fun (a : α) => fun (b : α) => fun (h : Eq.{u} α a b) => Eq.subst.{u} α (fun (x : α) => Eq.{u} α x a) a b h (Eq.refl.{u} α a)` |
| U5·6 | `def predict_then_prove : Eq.{1} Nat ((fun (x : Nat) => x + 1) 41) 42 := sorry` | 先预测 `#reduce` 的输出，再填出等式 | `Eq.refl.{1} Nat 42` |
| U6·1 | `def four : Nat := sorry` | 数字用构造子搭出来，不再有字面量的魔法 | `succ three` |
| U6·2 | `def double : Nat -> Nat := sorry` | 递归定义 = 选好 motive/mz/ms 三样再喂 n | `fun (n : Nat) => Nat.rec.{1} (fun (x : Nat) => Nat) zero (fun (k : Nat) => fun (ih : Nat) => succ (succ ih)) n` |
| U6·3 | `def add : Nat -> Nat -> Nat := sorry` | 归纳变量是 m：zero 处返回 n，succ 处叠一层 | `fun (m : Nat) => fun (n : Nat) => Nat.rec.{1} (fun (x : Nat) => Nat) n (fun (k : Nat) => fun (ih : Nat) => succ ih) m` |
| U6·4 | `def relabel (c : Color) : Color := sorry` | `match` 一次覆盖全部构造子 | `match c with \| red => red \| green => green` |
| U6·5 | `def swapOpen (c : Color) : Color := match c with \| red => green \| green => sorry` | 在 `match` 的一个分支里留洞 | `\| green => red` |
| U6·6 | `def recDouble (n : Nat) : Nat := match n with \| zero => zero \| succ m => sorry` | 递归字段后自动插入归纳假设 `ih` | `\| succ m => succ (succ ih)` |
| U7·1 | `def valueOrZero (x : Option Nat) : Nat := match x with \| none => zero \| some a => sorry` | 参数化归纳：参数从 scrutinee 书写类型代入 | `\| some a => a` |
| U7·2 | `theorem nat_induction_open (P : Nat -> Prop) (hz : P zero) (hs : (k : Nat) -> P k -> P (succ k)) (n : Nat) : P n := match n with \| zero => hz \| succ k => sorry` | 依赖 `match` = 数学归纳法；succ 支 `ih : P k` | `hs k ih` |
| U7·3 | `def predOpt (x : Option Nat) : Nat := match x with \| some zero => zero \| some (succ k) => sorry \| none => zero` | 嵌套模式把 succ 的字段 `k` 绑出来 | `k` |
| U7·4 | `def vhead (A : Type) (d : A) (n : Nat) (v : Vec A n) : A := match v with \| vnil => d \| vcons a m w => sorry` | 带索引归纳；vcons 把头绑成 `a` | `a` |

> 单元④原有 `by_ex5`（`(a : Prop) -> a -> a`，与 `by_ex1` 完全重复、hint
> 描述也有误）已在 P1 删除，故上表没有 `U4·5`。

收尾：`two_def : Eq.{1} Nat two (1 + 1) := Eq.refl.{1} Nat two` —— 画布里先
注释着，U2·1 解出后放开；变绿 = 内核回判了 U2·1 的值。

### 第二课：量词（playground 练习 6–12；course/ 单元⑧）

> 画布在 `forall_demo` 后引入：`axiom Person : Type 0`、`axiom someone : Person`
> 与 `Exists`/`Exists.intro`/`Exists.elim`（显式公理版，与官方 Lean 的
> `Exists` 同构）。结构：∀ 引入=fun、消去=应用；∃ 引入=交证人、消去=把
> 函数交给 elim（结论不提证人）。

| # | 练习 | 目标误解 | 钥匙（kernel 验证） |
|---|---|---|---|
| 6 | `forall_elim (P : Person -> Prop) (w : Person) (h : forall (x : Person), P x) : P w := sorry` | ∀ 消去就是应用 | `h w` |
| 7 | `forall_and ... : And (forall (x : Person), P x) (forall (x : Person), Q x) := sorry` | 配对的 ∀ 分成两个 ∀，两处 fun 引入 | `And.intro (forall (x : Person), P x) (forall (x : Person), Q x) (fun (x : Person) => And.left (P x) (Q x) (h x)) (fun (x : Person) => And.right (P x) (Q x) (h x))` |
| 8 | `and_forall ... : forall (x : Person), And (P x) (Q x) := sorry` | 反向；外层 fun 引入 ∀ | `fun (x : Person) => And.intro (P x) (Q x) (And.left (forall (y : Person), P y) (forall (y : Person), Q y) h x) (And.right (forall (y : Person), P y) (forall (y : Person), Q y) h x)` |
| 9 | `exists_intro_rule (P : Person -> Prop) (w : Person) (hw : P w) : Exists Person P := sorry` | ∃ 引入=证人+性质证明 | `Exists.intro Person P w hw` |
| 10 | `exists_elim_rule (P : Person -> Prop) (Q : Prop) (h : Exists Person P) (f : forall (x : Person), P x -> Q) : Q := sorry` | ∃ 消去=把函数交给 elim | `Exists.elim Person P Q h f` |
| 11 ★ | `forall_exists (P : Person -> Prop) (h : forall (x : Person), P x) : Exists Person P := sorry` | 空论域反例意识；证人用 someone | `Exists.intro Person P someone (h someone)` |
| 12 ★★ | `exists_mono (P : Person -> Prop) (Q : Person -> Prop) (f : forall (x : Person), P x -> Q x) (h : Exists Person P) : Exists Person Q := sorry` | 消去后重新装回；motive 是 `Exists Person Q` | `Exists.elim Person P (Exists Person Q) h (fun (w : Person) => fun (hw : P w) => Exists.intro Person Q w (f w hw))` |

单元⑧画布（`course/unit8-quantifiers.sokonanoda`）同题重编号为练习 1–7，
钥匙见 `course/solutions/unit8-quantifiers-solution.sokonanoda`；`#reduce`
自测 `(fun (x : Person) => x) someone` 应化简为 `someone`。

### 第三课：关系、联结词与读证明（course/ 单元⑨–⑩，2026-09-16 上线）

> 单元⑨把单元①的 `Or` 从公理升级为真正的 `inductive`（前端自动派生
> `Or.rec`），`Iff` 是 `And (A -> B) (B -> A)` 的 `def`，`Le`/`Even` 是带
> 索引的归纳关系（各自**手写** `rec`/`iota`——见 HANDOVER §3 的 gap 1）；
> 单元⑩不再新增语法，练「读」：自解释三问、formal↔informal 互译、评阅错
> 证明、期末小项目——每道题的成品仍必须过内核。
> 钥匙见 `course/solutions/unit{9,10}-*-solution.sokonanoda`。

| 单元·题 | 练习 | 目标误解 | 钥匙（kernel 验证） |
|---|---|---|---|
| U9·1 | `theorem or_comm (A : Prop) (B : Prop) (h : Or A B) : Or B A := sorry` | `Or` 是归纳类型：`match` 拆开再装到另一边的构造子 | `match h with \| inl a => inr B A a \| inr b => inl B A b` |
| U9·2 | `theorem or_elim (A B C : Prop) (h : Or A B) (f : A -> C) (g : B -> C) : C := sorry` | 显式用消去子 `Or.rec`：motive + 两分支 + 证据 | `Or.rec A B (fun (x : Or A B) => C) f g h` |
| U9·3 R | `theorem or_id (A : Prop) (h : Or A A) : A := sorry` | `match` 必须覆盖**全部**构造子（原错证明只写了 `inl`） | `match h with \| inl a => a \| inr a => a` |
| U9·4 | `theorem iff_mp (A B : Prop) (h : Iff A B) : A -> B := sorry` | `Iff` 是定义，展开成 `And`，正向在左 | `And.left (A -> B) (B -> A) h` |
| U9·5 | `theorem iff_mpr (A B : Prop) (h : Iff A B) : B -> A := sorry` | 与上题对称：反向在右 | `And.right (A -> B) (B -> A) h` |
| U9·6 L | `theorem le_zero (n : Nat) : Le Nat.zero n := sorry` | 对 `n` 归纳：zero 用自反，succ 支把 `ih` 经 `le_succ` 抬层 | `match n with \| Nat.zero => le_refl Nat.zero \| Nat.succ k => le_succ Nat.zero k ih` |
| U9·7 X | `theorem le_trans (m n k : Nat) (h1 : Le m n) (h2 : Le n k) : Le m k := sorry` | 对**第二个证据** `h2` 用 `Le.rec` 归纳，motive 记着 `h1` | `(Le.rec (fun (a : Nat) (b : Nat) (h : Le a b) => Le m a -> Le m b) (fun (b : Nat) => fun (hx : Le m b) => hx) (fun (a : Nat) (b : Nat) (h : Le a b) (ih : Le m a -> Le m b) => fun (hx : Le m a) => le_succ m b (ih hx)) n k h2) h1` |
| U9·8 | `theorem even_four : Even (Nat.succ (Nat.succ (Nat.succ (Nat.succ Nat.zero)))) := sorry` | `even_succ` 把「n 是偶数」抬成「n+2 是偶数」，前提用 `even_two` | `even_succ (Nat.succ (Nat.succ Nat.zero)) even_two` |
| U10·1 X | `theorem or_intro_x (A B : Prop) (a : A) : Or A B := sorry` | 非形式「如果…那么…」→箭头；结论是 `Or`，用对应构造子 | `inl A B a` |
| U10·2 X | `theorem iff_intro_x (A B : Prop) (mp : A -> B) (mpr : B -> A) : Iff A B := sorry` | 「等价」就是 `Iff`，把两个方向打包 | `And.intro (A -> B) (B -> A) mp mpr` |
| U10·3 X | `theorem nat_eq_self_x (n : Nat) : Eq.{1} Nat n n := sorry` | 「每个自然数」=∀；等式自反，注意 `.{1}` | `Eq.refl.{1} Nat n` |
| U10·4 R | `theorem or_comm_fixed (A B : Prop) (h : Or A B) : Or B A := sorry` | 原错证明 `match` 漏了 `inr` 支 | `match h with \| inl a => inr B A a \| inr b => inl B A b` |
| U10·5 R | `theorem or_right_fixed (A B : Prop) (b : B) : Or A B := sorry` | 原错证明把 `b : B` 交给了 `inl`（吃 `A` 的证明） | `inr A B b` |
| U10·6 综合 | `theorem and_or_imp (A B C : Prop) : And A (Or B C) -> Or (And A B) (And A C) := sorry` | 用 `let` 命名 `h` 的右半再 `match`，两种分支重新打包 | `fun (h : And A (Or B C)) => let bc : Or B C := And.right A (Or B C) h; match bc with \| inl b => inl (And A B) (And A C) (And.intro A B (And.left A (Or B C) h) b) \| inr c => inr (And A B) (And A C) (And.intro A C (And.left A (Or B C) h) c)` |

## 4. Gotchas（全部验证过，别踩）

1. **裸 `Eq` 默认 u=0**（Prop 层）；Nat 级必须 `Eq.{1}`/`Eq.refl.{1}`/`Eq.subst.{1}`。
2. **无隐式参数补全**：隐式 binder 也按位置显式给全；`@` 只是语法糖（等价不带 @）。
   教学画布的 axiom 全用显式 binder（`And.intro a b ha hb` 风格），只有 prelude 的
   Eq 与毕业题 12 用隐式。
3. **open 声明不进环境**：后面的代码引用它会得到 `elab-unknown-identifier`
   （两_def 初期报错即此——所以 two_def 先注释）。
4. **kernel 只判类型**：`double := fun n => n` 也能过；语义要求用证明形状表达。
5. **`Type n` = `Sort (n + 1)`**：`Type 0` 等于 `Sort 1`，单独一个 `Type`
   也是 `Sort 1`；`Type u`（宇宙变量）不支持，写 `Sort u`。
6. **命名要防撞 prelude**：若画布自己声明 `Eq`/`Eq.refl`/`Eq.subst` 任一，
   整个 Eq prelude 跳过（all-or-nothing，与显式 `Nat` 块行为一致）。
7. **排中律不在课内**（`em`/`by_contra` 在白名单外）。`Or` 在**单元⑨**升级为
   真正的 inductive（前端自动派生消去子 `Or.rec`），`or_comm`/`or_elim`/`or_id`
   都是该单元的练习；**单元⑨之前**（画布第一课、单元①–⑧）不要许诺 `Or.rec`。

## 5. 后续课程（单元④–⑩已上线）

`by` 写法（tactic 证明）与值位 `intro`/`apply` 对照在 **course/ 单元④**
（0.18.0 起，P2 从旧单元⑥提前）；宇宙（`Sort n`/`Type n` 阶梯、`Eq.{1}`
由来、`#check` 读输出）在 **单元⑤**。显式 `inductive Nat` 块 +
`Nat.rec`/iota 归纳、`match` 分情况/递归在 **单元⑥**（归纳与递归 Ⅰ）；
参数化 `Option`/依赖 match/嵌套与通配模式/带索引 `Vec` 在 **单元⑦**
（归纳与递归 Ⅱ）。手写 `eq_trans`（以及 `eq_symm`、等式接力）在
**course/ 单元②**。量词（`forall`/`Exists`，Person 论域 + `Exists`
公理三件套）在**单元⑧**（0.26.0 起），就是画布的第二课。关系与联结词
（`Or` 升级为真 inductive + 自动派生 `Or.rec`、`Iff` 定义、`Le`/`Even`
归纳关系 + 手写消去子）在**单元⑨**（P3，2026-09-16 上线，见“第三课”）；
读证明与综合（自解释三问、formal↔informal 互译、评阅错证明、期末小项目）
在**单元⑩**（P3）——`or_comm`/`or_elim`/`Or.rec` 都从单元⑨起才出现，
单元①–⑧没有。语料参照 `examples/py-nat.sokonanoda`
与 `examples/fol-basics.sokonanoda`。
