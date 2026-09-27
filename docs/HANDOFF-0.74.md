# 交接单 —— **v0.74.0 已收口**（下一会话从 v0.75.0 起）

> 写于 2026-09-27 深夜（夜班会话收尾）。**本文件只是起点快照**，不取代
> `docs/PLAN-0.74-0.79.md`（**唯一真相**：顶部「⚡ 执行索引」，环节与判据以当前发版点章节为准）
> 或 census ✓。

## 1. v0.74.0 = **11/11 全部 ✅**（「记法补齐 + 编译体验修复 + Infoview 面」）

| 环节 | 状态 | 证据 / commit |
|---|---|---|
| E01 `•` / `∘` 记法 | ✅ | `7a4a58b`（判据三层 + 反向验证 · G-45） |
| E02 `Set.prod` 收进 `lib/Prod` | ✅ | `8376391`（G-46；checked 328→327） |
| E03 `r ⁻¹` / `A ≈ B` 记法 | ✅ | `4e093dd`（G-47 fixed · G-48/G-49 open） |
| E04 hover 折记法 | ✅ | `8660908`（判据三层 · G-50 fixed） |
| E21 Infoview「目标 ⊢」行 | ✅ **不改（resolved-no-change）** | `ce0371b`（复核依据 + 防漂移判据 + 反向验证；依据进 census） |
| E22 build/rebuild 以**项目**为目标 | ✅ | `b90c0ff`（+ rebuild 的 `--clean` 带目标 · G-51 fixed） |
| E23 build/rebuild **进度** | ✅ | `3a5e94f` + CLI 事件 `0ad970f`（三处同一份 + **可取消** · G-52 fixed） |
| E27 Infoview 内跳到定义 | ✅（**一半**，见 §3.1） | `1444780`（声明名 ⇒ `executeDefinitionProvider`；**记法符号未接 ⇒ G-53 open**） |
| E28 空态/错误态判据 | ✅ | `086c220`（主机侧三态判据；不改行为） |
| E29 Infoview 进度区覆盖 build | ✅ | `3a5e94f`（与 E23 同一份判据；反向验证「只留状态栏」判红） |
| E30 Infoview「项目」区块 | ✅ | `5323889`（转发 `soko/project`；`requires_warning` **可见块不是 tooltip**；顺手补 wire 守卫的 front 侧解析 = 审计 #17） |
| E31 `Clean Cache` 命令 | ✅ | `bc9a62c`（只清不编；三个数来自 CLI 事件） |

**发版**：`scripts/bump.py 0.74.0` ⇒ 提交 `404b3f6` ⇒ auto-tag ⇒ release（**`v0.73.0 → v0.74.0`**；
tag 与 release 用 `gh release list` 核对，见 §2）。发版点的收尾提交（PLAN/STATUS 的那两行）紧随其后。

## 2. 实测验收数字（**接手先认这几个数**）

| 判据 | 结果 |
|---|---|
| 课程门禁 | **36 目标 · 327 checked · 99 open · 0 判负** ✓（`python3 courses/set-theory/tools/check.py`） |
| webview 渲染 | **23/23** ✓（`node editor/vscode/test-webview.js`） |
| stub 宿主 | **43/43** ✓（`node editor/vscode/test-extension-host.js`） |
| 真宿主 e2e | **35/35** ✓（CI 三平台；`docs/e2e/ledger.jsonl`） |
| 本地全量 | `scripts/ci-local.sh` 全绿 ✓ |
| CI（发版前那一推） | run **`36329403671`** ⇒ `python3 scripts/ci-green.py --run 36329403671` **exit 0**（重活 10/10、failure 0、skipped 仅 `fast-fail`）✓ |
| CI（往轮真绿） | `36321102036`（E01–E04）· `36325452112`（E23/E29）· `36327371460`（E31/E28）✓ |
| CI（bump 那一推） | run **`36331547892`**（headSha `404b3f6`）—— 见 `gh run view 36331547892` / `docs/e2e/ledger.jsonl` |

## 3. ⚠ 已知未做 / 下一会话要接的

1. **G-53（open）**：Infoview 的**记法符号**（`{a}`/`∈`）点不动 —— wire 的 runs
   **只有 `{text, kind}`**、没有源位置 ⇒ 要改 front 的 `semantic::goal_runs`/`tag_runs` + LSP `RunInfo`
   （additive）+ `docs/protocol.md` + wire 守卫。自足复现件
   `docs/gaps/repro/G53-runs-have-no-positions.sh`（exit 0 = 缺口仍在）。接上之后 webview 的
   `tok-*` span 直接复用 E27 已有的 `gotoDefinition` 链 ✓。
   ⚠ 另有一条实测：本 LSP 的 definition 解析的是**使用处** ⇒ 点**声明名**会看到「这里没有可跳转的定义」
   （诚实但用处有限）—— 真正有价值的落点是符号，正被这条挡住。
2. **E23 少做了 `{viewId}`**（如实记账，见 `docs/design/compile-progress-ui.md` §5）：设计里的
   `{viewId: "sokonanoda.infoview"}`（Infoview **标题栏**原生进度条）**没做** ✗ —— 它的效果在
   视图 chrome，扩展宿主看不见 ⇒ 无判据可咬；面板内已有进度区（同一份数字）。
3. **G-48 / G-49（open）**：`≈` 两侧都是零元糖补不出论域；类型错误报裸 de Bruijn 编号（`$4`）。
4. **E05–E08（v0.75.0「跳转与高亮」）**：E05 记法目标名 F12 落点（一行级）· E06 G-37 判据升级 ·
   E07 跨模块目标语义定案 + 颜色一致 · E08 `documentHighlight` 对记法目标名。
   **E27 与 E05/E06 是两条不同的链** ✗✓ —— E27 修的是 **Infoview（webview）里从来没有这条链**，
   E05/E06 修的是**编辑器 F12** 的 span 丢参（`crates/lsp/src/lib.rs:1896`）—— **别混成一个 commit** ✗。

## 4. 本轮踩过的坑（**别再重新发现一遍** ✗）

| 坑 | 应对 |
|---|---|
| **bump 之后 pre-push 会红** ✗（判卷二进制与仓库版本不一致 ⇒ 课程门禁 exit 2） | bump 后先 `cargo build -p sokonanoda-cli --bin sokonanoda`，再用 **`SOKONANODA_BIN=$PWD/target/debug/sokonanoda`** 推 —— ⚠ 不带它走 debug 全量，课程门禁 **357s**（实测），带了走缓存 **~2–5s** |
| **CI 的 `e2e ledger` 会往 main 回写一条** ⇒ 每次推都非 fast-forward ✗ | `git fetch origin && git rebase origin/main` 再推（**禁止 force push** ✗）；e2e 台账冲突的解法：`ledger.jsonl` **取并集**（按 `date` 排序去重）、`latest.json` **取 theirs** |
| **动共享缓存的用例会影响后续用例** ✗ | E31 的 e2e 清空项目缓存后**必须预热回去**（否则 T-A60-2 的前置 `stamp.length > 0` 判红，实测踩到）；T-A60-2 现在也**先等缓存静止**再取基线（`90cb15b`；run `36323798295` 的 ubuntu 1.138.0 就栽在这） |
| **`audit-wire-fields.py` 的扫描器**（E30 收紧） | 去注释必须**先 `//` 再 `/*`** ✗✓ —— 反过来会把散文里的 `compiled/*.tmp` 当成块注释开头 ⇒ `in_block` 卡住 ⇒ **整份文件后半段被静默跳过**（实测把 `decl.value_runs` 扫没了、`--selftest` 也不咬了 ✗）；命中点在引号里要丢掉、`module.exports` 定点排除 |
| **预算放宽的口径**（用户 2026-09-27 23:12 拍板） | 放宽**没问题**，但要在**同一个 commit** 里记一笔：哪个文件 · 从→到（净增几行）· 为什么 · 「后续 refactor 时清理」。本轮放宽两处：`docs/protocol.md` **885→888**（`build.begin` 事件）、`STATUS.md` **200→240**（`scripts/status-lint.py` 的 `MAX_TOTAL`；一轮落 6 个环节写不下）—— 两处都记了 ✓ |
| **反向验证是硬要求** | 每个环节都要"撤掉改动 ⇒ 判据逐字复红"（本轮 6 个环节各自实测：E21 是"把原判的过滤加回去"、E22/E23/E27/E29/E30/E31 各自撤对应实现） |
| **漏标状态列** ✗ | E28 的 commit 落了、**总览表的 ✅ 忘了标** ✗ ⇒ 发版前用脚本逐行扫 11 个环节（`python3 -c` 扫表）才发现 ✓ —— 收尾必做这一步 |

## 5. 关键指针（file:line）

- **显示层唯一接口**：`crates/front/src/display.rs` 的 `DisplayNotations::{fold, render, runs}`；
- **项目数据**：LSP `soko/project` → `crates/front/src/query/types.rs::{ProjectView, ProjectModule, ProjectCounts, ProjectArtifacts}`；
  Infoview 的「项目」区块走**主机转发**（`extension.js` 的 `setProject/postProject`），**不额外取数** ✗；
- **跳定义链**：`media/infoview.js` 的 `decl-name` 点击 ⇒ `{type:"definition"}` ⇒
  `extension.js` 的 `gotoDefinition()`（= `vscode.executeDefinitionProvider`）；
- **build 进度链**：`crates/cli/src/build.rs` 的 `build.begin`/`build.file`/`build.summary`
  ⇒ `runBuildProcess(onLine)` ⇒ `applyProgress`（状态栏 / Infoview / 概览尺三处同一份）；
- **判据落点**：`crates/front/tests/prelude_shape.rs`（记法真相）· `crates/cli/tests/notation.rs`（真课程库契约）·
  `crates/front/src/compile/tests.rs`（hover/显示面）· `crates/lsp/src/tests/hover_brackets.rs`（wire）·
  `editor/vscode/test-webview.js`（渲染）· `editor/vscode/test-extension-host.js`（宿主接线）·
  `editor/vscode/src/test/extension.test.js`（真宿主 e2e）；
- **台账**：`docs/gaps/ledger.jsonl`（G-45…G-53）+ `docs/gaps/repro/`（自足复现件）；
- **as-built**：`docs/design/notation-subset.md` §16/§17/§18 · `docs/design/compile-progress-ui.md` §5 ·
  `docs/gaps/criteria-census.md`（E21 的复核结论）。
