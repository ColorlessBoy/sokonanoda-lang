# 交接单 —— v0.74.0 进行中（**E21 起**）

> 写于 2026-09-27（第二个会话收尾）。本会话完成 **E03 / E04** 两个环节（全程带验证），
> 并把 **E01–E03 那一批推上去、拿到 v0.74.0 第一次逐 job 真绿** ✓。
> **唯一真相 = `docs/PLAN-0.74-0.79.md`**（顶部「⚡ 执行索引」；环节与判据**以当前发版点章节为准，不凭记忆** ✗）。
> 本文件只是**起点快照**，不取代 PLAN 或 census ✓。

## 1. 当前进度（v0.74.0：**4/11** ✓）

| 环节 | 状态 | 证据 |
|---|---|---|
| **E01** `Rel.comp`/`Function.comp` 记法（`•`/`∘`） | ✅ **7a4a58b**（状态 `95a831b`） | 判据三层 · 反向验证 ✓ · 台账 **G-45** |
| **E02** `Set.prod` 收进 `lib/Prod` | ✅ **8376391**（状态 `d55701f`） | 判据两层 · 反向验证 ✓ · 台账 **G-46**（checked 328→327） |
| **E03** `r ⁻¹` / `A ≈ B` 记法 | ✅ **4e093dd**（状态 `37a8d44`） | 判据两层（含 `⁻¹`/`⁻¹'` 共存）· 反向验证逐字一致 ✓ · 台账 **G-47**（fixed）+ **G-48**/**G-49**（新登记 open） |
| **E04** hover 的类型面接上折叠 | ✅ **8660908**（状态 `2e4ef58`） | 判据**三层**（front 真相 / LSP wire / e2e 子表达式 hover）· 反向验证 `SOKO_NO_NOTATION_FOLD=1` 判红 ✓ · 台账 **G-50**（fixed） |
| E21 · E22 · E23 · E27 · E28 · E29 · E30 · E31 | ⬜ **7 个未开工** | — |

**push 状态** ⚠：**已推** = `b63e77f..a9b4f4b`（E01–E03 那批，含 E02 计数钉子的修复 `7b77589`）；
**未推** = **3 个**（`cafb7dd` 交接单 CI 结论 · `8660908` E04 · `2e4ef58` PLAN 状态）。
下一批收尾时 `git fetch origin && git rebase origin/main` 再推 ✓
（**远端会多一条 CI 回写的 `perf(e2e): 台账 …` 提交** —— 非 fast-forward 是**常态**，**禁止 force push** ✗）。

## 2. ✅ 验收数字已定（**用户 2026-09-27 拍板：按 327 读**）

课程门禁 = **36 目标 · 327 checked · 99 open · 0 判负**（`Set.prod` 收进库 +1、删两份副本 −2 ⇒ 净 −1）。
**别再追 328** ✗；**记法零事件** ⇒ E01/E03/E04 都是**计数中性**的 ✓。

## 3. 本会话实测的验收状态（全部在**冻结树**上跑 ✓）

| 判据 | 结果 |
|---|---|
| 完整 `scripts/soko gate`（fmt/clippy/workspace test/锚点/课程门禁/缺口台账） | ✅ **exit 0** |
| 课程门禁 `check.py` | ✅ **36 目标 · 327 checked · 99 open · 0 判负**（EXIT=0） |
| `cargo test -p sokonanoda-front --lib` | ✅ **748 passed / 0 failed** |
| `cargo test -p sokonanoda-lsp` | ✅ **164 passed / 0 failed** |
| `cargo test -p sokonanoda-cli` | ✅ **25 个 target / 0 FAILED** |
| `scripts/notation-lint.py` | ✅ 84 文件零旧写法（572 处显式豁免） |
| `scripts/gap.py check`（`--strict` 同 CI） | ✅ 全部与台账一致（G-45/46/47/50 fixed · G-48/49 open） |
| `scripts/docs-lint.py` / `status-lint.py` / `audit-notation-paths.py` | ✅ 全绿（记法路径守卫仍 59 = 地板） |

**CI（v0.74.0 第一次逐 job 真绿 ✓）**：run **`36316083145`**（headSha `5cd382b`）——
`python3 scripts/ci-green.py --run 36316083145` ⇒ **exit 0**：
**28 success · 1 skipped（`fast-fail`，条件 job）· 0 failure · 重活 10/10 实跑且 success** ✓。
（`auto-tag` 是 no-op ✓：版本仍 0.73.0、无新 tag/release ✓。
更早那轮 `36314757444` 红了一个 job —— `query.rs` 的计数钉子，已修 + 记进 `docs/CI-FAILURES.md` ✓。）

## 4. 已知坑（**别再重新发现一遍** ✗）

| 坑 | 应对 |
|---|---|
| **改了课程计数 ⇒ 测试钉子会漏** ✗（E02 后 `query.rs` 钉着 `decl_checked == 5`，实际 4 ⇒ CI 红了一个 job） | **全测试树 grep 计数钉子** ✓（`decl_checked`/`exercise_open` + 被改文件名），**不许只 grep 名字带 course 的文件** ✗；改完**本地跑全套** `cargo test -p sokonanoda-cli`（25 target）确认"只有这一处" |
| **hover/显示面判据钉的是点形式** ✗（E04 一次红了 5 条 front + 1 条 LSP） | 改显示行为前先 `grep` 期望值；**期望值按实测文本定**（先 dump 再写 ✗ 别一条条猜）——`a ∧ (¬ a)` 这种**括号**只有实测才知道 |
| **反向验证的开关** | 折叠类：`SOKO_NO_NOTATION_FOLD=1 cargo test …`（**必须判红** ✓）；`scripts/soko gate` 里那条"折叠判据的反向验证"门禁用的就是它 |
| **pre-push 钩子很慢**（课程门禁 + 缺口台账 + 6 个守卫） | 推时带上 `SOKONANODA_BIN=$HOME/.local/share/sokonanoda/bin/sokonanoda` ✓（钩子会继承环境；不带就走 `target/debug`，课程门禁从 ~2 分钟变 10+ 分钟 ✗）。**别用 `--no-verify`** ✗ |
| **CI 监控别只看整轮** ✗（matrix 里一个 job 早挂了、整轮还是 `in_progress`） | **60–90 秒一次、逐 job** 看：`gh run view <id> --json jobs \| jq -r '.jobs[] \| "\(.conclusion // .status)\t\(.name)"'`；`jq` 里 `.conclusion` 是**空串**（不是 `null`）⇒ 判红用 `select(.status == "completed" and .conclusion != "success" and .conclusion != "skipped")` ✓ |
| **push 被拒（非 fast-forward）** | 常态：CI 的 `e2e ledger (commit back on main)` 会往 main 回写一条 ⇒ `git fetch origin && git rebase origin/main` 再推 ✓，**禁止 force push** ✗ |
| **`cargo fmt` 只 fmt 教学 crates** | `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp` ✓（**禁 `--all`** ✗：kernel 的 rustfmt 要 nightly）；fmt 红了**单独一个小 commit**，**别 amend** 已写进 PLAN/台账的 commit hash ✗ |
| **本地跑判卷要钉发布产物** | `export SOKONANODA_BIN=$HOME/.local/share/sokonanoda/bin/sokonanoda` ✓（仓库构建 `target/debug` 慢 10× ✗） |
| **"某面已迁 ✓"这种结论会把缺口盖住** ✗（E04 的真教训：审计文档写着 LSP hover「已迁 ✓」，实际那条路从没过 `fold`） | 结论必须能指到**调用点**或**一条会判红的测试** ✓ |

## 5. 下一步第一件事：**E21**（Infoview 声明卡片去掉多余的「目标 ⊢」行）

**要什么**（PLAN §v0.74.0 的 E21）：Infoview 的**声明卡片**别在 `theorem` 上重复语句 ——
现在卡片上多一行「目标 ⊢ …」，而那一行对 `theorem`/`def` 只是**把签名又说了一遍** ✗。

**怎么开工**：
1. 先回答**屏幕上会少什么**（AGENTS.md 四条硬规则第 1 条：答不上就不算验收完整 ✗）——
   看 Infoview 渲染侧（`editor/vscode/` 的 webview + `crates/lsp` 的 wire）；
2. 判红：拿现有渲染判据/e2e 里那条「目标 ⊢」断言先钉住"现在**有**这行" ✓；
3. 改**渲染侧**（**判定侧与 wire 字段别动** ✗：数据留着，只是卡片不渲染那行）；
4. 三层判据照 E04 的样子：真相（wire 还在）· 渲染（卡片**不含**那行）· e2e（屏幕上少一行）；
5. 反向验证 + 台账（**G-51**，若确认是缺陷）+ STATUS + 总览表 `✅ <commit>`。

**E22–E31 的顺序与判据**都在 PLAN 总览表 `:225-233` 与 §v0.74.0（`:296` 起）里，
**开工前读那两处** ✓（尤其 E27/E29/E30 是「中」风险，别按记忆做 ✗）。

## 6. 关键指针（file:line）

- **显示层唯一接口**：`crates/front/src/display.rs` 的 `DisplayNotations::{fold, render, runs}`；
  E04 起 `resolve_hovers`（`crates/front/src/compile/check/mod.rs`）也走它 ✓；
  记法路径守卫 `scripts/audit-notation-paths.py`（**59 = 地板**，再降要改记账口径）。
- **折叠开关**：`SOKO_NO_NOTATION_FOLD=1`（`check/mod.rs` 建表处 ⇒ 返回空表）。
- **课程库记法**：`courses/set-theory/lib/{Set,Rel,Fun,Equiv,Prod}.sokonanoda`；
  速查表 `courses/set-theory/units/notation-cheatsheet.sokonanoda`（表 2b/2c + 梯子）。
- **判据落点**：`crates/front/tests/prelude_shape.rs`（记法真相）·
  `crates/cli/tests/notation.rs`（真课程库契约）· `crates/front/src/compile/tests.rs`（hover/显示面）·
  `crates/lsp/src/tests/hover_brackets.rs`（wire）· `editor/vscode/src/test/extension.test.js`（用户可见）。
- **台账**：`docs/gaps/ledger.jsonl`（G-45…G-50）+ `docs/gaps/repro/`（自足复现件）。
- **as-built**：`docs/design/notation-subset.md` §16（E01）· §17（E03）· §18（E04）。
