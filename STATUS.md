## M1 单元㉘ ℚ 乘法的良定义性（2026-11-10 · issue #17 / PR #24）
- `unit28-rat-mul-wd`：乘法两条代表元层义务 `ratMulRep_wd`（x~x' ⇒ x*y~x'*y）与 `ratMulRep_wd_snd`（y~y' ⇒ x*y~x*y'）。`rat_cross` 先把同分母 `ratRel` 化成 `a+b'=a'+b`，整数层 `mulRep_wd` / `mulRep_wd_snd` 把相等搬到两个分子，最后乘回公分母。画布 **80 checked / 2 open**，解答 **82 checked / 0 open**.
- 判据：课程门禁 **67 目标 · 1580 checked · 132 open · 0 判负**（0.98.4 仓库构建）；M1 单元 **28/28**（II.1 **19/19** · II.2 **9/9**）。
- 未收口：`notation-lint --root courses/analysis` 仍 **rc=1**（519 处，issue #29）；在 notation 清零与审计阻塞项修完前不宣称 M1 完成。仍不下降成 `Rat → Rat → Rat`（无 `funext`，issue #28），不加公理.
## M1 单元㉗ ℚ 乘法的代表元公式与具体计算（2026-11-10 · issue #17 / PR #24）
- `unit27-rat-mul`：`ratMulRep` 的分子是整数代表元乘法 `mulRep`（`(ac+bd, ad+bc)`），分母字段仍取 `ratDenRep m n`；`ratMulRep_mk` / `ratMul_den` / `ratMul_num_comm` + 三个具体计算（`1/2*1/3=1/6`、`1/2*2/3=1/3`、`2/3*3/4=1/2`）。画布 **74 checked / 5 open**，解答 **79 checked / 0 open**.
- 判据：课程门禁 **65 目标 · 1418 checked · 130 open · 0 判负**（0.98.4 仓库构建）；`notation-lint` / `docs-lint` / `status-lint` 绿.
- 边界：本单元**只到公式与计算**；乘法的两条 `Quot.lift` 良定义义务留下（同 #28：无 `funext`，不下降成 `Rat → Rat → Rat`，不加公理）。M1 = **27/28**（II.1 **19/19** · II.2 **8/9**），M1 未完成.
## M1 单元㉖ ℚ 加法的代表元公式与良定义性（2026-11-10 · issue #17 / PR #24）
- `unit26-rat-add`：`ratDenRep m n = m*n + m + n`（㉖.1 证 `succ (m*n+m+n) = (m+1)(n+1)`）· `ratAddRep`（交叉乘加、分母字段取 `ratDenRep`）· 两条良定义义务：`ratAdd_cross`（消去同一正分母）→ `ratAdd_num_wd` / `_snd`（分子四步搬运）→ `mul_shift_c` → `mulRep_den` + `intRel_trans` 收口；画布 **54 checked / 6 open**，解答 **60 checked / 0 open**.
- 判据：课程门禁 **63 目标 · 1265 checked · 125 open · 0 判负**（0.98.4 仓库构建）；`notation-lint` / `docs-lint` 绿.
- 边界：加法仍停在**代表元层**（`Rat → Rat → Rat` 需要 `funext`，issue #28），不加公理。M1 = **26/28**（II.1 **19/19** · II.2 **7/9**），M1 未完成.
