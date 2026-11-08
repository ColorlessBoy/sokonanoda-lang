## M1 单元⑨ 整数减法（2026-11-08 · issue #17 / PR #24）
- `unit09-int-sub`：`subRep p q = addRep p (negRep q)` 的良定义性（`subRep_wd`）· `x - x = 0`（`int_sub_self`）· `(x - y) + y = x`（`int_sub_add`）+ 3 条闭值演示；画布 7 open，解答 25 checked / 0 open.
- 判据：课程门禁 **29 目标 · 444 checked · 53 open · 0 判负**；`notation-lint` 332 文件 0 旧写法；docs/status-lint 绿.
- 边界：II.1 九单元齐（8→9 / 18）；乘法良定义与整数序仍未落盘，留给后续单元，不在本单元偷证.
## 修复 `apply` 目标不对齐（2026-11-05 · issue #10 · `Set.ext` 之后 `apply h1`）
- **根因**：`apply` 只做「把被应用项的 Π 层**全部**剥成子目标、拿结论对**整条**目标」这一档；目标本身是 `→`/`∀` 时对不上（Lean 4 的 `MVarId.apply` 从 `i = n - m` 起逐档试，`h1 x` 一个函数就关掉目标）。新增 `apply_partial.rs`（失败边），`apply h1` 在 `x ∈ A → x ∈ B` 上直接关掉；方向错仍报原诊断.
- **显示根因**：`Set.ext` 的陈述原来写 `A x ↔ B x`（没用 `Set.mem`）⇒ `apply Set.ext` 之后目标面板是 `A x ↔ B x`（`A`/`x` 无着色）。改成 `Set.mem x A ↔ Set.mem x B` ⇒ 面板 `x ∈ A ↔ x ∈ B`.
- 判据：front ×2（正向 + 方向错仍红）· CLI `tests/apply_pi_goal.rs` ×5（真课程库骨架 · 记法面板 · 不许吞错 · 无约束层仍是子目标 · 逃生门回退）· 课程门禁 258 目标 / 2191 checked / 0 判负 · front `--lib` 1198 passed.
## 扩展升级后建议 rebuild（2026-11-05 · issue #9）
- 升级后第一次在 sokonanoda **项目**里打开 `.sokonanoda` ⇒ 通知建议 rebuild，按钮 **确认 / 取消**；确认走 `rebuild --json <模块根>`（与 `alt+shift+b` 同一目标，来自 `soko/project`）；取消不重建、本会话也不再静默预热（否则 2026-10-11 的自动预热会让「取消」变谎话）。一次升级只问一次（`globalState` 基线）；首跑兜底看项目产物写者版本（上线那一版不会错过）。
- 判据：stub **82/82**（取消 ⇒ 零 spawn 且不补跑 · 确认 ⇒ 恰好一次 `rebuild --json <模块根>` · 单文件不问 · 降级不问 · 首跑兜底）· `cargo test -p sokonanoda-cli --test extension` 43/43 · `npm run test:unit` 全绿.
- 边界：只问**第一次**遇到的模块根；升级后没打开项目文件就不问（基线已记下）.
## 更早轮次（`·` 块内 goal state 2026-11-04 · 课程目录迁移 2026-11-04 · metavar 2026-10-26 · LKA 2026-10-24）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9；课程目录迁移 = issue #3（`course/` → `courses/quick-start/`）.

## 更早轮次（原文已删 ⇒ `git log --all -- STATUS.md`）：v0.97.4–v0.97.8 已发版（资产核对 = `gh release list`）· v0.97.6 前 `apply Eq.subst h hx` 判绿 ✓
## 未决项：**权威 = `docs/ONBOARDING.md` §0.2**（本节不自报条数；2026-10-13 订正过"6 条"与实测不符 ✗）
## 已发布线：`v0.79.0`…`v0.83.0` ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz`；**"最新是哪版/多少没发"不许写这里**（死格，2026-10-16 实测落后 15 个 release ✗）⇒ 活读数 `python3 scripts/release-metric.py` ✓
## 硬事实（接手先读）⇒ 原文已删（`docs-lint` ⑦ 顶格，按本文件规矩腾空间）· `git log --all -- STATUS.md` ✓
