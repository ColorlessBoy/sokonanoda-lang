## M1 单元㉖ ℚ 加法的代表元公式与良定义性（2026-11-10 · issue #17 / PR #24）
- `unit26-rat-add`：`ratDenRep m n = m*n + m + n`（㉖.1 证 `succ (m*n+m+n) = (m+1)(n+1)`）· `ratAddRep`（交叉乘加、分母字段取 `ratDenRep`）· 两条良定义义务：`ratAdd_cross`（消去同一正分母）→ `ratAdd_num_wd` / `_snd`（分子四步搬运）→ `mul_shift_c` → `mulRep_den` + `intRel_trans` 收口；画布 **54 checked / 6 open**，解答 **60 checked / 0 open**.
- 判据：课程门禁 **63 目标 · 1265 checked · 125 open · 0 判负**（0.98.4 仓库构建）；`notation-lint` / `docs-lint` 绿.
- 边界：加法仍停在**代表元层**（`Rat → Rat → Rat` 需要 `funext`，issue #28），不加公理。M1 = **26/28**（II.1 **19/19** · II.2 **7/9**），M1 未完成.
## M1 单元㉓ ℤ 乘法的结合律（2026-11-09 · issue #17 / PR #24）
- `unit23-int-mul-assoc`：代表元层结合律 `mulRep (mulRep p q) r ~ mulRep p (mulRep q r)`。四步链：`fst/snd_expand_left`（右分配）→ `fst/snd_normalize`（ℕ 结合律规范形）→ `fst/snd_shift`（`add_shift` 换配对）→ `fst/snd_expand_right` 对称收回；最后 `add_congr` 拼成 `intRel`。画布 **38 checked / 3 open**，解答 **41 checked / 0 open**.
- 判据：课程门禁 **57 目标 · 957 checked · 109 open · 0 判负**（0.98.4 仓库构建 `target/debug`）；`notation-lint` / `docs-lint` / `status-lint` 绿.
- 边界：依赖链 ② 结清；下一步 ③ ℕ 正因子消去，再到 `ratRel_trans`。M1 = **23/28**（II.1 **19/19** · II.2 **4/9**），M1 未完成；二元运算仍停在代表元层（#28）.
## M1 单元㉒ ℕ 乘法的分配律与结合律（2026-11-09 · issue #17 / PR #24）
- `unit22-nat-mul-laws`：`add_mid`（加法搬运）· `mul_add`（左分配，对 `c` 归纳 + `congrArg` + `add_mid`）· `mul_add_right`（右分配，交换律 + 左分配）· `myMul_assoc`（结合律，对 `c` 归纳，步例用 `mul_add`）+ 两条具体数字演示；画布 **18 checked / 3 open**，解答 **23 checked / 0 open**.
- 判据：课程门禁 **55 目标 · 878 checked · 106 open · 0 判负**（0.98.4 仓库构建 `target/debug`）；`notation-lint` / `docs-lint` / `status-lint` 绿.
- 边界：这是 `ratRel_trans` 依赖链 ② 的 **ℕ 引擎**；下一步把这三条搬到 `mulRep` 上（ℤ 乘法结合律），再到 ③ ℕ 正因子消去、`ratRel_trans`。M1 = **22/28**（II.1 **19/19** · II.2 **3/9**），M1 未完成；二元运算仍停在代表元层（#28）.

