## M1 单元⑬ ℕ 序的传递性与单调性（2026-11-09 · issue #17 / PR #24）
- `unit13-nat-order-lemmas`：`leNat_trans` / `leNat_succ_succ` / `leNat_add_right`（三条都用显式 `LeNat.rec`——`match` 只代参数、不代指标）；画布 7 checked / 3 open，解答 10 checked / 0 open.
- 判据：课程门禁 **37 目标 · 539 checked · 69 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；notation-lint 332 文件零旧写法 · docs/status-lint 绿.
- 边界：本单元只证 ℕ 序的传递与右加单调；整数序良定义性仍差「加法消去」（`m+k ≤ n+k ⇒ m ≤ n`）与代表元搬运，留给单元⑭.
## M1 单元⑫ 整数序的代表元公式（2026-11-09 · issue #17 / PR #24）
- `unit12-int-order-reps`：课程自定义 `LeNat`（`le_refl` / `le_succ`）· `leRep p q := a+d ≤ c+b` · `leRep_mk` / `leRep_refl` / `leRep_to_leNat` / `leRep_of_leNat` / `leNat_zero_le` / `leRep_one_two`；画布 6 checked / 6 open，解答 13 checked / 0 open.
- 判据：课程门禁 **35 目标 · 522 checked · 66 open · 0 判负**（修复后的 0.98.4 构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 绿.
- 边界：本单元只落代表元公式（与单元④平行），序的良定义性与序公理留给单元⑬；II.1 仍需序收尾后转 ℚ.
## M1 单元⑪ 整数乘法的良定义性（2026-11-09 · issue #17 / PR #24）
- `unit11-int-mul-wd`：`mul_add`（分配律 (a+b)·c = a·c + b·c）· `mul_shift_c`（等价假设乘开）· `mulRep_wd`（p ~ p' ⇒ p*q ~ p'*q，第二个分量固定）；画布 15 checked / 3 open，解答 18 checked / 0 open.
- 判据：课程门禁 **33 目标 · 503 checked · 60 open · 0 判负**（修复后的 0.98.4 构建）；docs-lint / status-lint 绿.
- 边界：II.1 仍未覆盖整数序；本单元 `myMul` 递归步写 prelude `Nat.add`（与单元⑤ `myAdd` 逐点相等、非定义性相等），理由见单元头注释.
## 更早轮次（2026-11-05 · `apply` 对齐 · 扩展升级 rebuild）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`（`apply` 修复线 `d9a701e4`；rebuild 提示 = issue #9）.
## 更早轮次（`·` 块内 goal state 2026-11-04 · 课程目录迁移 2026-11-04 · metavar 2026-10-26 · LKA 2026-10-24）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9；课程目录迁移 = issue #3（`course/` → `courses/quick-start/`）.

## 更早轮次（原文已删 ⇒ `git log --all -- STATUS.md`）：v0.97.4–v0.97.8 已发版（资产核对 = `gh release list`）· v0.97.6 前 `apply Eq.subst h hx` 判绿 ✓
## 未决项：**权威 = `docs/ONBOARDING.md` §0.2**（本节不自报条数；2026-10-13 订正过"6 条"与实测不符 ✗）
## 已发布线：`v0.79.0`…`v0.83.0` ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz`；**"最新是哪版/多少没发"不许写这里**（死格，2026-10-16 实测落后 15 个 release ✗）⇒ 活读数 `python3 scripts/release-metric.py` ✓
## 硬事实（接手先读）⇒ 原文已删（`docs-lint` ⑦ 顶格，按本文件规矩腾空间）· `git log --all -- STATUS.md` ✓
