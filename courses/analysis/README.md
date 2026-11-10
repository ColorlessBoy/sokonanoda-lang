# 卷 II/III《从集合论到实分析》

> 从卷 I《集合论》（`courses/set-theory/`）继续，目标 Tao《Analysis I》第 4–11 章：
> ℤ / ℚ / ℝ 的构造 → 序列与级数 → 连续与微分 → Riemann 积分。

## 状态

- **C0（M0）完成并 merge 进 `agent/real-analysis-integration`**：P1–P5 可行性探针 + `ROUTE.md`。
- **M1 单元写作完成（28/28）**：ℤ + ℚ，**28 单元**（II.1 19 + II.2 9）；当前 **II.1 19/19 · II.2 9/9**。已落 `course.json`、`lib/`（10 个模块 + Demo）、单元①–㉘（画布 + 解答）。**M1 仍未收口**：`notation-lint --root courses/analysis` 仍 rc=1（issue #29），审计遗留项也另跟踪；在两者清零前不宣称 M1 完成。
- **单元⑲ `unit19-quot-lift`**：第一次真正下降 —— 取负经 `Quot.lift` 成为 `Int → Int` 的函数（`intNeg`），并说明**二元**运算为何仍留在代表元层（`Int → Int` 的 lift 需要 `funext`，本语言没有）。
- **单元⑳ `unit20-rat-quotient`**：ℚ 的载体 `Rat := Quot ratRel`，分母用 `Nat.succ n` 编码成正整数（分母永不为 0），`ratRel` 在**整数代表元**上交叉相乘；本单元结清自反与对称两条腿，**传递性**（交叉相乘后消去正因子）留给单元㉕。
- **单元㉑ `unit21-int-rel-trans`**：`intRel` 的传递性（`intRel p q ⇒ intRel q r ⇒ intRel p r`）——两条假设各加一个数把中间项对齐，`add_rotate` 把同一 `d` 转到最右，`nat_add_cancel_right` 消掉。这是 `ratRel_trans` 依赖链的**第一块**，也是唯一能独立成单元的一块。
- **单元㉒ `unit22-nat-mul-laws`**：ℕ 乘法的**左/右分配律**（`mul_add` / `mul_add_right`）与**结合律**（`myMul_assoc`）—— ℤ 乘法结合律的引擎；证明全部停在 ℕ 上（对 `c` 归纳 + `add_mid` 搬运 + 交换律），不碰商、不碰 `funext`、不加公理。画布 20 checked / 3 open，解答 23 checked / 0 open。
- **单元㉓ `unit23-int-mul-assoc`**：ℤ 乘法在**代表元层**的结合律 `mulRep (mulRep p q) r ~ mulRep p (mulRep q r)`。四步：右分配律展开 → ℕ 结合律规范形 → `add_shift` 换配对 → 左分配律收回；两个分量各走一遍，最后用 `add_congr` 拼成 `intRel`。仍停在代表元层，不碰 `funext`、不加公理（#28）。画布 38 checked / 3 open，解答 41 checked / 0 open。
- **`ratRel_trans` 的剩余前置（2026-11-09 实测的依赖链，别再按「一步」估）**：`ratRel` 传递性要消去同一个正分母，链上是 ① `intRel` 传递（纯 ℕ 加法旋转 + 消去，可独立成单元）→ ② **ℤ 乘法结合律**（II.1 只做了交换/单位/良定义，**没有结合律**）→ ③ ℕ 正因子消去 `a·(n+1)=b·(n+1) ⇒ a=b`（要 ② 与 ≤ 反对称）。⇒ ① 已在单元㉑、② 已在单元㉓、③ 已在单元㉔ 结清；㉕ 已把 `ratRel_trans` 本身做完。
- **单元㉔ `unit24-nat-mul-cancel`**：ℕ 的**正因子消去律** `a·(n+1)=b·(n+1) ⇒ a=b`。对 `a` 归纳：零情形用「非零因子乘出 0 ⇒ 因子为 0」（`mul_eq_zero_right`），后继情形把 `(a+1)·d` 展开成 `a·d + d` 后用单元⑭的加法消去律剥掉。证明全部停在 ℕ 上，不碰商、不碰 `funext`、不加公理（#28）。画布 4 个练习，解答 0 open。
- **单元㉕ `unit25-rat-rel-trans`**：把 ㉑–㉔ 三块地基拼起来，结清 `ratRel` 的**传递性**（`mulRep_den` 规范形 → `mulRep_den_cancel` 消去正分母 → `mulRep_den_swap` 对齐 → 五步链）。全部停在 ℤ 代表元层，不碰商、不碰 `funext`、不加公理（#28）。画布 68 checked / 6 open，解答 74 checked / 0 open。
- **单元㉖ `unit26-rat-add`**：ℚ **加法**的代表元公式 `(a-b)/(m+1) + (c-d)/(n+1) = [(a(n+1)+c(m+1)) - (b(n+1)+d(m+1))] / ((m+1)(n+1))`，分母字段取 `m*n + m + n`（㉖.1 证 `succ (m*n+m+n) = (m+1)(n+1)`）。两条良定义义务（第一、第二因子）都在代表元层：`ratAdd_cross` 消去同一个正分母，`ratAdd_num_wd`/`_snd` 做分子搬运，`mul_shift_c` 得到交叉和。**不下降成 `Rat → Rat → Rat`**（需要 `funext`，issue #28），不加公理。画布 54 checked / 6 open，解答 60 checked / 0 open。
- **单元㉗ `unit27-rat-mul`**：ℚ **乘法**的代表元公式 `(a-b)/(m+1) * (c-d)/(n+1) = [(ac+bd) - (ad+bc)] / ((m+1)(n+1))`，分子就是整数代表元乘法 `mulRep`，分母字段仍是 `ratDenRep m n`；给出具体数字 `1/2*1/3=1/6`、`1/2*2/3=1/3`、`2/3*3/4=1/2`。**本单元只到公式与计算**：乘法的两条 `Quot.lift` 良定义义务（需要整数层 `mulRep_wd` + 分母 `(m+1)(n+1)` 的重排）留在 ㉘ 与序/嵌入一起收口，不假装已经下降成 `Rat → Rat → Rat`（无 `funext`，issue #28）。画布 74 checked / 5 open，解答 79 checked / 0 open。
- **单元㉘ `unit28-rat-mul-wd`**：ℚ **乘法**的良定义性，两条义务 `ratMulRep_wd`（x~x' ⇒ x*y~x'*y）与 `ratMulRep_wd_snd`（y~y' ⇒ x*y~x*y'）在代表元层结清。先把同分母的 `ratRel` 化成整数代表元相等（`rat_cross`），再交给整数层已证的 `mulRep_wd` / `mulRep_wd_snd` 把相等搬到两个分子，最后乘回公分母。仍不下降成 `Rat → Rat → Rat`（无 `funext`，issue #28），不加公理。画布 80 checked / 2 open，解答 82 checked / 0 open。
- **M2 单元㉛–㉜（2026-11-10）**：**负元与减法**。㉛ 在代表元层定义 `ratNegRep`（交换分子序对），结清交叉乘积形状、良定义性与对合 `-(-x) ~ x`；㉜ 用 `ratSubRep := ratAddRep x (ratNegRep y)` 定义减法，结清计算规则、两条良定义义务与 `1 - 1/2 = 1/2`。两单元画布各 4 open / 0 rejected，解答 0 open。
- **M2 单元㉚（2026-11-10）**：ℚ 的**严格序** `ratLtRep x y := ratLeRep x y ∧ ¬ ratRel x y`。结清 `<` 不⊂自反（`ratLtRep_irrefl`）、`< ⇒ ≤`、`≤ + ≠ ⇒ <`，以及 `0 < 1`（序用 `le_succ`，不等把 `ratRel 0 1` 化成 `0 = 1` 后交给 `succ_ne_zero`）。画布 4 open / 0 rejected，解答 0 open。
- **M2 开工（单元㉙，2026-11-10）**：ℚ 的**序**。`ratLeRep` 用交叉相乘后的 ℕ 不等式定义：`a(n+1)+d(m+1) ≤ c(m+1)+b(n+1)`；本单元结清自反性与「相等 ⇒ 序」(`ratLeRep_of_ratRel`)，并给出 0≤1、1/2≤3/4 两个具体计算。传递性、全序与加法/乘法相容留给后续单元。同时把 M1 已判卷的 ℤ/ℚ 算术内核收进 `lib/Rat`（定义 + 已教法律），M2 单元不再重复抄 1000 行前序证明。画布 4 open / 0 rejected，解答 0 open。
- **`lib/Extensionality`（#28 A0，2026-11-10）**：`funext` 从 `Quot.sound` 证成**定理**（零新增信任），`propext` 为显式 axiom（卷 I 先例）；不动 kernel/front/prelude。M2 起 `Int.add`/`Int.le` 可下降成函数。
- **课程门禁**：`tools/check.py` 已从卷 I 移植，用 0.98.4 构建跑通；C3 收口前必须 0 判负。
- 提案与追踪：issue #13；C0 子 issue #14；C0 PR #23。

## 入口

- `ROUTE.md`：路线、探针结论、前置、里程碑、风险。
- `probes/`：M0 可行性探针（P1–P5），不进课程门禁。
- `lib/`：M1 从卷 I 移植的 L2 库（`Logic` / `Exists` / `Set` / `Prod` / `Rel` / `Fun` / `Image` / `Equiv` / `Order`）+ `Extensionality`（#28 A0：`funext` 定理 + `propext` axiom）。
- `units/II.1/`：M1 的整数单元（序对商、代表元公式、加法良定义与律、零元与负元、减法良定义与基本律、乘法良定义、序的代表元公式、ℕ 序的传递与单调、ℕ 序的加法消去律、整数序的良定义性、整数序的传递性与加法单调、整数乘法的交换律、乘法第二个因子良定义、取负下降成函数）；解答在 `units/solutions/II.1/`。
- `units/II.3/`：M2 的有理数性质（当前 4 个：序的代表元公式与自反性；严格序与不相等；负元；减法）；解答在 `units/solutions/II.3/`。
- `units/II.2/`：M1 的有理数单元（9 个：ℚ 的商载体与相等的自反/对称；整数相等的传递性；ℕ 乘法分配律与结合律；ℤ 乘法结合律；ℕ 正因子消去律；`ratRel` 的传递性；ℚ 加法的代表元公式与良定义性；ℚ 乘法的代表元公式与具体计算；ℚ 乘法的良定义性）；解答在 `units/solutions/II.2/`。
- `course.json`：M1 课程清单（当前只列已落地的单元，随写作增长）。

## 边界

- 本目录是独立课程根（`sokonanoda.toml`），不复用 `courses/set-theory/` 的 `lib/`；
  M1 时把需要的库模块**移植**过来（不是跨根 import）。
- C0 探针用 axiom 隔离算术证明，只验证类型/写法骨架；完整 ℤ/ℚ/ℝ 证明在 M1–M2。
