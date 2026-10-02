# 卷 I 未结项（课程侧登记 · 下一位可直接接手）

> 为什么不在 `docs/gaps/ledger.jsonl` 里：那份台账记的是**语言/内核缺口**（G 系列），
> 且**当时正挂在内核线的在途改动里**（本仓库有并行会话）⇒ 课程侧的未结项登记在本文件 ✓
> （它是**课程**的未完成度，不是语言的缺口 ✓）。**格式固定**：编号 · 状态 · 零件判据 · 缺什么。

## C-01（open）序数三歧的**装配**：主定理 `ordinal_trichotomy` 未落地

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
- **怎么验**：把主定理写进 `unit108` 的解答（或新单元）判绿 ⇒ 把本条改成 `closed` 并把
  判据路径写在这里 ✓。

## C-02（open · 待内核线分诊）`Acc.rec` 的**计算规则**无法写成 Type 层等式

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

## C-03（open · 环境）运行器拒绝运行：版本钉已升 `0.81.0`，而构建/缓存落后

- **现象**（第 679 轮 ✓）：`scripts/soko` 报
  `refusing to run an unverified sokonanoda binary — cache(STALE: expected 0.81.0 darwin-arm64, found 0.73.0 …)` ✓。
- **成因** ✓：版本钉（`sokonanoda-version.txt` ✓）已被**内核线**升到 `0.81.0` ✓（`Cargo.toml`/`Cargo.lock`/
  `courses/set-theory/sokonanoda.toml` 都在其**在途**改动里 ✓），而仓库构建仍是 `0.80.0` ✓、缓存 `0.73.0` ✓。
- **影响**：**课程门禁与一切判卷暂不可跑** ✗（本轮因此**未开工新单元** ✓）。
- **出路** ✓：等内核线完成 bump（构建/发布到位 ✓）后即可恢复 ✓；或临时 `SOKONANODA_BIN=<匹配钉的二进制>` ✓
  （**会绕过版本校验** ✓，只适合探针，不适合当门禁判据 ✗）。
