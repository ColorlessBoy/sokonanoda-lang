## M1 单元⑲ 把取负下降成函数（`Quot.lift`）（2026-11-09 · issue #17 / PR #24）
- `unit19-quot-lift`：`negRep_wd`（交叉和两边各交换一次 + `Eq.symm`）· `intNeg`（`Quot.lift` 一元桥；义务 = `negRep_wd` + `Quot.sound`）· `intNeg_mk`（ι 归约 ⇒ `Eq.refl`）· `intNeg_invol`（`Quot.ind` + 两次 ι + `Quot.sound`）；画布 13 checked / 2 open，解答 15 checked / 0 open.
- 判据：课程门禁 **49 目标 · 780 checked · 94 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：这是 `Int` 上**第一个真正的函数**；二元运算（加法/乘法）仍停在代表元层 —— `Int → Int` 的 `Quot.lift` 要证函数相等，需 `funext`（本语言没有）。ℚ 仍需整数乘法的结合/分配与消去律.
## M1 单元⑱ 整数乘法的第二个因子良定义（代表元层）（2026-11-09 · issue #17 / PR #24）
- `unit18-int-mul-wd-snd`：`intRel_symm`（`Eq.symm`；`intRel` 展开后等式方向要看清）· `mulRep_wd_snd`（`mulRep_comm` 把第二个因子换到前面 → 单元⑪ 的 `mulRep_wd` → 两次 `Eq.subst` 搬回）；画布 18 checked / 8 open，解答 26 checked / 0 open.
- 判据：课程门禁 **47 目标 · 750 checked · 92 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：`Int` 乘法两条 `Quot.lift` 义务（两个因子各一条）在代表元层已齐，但尚未真正下降成 `Int → Int → Int`；ℚ 构造仍未开工.
## M1 单元⑰ 整数乘法的交换律（代表元层）（2026-11-09 · issue #17 / PR #24）
- `unit17-int-mul-comm`：`zero_mul` · `succ_mul`（对 b 归纳，`Eq.subst` 把 ih 换进 `add (a+1)` 的第二位）· `myMul_comm`（`congrArg` 改写 + `add_comm` + `succ_mul` 的对称）· `mulRep_comm`（逐分量 `congrArg` 装回序对）；画布 16 checked / 3 open，解答 19 checked / 0 open.
- 判据：课程门禁 **45 目标 · 706 checked · 84 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：这一步补上「第二个因子换代表元」的前提（`mulRep_comm` + `intRel` 对称）；`Int` 乘法的结合/分配与 ℚ 构造仍未做.
