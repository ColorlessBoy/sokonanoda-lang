## M1 单元㉒ ℕ 乘法的分配律与结合律（2026-11-09 · issue #17 / PR #24）
- `unit22-nat-mul-laws`：`add_mid`（加法搬运）· `mul_add`（左分配，对 `c` 归纳 + `congrArg` + `add_mid`）· `mul_add_right`（右分配，交换律 + 左分配）· `myMul_assoc`（结合律，对 `c` 归纳，步例用 `mul_add`）+ 两条具体数字演示；画布 **18 checked / 3 open**，解答 **23 checked / 0 open**.
- 判据：课程门禁 **55 目标 · 878 checked · 106 open · 0 判负**（0.98.4 仓库构建 `target/debug`）；`notation-lint` / `docs-lint` / `status-lint` 绿.
- 边界：这是 `ratRel_trans` 依赖链 ② 的 **ℕ 引擎**；下一步把这三条搬到 `mulRep` 上（ℤ 乘法结合律），再到 ③ ℕ 正因子消去、`ratRel_trans`。M1 = **22/28**（II.1 **19/19** · II.2 **3/9**），M1 未完成；二元运算仍停在代表元层（#28）.
## M1 单元㉑ 整数相等的传递性（2026-11-09 · issue #17 / PR #24）
- `unit21-int-rel-trans`：`intRel_trans_mid`（两条假设各加一个数把中间项对齐，`add_rotate` 把 `(c+b)+f` 转成 `(c+f)+b`）· `intRel_trans`（`add_rotate` 把同一个 `d` 转到最右，`nat_add_cancel_right` 消掉）· `intRel_chain4`（传递性用两次）· 具体数字演示；画布 **10 checked / 5 open**，解答 **15 checked / 0 open**.
- 判据：课程门禁 **53 目标 · 835 checked · 103 open · 0 判负**（0.98.4 仓库构建；加单元前 51/810/98）；`notation-lint` / `docs-lint` / `status-lint` 绿.
- 边界：这是 `ratRel_trans` 依赖链的 ①（PR #24 已记）；仍差 ② ℤ 乘法结合律 · ③ ℕ 正因子消去，才轮到 `ratRel_trans`。M1 = **20/28**（II.1 **19/19** · II.2 **2/9**），M1 未完成；二元运算仍停在代表元层（#28）.
## M1 单元⑳ ℚ 的商载体与相等关系（2026-11-09 · issue #17 / PR #24）
- `unit20-rat-quotient`：`ratRep := (ℕ×ℕ 的整数代表元) × ℕ`（分母 = `Nat.succ n`，永不为 0）· `ratRel` 交叉相乘 `a·(n+1) = c·(m+1)`（仍停在代表元层，避免 `funext`，issue #28）· `Rat := Quot ratRel` · `ratOf`；练习 = `intRel_refl` · `ratRel_refl` · `ratRel_symm` · `rat_sound`。画布 13 checked / 4 open，解答 17 checked / 0 open.
- 判据：课程门禁 **51 目标 · 810 checked · 98 open · 0 判负**（0.98.4 仓库构建 `target/debug`；单卡 20 → 30 秒级）；`notation-lint` / `docs-lint` 绿.
- 边界：传递性（交叉相乘后消去正因子）留给单元㉑；M1 = **19/28**（II.1 19/19 · II.2 1/9），M1 未完成。二元运算仍停在代表元层（#28）.
