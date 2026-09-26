# 记法路径审计：68 条基线**逐条结论**（A0 / T-N3，2026-09-26 ✓）

> **口径**：`scripts/notation-paths-baseline.txt` 的 68 行 = **68 个活跃调用点**
> （只有 53 个不同指纹：`--rebless` 写的是 `sorted(...)`，**不去重**）。逐条追
> 「这一行的产物最终流向哪里」，三选一：**迁移**（产出用户可见文本且能改）/
> **立判据**（产出用户可见文本，但折了会改判定 ⇒ 用判据钉住"**必须不折**"）/
> **台账不做**（判定输入 / 内部探针 / 测试夹具）。
> 口径与守卫：`docs/design/notation-display.md` ✓（该文的接口形状已于同日对齐
> **as-built** —— 原文写着的 `render_text`/`Rendered` 与代码不符 ✗）。

## 1. 结论总表

| 文件 | 活跃 | 迁移 | 立判据 | 台账不做 |
|---|---|---|---|---|
| `crates/front/src/by.rs` | 22 | **8** | 0 | 14 |
| `crates/front/src/compile/check/walk.rs` | 8 | 0 | **2** | 6 |
| `crates/front/src/compile/elab.rs` | 15 | 0 | 0 | **15** |
| `crates/front/src/compile/goals.rs` | 17 | 0 | **17** | 0 |
| `crates/front/src/query/mod.rs` | 1 | **1** | 0 | 0 |
| `crates/front/src/semantic.rs` | 5 | 0 | 0 | **5** |
| **合计** | **68** | **9** | **19** | **40** |

**基线数字已下削 68 → 59** ✓（T-N4 做完那 9 条之后 `--rebless`）。

## 2. 迁移 9 条（**已完成 ✓**，T-N4）

| 位置 | 流向（证据链） | 改法 |
|---|---|---|
| `query/mod.rs::runs` | Infoview 里**每一个**目标/类型/假设的着色：7 条 wire 字段全出自它（`query/mod.rs:700/707/767/795/809/819/830`） | 改调唯一接口 `DisplayNotations::runs`（分段与记法表无关 ⇒ `default()` 调它是**约定**不是绕过，已写进接口文档） |
| `by.rs:965`（`have` 值类型不匹配）、`:1349/:1350/:1351`（`apply` 目标不匹配，同一条 `format!`）、`:1486`（`cases` dependent elimination）、`:1550`（`cases` 被消去项不是归纳值）、`:1560`（`cases` 头不在归纳表）、`:2015`（`exact` 类型不匹配） | **用户可见的诊断消息**（错误路径）；`prefix_src` 本来就在每个函数签名里 | 新增 `display_expr(prefix_src, expr)` = `fold_for_display(prefix_src, &render_expr(expr))`（**零签名改动** ✓） |

**⚠ 修正既有审计**：`docs/design/duplication-audit.md` 的 C 组把 `by.rs` 22 条全判成
"**全部在判定侧，一个显示面的都没有**" ✗ —— 实测 **8 条是显示面**（证据
`by.rs:963-967 / 1347-1352 / 1483-1487 / 1548-1551 / 1558-1561 / 2013-2016`）。
该文件已被文档预算冻结（225 行）⇒ 更正记在这里。

**判据 + 反向验证**（都在 `scripts/soko gate` 里 ✓）：
`python3 scripts/audit-notation-paths.py` ⇒ OK（基线 68 → **59** ✓）；
把 `by.rs` 一处改回 `render_expr(…)` ⇒ 守卫当场判红
（`**新增** 1 处绕过 … crates/front/src/by.rs:2026`，exit 1 ✓）；还原 ⇒ 绿 ✓；
`--self-test` 仍咬得住 walk.rs 的 8 处 ✓。

## 3. 立判据 19 条（**不折 + 判据钉住**）

**判据是什么**：这些行是**源级渲染面**（不是内核 pp），它们的产物直接喂判卷
（`render → 回读`），折了**就改判定** ⇒ 判据必须断言"**它按源级形态走，且折叠
开关不动它**"。

| 组 | 位置 | 判据 |
|---|---|---|
| `walk.rs` 2 条（`:806`/`:1148`） | `signature: Some(render_expr(ty))` 的源级面 | ✅ `crates/cli/tests/notation_fold.rs:236` `source_rendered_surfaces_ignore_the_fold_switch`（断言含 `⊆`/`∈` **且**开关关掉逐字节不变 ✓） |
| `goals.rs` 8 条（`:1092/:1234/:1354/:1456` 的 `goal`，`:1386/:1443/:1467` 的 `GoalBinder.ty`） | `OpenGoalInfo.goal` / `GoalBinder.ty` ⇒ 目标栏 + judge 输入 | ✅ 同上一条 ✓（T-U4 实测：就地折会让 5 条 `suggest::*` 当场红 ✓） |
| `goals.rs` **9 条**（`:440/:513/:701/:702/:1033/:1051/:1178/:1288/:1484/:1540`） | `SubGoal.ty` ⇒ wire（`query/mod.rs:857-866`）+ LSP hover（`lsp/lib.rs:1698-1706`「此处 `sorry` 的期望类型」） | ⏳ **判据缺失** ⇒ **待补**（照 `notation_fold.rs` 组 1 的形状；它同时走内核 pp 与源级渲染两条路，**不许猜**） |

## 4. 台账不做 40 条（**不做 + 理由**）

| 组 | 条数 | 为什么不动 |
|---|---|---|
| `elab.rs` 全部（`judge_infer`/`judge_type_of`/`signature` 回读输入一族） | 15 | 这些 `render_expr` 的产物是**判定的输入**（折了 = 改判定 = 内核红线 ✗）。该文件的 4 处**消息**已在 T-U11 迁完（`render_msg`）✓ |
| `by.rs` 的判定侧（`judge_infer`/`judge_render_type` 回读、`rfl` 候选文本等） | 14 | 同上；`by.rs` 的 8 处**消息**已单列上表迁走 ✓ |
| `walk.rs` 6 条（`goal: render_expr(ty)` ×2、`signature` ×4） | 6 | `KnownName::Decl.signature` 的**唯一消费者**是 `elab.rs:2102 declared.signature()` ⇒ `telescope(ty_text)` = **判定输入**；`DeclState.goal` 同理（**一个字节都不能折** ✗） |
| `semantic.rs` 5 条 | 5 | 4 条是**测试夹具**里的期望串（往返测试的内部期望 ✗）；第 5 条（`:241`）是 `pub fn tag_runs` 的**定义/再导出行**——守卫早已按"名字即调用名 + 出现在 `fn` 签名里"豁免 ✓ |
| `walk.rs` 8 条 **E 组**（`--self-test` 的靶子） | （含在上面） | ⚠ 它们**故意留在基线里** ✗：把 `walk.rs` 整体加白名单 ⇒ `--self-test` **瞎掉** ✗（round 171 实测）。**守卫的可验证性 > 数字好看** ✓ |

## 5. 用户可见面的逐条结论（A0 要求的那五个面）

| 面 | 结论 |
|---|---|
| **Infoview 目标/类型/假设的着色** | ✅ 已迁（`query::runs` ⇒ 唯一接口）；A1/A2 另修了两处**折叠本身**的缺口（`->` 不折、`{a}` 不折）✓ |
| **LSP hover** | ✅ 走 `HoverType.text`（内核 pp + `fold_for_display` 一族，T-U11 A 组已迁 ✓）；**新发现**：开放练习的**签名**以前一条 hover 行都没有 ⇒ 在未解出的练习里 hover/F12/高亮/引用**全部失效**（A3 的根因，已修 ✓） |
| **诊断 message** | ✅ 已迁（`elab.rs` 的 `render_msg` + `by.rs` 的 8 处 `display_expr`）✓ |
| **状态栏 / 项目树** | 读 wire 的**计数**（不是文本）⇒ 与记法无关 ✓；但**未处理区域不给 goal/hover/诊断**那条"诚实降级"本轮**未做**（见批次 N 之外的进度展示需求）⏳ |
| **CLI `--json` 的 display 字段** | 与 wire 同源（`ty_text`/`val_text`/`goal*`）⇒ 随上面几条一起好 ✓ |

**相关**：`walk.rs::open_signature` 把签名 hover 收进局部 `Vec` 然后丢掉（A3 根因，
已修 ✓，判据 `crates/front/src/compile/tests.rs::an_open_declaration_records_hovers_for_its_signature`，
反向验证已跑 ✓）。
