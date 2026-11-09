## M1 单元⑰ 整数乘法的交换律（代表元层）（2026-11-09 · issue #17 / PR #24）
- `unit17-int-mul-comm`：`zero_mul` · `succ_mul`（对 b 归纳，`Eq.subst` 把 ih 换进 `add (a+1)` 的第二位）· `myMul_comm`（`congrArg` 改写 + `add_comm` + `succ_mul` 的对称）· `mulRep_comm`（逐分量 `congrArg` 装回序对）；画布 16 checked / 3 open，解答 19 checked / 0 open.
- 判据：课程门禁 **45 目标 · 706 checked · 84 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：这一步补上「第二个因子换代表元」的前提（`mulRep_comm` + `intRel` 对称）；`Int` 乘法的结合/分配与 ℚ 构造仍未做.
## M1 单元⑯ 整数序的传递性与加法单调（2026-11-09 · issue #17 / PR #24）
- `unit16-int-order-axioms`：`add4`（四项交换）· `leRep_trans`（两条假设各加一个见证数 → `add_rotate` 对齐 → `leNat_trans` → `leNat_add_cancel_right` 消去）· `leRep_add_right`（平移不变）；画布 23 checked / 2 open，解答 25 checked / 0 open.
- 判据：课程门禁 **43 目标 · 671 checked · 81 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：序公理仍在代表元层（无 `funext`/`propext`，序不下降成 `Int → Int → Prop`）；M1 还差 2 单元（ℚ 构造）.
## M1 单元⑮ 整数序的良定义性（2026-11-09 · issue #17 / PR #24）
- `unit15-leRep-wd`：`intRel_to_eq`（假设位不自动展开 `intRel`，先 `Eq.subst` 取等式）· `intRel_symm` · `leRep_wd_fst` / `leRep_wd_snd`（同加见证数 → `add_rotate` 搬运 → `leNat_add_cancel_right` 消去）· `leRep_wd` · `leRep_wd_iff`；画布 23 checked / 5 open，解答 28 checked / 0 open.
- 判据：课程门禁 **41 目标 · 623 checked · 79 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`，两组 `--only` 实测 51 checked / 5 open）；`notation-lint` 绿.
- 边界：`leRep_wd_iff` 是 `Quot.lift` 所需的良定义性；整数序公理（全序/加法单调）与 ℚ 构造仍差 3 单元.
