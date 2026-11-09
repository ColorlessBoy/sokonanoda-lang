## M1 单元⑮ 整数序的良定义性（2026-11-09 · issue #17 / PR #24）
- `unit15-leRep-wd`：`intRel_to_eq`（假设位不自动展开 `intRel`，先 `Eq.subst` 取等式）· `intRel_symm` · `leRep_wd_fst` / `leRep_wd_snd`（同加见证数 → `add_rotate` 搬运 → `leNat_add_cancel_right` 消去）· `leRep_wd` · `leRep_wd_iff`；画布 23 checked / 5 open，解答 28 checked / 0 open.
- 判据：课程门禁 **41 目标 · 623 checked · 79 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`，两组 `--only` 实测 51 checked / 5 open）；`notation-lint` 绿.
- 边界：`leRep_wd_iff` 是 `Quot.lift` 所需的良定义性；整数序公理（全序/加法单调）与 ℚ 构造仍差 3 单元.
## M1 单元⑭ ℕ 序的加法消去律（2026-11-09 · issue #17 / PR #24）
- `unit14-leNat-cancel`：`nat_add_cancel_right`（等式消去，靠 `Nat.succ.inj`）· `leNat_exists_add` / `leNat_of_exists_add`（把序刻画成 `∃ k, n = m + k`）· `leNat_self_add` · `leNat_add_cancel_right`（序的消去律）；画布 14 checked / 5 open，解答 19 checked / 0 open.
- 判据：课程门禁 **39 目标 · 572 checked · 74 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；`notation-lint` 332 文件零旧写法 · status-lint 绿.
- 边界：`leNat_add_cancel_right` 是整数序良定义性的最后一块 ℕ 垫脚石；`leRep_wd` 与序公理留给单元⑮，之后转 ℚ.
## M1 单元⑬ ℕ 序的传递性与单调性（2026-11-09 · issue #17 / PR #24）
- `unit13-nat-order-lemmas`：`leNat_trans` / `leNat_succ_succ` / `leNat_add_right`（三条都用显式 `LeNat.rec`——`match` 只代参数、不代指标）；画布 7 checked / 3 open，解答 10 checked / 0 open.
- 判据：课程门禁 **37 目标 · 539 checked · 69 open · 0 判负**（0.98.4 修复构建，`--bin /tmp/soko-kernel/target/debug/sokonanoda`）；notation-lint 332 文件零旧写法 · docs/status-lint 绿.
- 边界：本单元只证 ℕ 序的传递与右加单调；整数序良定义性仍差「加法消去」（`m+k ≤ n+k ⇒ m ≤ n`）与代表元搬运，留给单元⑭.
## 更早轮次（2026-11-05 · `apply` 对齐 · 扩展升级 rebuild）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`（`apply` 修复线 `d9a701e4`；rebuild 提示 = issue #9）.
## 更早轮次（`·` 块内 goal state 2026-11-04 · 课程目录迁移 2026-11-04 · metavar 2026-10-26 · LKA 2026-10-24）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9；课程目录迁移 = issue #3（`course/` → `courses/quick-start/`）.

## 更早轮次（原文已删 ⇒ `git log --all -- STATUS.md`）：v0.97.4–v0.97.8 已发版（资产核对 = `gh release list`）· v0.97.6 前 `apply Eq.subst h hx` 判绿 ✓
## 未决项：**权威 = `docs/ONBOARDING.md` §0.2**（本节不自报条数；2026-10-13 订正过"6 条"与实测不符 ✗）
## 已发布线：`v0.79.0`…`v0.83.0` ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz`；**"最新是哪版/多少没发"不许写这里**（死格，2026-10-16 实测落后 15 个 release ✗）⇒ 活读数 `python3 scripts/release-metric.py` ✓
## 硬事实（接手先读）⇒ 原文已删（`docs-lint` ⑦ 顶格，按本文件规矩腾空间）· `git log --all -- STATUS.md` ✓
