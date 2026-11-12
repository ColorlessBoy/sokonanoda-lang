## M3 单元86：扩展实数 ±∞ 与序（2026-11-13 · issue #19 / PR #41）
- `lib/ExtReal`：载体 `ExtReal`（`negInf` / `fin` / `posInf`）、序 `ele`、嵌入 `ofReal`、判别式 `isNegInf` / `isPosInf`，以及三条定义性否定事实。`ele` 的两行拆成 `eleFin` / `eleTop`，因为 `def` 里不能嵌套 `match`。
- `unit86-ext-real`：`negInf_le`、`le_posInf`、`ele_fin_fin`、`ele_trans`（三种点组合，`ExtReal.rec.{0}` 把下一个点写进 motive）、`ele_antisymm`（有限支用 `realLe_antisymm` + `congrArg`）、`negInf_ne_fin` / `fin_ne_posInf`（判别式 + `Eq.subst`）、`demo_fin_zero_refl`。画布 0 checked / 8 open / 0 rejected，解答 13 checked / 0 open。
- 判据：`check.py --json` rc=0（212 目标 · 2514 checked · 434 open · 0 判负，M3 14/51）；`notation-lint` rc=0（234 文件，15 处豁免）；III.2 画布与解答文案门禁 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：只做载体与序；自反只对带 Cauchy 见证的有限点成立（`demo_fin_zero_refl`），不写成对所有 `Real` 成立的公理。不新增公理、不碰 kernel/front。下一单元 = 扩展实数上的上确界。

## M3 单元84–85：上确界与下确界的唯一性与比较（2026-11-13 · issue #19 / PR #41）
- `unit84-sup-unique`：`lib/Order` 的 `UpperBound` / `IsSup` 固定到 `Real` 与 `realLe`；`upperBound_of_le`（上界放大）、`sup_le_iff`（`s ≤ M` ⟺「M 是上界」）、`sup_unique`（`realLe_antisymm`）、`sup_of_max`、`sup_singleton_mk`（`Eq.subst` + `realLe_refl_mk`）、`demo_sup_zero`。画布 0 checked / 6 open，解答 6 checked / 0 open。
- `unit85-inf-unique`：`LowerBound` / `IsInf` 同法翻转；`lowerBound_of_le`、`inf_le_iff`、`inf_unique`、`inf_of_min`、`inf_singleton_mk`、`inf_le_sup_of_nonempty`（非空见证 + 两次传递）、`demo_inf_zero`。画布 0 checked / 7 open，解答 7 checked / 0 open。新建 `lib/RealOrderLaws`（67 的序律进库）与 `lib/Sup`（实数实例的比较律）；`lib/Demo` 补两个 import。
- 判据：`check.py --json` rc=0（209 目标 · 2491 checked · 426 open · 0 判负，M3 13/51）；`notation-lint` rc=0（217 文件，15 处豁免）；两个画布与解答文案门禁 0 处；`style-lint --selftest` rc=0；`docs-lint` rc=0。
- 边界：只证唯一性与比较，不证存在性（要选择公理或 Dedekind 切割）；不新增公理、不碰 kernel/front。下一单元 = 扩展实数 ±∞ 的载体与序。

## M3 单元83：收敛 ⇒ Cauchy（2026-11-13 · issue #19 / PR #41）
- `unit83-converges-cauchy`：`close_pair_of_close_half`（两条 ε/2 接近性用 `close_symm` + `close_trans` 合成）；主定理 `converges_cauchy`（ε 折半，阈值同 N）；`converges_realRel_const`（三个分量：83.2、库里的 `cauchy_const`、收敛性本身）；`converges_real_eq_const`（`Real.sound`）；`demo_zero_cauchy`。画布 1 checked / 4 open，解答 5 checked / 0 open。
- 判据：`check.py --json` rc=0（203 目标 · 2466 checked · 413 open · 0 判负，M3 11/51）；`notation-lint` rc=0（211 文件，15 处豁免）；III.1 文案门禁 0 处；`style-lint --selftest` rc=0。
- 边界：只证收敛 ⇒ Cauchy；反方向在 ℚ 上不成立，要等 ℝ 的完备性。不新增公理、不碰 kernel/front。下一单元 = III.2 扩展实数与上确界。
