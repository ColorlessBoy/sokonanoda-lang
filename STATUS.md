## M1 单元⑱ 整数乘法的第二个因子良定义（代表元层）（2026-11-09 · issue #17 / PR #24）
- `unit18-int-mul-wd-snd`：`intRel_symm`（`Eq.symm`；`intRel` 展开后等式方向要看清）· `mulRep_wd_snd`（`mulRep_comm` 把第二个因子换到前面 → 单元⑪ 的 `mulRep_wd` → 两次 `Eq.subst` 搬回）；画布 18 checked / 8 open，解答 26 checked / 0 open.
- 判据：课程门禁 **47 目标 · 750 checked · 92 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：`Int` 乘法两条 `Quot.lift` 义务（两个因子各一条）在代表元层已齐，但尚未真正下降成 `Int → Int → Int`；ℚ 构造仍未开工.
## M1 单元⑰ 整数乘法的交换律（代表元层）（2026-11-09 · issue #17 / PR #24）
- `unit17-int-mul-comm`：`zero_mul` · `succ_mul`（对 b 归纳，`Eq.subst` 把 ih 换进 `add (a+1)` 的第二位）· `myMul_comm`（`congrArg` 改写 + `add_comm` + `succ_mul` 的对称）· `mulRep_comm`（逐分量 `congrArg` 装回序对）；画布 16 checked / 3 open，解答 19 checked / 0 open.
- 判据：课程门禁 **45 目标 · 706 checked · 84 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：这一步补上「第二个因子换代表元」的前提（`mulRep_comm` + `intRel` 对称）；`Int` 乘法的结合/分配与 ℚ 构造仍未做.
## M1 单元⑯ 整数序的传递性与加法单调（2026-11-09 · issue #17 / PR #24）
- `unit16-int-order-axioms`：`add4`（四项交换）· `leRep_trans`（两条假设各加一个见证数 → `add_rotate` 对齐 → `leNat_trans` → `leNat_add_cancel_right` 消去）· `leRep_add_right`（平移不变）；画布 23 checked / 2 open，解答 25 checked / 0 open.
- 判据：课程门禁 **43 目标 · 671 checked · 81 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：序公理仍在代表元层（无 `funext`/`propext`，序不下降成 `Int → Int → Prop`）；M1 还差 2 单元（ℚ 构造）.
