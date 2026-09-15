# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-15（第七十四轮：Infoview 声明类型提示 + 点击跳转；0.44.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-15，第七十四轮：Infoview 声明类型提示 + 点击跳转）

> 用户：Infoview 的「声明」除名字外，用小字写出类型做提示，注意排版（保持
> 每行一个声明）；并支持鼠标点击跳转。

1. **协议**：`soko/goals` 每条声明增 `ty`（内核渲染的声明类型）与 `ty_runs`
   （`front::semantic` runs，与 goal 同一分类源）；`docs/protocol.md` 同步。
2. **Webview 排版**：声明项改「名字 + kind/status 徽标」一行、下面一行
   `.decl-ty` 小字（0.78em、暗色、等宽、单行省略）按 `tok-*` 着色——保持
   「每行一个声明」的节奏；`codeBlock` 复用同一渲染路径。
3. **点击跳转**：点击声明 post `focusExercise` 带 `range`；扩展处理器在
   `focusDeclaration` + 聚焦练习树之外，把编辑器光标移到该声明并 reveal。
4. **测试**：LSP `goals_request_lists_open_exercise_with_hole_range` 增 `ty`/
   `ty_runs`（重建 + sort kind）断言；扩展契约
   `infoview_declaration_list_shows_types_and_jumps`（webview/css/host 三处）。
5. **验收**：`sokonanoda gate` PASS；版本 0.43.0 → **0.44.0**（新能力 minor）。

## 本轮进度（2026-09-15，第七十三轮：呈现面高亮统一）

> 用户追问「各个地方的高亮统一」后补做（HANDOVER §3 A″）：0.40.0 只统了 goal
> 状态，其余渲染 `.sokonanoda` 的面仍各自为政。原则：**着色只来自
> `front::semantic`**（语义 token + TM 语法 + runs），手段是统一 `{sokonanoda}`
> markdown 围栏。

1. **LSP**：新增 `CODE_LANG`/`code_block`/`goal_block`；`hover_markup`（表达式/
   签名 hover）由 ` ```text ` 改 ` ```sokonanoda `；声明 hover 的签名、洞期望
   类型、目标态都用代码块；tactic hover 的 tactic 片段、半表达式 hover 的
   推断类型/目标也从行内代码改成代码块；补全 `documentation` 给出签名的
   `sokonanoda` 围栏。
2. **扩展**：`codeMarkdown`/`goalTooltip`——练习树「目标」「假设」tooltip 用
   `MarkdownString.appendCodeblock(…, "sokonanoda")`。
3. **刻意保持纯文本**（VS Code 不渲染 markdown / 不给行内代码语言）：诊断消息、
   inlay hint、TreeItem.description、CodeAction 标题；hover 里「散文提到单个词」
   也保持行内代码。文档写明（`goal-rendering.md §7`）。
4. **契约**：LSP `code_fences_always_use_the_sokonanoda_language` + hover/
   completion 断言；扩展 `rendered_language_text_uses_the_sokonanoda_fence`。
5. **验收**：`sokonanoda gate` PASS；版本 0.42.0 → **0.43.0**（行为统一，minor）。

## 本轮进度（2026-09-15，第七十二轮：`match` 模式编译器 v1）

> 续 HANDOVER §3 B / ROADMAP I6：把「每构造子一条 arm」换成有序 arm + 列式
> 模式编译，支持字面量/嵌套/通配/守卫。设计 `docs/design/match-patterns.md`。

1. **AST/parser**：`Pattern { Wild, Num, Ident{name,args} }`；`MatchArm` 改
   `{pattern, guard, body}`；`parse_pattern`（递归、`(...)`、`_`、数字）；
   守卫 `if` 只在 arm 里识别（不升全局关键字）。
2. **编译器（核心）**：**源到源 canonical 化**——`compile_pattern_body` 选可反驳
   列、按构造子特化，生成嵌套 `Expr::Match`，每层仍走既有 motive/IH/level/
   recursor 构造（**不手搓 de Bruijn**）；守卫复用 prelude `Bool` 的 match。
   字段名取绑定名（canonical 幂等）、撞构造子名用新鲜名；参数化字段先代入参数
   （`some (a : A)` 在 `Option Nat` → `Nat`）。
3. **语义**：有序、首个匹配者胜；未知裸名 = 绑定变量（带子模式才 bad-arm）；
   覆盖不全/守卫无兜底 = `elab-match-non-exhaustive`；error hint 措辞更新。
4. **消费者**：`semantic`（模式绑定着色 + 守卫）、`proof::render_pattern`、
   `spine`（mentions/substitute 含守卫与模式阴影）、`goals`（hole/替身/依赖
   子目标；嵌套/守卫退回常量 R）。
5. **测试**：front +8、CLI +3；课程 unit5 增嵌套模式节 + 练习 9
   （golden `(10,8,4)→(11,9,6)`、汇总 `checked 54→55 / open 41→42`）。
6. **文档**：architecture §2/§4.1、design `match.md` §2/§10 Phase 6、
   `match-patterns.md` as-built、TESTING、protocol、CHANGELOG。
7. **验收**：`sokonanoda gate` PASS；版本 0.41.0 → **0.42.0**（新语法 minor）。
   已知限制：`as`/or 模式、多 scrutinee、`if/then/else` 表达式不做。

## 本轮进度（2026-09-15，第七十一轮：prelude `Bool`）

> 续 HANDOVER §3 C / ROADMAP I6：把 `Bool` 作为真实可信归纳加进 prelude，
> 与 `Nat`（0.36.0）同法，供 `match` 与后续布尔例子使用。

1. **安装**：`prelude.rs::install_bool_prelude` 调用既有
   `install_inductive_block`，`Bool` **非递归** → 构造子 `Bool.true`/`Bool.false`
   + 派生 `Bool.rec`（两分支、无 IH），登记进 `known` 与 `match` 的
   `InductiveTable`；`PRELUDE_NAMES` 增 4 个名字（补全/目标视图）。
2. **闸**：`check.rs::run_pass` 增 `explicit_bool`——文件自带 `inductive Bool`
   时 prelude 让位（否则重复声明 panic）；`session.rs::PreludeShape` 扩成
   `(mode, explicit_nat, explicit_bool, eq_taken)`，任一变化整体重编译。
3. **内核零改动**：`Bool.true`/`Bool.false` 的 name-cache 槽位早已存在
   （原生 `Nat.beq`/`Nat.ble` 用），归约走通用构造子 iota。
4. **测试**：front `prelude_bool_is_available_without_a_source_block` /
   `match_prelude_bool_not_checks_and_reduces`（`#reduce bnot Bool.true =>
   Bool.false`）/ `prelude_bool_definitions_compose` /
   `explicit_bool_block_yields_to_the_source_declaration`；CLI
   `cli_match_on_prelude_bool_checks_and_reduces`；既有源内 `inductive Bool`
   （`tt`/`ff`）用例继续通过=闸生效。
5. **文档**：`architecture.md §5.4`、`design/match.md §2/§10 Phase 5`、
   `TESTING.md`、`protocol` 错误文案（`Nat/Bool`）；错误提示改为
   「prelude 内建的 Nat/Bool」。
6. **验收**：`sokonanoda gate` PASS；版本 0.40.0 → **0.41.0**（新能力 minor）。

