# v0.77.0 的内核不足清单（ST15 交付物）

> **这份文件就是 v0.77.0 的验收产出**（用户 2026-09-28 拍板：**「v0.77 不要求全绿，
> 产出物是 ST15 的 kernel 不足清单」「把不足逼出来才算是做完，把它修好不是本版目标」**）。
> 每条都带**复现件**（`docs/gaps/repro/`）与**台账条目**（`docs/gaps/ledger.jsonl`）；
> 判据 `python3 scripts/gap.py check`（缺口仍在 / 行为已变 由复现件自己判）。
> **本版一行内核判定都没改** ✓（`git diff` 对 `crates/kernel/` 为空）。

## 一、三条 blocker（本版课程被它们切掉一整块）

### G-56 · `Acc`（良基性）立不起来 ⇒ 没有良基递归

- **今天**：`inductive Acc (α : Type) (r : α → α → Prop) (x : α) : Prop` +
  `ctor Acc.intro (h : ∀ y, r y x → Acc α r y) : Acc α r x` ⇒ `kernel-rejected` /
  「rejected: inductive occurrence is not applied uniformly to the block parameters and universe levels」。
  **对照组** `Even : Nat → Prop`（同形状、下标不变）**checked** ✓。
- **拦路点（只读定位，唯一）**：`crates/kernel/src/inductive.rs:44`（`check_ctor` 循环）调
  `check_uniform_inductive_occurrences`；它要求递归出现**恰好**套用 `num_params` 个实参
  （`args_rev.len() <= num_params` 才进断言）⇒ `Acc r b` 有 **2** 个实参 > `num_params=1`
  ⇒ 直接 assert 失败（索引 `b` 根本轮不到判定）。前端**零镜像**。
  ⚠ 同文件 `which_valid_ind_app_v`（`:1049`）用 `is_bvar_at` 判参数位置、**允许索引位任意**
  ⇒ 最小放宽应**对齐它**，不必新发明规则。
- ⚠ **2026-09-28 探针把机制钉死了，并证明「放宽不是挪一个比较符」**（用户授权的可行性探针）：
  - 判定本体的卫是 `args_rev.len() <= num_params` 才进断言；对 `Acc`（出现 `Acc α r y`，
  2 个实参 > `num_params=1`）**卫为假 ⇒ 完全跳过**，根本不检查；
  - 而**判红来自另一个块**（插桩实测：`num_params=3`、单构造子、`head_is_target=true`、
  `args_rev.len=3`、`offset=5`）：它的三个实参是 `Var(4) · Var(3) · Var(1)`，
  前两个是参数（对齐 ✓）、第三个不是（期望 `Var(2)`）⇒ 卫为真 ⇒ 断言触发；
  - 把卫改成 `>=` 并只校验**前** `num_params` 个实参 ⇒ **打破了上面那个原本能过的块**
  （它原本靠"跳过"过关）⇒ **判红，实测**；实验**已完整还原**，内核**零改动**。
  - ⇒ **修复的正确形状**：必须区分「**参数位**」（必须逐字是块参数）与「**指标位**」
  （允许任意、且 `Acc` 那种尾部多出的实参也是指标）**两段语义**，而不是挪一个比较符。
- ⚠ **另一条路（下标写返回位）也被堵死 ⇒ G-64**：`inductive Acc (α : Type) (r : α → α → Prop) :
  α → Prop` + `ctor Acc.intro (x : α) (h : ∀ y, r y x → Acc α r y) : Acc α r x`（**Lean core
  的官方写法**）能过 uniform 与 `SPEC0`（实测 `local_params.len=2 indices.len=1` ✓），
  但死在**递归子的宇宙代入** —— `crates/kernel/src/expr.rs:383`
  `assert_eq!(self.read_levels(ks).len(), self.read_levels(vs).len())` ⇒
  **`left: 0`（目标常量无宇宙参数）/ `right: 1`（要代入 1 个层级）**。
  ⇒ **带索引归纳有两道门**：G-56（声明级 uniform）与更晚的 G-64（递归子宇宙代入），
  本版**两条都不修**（用户 2026-09-28：「不可行 ⇒ 登记 + 补清单，登记本身就是交付物」）✓。
- **改动局部性**：只跑在**声明级**（`check_ctor` 里、`mk_elim_level` 之前），**不参与**
  `check_generated_recursors` / `check_positivity1` / 归约（`eval.rs` 的 `fire_quot`、iota）。
- **堵住**：ST6 传递闭包 · ST7 秩 · ST9 超限递归 · ST11 序型/Aleph。
- **复现**：`docs/gaps/repro/G56-acc-well-founded-recursion.sh`。

### G-58 · 大消去不可用（`Prop` 的归纳类型消去不到 `Type`）

- **今天**：`def andToType (A B : Prop) (h : A ∧ B) : Type :=
  And.rec A B (fun (_ : A ∧ B) => Type) A B h` ⇒ `kernel-rejected` /
  「类型不匹配：期望 `Pi (x : ((And.[] $2) $1)), Sort(0)`，实际是 `Pi (_ : ((And.[] $2) $1)), Sort(2)`」。
  **对照组**（消去到 `Prop`）**checked** ✓。
- **机制**：`mk_elim_level` 问 `large_elim_test`；false ⇒ `st.elim_level = Some(zero())`
  ⇒ `mk_motive_dep` 把 motive 钉在 `major → Sort(0)`。对 `Prop` 值归纳块走
  `large_elim_test_aux`（要求「每个非 Prop 的构造子字段都是参数或指标」）—— `And`
  （字段全是 Prop）**应当**通过，但实测**没通过**。**根因未定位**（时间盒到了）。
- **堵住**：**任何「`Prop` 入、`Type` 出」的定义** ⇒ ST7 的 `rank` **即使 G-56 修好也过不去**
  （`Acc` 在 `Prop`、`Ordinal` 在 `Type`）；ST9/ST11 同。
- **复现**：`docs/gaps/repro/G56-large-elim-into-type.sokonanoda`。

### G-59 · `Type` 值归纳块的 recursor 也消去不到 `Type`（默认与显式都不行）

- **今天**：`inductive MyBox : Type` + `ctor MyBox.mk (n : Nat) : MyBox` ⇒
  `#check MyBox.rec` 给 `motive : MyBox -> Prop`（**默认是 Prop** ✗）；
  `#check MyBox.rec.{1}` 给 `motive : MyBox -> Type 0` ✓（**这一半对**）——
  但 `def boxElim (h : MyBox) : Type := MyBox.rec.{1} (fun _ => Type) …` **仍被拒**
  （「期望 `Pi (x : MyBox.[]), Sort(1)`，实际是 `Pi (_ : MyBox.[]), Sort(2)`」）⇒ **没有绕法**。
- **内核探针实测**（临时 `eprintln`，**已还原**）：该块 `large_elim_test` 返回 **true**、
  `is_nonzero=Some(true)`、`mk_elim_level: LARGE` ⇒ **内核自己认为该大消去**，但默认 motive
  仍是 `Prop` ⇒ 疑似**前端派生递归子的宇宙参数默认值**与内核期望不一致。**根因未定位**。
- **复现**：`docs/gaps/repro/G56-type-valued-large-elim.sokonanoda`；防漂移判据
  `crates/front/src/compile/tests.rs::g58_g59_large_elimination_is_unavailable`。

## 二、四条 painful（有绕法，但都让写法变丑）

### G-60 · 集合建构式 `{x ∈ A | P x}` / `{x : α | P x}` 写不出来

- **今天**：`{x | P x}` ⇒ `set-literal-shape`（「集合字面量要写成 {a} 或 {a, b}」）；
  `{x : α | x ∈ A}` ⇒ `unexpected-token` /「expected `)` to close binder group, found Pipe」。
- **根因**：记法命令只有六种形状（`notation`/`infix*`/`prefix`/`postfix`/`binder_notation`/`scoped`），
  **没有「操作数在括号里」这种形状**；`binder_notation` 的符号还必须是**数学符号**
  （普通标识符被拒）；`{` 在 `parse_atom` 里被判成 binder 组。
- **边界**：本语言**不给元变量**（第一刀 N4.2）⇒ `{x | P x}` 的 `α` 没有来源；
  `{x : α | P x}` 的 `α` 有来源但花括号没有落点。
- **绕法**：点名写 `Set.sep α A P` ✓（`Set.mem_sep_iff` 让证明仍然自然）。
- **复现**：`docs/gaps/repro/G60-set-builder-notation.sh`。

### G-61 · 没有 η ⇒ Lean core 的 `Quotient`/`Setoid` 做不出来（`Quot` 本体可用 ✓）

- **今天**：`Box.get α (Box.mk α a) = a` **checked** ✓（iota 正常），但
  `def eta_test (b : Box α) : Box.get α b = b := Eq.refl …` ⇒ `kernel-rejected` /
  「期望 `$1`，实际是 `(Box.[] $1)`」。
- **影响**：`Setoid.mk s.r s.iseqv = s` 不成立 ⇒ `Quotient s := Quot s.r` 定义不出来。
- **绕法**：**直接用 `Quot`** ✓（ST10 已实测：`Cardinal` 7 条全 checked）。
- **复现**：`docs/gaps/repro/G61-no-eta.sh`。

### G-62 · def 形态与展开形态不同一 ⇒ 三条等价律写不出来

- **今天**：`Type.Equiv.refl`/`.symm`/`.trans` 三种写法全判红 —— ① 直接 `exact Exists.intro …`
  （展开形态）⇒「期望 `Type.Equiv α α`，实际是 `Exists …`」；② `exact Iff.mpr (Type.Equiv.iff …)`
  ⇒「期望 `Type.Equiv α α`，实际是 `Type.Equiv α α`」（**同字面却判不等**）；③ `by exact` ⇒ 同 ②。
- **绕法**：`Cardinal` 核心**不需要**这三条律（`Quot` 只要求关系**成立**）✓。
- **复现**：`docs/gaps/repro/G62-equiv-laws.sh`。

### G-63 · `Quot.lift` 的显式宇宙实参对不上内核签名

- **今天**：`Quot.lift.{1, 2}` **能**消去进 `Type` ✓（`A = α : Type 0`、`B = Type`）；
  `Quot.lift.{1, 1}` 对 `B = Nat` ✓；但 `#check Quot.lift.{1, 0}` 渲染成 `{B : Prop}`，
  照它写就报「期望 `Sort(0)`，实际是 `Sort(1)`」✗。**同一目标在不同上下文里一个过一个不过**。
- ⚠ **实测推翻了初判**：「`Quot` 消去疑似只进 Prop」**是错的** —— `Quot.lift.{1, 2}`
  **能**进 `Type` ✓ ⇒ 是**宇宙实参难对准**，**不是 kernel 缺能力** ✓。
- **绕法**：照抄已判过的库写法（`Cardinal.sound` / `Cardinal.lift` 是本版验证过的模板 ✓）。
- **复现**：`docs/gaps/repro/G63-quot-lift-universes.sh`。

## 三、卡住未做的四章（全部因 G-56 / G-58）

| 章节 | 内容 | 被谁挡住 | 本版状态 |
|---|---|---|---|
| **ST6** | 传递闭包（递归定义） | G-56 | 未做（台账已登记） |
| **ST7** | 秩 `rank` | **G-56 + G-58**（两条都要） | 未做 |
| **ST9** | 超限递归 / `V` 层级 | **G-56 + G-58** | 未做 |
| **ST11** | 序型 / Aleph / ω₁ / Cantor 正规形 | **G-56 + G-58** | 未做 |

用户 2026-09-28：「**卡住的四项放到最后**……等上面都收口之后，再回头评估要不要动内核修 `Acc`。
那条路成本最高（要动一致性相关的检查、走四道保险），到时候我会再确认一次，**你别自己开工**」✓。

## 四、本版实际完成（对照）

**已完成 9 章**：ST1（分界决策）· ST2（`Quot` 进源语言）· ST3（分离）· ST4（集族并交）·
ST5（不交并/函数空间）· ST8（序数谓词式）· ST10（基数 = 类型的商）· ST12（选择公理）·
ST13（ZF 公理表）· ST14（`propext`/`funext`）。

**课程门禁**：**43 个目标 · 375 checked · 99 open · 0 个被判负** ✓。

**改内核判定**：**零**（`git diff` 对 `crates/kernel/` 为空）✓ —— 用户 2026-09-28：
「**不许为了让它 checked 通过去改内核判定**（真要改也得先留判红证据、单独 commit、
三层回归 + 语料对拍 + `--json` 逐字节不变）」✓。

## 五、若要修，动手顺序建议（给后来者）

1. **G-58 先修**（收益最大）：它一条堵住 ST7/ST9/ST11 三章，且**不涉及一致性风险**
   （只放宽"哪些归纳块允许大消去"，不放宽"接受哪些项"）；改前先读
   `docs/architecture.md` §6 的内核改动台账与 §8 gotchas ✓。
2. **G-59 与 G-58 同批**（都是 recursor motive 的宇宙）—— 它有**内核探针实测**在手
   （`large_elim_test` 返回 true 但默认 motive 是 Prop），是最有线索的一条 ✓。
3. **G-56 最后**（成本最高）：要动 `check_uniform_inductive_occurrences_at` 的断言，
   且**必须配反向判据**（坏类型仍被拒，证明没放宽过头）—— 用户 2026-09-28 明确
   「**如果哪天你认为 `Acc` 那条路能打通，先来问我**」✓。
4. **G-60…G-63 是前端/语法侧**，与内核判定无关，可独立做 ✓。
