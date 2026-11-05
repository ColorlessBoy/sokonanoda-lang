## 扩展升级后建议 rebuild（2026-11-05 · issue #9）
- 升级后第一次在 sokonanoda **项目**里打开 `.sokonanoda` ⇒ 通知建议 rebuild，按钮 **确认 / 取消**；确认走 `rebuild --json <模块根>`（与 `alt+shift+b` 同一目标，来自 `soko/project`）；取消不重建、本会话也不再静默预热（否则 2026-10-11 的自动预热会让「取消」变谎话）。一次升级只问一次（`globalState` 基线）；首跑兜底看项目产物写者版本（上线那一版不会错过）。
- 判据：stub **82/82**（取消 ⇒ 零 spawn 且不补跑 · 确认 ⇒ 恰好一次 `rebuild --json <模块根>` · 单文件不问 · 降级不问 · 首跑兜底）· `cargo test -p sokonanoda-cli --test extension` 43/43 · `npm run test:unit` 全绿.
- 边界：只问**第一次**遇到的模块根；升级后没打开项目文件就不问（基线已记下）.
## 修复 issue #7（2026-11-04 · goal 里的 `{a}` · `apply Eq.subst hx` · 请求期探针 panic）
- 显示：显式脊 `@Set.singleton α a` 折回 `{a}`（部分应用仍不折）；elaboration：`apply Eq.subst hx` 判绿（错 motive 仍红）；健壮性：请求期探针 panic 降级成「算不出」.
- ⚠ 用户报的「写错 `p` 后 lsp 死掉」在 0.98.2 与 HEAD 都复现不出来 ⇒ 修的是同族可证的机制（请求期 panic → 永不回答），不是已确认现场.
- 判据：display ×3 + front ×2 + LSP 集成 ×1；截图见 PR.
## 修复 `·` 块内 goal state（2026-11-04 · issue #5）
- 块内最后一条 tactic 之后的缩进行答块内状态；嵌套 `·` 按缩进配对。判据：front ×2 + LSP wire ×1；截图见 PR.
## 更早轮次（metavar 2026-10-26 · LKA 2026-10-24 · 课程目录迁移 2026-11-04）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9；课程目录迁移 = issue #3（`course/` → `courses/quick-start/`）.

## 更早轮次（原文已删 ⇒ `git log --all -- STATUS.md`）：v0.97.4–v0.97.8 已发版（资产核对 = `gh release list`）· v0.97.6 前 `apply Eq.subst h hx` 判绿 ✓
## 未决项：**权威 = `docs/ONBOARDING.md` §0.2**（本节不自报条数；2026-10-13 订正过"6 条"与实测不符 ✗）
## 已发布线：`v0.79.0`…`v0.83.0` ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz`；**"最新是哪版/多少没发"不许写这里**（死格，2026-10-16 实测落后 15 个 release ✗）⇒ 活读数 `python3 scripts/release-metric.py` ✓
## 硬事实（接手先读）⇒ 原文已删（`docs-lint` ⑦ 顶格，按本文件规矩腾空间）· `git log --all -- STATUS.md` ✓
