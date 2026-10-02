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

## C-112（open · **未开工** · 2026-10-02 第 684 轮登记）Cantor–Bernstein（§四 第 6 项）

- **目标命题**（Enderton §6.4 定理 6B · Halmos §22）：`A ≼ B` 且 `B ≼ A` ⇒ `A ≈ B`。
- **状态：未交付** ✗ —— 卡点已从"预算"变成**精确的墙** ✓（第 685 轮实测）：
  **CB 双射的"直接定义"判红** ✗ —— 按 `x ∈ C₀` 分支需要 **Prop→Type 的情形分析**
  ⇒ `类型不匹配：期望 Sort(0)，实际是 Sort(1)` ✓（判据 = `gaps/C-112-cb-direct-def-reject.sokonanoda` ✓）。
  这是 **C-04 同一条墙** ✓，也正是本条目下面写的「零公理 CB 要 `Exists`→Type 的安全消去
  （**G-6** 一族，仍 open ✗）」**在 CB 上的具体面孔** ✓。
- **出路（已定，未走完）** ✓：把 CB 的**图**写成 **Prop**（关系版 ✓，不做 Type 层分支 ✓），
  再证"全 + 单值 + 单射 + 满射" ✓，最后用 **`choice`** 取出双射函数 ✓ ——
  **这条路要 3–4 条零件引理 + 链构造**（`C₀` 的不动点 ✓），是多轮工作量 ✗。
- **G-63（`Quot`）与 CB 无关** ✓（已核）
  ✓（CB 走单射/满射/子集：`lib/Equiv` 的 `Set.InjOn` ✓ + `lib/Choice` 的 `choice` ✓，
  `grep -c Quot lib/Equiv.sokonanoda` = **0** ✓）。
- **⚠⚠ 诚实标注（写单元时必须原样带上，不许写成"标准证法"）**：本课的 CB 打算**经选择公理**
  —— 用 `choice` 从 `∀ b ∈ B, ∃ a ∈ A, …` 里**取出前像函数** ✓。而 **CB 数学上不需要选择**
  ⇒ **这是偏离** ✗。零公理的 CB 要 `Exists`→Type 的安全消去（**G-6** 一族，仍 open ✗）。
- **复现件（登记在册 ✓，G7 重放）**：`courses/set-theory/gaps/C-112-cantor-bernstein.sh` ——
  一条命令：`bash courses/set-theory/gaps/C-112-cantor-bernstein.sh`；登记 **open**
  ⇒ 期望"**缺口仍在**"，三条断言：① 零件在（`Set.InjOn` + `choice` ✓）；
  ② CB 主定理**不在**任何解答里 ✓；③ **墙是真的**（直接定义仍被判红 ✓）。
  交付 CB 时**三件事必须一起做** ✓：登记改 `closed` · 本脚本期望**翻转为正向**（断言主定理在位且判绿）·
  反向验证改成"拿掉/换 `sorry` ⇒ 判红" ✓ —— 否则就是"声明与守卫之间有缝" ✗。
- **反向验证实测（三段）**：正向 **exit 0** ✓；注入"CB 主定理已存在" ⇒ **exit 1**
  「BAD 112: a CB main theorem IS present now: …」✓；注入"墙消失"（把 reject 探针换成判绿内容）⇒
  **exit 1**「BAD 112: the wall is GONE …」✓；恢复 ⇒ **exit 0** ✓（`git diff` 为空 ✓）。
- **反向验证实测**：临时把 `theorem cantor_bernstein …` 塞进解答 ⇒ 脚本 **exit 1**
  「BAD 112: a CB main theorem IS present now: …」✓；删掉 ⇒ **exit 0** ✓（`git diff` 为空 ✓）。
- **下一步（装配计划）**：① 立 `C₀ = ⋂{C | A∖B ⊆ C ⊆ A ∧ f(C) ⊆ C}` 的存在性（用 `lib/Set` 的
  `Set.sInter` 一族 ✓）；② 由 `C₀` 的最小性得**不动点**性质 `f(C₀) = C₀ ∩ B` ✓；
  ③ 用 `choice` 取前像造双射 ✓；④ 全程在单元头写明偏离 ✓。
