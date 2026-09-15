# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-15（第七十五轮：应用位置 binder 类型推断；0.45.0）
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

## 本轮进度（2026-09-15，第七十五轮：应用位置 binder 类型推断）

> 续 HANDOVER §3 C / ROADMAP I6：elaborator 最后一项——无期望类型时从实参
> 推断 `fun x => …` 的 binder 类型。

1. **现状**：`fun x => …` 在有期望望远镜时已能推断（`Expr::Lambda` +
   `peel_expected`）；缺的是 `(fun x => x) 1` 这类无期望的应用位置。
2. **实现**：`annotate_application_lambda`——处理 `Expr::App` 前展平 spine
   `f a1 … an`；头部是带未注解 binder 的 `Lambda` 时，用 `judge_infer`
   推断 `a_i` 类型作为 binder 注解，**源到源改写**后交回正常路径；支持
   柯里化 `(fun x y => x) a b`。
3. **边界**：实参不足以覆盖全部未注解 binder → 仍报 `elab-untyped-binder`
   （`(fun x y => x) 1`、`#check fun x => x`）。
4. **测试**：front +3（应用/柯里化/实参不足）、CLI +1；既有 `untyped_binder_*`
   回归不破。
5. **文档**：architecture §elab、`elaborator-let-match.md` as-built、TESTING。
6. **验收**：`sokonanoda gate` PASS；版本 0.44.0 → **0.45.0**（新能力 minor）。

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

