# 卷 I 未结项（课程侧登记 · 下一位可直接接手）

> 为什么不在 `docs/gaps/ledger.jsonl` 里：那份台账记的是**语言/内核缺口**（G 系列），
> 且**当时正挂在内核线的在途改动里**（本仓库有并行会话）⇒ 课程侧的未结项登记在本文件 ✓
> （它是**课程**的未完成度，不是语言的缺口 ✓）。**格式固定**：编号 · 状态 · 零件判据 · 缺什么。

## C-01（**closed** · 2026-10-02 第 681 轮）序数三歧的**装配** —— `ordinal_trichotomy` **已判绿** ✓

- **目标命题**（Enderton §7.4 定理 7M）：`E x y ∨ x = y ∨ E y x` ✓（对序数 `x, y` ✓）。
- **零件状态：全绿** ✓ —— 判据 `courses/set-theory/units/solutions/I.6/unit108-solution.sokonanoda`
  （10 条声明 `bisect` 全 ok ✓）：`ordinal_elems_ordinal` · `acc_of_wellFounded` · `ordinal_ind` ·
  `pred_subset_of_not_mem`（半边一）· `mem_of_exists_gap`（半边二）· `pred_subset_of_no_gap` ·
  `mem_or_pred_subset`（合流）✓。
- **装配状态：判红 3 次** ✗（第 678 轮 ✓）⇒ 按手册纪律「同一处连红 3 次 ⇒ 换招或降级」**降级交付** ✓，
  主定理**未进课程树** ✓（不在画布、不在解答 ✓）。
- **缺什么**（三条已定位的工程坑，**都不是内核墙** ✗）：
  1. **嵌套归纳的 motive 接线**：外层归纳的 step 必须返回 **`∀ y'` 形式的 motive** ✓
     ⇒ 内层归纳必须**用 λ 包住**再交出去 ✗（直接应用到具体 `y` 会报「期望 `Π z, …`」✓）；
  2. **`Or.elim` 的第二命题位**：`Classical.em P` 给的是 **`P ∨ ¬P`** ✓ ⇒ Q 位**必须写 `¬ P`** ✗
     （写成别的命题会报类型不匹配 ✓）；
  3. **`Eq.subst` 的谓词槽位**：要 `E x y` 就写 `fun w => E w y` ✓（写成 `fun w => E x w` 会把 `E x x` 证出来 ✗）。
- **还需要的数学前提**（**假设**，不是定理 ✓）：`E` 的**外延性**
  `∀z, E z x → E z y ⇒ ∀z, E z y → E z x ⇒ x = y` ✓ —— 本课里 `E` 是**抽象关系** ✓，
  所以外延性要作为**参数**引进 ✓（画布头部已如实标注 ✓）。
- **复现件（登记在册 ✓，G7 重放）**：`courses/set-theory/gaps/C-01-trichotomy-assembly.sh` ——
  一条命令：`bash courses/set-theory/gaps/C-01-trichotomy-assembly.sh`（断言：定理在 · `decl_checked>0` ·
  `exercise_open==0` · 无拒绝）。**反向验证实测（第 682 轮）**：证明体换成 `sorry` ⇒ **exit 1**
  「✗ C-01 回归：exercise_open=1」✓；恢复 ⇒ **exit 0**「✓ … checked=11 · exercise_open=0 · failed=0」✓。
  门禁侧：`python3 courses/set-theory/tools/check.py --gaps-only` ⇒ 正向 exit 0 ✓ / 注入后 exit 1 ✓。
- **怎么验** ✓ **已完成**：判据 = `courses/set-theory/units/solutions/I.6/unit108-solution.sokonanoda` 的
  `ordinal_trichotomy` ✓ —— `python3 /tmp/soko/bisect.py <该文件>` 输出
  **`[11] ok theorem ordinal_trichotomy … (checked=11)` · ALL GREEN** ✓；
  画布同步加了这条练习（`units/I.6/unit108-trichotomy.sokonanoda`，8 个练习 ✓），
  单元标题改为「序数三歧（引理与装配）」✓，`course.json` 配额 78 ✓；
  门禁 **236/1998/881/0** ✓（`checked` 比上一轮 +1 = 这条主定理 ✓）。
- **⚠ 上一轮记的三条"坑"里，只有第 ③ 条是真因** ✓（本轮用 `sorry` 叶子做**逐层隔离**换来 ✓）：
  * ①②（"嵌套归纳的 motive 要 λ 包住" ✗ / "`Or.elim` 的 Q 位要写 `¬P`" ✗）**其实是**：
    motive 接线**本来就对**（隔离件 `t_outer` / `t_inner` 都 ok ✓），Q 位也**本来就没写错** ✗；
  * **真因三条** ✓：**(a)** `hext z v` 的**两个包含关系顺序传反**（第一参数要 `∀z, E z z₀ → E z z₁`）✗；
    **(b)** `ihz w hw v …` 的**序数性参数给了 `w` 的而不是 `v` 的** ✗；
    **(c)** 第二分支返回的**析取太窄**（缺一层 `Or.inr` 包裹）✗。
  ⇒ **教训** ✓：**"猜根因"不如"逐层隔离"** —— 用 `sorry` 当叶子（它**不算失败** ✓）把大证明切成
  外层 / 内层 / 叶子三段，一轮就定位 ✓（而按猜测改了三轮都没中 ✗）。

## C-02（**closed** · 2026-10-02 第 680 轮）`Acc.rec` 的计算规则 —— **是拼法边界，已解** ✓

- **目标**（序数算术的前提 ✓）：把良基递归子的**计算规则**写成定理 ✓，例如
  `Eq Nat (Acc.rec α E (fun z _ => Nat) step x (Acc.intro α E x hr)) (step x hr (fun y hy => Acc.rec … y (hr y hy)))` ✓
  —— 有了它才能证 `x + 0 = x` 与加法结合律 ✓。
- **实测（第 679 轮）** ✓：
  * ✅ **定义侧判绿**：`def p_one (α) (E) (x) (h : Acc α E x) : Nat := Acc.rec α E (fun z _ => Nat) (…) x h`
    ⇒ `bisect` **ok** ✓（**用良基递归真的能定义到 Type 层的函数** ✓ —— G-56/G-58 的收益 ✓）；
  * ❌ **等式侧判红**：把同一条归约写成定理 ⇒ `类型不匹配：期望 Sort(0)，实际是 Sort(1)` ✓。
- **已试的拼法**（两种都红 ✗）：① motive 用**变量** `β : Type` ✓；② motive 用**具体类型** `Nat` ✓。
  **未试**（因运行器不可用 ✗，见下）：③ 显式宇宙 `Eq.{1}` / `Eq.refl.{1}` 的写法 ✓ ——
  手册里「等式族证明项的宇宙层级」是**已记的同类边界** ✓，所以 ③ **值得先试** ✓。
- **最小复现**（放课程树内判卷 ✓；`import lib.Order` 提供 `Acc` ✓）：
  ```text
  def p_one (α : Type) (E : α → α → Prop) (x : α) (h : Acc α E x) : Nat :=
    Acc.rec α E (fun (z : α) (_ : Acc α E z) => Nat)
      (fun (z : α) (_hz : ∀ (y : α), E y z → Acc α E y)
        (_ih : ∀ (y : α) (hy : E y z), Nat) => Nat.succ Nat.zero) x h
  -- 判红：期望 Sort(0)，实际是 Sort(1)
  theorem p_one_eq (α : Type) (E : α → α → Prop) (x : α)
      (hr : ∀ (y : α), E y x → Acc α E y) :
      Eq Nat (p_one α E x (Acc.intro α E x hr)) (Nat.succ Nat.zero) :=
    Eq.refl Nat (Nat.succ Nat.zero)
  ```
- **影响**：**单元109（序数算术·加法）** 卡在这里 ✗ —— 递归**定义**能做 ✓，但**算不出来**（证不了计算规则 ⇒ 结合律无从谈起 ✗）。
- **收编去向**：应进 `docs/gaps/ledger.jsonl`（G 系列 ✓）—— **当时 `docs/gaps/` 正挂在内核线的在途改动里** ✓，
  故先登记在此 ✓；内核线空出手后请搬过去 ✓。
- **✅ 已收编为 G-73**（2026-10-02 内核线 · 台账搬运）：结论与本条一致（**不是墙，是等式族证明项的宇宙层级拼法** ✓），
  故**归并**进 `docs/gaps/ledger.jsonl` 的 **G-73**（形状 ③「`≠`/`=` 解不出宇宙层级」的**第二处落点**：等式在**定理陈述**里）✓；
  最小复现已并入 `docs/gaps/repro/G73-def-headed-term-boundaries.sokonanoda`（**②判红 / ③写显式宇宙判绿** 两态齐 ✓，
  自包含零 import ✓）—— 判据数字：`./target/debug/sokonanoda --json --no-project <该复现件>` ⇒ `decl.checked` **6 条**
  （含 `accConst` ✓ 与 `accConst_eq_explicit` ✓）· `diagnostic` **2 条** ⇒ rejected ✓。G-73 保持 **open**（绕法只摊薄代价 ✓）。

## C-03（**closed** · 2026-10-02 第 681 轮）运行器拒绝运行 —— **已解封** ✓

- **现象**（第 679 轮 ✓）：`scripts/soko` 报
  `refusing to run an unverified sokonanoda binary — cache(STALE: expected 0.81.0 darwin-arm64, found 0.73.0 …)` ✓。
- **成因** ✓：版本钉（`sokonanoda-version.txt` ✓）已被**内核线**升到 `0.81.0` ✓（`Cargo.toml`/`Cargo.lock`/
  `courses/set-theory/sokonanoda.toml` 都在其**在途**改动里 ✓），而仓库构建仍是 `0.80.0` ✓、缓存 `0.73.0` ✓。
- **影响**：**课程门禁与一切判卷暂不可跑** ✗（本轮因此**未开工新单元** ✓）。
- **复现件（登记在册 ✓，G7 重放）**：`courses/set-theory/gaps/C-03-runner-usable.sh` ——
  一条命令：`bash courses/set-theory/gaps/C-03-runner-usable.sh`；两态：
  (a) 真判卷一次 ⇒ `exit 0` 且自述版本 == 版本钉 ✓；(b) 注入原始坏状态
  （`SOKONANODA_VERSION=0.99.9` + 真判卷 ⇒ "钉到没有任何二进制匹配的版本"）⇒ **必须被拒**（实测 `exit 3`，
  「refusing to run an unverified sokonanoda binary … STALE」= 事故原样 ✓）。
  **反向验证实测**：把 (b) 的期望反过来 ⇒ 脚本判红 ✓；恢复 ⇒ 判绿 ✓。
  ⚠ 两条假注入教训（都踩过）：`0.73.0` **缓存里正好有** ⇒ 合法使用、不判红 ✗；
  `version --json` **不需要二进制** ⇒ 不经过"拒绝运行"那条路 ✗ ⇒ 注入必须配**真判卷调用** ✓。
- **出路** ✓ **已完成**：版本 bump 落地（内核线 `59201738` ✓，`Cargo.toml` = `requires` = `0.81.0` ✓）
  ⇒ **重建** ✓：`cargo build --release -p sokonanoda-cli` ⇒ `./target/release/sokonanoda --version`
  输出 **`sokonanoda 0.81.0`** ✓（1m48s ✓，**无版本耦合问题** ⇒ 无需另立条目 ✓）；
  判据 = `python3 courses/set-theory/tools/check.py --json` ⇒
  **234 目标 · 1983 checked · 876 open · 0 判负** ✓ —— 与重建前**逐项一致** ✓。

- **结论** ✓：**不是墙，是等式族证明项的宇宙层级拼法** ✓ —— 把等式写成**显式宇宙**
  `Eq.{1}` / `Eq.refl.{1}` 就**判绿** ✓（判据：`units/solutions/I.6/unit109-solution.sokonanoda`
  的 `ordinal_rec_nat_eq` ✓ —— 递归子对 `Acc.intro` 的归约是**定义性等式** ✓）。
  ⇒ 手册里「等式族证明项的宇宙层级」那条边界**又兑现一次** ✓。

## C-04（**closed · 非缺口（内核的正确行为）** · 2026-10-02 第 681 轮）**Prop → Type 的情形分析**判红

- **目标**：序数加法的**函数版**（按"是不是零 / 是不是后继"分支定义 ✓）。
- **实测**（第 680 轮）✗：`Or.elim` 的**动机位要落 `Type`**（如 `Nat`）时判红 ✓ ——
  `类型不匹配：期望 Sort(0)，实际是 Sort(1)` ✓（`Or` 是 `Prop` 且两个构造子 ⇒ 大消去被拒 ✓，
  这与 `Exists` 的情形同源 ✓）。
- **最小复现**：
  ```text
  def p_case (P : Prop) (a : Nat) (b : Nat) (h : P ∨ ¬ P) : Nat :=
    Or.elim P (¬ P) Nat (fun (_ : P) => a) (fun (_ : ¬ P) => b) h
  ```
- **影响**：**函数版**序数加法 ✗。**绕法（已采用 ✓）**：**关系版**加法（`AddsTo` 归纳关系 ✓，
  见单元109 ✓）—— 关系版不需要 Prop→Type 的情形分析 ✓，且结合律可在其上证明 ✓。
- **复现件（登记在册 ✓，G7 重放）**：`courses/set-theory/gaps/C-04-prop-to-type.sh`
  （两态：`…-reject.sokonanoda` **必须被拒** ✓ · `…-control.sokonanoda` 动机位落 `Prop` **必须判绿** ✓）
  —— 一条命令：`bash courses/set-theory/gaps/C-04-prop-to-type.sh`。
  **反向验证实测**：**原地**对调两条断言的角色 ⇒ **exit 1**（报「BAD C-04 #1: the large-elimination
  probe was ACCEPTED …」）✓；恢复 ⇒ **exit 0** ✓。⚠ 第一次反向验证是**空转**（脚本拷到 `/tmp` 跑 ⇒
  `ROOT` 解析成 `/` ⇒ 因路径缺失而失败）⇒ 已在原地重做 ✓。
- **定性（本轮，含判据）** ✓ **不是缺口** ✗ —— 是**内核的正确行为** ✓：
  * 判据一（**签名**）：`prelude/Prelude.sokonanoda:20` 是
    `def Or.elim {a b c : Prop} (f : a -> c) (g : b -> c) (h : Or a b) : c` ✓
    ⇒ 动机位**签名上就固定是 `Prop`** ✓ ⇒ **没有宇宙参数可以写成显式** ✗
    ⇒ C-02 那招（把等式写成 `Eq.{1}` ✓）**在此结构上不适用** ✓（C-02 能修是因为 `Eq.{u}` 有宇宙变量 ✓）；
  * 判据二（**探针**）：`def p_case_elim (P : Prop) (a b : Nat) (h : P ∨ ¬ P) : Nat :=
    Or.elim P (¬ P) Nat (fun _ => a) (fun _ => b) h` ⇒ `类型不匹配：期望 Sort(0)，实际是 Sort(1)` ✓；
  * **为什么这是对的** ✓：`Or` 是**两个构造子的 `Prop`** ✓，往 `Type` 消去会让结果**区分证明**
    ✗（与证明无关性冲突 ✓）—— 与 `lib/Exists` 头部已记的"**大消去仍然不可用，而且这是正确的行为**"✓ **同族** ✓。
- **绕法（已采用 ✓）**：**关系版**编码（`AddsTo` 归纳关系 ✓，见单元109 ✓）—— 不需要 Prop→Type 的情形分析 ✓。
- **收编去向**：可并入 `docs/gaps/ledger.jsonl` 的 **G-6** 那条作**补充说明**（"`Or` 同此，且是正确行为" ✓），
  但**不作为新缺口** ✗。

## C-05（open）序数加法的**结合律**（关系版）待装配 —— **复现件已在册** ✓

- **目标**：`AddsTo x y z ⇒ AddsTo z w v ⇒ ∃ u, AddsTo y w u ∧ AddsTo x u v` ✓（即 `(x+y)+w = x+(y+w)` ✓）。
- **零件状态** ✓：`AddsTo` 的两条构造子（`addsTo_zero` ✓ / `addsTo_succ` ✓）**判绿** ✓；
  递归子与计算规则（`ordinal_rec_nat` ✓ / `ordinal_rec_nat_eq` ✓ / 唯一性 ✓）**判绿** ✓。
- **装配缺什么** ✗：先要**逆引理** `AddsTo x (σ y) v ⇒ ∃ z, v = σ z ∧ AddsTo x y z` ✓
  —— 用 `AddsTo.rec` 写时**调用形状判红一次** ✗（参数/动机位待调 ✓，属工程问题、非内核墙 ✓）；
  有了它，结合律就是**对第一条推导做归纳** ✓（零情形取 `u := w` ✓、后继情形用逆引理 ✓）。
- **复现件（登记在册 ✓，G7 会重放）**：`courses/set-theory/gaps/C-05-addsTo-assoc.sh` ——
  一条命令：`bash courses/set-theory/gaps/C-05-addsTo-assoc.sh`；登记状态 **open**
  ⇒ 期望是"**缺口仍在**"（零件在 ✓、装配件不在 ✓）⇒ **exit 0**；
  一旦 `addsTo_succ_inv` / `addsTo_assoc` 进了解答 ⇒ **exit 1**（提示把本条改成 `closed`
  并把脚本期望翻转 ✓）。
- **反向验证（实测，第 682 轮）**：把带 `sorry` 的 `addsTo_assoc` 临时塞进解答 ⇒ 脚本
  **exit 1**，报「BAD C-05: assembly lemma IS present now: addsTo_assoc」✓；删掉 ⇒ **exit 0** ✓
  （`git diff` 为空 ⇒ 解答已还原 ✓）。
- **怎么验（修的时候）**：把逆引理与结合律写进 `unit109` 的解答判绿 ⇒ 本条改 `closed` ✓、
  同时把复现件的期望翻转 ✓（两件事必须一起做，否则 G7 会判负 —— 这正是"断言与登记一致"的守卫 ✓）。

## 110（**closed-green** · 2026-10-02 第 683 轮）选择公理的取数据 —— 关键步靠 **G-58 已修**

- **内容**（`courses/set-theory/units/I.6/unit110-choice.sokonanoda`，3 练习 + 1 演示，解答 **4 条全绿** ✓）：
  `choice_family`（**族版选择函数** `∀ i ∈ I, ∃ x, x ∈ F i ⇒ ∃ f, ∀ i ∈ I, f i ∈ F i` ✓ ——
  良序定理/基数可比性的第一步 ✓）· `choice_family_apply` · `choice_right_inverse`（满射的右逆 ✓）。
- **为什么以前写不了** ✓：从 `∀ a, ∃ b, …` **取出函数**要 Prop→Type 的取数据 ⇒ 台账 **G-58** 已修 +
  `lib/Choice` 的公理到位才成立 ✓。
- **复现件（登记在册 ✓，G7 重放）**：`courses/set-theory/gaps/C-110-choice-extraction.sh` ——
  一条命令：`bash courses/set-theory/gaps/C-110-choice-extraction.sh`；断言三件：
  ① 两条定理在；② **证明里真的调用了 `choice`**（两处调用点都在，不是被绕成别的东西）；
  ③ 判卷 `checked>0` · `exercise_open==0` · 无拒绝。
- **反向验证实测**：把 `choice_family` 的证明体换成 `sorry` ⇒ 脚本 **exit 1**
  「BAD 110: checked=… open=1（a proof was replaced by sorry?）」✓；恢复 ⇒ **exit 0**
  「OK 110 matches the register: checked=4 exercise_open=0 failed=0」✓（`git diff` 为空 ✓）。
- **写法教训（本轮换来，已写进画布头）**：`Exists.elim` 是**隐式实参**签名
  `{A}{p}{Q} (h) (f)` ⇒ 补不出来时**逐位写全** ✓；`choice` 的谓词里**别再写 `∈ B`** ✗（结论自带 ✓）；
  嵌套位置的等式写 **`Eq.{1}`** ✓。

## 111（**closed-green** · 2026-10-02 第 684 轮）ℕ 的算术律 —— 关键步靠 **G-76 已修**

- **内容**（`courses/set-theory/units/I.6/unit111-nat-arith.sokonanoda`，**7 练习** + 2 已证 `def`，
  解答 **13 条全绿** ✓）：`add` / `mul`（**自建**，对第二个参数用 `Nat.rec` 递归 ✓）·
  `add_zero` / `add_succ` / `mul_zero` / `mul_succ`（**定义性等式**，`Eq.refl` ✓）·
  `zero_add` / `succ_add` / `add_assoc` / `add_comm` · `add_rotate`（搬运助手 ✓）·
  `zero_mul` · **`mul_distrib`**（分配律 ✓）。
- **为什么必须自建** ✓：prelude **只有 `Nat` 的构造子与 `Nat.rec`**，**没有 `Nat.add`/`Nat.mul`** ✗
  ⇒ 课程自己定义再证律 ✓；**定义动机落 `Type`**（大消去 ✓）⇒ 靠 **G-76** 已修 ✓。
- **复现件（登记在册 ✓，G7 重放）**：`courses/set-theory/gaps/C-111-nat-induction.sh` ——
  一条命令：`bash courses/set-theory/gaps/C-111-nat-induction.sh`；断言四件：
  ① `add` **真的**由 `Nat.rec`（Type 动机）定义；② `mul` 同；③ 11 条律都在；
  ④ 判卷 `checked=13` · `exercise_open==0` · 无拒绝。
- **反向验证实测**：把 `add_comm` 的证明体换成 `sorry` ⇒ 脚本 **exit 1**
  「BAD 111: grading rejected (exit=1, failed=2)」✓；恢复 ⇒ **exit 0**
  「OK 111 matches the register: checked=13 exercise_open=0 failed=0」✓（`git diff` 为空 ✓）。

## C-112（**closed-green** · 2026-10-03 交付）Cantor–Bernstein（§四 第 6 项）—— **经选择公理**

- **目标命题**（Enderton §6.4 定理 6B · Halmos §22）：`A ≼ B` 且 `B ≼ A` ⇒ `A ≈ B` —— **已交付** ✓。
- **落在哪**：画布 `units/I.3/unit112-cantor-bernstein.sokonanoda`（20 条给定件 + **12 条练习** ✓）·
  解答 `units/solutions/I.3/unit112-solution.sokonanoda`（**32 条全绿 · 0 open · 0 判负** ✓，
  **全项风格、无 `by` 块** ✓ —— 手册「解答写法：项风格」）· `course.json` 章 **I.3**（像、原像与基数 ✓）。
- **数学骨架**（三段全绿 ✓）：① **交集版不动点** —— `C₀ = ⋂{C | C ⊆ A ∧ K(C) ⊆ C}`
  （`K(C) = A ∖ g '' (B ∖ f '' C)`，**单调** ✓）⇒ `K(C₀) ⊆ C₀`（`cbC0_in_fix` ✓）+
  `C₀ ⊆ K(C₀)`（`cbC0_subset_K` ✓，靠"`K(C₀) ∈ 𝓕` + 最小性"）⇒ **`K(C₀) = C₀`** ✓；
  ② **关系版图** `R(x,y) = (x ∈ C₀ ∧ f x = y) ∨ (x ∉ C₀ ∧ g y = x)`（`cbRel`，**Prop 层** ✓）
  ⇒ **四件**：全（`cbRel_total`）· 单值（`cbRel_single`）· 单射（`cbRel_inj`，用 `cbC0_gf`）·
  满（`cbRel_surj`，用 `C₀ ⊆ K(C₀)`）✓；③ **`choice` 取两次**（前向 + 反向）⇒ `Set.Equiv.mk` ✓。
- **⚠⚠ 诚实标注（原样保留，不许弱化）**：装配那一步用 **`choice`** 从"全 + 单值"的关系里取双射
  ⇒ **本课 CB 经选择公理** ✓；而 **CB 数学上不需要选择**（`g` 单射 ⇒ 前像唯一 ⇒ 确定摹状词即可）
  ⇒ **这是偏离，不是"标准证法"** ✗。零公理的 CB 要 `Exists` → `Type` 的安全消去（**G-6**，仍 open ✗）。
- **复现件（登记在册 ✓，G7 重放）**：`gaps/C-112-cantor-bernstein.sh` —— 登记 **closed-green**，
  三条断言：① 零件在（`Set.InjOn` / `choice` / `Set.sInter` ✓）；② **主定理在位且判绿**
  （`decl_checked>=32` · `exercise_open==0` · `failed==0` ✓）；③ **墙仍在**（CB 双射的**直接定义**
  仍判红 = **C-04 / G-6** ✓）。
- **反向验证实测（三段 ✓）**：正向 **exit 0** ✓；把解答里 `cantor_bernstein` 的**证明体换 `sorry`**
  ⇒ **exit 1**（「BAD 112: checked=31 (expect >= 32) open=1」✓）；注入"墙消失"（reject 探针换成
  判绿内容）⇒ **exit 1**（「BAD 112: the wall is GONE」✓）；两次恢复后 **exit 0** ✓（工作树无残留 ✓）。
- **推导件（留档 ✓）**：`gaps/C-112-cb-derivation.md`（§8 案情）· `gaps/C-112-cb-prop.sokonanoda`
  （第 1–4 步 7 条）· `gaps/C-112-cb-step5.sokonanoda`（第 5 步 32 条 = 单元解答的来源 ✓）。
- **语言侧边界（如实记 ✓，不是课程缺口）**：写这个单元实测到三条**子集边界**（已写进画布头部）：
  ① 同一个 `cases` 的**两条臂都再嵌 `cases`** ⇒ 声明被**吞掉** ✗；② `Exists.intro` 的**字面 λ 谓词**
  判红 ✗（改用**具名 def 谓词** ✓）；③ `Eq.{1}` 出现在 **`have` 的类型位**会解析错位 ✗。
  ⇒ 解答因此写成**全项风格 + `Or.elim` 证明项**（**多层 `cases` 一律不用** ✓）。

## C-113（**open · 绕行已交付** · 2026-10-03 课程线第 4 轮）结论是 `Or` 且两支为 **def-应用** 的声明被判 `rejected` ✗

**症状**（`node scripts/soko query check --file <解答>` ✓）：

```
rejected: expected a pi type, got: ((Or.[] ((((Set.Equiv.[] Nat.[]) (Set.[] Nat.[])) (Set.univ.[] Nat.[])) $2)) …
```

**最小对照**（差别只在「`Or` 的两支是不是 def-应用」✓）：

```sokonanoda
axiom P : Prop
axiom Q : Prop
axiom h : P ∨ Q
theorem bare_or : P ∨ Q := h      -- ✓ 判绿（两支是 Prop 变量）
-- 同一形状，两支换成 `Set.Equiv`-应用（`≈`）⇒ ✗ rejected（上面那条报文）
```

**影响与绕行** ✓：单元113 的 `ch_apply` / `ch_left`（结论 = `Or` + 两支 `Set.Equiv`-应用 ✗）
**只能写成 ∀-headed 形态** ✓ —— 已按此绕行交付（单元113 判绿 ✓：`checked 4 · open 0 · failed 0` ✓）；
**本条目记录未修的边界本身** ✗。

**归属**：**G-73 家族**（def-headed 期望类型处补不出前导实参 ✗）⇒ 内核侧 ✓。
**复现口径**：本条目**不带 `.sh`** ✓（G7 的 7 条复现件不变 ✓）—— 上面两行就是完整复现 ✓。

## C-114（**open · 绕行 = 暂不做** · 2026-10-03 课程线第 16 轮）Cantor 对角线证明的**形态链** ✗

**目标**：单元113 想加 `¬ (𝒫 ℕ ≼ ℕ)`（Cantor ✓）。**五个形态全部实测判红** ✗（判据 = `query check --root courses/set-theory`）：

| # | 形态 | 读数 |
|---|---|---|
| 1 | `Or.elim (Classical.em P) h1 h2`，目标位 `False` | ✗ `期望 Pi (_ : P), False，实际是 Or P (Not P)` |
| 2 | 同上但**写全前导实参** `Or.elim P (P → False) False …` | ✗ **同一条报文** ⇒ 写全也救不了 |
| 3 | `Set.InjOn` 应用，实参含 **`diagonal f`（def-应用）** | ✗ `期望 Sort(0)，实际是 Nat` |
| 4 | 假设式 `hDdef : ∀ x, D x ↔ ¬ (f {x} = x)`（想避开 def 展开） | ✗ `期望 Sort(0)，实际是 Iff …`（= 手册「`↔`+`¬` 绑定注解」那条 ✓） |
| 5 | 同 4 但**拆成两条蕴含** ✓（手册的绕法 ✓） | ✗ `期望 Sort(0)，实际是 Nat` ⇒ **露出下一层** ✗ |

**已判绿的件**（留档，别重做 ✓）：`def diagonal` ✓（主干 `94e96775`）· `Classical.byContradiction P h : P` ✓ ·
`Set.InjOn` 应用（实参**全是不透明名字**）✓ · 单射性顶矛盾的 `Eq.subst` 目标形态 ✓（`inj-use` 探针 checked 7 ✓）。

**归属**：全部落在 **G-73 家族**（def-headed / 绑定注解处补不出前导实参 ✗）⇒ 内核侧 ✓。
**处置** ✓：**暂不做**（unit113 已有 `CH` 陈述 + 展开 + 独立性读评 ✓ 判绿 ✓）；Cantor 留待内核侧收敛后重开 ✓。

## C-115（**open · 绕行 = 写点态形态** · 2026-10-03 课程线第 36 轮）`𝒫` 出现在**期望类型位**时补不出前导类型参数 ✗

**实测两处**（判据 = `query check --root courses/set-theory` ✓）：

| # | 位置 | 读数 |
|---|---|---|
| 1 | `Set.Le.elim α β A B ((𝒫 A) ≼ (𝒫 B)) h …` 的**动机位** | ✗ `类型不匹配：期望 Sort(0)，实际是 $13`（**改点名写法也同红** ✗ —— 已验证 ✓） |
| 2 | 声明类型位：`Set.InjOn (Set α) (Set β) (fun (S : Set α) => f '' S) (𝒫 A)` | ✗ `期望 Sort(0)，实际是 $10` |

**归属** ✓：**G-73 家族第 6 形态**（期望类型位上的 `𝒫`/def-应用补不出前导类型参数 ✗）⇒ 内核侧 ✓。
**绕行** ✓（推荐）：**把 `𝒫` 从声明类型里拿掉** —— 用**点态/显式前提**表达 ✓：
`S ∈ 𝒫 A` 改写成前提 `S ⊆ A` ✓；结论写成 `∀ S, S ⊆ A → f '' S ⊆ B` 这类形态 ✓。
**留档已判绿** ✓（别重做 ✓）：`Set.Le.trans`（`Set.Le.elim` 动机位放**不含 `𝒫`** 的 `(A ≼ C)` ✓ **判绿** ✓）·
`Set.Le.refl` ✓ · `Set.mem_image_intro/elim` ✓ · `Set.mem_powerset_iff`（**已在 `lib/Set` 的 `namespace Set` 里** ✓）·
像集单射的**证明体**（`Set.ext` + `mem_image_elim` + `hf` 先给 `y = x` 再搬运 ✓）**本身判绿** ✓，红的只是**声明类型** ✗。

### C-115 补记（2026-10-03 第 37/38 轮）：病根比"期望类型位上的 `𝒫`"**更深** ✗

**三版绕行全红** ✗（判据 = `query check --root courses/set-theory` ✓）：

| 版本 | 声明里还剩什么 | 读数 |
|---|---|---|
| v1 | `Set.InjOn (Set α) (Set β) (fun S => f '' S) (fun S => S ⊆ A)`（`𝒫` 已换成点态 λ ✓） | ✗ `期望 Sort(0)，实际是 $10` |
| v2 | 集合族**参数化** ✓：`(g : Set α → Set β) (hg : ∀ S, S ⊆ A → g S = f '' S)`，函数位与集合位**全是名字** ✓ | ✗ `期望 Sort(0)，实际是 **Sort(1)**` |
| v0 | `Set.InjOn (Set α) (Set β) (fun S => f '' S) (𝒫 A)` | ✗ 同族 |

⇒ **精确表述** ✓：**`Set.InjOn` 作用在"高阶论域"（`Set α` 作为元素类型）上的声明本身过不去** ✗ —— 与 `𝒫` 无关 ✓（v2 里已无 `𝒫`、无 λ ✓ 仍红 ✓）。
**绕行（最终）** ✓：**这类命题在本子集里不做** ✗ —— 改推**一阶/点态**条目 ✓（声明里不出现 `Set α` 作为元素类型 ✓，如已判绿的 `Set.Le.trans`/`Set.Le.refl` ✓）。
**归属** ✓：G-73 家族（本子集的类型层限制 ✗）⇒ 内核侧 ✓。

## C-116（**open · 绕行 = 换靶** · 2026-10-03 课程线第 45 轮）`Eq` 嵌套链出现在 `Set.InjOn` 证明体里判红 ✗

**目标**：`lib/Equiv` 加 `Set.Le.of_equiv`（`A ≈ B → A ≼ B` ✓）。**定位过程全部实测** ✓（判据 `query check --root courses/set-theory`）：

| 对照 | 读数 | 排除了什么 |
|---|---|---|
| `a1 : Set.InjOn α β f A → Set.InjOn α β f A` | **✓ 绿** | `Set.InjOn` **无罪** ✓ |
| `a2 : Set.LeftInvOn α β g f A → Set.LeftInvOn α β g f A` | **✓ 绿** | `Set.LeftInvOn` **无罪** ✓ |
| `a3 : A ≈ B → A ≈ B`（跨类型 ✓） | **✓ 绿** | `≈` 记法 **无罪** ✓ |
| `Set.Equiv.elim … (A ≈ B) h (fun f g … => h)` | **✓ 绿** | `Set.Equiv.elim` + 6 参续延 **无罪** ✓ |
| 辅助引理 `Set.injOn_of_leftInvOn … : Set.InjOn α β f A`（体 = `Eq.trans` 套 `Eq.trans` 套 `Eq.symm` + `Eq.subst` ✗） | ✗ `line 3`：`期望 Sort(0)，实际是 Sort(1)` | **病根 = 证明体里的 `Eq` 嵌套链** ✗ |

**归属** ✓：与 C-115 同族（本子集的**类型层/elaboration 限制** ✗）⇒ 内核侧 ✓。
**绕行（本轮采用）** ✓：**换靶** ✗ —— S-B 内容推进改走**已判绿的形态族** ✓（一阶 `≼` 链 ✓、`Set.Le.trans/refl` ✓）；
`Set.Le.of_equiv` 与 `Set.Le.powerset` **一并搁置** ✗，待内核侧收敛后重开 ✓。
**留档已判绿** ✓（别重做 ✓）：`Set.Le.trans` ✓ · `Set.Le.refl` ✓ · 上述 `a1/a2/a3` 与 `t3` ✓ · `Set.mem_image_intro/elim` ✓ · `mem_powerset_iff`（库内 ✓）。

## C-117（**open · 低优先 · 建议迁移** · 2026-10-03 课程线第 60 轮）`unit19` 局部引理与库 `Set.Le.*` **同义并存** ✗

**实测**（`grep -n "^theorem" courses/set-theory/units/I.8/unit19-cardinal-le.sokonanoda` ✓）：
`unit19` 画布**早就自建**了 `le_refl` ✓ · `subset_le` ✓ · `le_trans` ✓ · `le_of_subset_le` ✓（**裸名** ✓），
而本轮又在 `lib/Equiv` 立了同义的 `Set.Le.refl` ✓ · `Set.Le.of_subset` ✓ · `Set.Le.trans` ✓ · `Set.Le.mono_right` ✓。
**名字不同 ⇒ 不判红** ✓（`import-name-collision` 查的是**同名** ✓），但按手册那条
「**库该有 ≠ 重复一份**：若是普遍需要，正确做法是把**单元里那份**搬进库并改调用点 ✓」⇒ 现状**不合手册口径** ✗。
**处置（建议，未做）** ✓：把 `unit19` 的局部四条**删掉** ✗、调用点改用 `Set.Le.*` ✓（画布 + 解答同步 ✓；
注意先分清哪几条是**画布练习**（删了会影响 G4 覆盖 ✗）哪几条只是**解答里的辅助** ✓）。
**优先级** ✓：低（当前全绿 ✓ 无冲突 ✓）；**不必**在预算紧时硬做 ✗。

### C-117 更正（2026-10-03 第 62 轮）：**"建议迁移"是错的** ✗ —— 实为**非缺口** ✓

**实测**（逐条数画布/解答 ✓）：`le_refl` · `subset_le` · `le_trans` · `le_of_subset_le` 四条
**在 `unit19` 画布里各 1 处、解答里各 1 处** ✓，且画布那四处**都是 `sorry` 练习** ✓
（例：`theorem le_trans … : A ≼ C := by sorry` ✓）。

⇒ **它们是单元的教学练习** ✓，**不是"意外的重复"** ✗ ⇒ **删掉就是删课** ✗✗（还会破 G4 覆盖 ✗）。
⇒ **结论** ✓：**单元练习 ≠ 库基础设施** —— 库的 `Set.Le.*` 是**给后续单元复用的地基** ✓，
`unit19` 的四条是**给学习者做的题** ✓，两者**并存是对的** ✓ ⇒ **C-117 撤销** ✓，归类为
**"非缺口（课程组织方式的正确行为）"** ✓（同 C-04 的先例 ✓）。
⇒ **不再做任何"迁移/删单元引理"的动作** ✗；后续单元若要用，**用库版** ✓ 即可（两者不冲突 ✓）。

## C-118（**open · 绕行 = 避开 `And.right`** · 2026-10-03 S-B 第 7 轮）对「`∧` 的右支是 `∀`-命题」取件时补不出实参 ✗

**实测四版全红** ✗（判据 = `query check --root courses/set-theory` ✓；目标 = 从 `IsSuccOf` 取出元素刻画）：

| # | 写法 | 读数 |
|---|---|---|
| 1 | `And.right (Iff.mp (isSuccOf_def α E x y) h) y`（隐式 ✓） | ✗ `期望 Sort(0)，实际是 Pi (α : Sort(0)), …` |
| 2 | 同上但 `Iff.mp`/`Iff.mpr` **两侧写全** ✓ | ✗ `期望 Sort(0)，实际是 第 5 个绑元` |
| 3 | **λ 落地** ✓ + `And.right` 的 `{a b}` **写全** ✓ | ✗ 同 `第 5 个绑元` |
| 4 | 拆成具名引理 `isSuccOf_elem_iff`（顶层位置 ✓ + λ 落地 ✓） | ✗ `期望 Sort(0)，实际是 第 6 个绑元` |

**归属** ✓：**G-73 家族**（与 C-115/C-116 同源 ✗）⇒ 内核侧 ✓。
**绕行** ✓：序数侧改推**不经过 `And.right`** 的条目 ✓（如 `E z x → IsOrdinal α E z` ✓ 走 `ordinal_elems_isTransitive` 一族 ✓），或转 ZFC / AC / 基数算术侧 ✓。
**留档已判绿** ✓（别重做 ✓）：`isSuccOf_parts` ✓（`63aac9ad` ✓，`lib Ordinal` **16 checked · 0 open · 0 判负** ✓，反向验证 **16→15→16** ✓）——
手册惯用法 #2（**绑定形态先落地成 λ** ✓）在**它**上面奏效 ✓，但在 `And.right` 那一步**无效** ✗。

## C-119（**open · 绕行 = 换靶** · 2026-10-03 S-B 第 19 轮）结论是 **`Or` 头** 的取件子补不出实参 ✗

**目标**：`lib/ZF` 加配对取件子（`IsPair α E x a b` ⇒ `E a x` ✓ / `z = a ∨ z = b` ✓）。**三版全红** ✗
（判据 = `query check --root courses/set-theory` ✓）：

| # | 写法 | 读数 |
|---|---|---|
| 1 | 一层 λ 落地 + `Iff.mpr (hp a) (Or.inl (Eq.refl α a))` | ✗ `期望 Sort(0)，实际是 第 6 个绑元` |
| 2 | **两层** λ 落地（先落 `hp`、再落 `hp a` 的 `↔`） | ✗ `第 7 个绑元` |
| 3 | **只走 `Iff.mp` 方向**（`Iff.mp (hp z) hz`，结论 `z = a ∨ z = b`） | ✗ `第 9 个绑元` |

**规律** ✓（本 goal 实证）：`Iff.mp` 方向**并非一律可用** ✗ —— 判绿的两条（`isEmpty_no_elem` ✓ · `isSuccOf_parts` ✓）
结论都**不是 `Or` 头** ✓；而**结论是 `Or` 头**（本条目 ✗）时，即便只走 `Iff.mp` 也补不出 ✓
⇒ 与 CH 单元时代的「`Or` 两支为 def-应用 ⇒ `rejected`」✗ **同源** ✓ ⇒ **G-73 家族** ⇒ 内核侧 ✓。

**绕行** ✓：**换靶** —— `lib/ZF` 的 `Or` 头条目搁置 ✗；转 **`lib/Cardinal` / `lib/Choice`** 侧 ✓
（候选 `Nonempty α → ∃ a, a = a` ✓ 结论是 `∃` 头 ✓ 非 `Or` ✓）。
**留档已判绿** ✓（别重做 ✓）：`isEmpty_no_elem` ✓（`ca5dffca`，`lib ZF` 12/0/0 ✓，反向验证 12→11→12 ✓）·
`lib/Ordinal` 三条 ✓（16→15→16 / 17→16→17 / 18→17→18 ✓）。

## C-120（**open · 绕行 = 搁置并换靶** · 2026-10-03 S-B 第 34 轮）`Exists.elim` **嵌套链**在 `def` 头目标下补不出实参 ✗

**目标**：`lib/Cardinal` 加 `Type.Equiv.symm`（`Type.Equiv α β → Type.Equiv β α` ✓）。**两版全红** ✗
（判据 = `query check --root courses/set-theory` ✓）：

| # | 写法 | 读数 |
|---|---|---|
| 1 | 两次 `Exists.elim`，动机写 **`def` 头** `Type.Equiv β α` | ✗ `期望 Sort(0)，实际是 第 5 个绑元` |
| 2 | 动机改**展开形态**（`∃ f', ∃ g', …` ✓），结论仍写 `Type.Equiv β α`（靠 defeq ✓） | ✗ **同一条报文** ⇒ 换动机**无效** |

**归属** ✓：**G-73 家族**（与 C-118「对 `∀`-右支取件」✗、C-119「`Or` 头结论」✗ 同源）⇒ 内核侧 ✓。
**绕行** ✓：`symm`/`trans` 一并**搁置** ✗；转**非嵌套 `∃`** 条目 ✓，或回 `lib/ZF` / `lib/Ordinal` 的**非 `Or` 头** ✓。
**留档已判绿** ✓（别重做 ✓）：`Type.Equiv.of_inverses` ✓ · **`Type.Equiv.of_inverses'`** ✓（外层包装 ✓ `3fc0d778` ✓，
`lib Cardinal` **10 checked · 0 open · 0 判负** ✓，反向验证 **10 → 9 → 10** ✓）· `Type.Equiv.refl` ✓（`067321b5` ✓）。
**形态规律累计** ✓（本 goal 实证 ✓）：① 绑定形态先落地成 λ ✓；② 内层 `∃` 拆具名引理 ✓；
③ 动机位 λ 体内**只能引用引理**、不内联嵌套 `∃` ✓；④ `Exists.elim` 动机**不写 `def` 头** ✗（且改成展开形态也无效 ✗）。

## C-121（**open · 绕行 = 转单元侧** · 2026-10-03 S-B 第 49 轮）三库（Ordinal/ZF/Cardinal/Choice）**安全区已尽** ✗

**本轮逐库盘点** ✓（判据 = 各库文件自述 + 签名实读 + 探针 ✓）：

| 库 | 本轮新增 | 剩余项的性质 |
|---|---|---|
| `lib/Ordinal` ✓ | 6 条 ✓（`isSuccOf_parts` · `ordinal_elem_isOrdinal` · `ordinal_elem_elem_isOrdinal` · `isSuccOf_isOrdinal` · `isSuccOf_transitive` · `isSuccOf_elems_transitive`） | 已覆盖 ✓ 或 **C-118 族** ✗（对 `∧` 右支取件 ✓） |
| `lib/ZF` ✓ | 2 条 ✓（`isEmpty_no_elem` · `isUnionOf_elim`） | 已覆盖 ✓（`isEmpty_intro`/`extensional_apply` 均在 ✓）或 **G-56** ✗（无 ∈-循环/无穷下降链 ✓） |
| `lib/Cardinal` ✓ | 3 条 ✓（`of_inverses` · `Type.Equiv.refl` · `of_inverses'`） | **C-120 依赖** ✗ —— `Cardinal.mk_inj` 需 `Quot.exact` ✓，而它**要求 `Type.Equiv` 的自反/对称/传递三件** ✗（诊断原文给出签名 ✓），`symm`/`trans` 正是 C-120 ✗ |
| `lib/Choice` ✓ | 0 条 ✗ | **G-58** ✗ —— 文件自述："从 `choice` 取函数要 `Exists.elim` 的数据版 ⇒ 撞 G-58 ⇒ 本版不做" ✓；`Zorn`/`wellOrdering`/基数可比性同理 ✗ |

**绕行** ✓：**S-B 推进转单元侧** —— 给**已判绿的库引理配课内练习** ✓（与 `unit113` 练习 3–6 同一手法 ✓：探针先绿 → 整文件追加 → `audit-pairs` → `check.py --only` → 路径限定 commit → 反向验证 ✓）。
**留档已判绿 10 项** ✓（别重做 ✓）：`lib/Ordinal` 6 ✓（反向链 16→15→16 / 17→16→17 / 18→17→18 / 19→18→19 / 20→19→20 / 21→20→21 ✓）·
`lib/ZF` 2 ✓（12→11→12 / 13→12→13 ✓）· `lib/Cardinal` 3 ✓（9→7→9 / 10→9→10 ✓）。
