## M1 单元⑳ ℚ 的商载体与相等关系（2026-11-09 · issue #17 / PR #24）
- `unit20-rat-quotient`：`ratRep := (ℕ×ℕ 的整数代表元) × ℕ`（分母 = `Nat.succ n`，永不为 0）· `ratRel` 交叉相乘 `a·(n+1) = c·(m+1)`（仍停在代表元层，避免 `funext`，issue #28）· `Rat := Quot ratRel` · `ratOf`；练习 = `intRel_refl` · `ratRel_refl` · `ratRel_symm` · `rat_sound`。画布 13 checked / 4 open，解答 17 checked / 0 open.
- 判据：课程门禁 **51 目标 · 810 checked · 98 open · 0 判负**（0.98.4 仓库构建 `target/debug`；单卡 20 → 30 秒级）；`notation-lint` / `docs-lint` 绿.
- 边界：传递性（交叉相乘后消去正因子）留给单元㉑；M1 = **19/28**（II.1 19/19 · II.2 1/9），M1 未完成。二元运算仍停在代表元层（#28）.
## M1 单元⑲ 把取负下降成函数（`Quot.lift`）（2026-11-09 · issue #17 / PR #24）
- `unit19-quot-lift`：`negRep_wd`（交叉和两边各交换一次 + `Eq.symm`）· `intNeg`（`Quot.lift` 一元桥；义务 = `negRep_wd` + `Quot.sound`）· `intNeg_mk`（ι 归约 ⇒ `Eq.refl`）· `intNeg_invol`（`Quot.ind` + 两次 ι + `Quot.sound`）；画布 13 checked / 2 open，解答 15 checked / 0 open.
- 判据：课程门禁 **49 目标 · 780 checked · 94 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：这是 `Int` 上**第一个真正的函数**；二元运算（加法/乘法）仍停在代表元层 —— `Int → Int` 的 `Quot.lift` 要证函数相等，需 `funext`（本语言没有）。ℚ 仍需整数乘法的结合/分配与消去律.
## M1 单元⑱ 整数乘法的第二个因子良定义（代表元层）（2026-11-09 · issue #17 / PR #24）
- `unit18-int-mul-wd-snd`：`intRel_symm`（`Eq.symm`；`intRel` 展开后等式方向要看清）· `mulRep_wd_snd`（`mulRep_comm` 把第二个因子换到前面 → 单元⑪ 的 `mulRep_wd` → 两次 `Eq.subst` 搬回）；画布 18 checked / 8 open，解答 26 checked / 0 open.
- 判据：课程门禁 **47 目标 · 750 checked · 92 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：`Int` 乘法两条 `Quot.lift` 义务（两个因子各一条）在代表元层已齐，但尚未真正下降成 `Int → Int → Int`；ℚ 构造仍未开工.
