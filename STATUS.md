## 修复 issue #7（2026-11-04 · goal 里的 `{a}` · `apply Eq.subst hx` · 请求期探针 panic）
- **显示**：`@Set.singleton α a`（内核装配的**显式脊**）现在折回 `{a}` ⇒ `intro hx` 之后目标面板是 `hx : {a} = ∅`，不再是 `@Set.singleton α a = ∅`. 判据：display ×3（含反向「显式脊部分应用不折」）+ CLI 真课程文件.
- **elaboration**：`apply Eq.subst hx`（**不给** `p := …`）判绿，与 Lean 同形. 三处修：`decompose_eq` 认内核形态 `@Eq T a b`（3 位）；端点**两端都不是局部名**时问 `apply` 要类型（`EndpointTy` 兜底）；目标里的闭项省略形与假设里的显式脊差前导实参（“剥前导实参再试”）. 正对照：写错 motive 仍判红.
- **健壮性**：`soko/goals`/inlay/hover 的**请求期**探针（直接调内核、无 `quiet_catch`）加 `catch_unwind` 兜底 ⇒ 内核 panic 降级成「算不出」，不再让那个请求永远不回答. 判据：front ×1（咬住：删掉兜底即 panic）+ LSP 集成 ×1（真子进程上把用户那两行**逐字符**敲进去 + 13 种请求全过）.
- ⚠ 如实记：用户报的「写错 `p` 后 lsp 进程直接死掉」在 0.98.2 与本仓 HEAD 上**都复现不出来**（VS Code 日志只有一次 SIGTERM = 本次排查自己杀的）⇒ 本轮修的是**同族可证的机制**（请求期 panic → 永不回答），不是已确认的同一现场.

## 修复 `·` 块内 goal state（2026-11-04 · issue #5）
- 块内最后一条 tactic 之后的**缩进行**（块未结束）答块内状态；嵌套 `·` 按缩进配对.
- 判据：front ×2 + LSP wire ×1；Infoview 截图见 PR（before `⊢ Q → P` / after `hp : P ⊢ Q`）.
## 课程目录迁移（2026-11-04）
- `course/` → `courses/quick-start/`（issue #3）：54 个文件纯移动；活引用同步（测试常量 · 脚本扫描根 · VS Code 课程树 + 旧布局回退 · 站点链接 · skills · 文档）.
- 判据：CLI 6 个测试套件 + notation 守卫 + `gap.py check` + `test-extension-host` 77/77 + `check-site` 10/10 + `docs-lint`；文档预算按实测腾挪，未抬上限.
- 追加：`board` 的 PR merge-ref selftest 3 条 FAIL 已修（`_patch_id` 对 merge commit 取第一父 diff + ⑥ 回归臂；normal/merge 两种形状 `--selftest` 均 rc=0）.
## 更早轮次（metavar 2026-10-26 · LKA 2026-10-24）
- 原文已删（只留最近 3 轮）⇒ `git log --all -- STATUS.md`。metavar 只认 `docs/design/metavar-pattern.md` §9。

## 更早轮次（原文已删 ⇒ `git log --all -- STATUS.md`）：v0.97.4–v0.97.8 已发版（资产核对 = `gh release list`）· v0.97.6 前 `apply Eq.subst h hx` 判绿 ✓
## 未决项：**权威 = `docs/ONBOARDING.md` §0.2**（本节不自报条数；2026-10-13 订正过"6 条"与实测不符 ✗）
## 已发布线：`v0.79.0`…`v0.83.0` ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz`；**"最新是哪版/多少没发"不许写这里**（死格，2026-10-16 实测落后 15 个 release ✗）⇒ 活读数 `python3 scripts/release-metric.py` ✓
## 硬事实（接手先读）⇒ 原文已删（`docs-lint` ⑦ 顶格，按本文件规矩腾空间）· `git log --all -- STATUS.md` ✓
