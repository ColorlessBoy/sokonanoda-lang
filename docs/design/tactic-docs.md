# tactic 文档体系：hover 摘要 · F12 跳转 · 完整介绍（设计先行，2026-10-10）

> **状态：设计评审中（未动任何源文件）** ✓。本轮只读代码、只产出本文，实现留待
> 评审通过后按 §5 的阶段 P1–P7 逐条落地。用户原话（2026-10-10，第一条）：
> 「`intro` / `exact` / `rfl` 这几个 tactic 没有好的文档，要求补齐三样：
> ① 代码区域 hover 显示简短 tactic 说明；② 支持 F12 跳转到对应文档；
> ③ 实现完整 tactic 介绍文档（markdown），含如何使用、适用场景、原理
> （背后内核规则/判定，如 `rfl` = defEq 自反、`exact` = defeq 统一）」。
>
> **用户同日补充（范围扩大，已并入本设计）**：「tactic 文档范围扩大到【所有】tactic，
> 不止 `intro`/`exact`/`rfl` 三个 …… 都纳入：hover 简短说明、F12 跳转、markdown
> 完整文档，一个都不能少。设计时：① 先盘点 `by.rs` 里全部已实现 tactic 的清单
> （作为文档覆盖基线，commit/验收按此清单核对）；② 设计留出机制保证【以后新增一个
> tactic 也自动要求补文档】；③ 按所有 tactic 规划多阶段拆分」⇒ 落点 = **§1.1 基线表**
> · **§5 P1 覆盖判据** · **§5 P3–P5 三族拆分** ✓。

## 0. 结论速览（先给评审看的十二条）

1. **三样都能做，且都不需要新的 LSP 能力**（用户第 4 条问的正是这一条 ⇒ **答"否"** ✓）：
   `definition_provider` 与 `hover_provider` **早已 advertised**
   （`crates/lsp/src/lib.rs:2253-2290`），F12 只是在 `goto_definition` 里**多插一个
   分支**（与 2026-10-10 刚落的「内建记法第二跳」同一处、同一形状）；
   **扩展侧连代码都不用改**（F12 是 VS Code 内置动作，经 `vscode-languageclient` 转发，
   `editor/vscode/package.json` 一行不动）✓。
2. **文档的落点是 `reference/tactics/*.md`（新目录），不是 `docs/`** —— 它是
   **产品内容**（随 LSP 二进制发出去、由 `include_str!` 内嵌），而 `docs/` 是
   **开发者台账**（`docs/README.md:110-113` 明说它是"含 CI 失败记录与内部取舍"的
   地方，`.github/workflows/pages.yml:10` 也据此拒绝拿 `docs/` 当站点源）。
   **理由三条见 §3**；代价（两个文档闸都不覆盖它）由 §5 P1/P3–P5 的**自己的判据**补 ✓。
3. **运行时两条通道、一份源**（§4.9 D10）：**① 文档也以真文件进 VSIX**
   （`editor/vscode/docs/tactics/`，扩展经 `SOKONANODA_DOCS_DIR` 交给 LSP ⇒
   F12 落在**插件自己的文件**上）**+ ② 二进制内嵌 + 物化**（`include_str!` ⇒
   缓存/临时目录 ⇒ `file:` URI）。
   **为什么两条都要**：实测**今天的** VSIX 一个仓库 `.md` 都不带
   （`unzip -l`：只有 `readme.md`/`changelog.md` 与 node_modules 里的）⇒ 通道 ① 是**新增**的；
   而通道 ② **不能砍** —— CLI / opencode / DSH **不经扩展**启动，砍了它们就没有文档 ✗
   （AGENTS.md：这两个 harness 是**一等公民**）。
4. ⭐ **"随插件分发"这条补充已纳入**（§4.9 D10，用户 2026-10-10）：文档**也**以真文件进 VSIX
   （`editor/vscode/docs/tactics/`，扩展经 `SOKONANODA_DOCS_DIR` 交给 LSP）⇒
   **装插件即自带、离线、零外部站点** ✓；实测两条关键事实支持这个形态：
   **`.vscodeignore` 不排除它** ✓、**`.gitignore` 不影响 `vsce`**（`bin/` 就是"生成目录 + 随包发"的既有先例）✓。
   判据是**机械的**：`unzip -l <vsix>` 里必须有那 15 个条目 + 打包脚本 `--check` 防漂移 ✓。
5. **覆盖范围 = 本语言已实现的【全部】14 条 tactic**（不是三条）⇒
   **文档覆盖基线 = §1.1 的清单**，**commit 与验收逐条按它核对** ✓。
6. **有一条既存判据会正面冲突，处置已定（P2 已落）** ✓：`crates/lsp/src/tests/hover.rs:1231`
   那条判据今天**强制**关键字上的 hover **整段**逐字节不变，而需求 ① 要求加摘要 ⇒ 两者
   **不可能同时成立**。处置（§4.1）：`APPLY_KEYWORD_BASELINE` **保留**、降级成"**goal state
   那一段**"的期望值；新断言 = "把新增那一行删掉后与它逐字节相同"（P2 同轮**更名**为
   `hover_on_a_tactic_keyword_adds_only_the_summary_line`）✓。**需求逼出来的必然变更，不是绕过判据** ✓。
7. **"以后新增 tactic 自动要求补文档"靠**四层机制**，不靠纪律** ✗→✓：
  ① **构造**（`is_tactic_keyword` 派生自文档表 ⇒ 表外关键字**根本不被当成 tactic**）；
  ② **编译期**（`include_str!`）；③ **判据**（手写清单 ↔ 表，双向）；④ **独立源码 lint**。
  ⚠ **初稿曾写"取枚举变体名做判据"，那是错的** —— Rust 没有反射，实现不了（§12 第 1 条）。
8. **tactic 清单今天有六份副本**（§1.2）：`parser.rs:3569-3589` 白名单（**真源**，14 条）·
  `Tactic` 枚举（13）· `span()` 穷尽臂 · `semantic::KEYWORDS`（13，`sorry` 故意是 `Hole`）·
  `render_tactic`（13）· **"未知 tactic"错误文案（13，已经漏了 `have`）**。
  其中**只有错误文案没有任何判据**，所以它已经漂了而没人知道 ⇒ 本设计把它**由表拼出** ✓。
9. ⚠ **用户清单里的 `exact?` 在本语言里【没有实现】**（实证：全仓只有
   `crates/front/src/judge.rs:10` 一句**注释**提到 Lean 的 `exact?` 教训，
   parser 没有对应分支）⇒ 它**不进文档基线** ✓；同理没有 `intros` 别名
   （只有 `intro` 一条，`intro a b c` 一次剥多层 ✓）。
10. ⭐ **14 条摘要已在设计阶段定稿**（§4.5：一份文本同时供 hover / 补全 / 索引 / 正文首句，
   四处消费）⇒ P2 只负责落地、不再重新措辞 ✓；**版式也定了**：§4.7 D8 给了模板 +
   **一篇填满且判绿的样板**（`intro.md`，48 行，两个例子都喂过真内核）⇒ P3–P5 是**填空** ✓。
11. ⭐ **归属已澄清（2026-10-10 用户）**：`apply` 的「目标匹配应做定义展开」是**内核/tactic 行为缺陷**，已改派给**修复线 `d9a701e4`** ⇒ 本文档**不纳入、不要求、不实现**（§7 边界 9）；`apply` 的**文档照写**，但**以落地当天的实际行为为准**（§1.3 已改成显式快照 + R20/R21）✓。**已落地（2026-10-10，G-109）**：`apply` 今天先做多轮定义展开、再按实参位对齐，对不上才报错（报错文案**逐字未变**）⇒ 文档按**新**行为写 ✓。
12. ⭐ **设计自审抓出六条错**（§12，第 6 条在 P1 实现期抓到），其中一条是**既有的用户可见 bug**：
   "未知 tactic" 的错误文案（`parser.rs:1546-1548`）列了 13 条而**漏了 `have`**，
   且**全仓零断言**覆盖它 ⇒ 同类漂移随时会再发生。本设计把它**由表拼出**顺手修掉 ✓。

## 1. 现状盘点（每条带 `文件:行`）

### 1.1 ⭐ 文档覆盖基线：本语言**全部**已实现 tactic（14 条）

> **这一节就是 commit / 验收要逐条核对的清单** ✓。口径 = "**用户能在 `by` 块里
> 写出来的关键字**"，权威来源是 `parser.rs::parse_tactic_inner` 的 match 臂
> （`parser.rs:1384-1560`）与白名单 `is_tactic_keyword`（`parser.rs:3569-3589`），
> **不是** `Tactic` 枚举（枚举少一条，见 §1.2）。

| # | 关键字 | AST 落点 | 引擎落点（`by.rs`） | 文档文件 |
|---|---|---|---|---|---|
| 1 | `intro` | `Tactic::Intro` | `:1563-1661` | `intro.md` |
| 2 | `exact` | `Tactic::Exact` | `:1662-1666`（→ `exact_tactic` `:3155`） | `exact.md` |
| 3 | `apply` | `Tactic::Apply` | `:1977-1990` | `apply.md` |
| 4 | `assumption` | `Tactic::Assumption` | `:1876-1904` | `assumption.md` |
| 5 | `rfl` | `Tactic::Rfl` | `:1905-1976` | `rfl.md` |
| 6 | `match` | ⚠ **无自己的变体** ⇒ 解析成 `Tactic::Exact { expr: Expr::Match }`（与值位 `match` 同一个 `Expr`） | 走 `Tactic::Exact` 那条 | `match.md` |
| 7 | `constructor` | `Tactic::Constructor` | `:1784-1799` | `constructor.md` |
| 8 | `left` | `Tactic::Left` | `:1800-1815` | `left.md` |
| 9 | `right` | `Tactic::Right` | `:1816-1831` | `right.md` |
| 10 | `use` | `Tactic::Use` | `:1832-1852` | `use.md` |
| 11 | `exfalso` | `Tactic::Exfalso` | `:1853-1875` | `exfalso.md` |
| 12 | `cases` | `Tactic::Cases` | `:1991-2008` | `cases.md` |
| 13 | `have` | `Tactic::Have` | `:1669-1783` | `have.md` |
| 14 | `sorry` | `Tactic::Sorry` | `:2009`（**不动作**：目标保持开放） | `sorry.md` |

**三条边界（写进判据，别靠嘴说）**：
* `with` **不是** tactic：它是 `cases … with` 的连接词（`parser.rs:3571-3589` 没有它）⇒
  **不进基线** ✓。
* `match` ⚠ **是本表里唯一"关键字 ≠ AST 变体"的一条**（§1.2 第 ③ 条）⇒
  **覆盖判据不能只比枚举**（只比枚举会让 `match` 永远漏项 ✗）。
* `exact?` / `intros` / `simp` / `rw` … **本语言没有** ⇒ **不进基线**；
  P4 的文档**不许**写"本语言支持 `exact?`"这类假话 ✗。

### 1.2 ⭐ tactic 清单今天有【六份】表示 + 一条已存在的断言链

> ⚠ **本条是设计自审后的更正版**（初稿写"五份"、并打算"取枚举变体名做判据" ——
> **那条判据在 Rust 里实现不了**，见 §12 的第 1 条更正）✓。

| # | 表示 | 位置 | 覆盖 | 漏一条会怎样 |
|---|---|---|---|---|
| ① | `Tactic` 枚举 | `ast.rs:331-411` | **13**（`match` 无自己的变体） | 引擎/`span()`/`render_tactic` 的 match 不穷尽 ⇒ **编译失败** ✓ |
| ② | `Tactic::span()` 的穷尽 match | `ast.rs:435-454` | 13，**无通配臂** | 编译失败 ✓（今天**唯一**能咬住"新变体"的守卫） |
| ③ | parser 白名单 `is_tactic_keyword` | `parser.rs:3569-3589` | **14**（**真源** ✓） | 无判据 ⇒ 静默漂移 ✗ |
| ④ | `semantic::KEYWORDS`（编辑器词表） | `semantic.rs:42-92` | **13** —— ⚠ **`sorry` 不在里面，且这是【有意设计】**：它被归成 `SemanticKind::Hole`（`semantic.rs:866-867`，有判据 `:999`/`:1419`）| 编辑器里不着色/不补全该条 ✗；但**`sorry` 属于 Hole 是有意的**，判据**不许**要求它出现在 `KEYWORDS` 里（否则假红 ✗） |
| ⑤ | `render_tactic`（tactic 回显） | `proof.rs:658-703` | 13，**无通配臂** | 编译失败 ✓。⚠ 它把 `match` 回显成 **`exact (match …)`**（既有行为，本文只记不改 ✓） |
| ⑥ | ⚠ **"未知 tactic" 错误文案里的白名单**（手写字面量） | `parser.rs:1546-1548` | **13 —— 漏了 `have`！** | **无任何判据覆盖它**（全仓零断言）⇒ 这个漏项**已经在代码里存在**，且**没有任何闸会红** ✗✗ |
| ⑦ | **TM 语法词表**（编辑器高亮） | `editor/vscode/syntaxes/sokonanoda.tmLanguage.json` | 13 tactic（`sorry` 另有 1 处） | **已有判据**：`crates/cli/tests/extension.rs:1240-1261` 断言 **TM 词表 == `KEYWORDS` + `sorts` + `forall/∀`**（逐词排序后 `assert_eq!`）✓ |

**⑥ 是一条真 bug（本轮发现的既有缺陷，用户可见）**：学习者打错一条 tactic 时，
唯一的提示是这条错误文案；它列了 13 条而**没有 `have`** ⇒ 用户会以为 `have` 不被支持 ✗。
`have` 是 L3.6 加的（`ast.rs:392-405`），加它时**四个副本都补了**（白名单/枚举/
`render_tactic`/`KEYWORDS`），**只有这条文案没补** —— 因为只有它没有判据 ✗。

**⑦ 是一条免费的断言链** ⇒ 本设计白拿一层保护：
`TM 语法 ↔ KEYWORDS ↔ TACTICS(新表)` 一旦接起来，**加一条 tactic 就必须同时**补
TM 语法词表、编辑器词表、文档表三处，否则既有判据（`extension.rs:1240-1261`）
或新判据立刻判红 ✓。**本轮 14 条都已在 TM 语法里** ⇒ 不必改那个文件，
但 **P1 的判据要显式覆盖这条链**（把"不必改"变成"改了就必须一起改" ✓）。

**⇒ 结论（本设计要建的机制，六件事）**：
1. **新增第 ⑧ 份** —— `front::tactics` 的 `TACTICS` 文档表（14 项，`include_str!` 内嵌正文）；
2. **③ 从表派生**（`is_tactic_keyword(name) = TACTICS.iter().any(|t| t.name == name)`）
   ⇒ ⭐ **"加了 tactic 忘了补文档"在构造上不可能**：`parse_by_block`（`parser.rs:1227-1248`）
   只在 `tactic_keyword_ahead()`（`:1331-1333`，走同一个 `is_tactic_keyword`）为真时
   才解析 tactic ⇒ **表里没有的关键字根本不被当成 tactic**（`by` 块直接收空、
   随后按普通表达式解析而报错）⇒ 功能用不了、不是"能跑但没文档" ✓；
3. **⑥ 从表派生**（错误文案由 `TACTICS` 拼出来）⇒ 顺手**修掉那条真 bug**，
   并补一条断言文案含全部 14 条（今天零覆盖 ✗）；
4. **④ 保持 `const`，不派生**（理由见 §4.2）⇒ 用**双向判据 + 一条具名的 `sorry` 例外**钉住；
5. **①②⑤ 交给编译器**（穷尽 match，今天已经在兜）⇒ 判据只需覆盖 ③④⑥⑧ 与⑦那条链；
6. **一条独立的、编译之外的守卫**（Python 源码 lint，第二意见）⇒ 见 §4.2 末条。

### 1.3 ⭐ tactic 的语义与**用户实际看到的消息**（逐字，已用内核逐条探过）

> ⚠ **第三列的每一条都是实测的**：把"复现片段"（见本节末）喂给真内核
> （`scripts/soko grade <临时文件>`，或 MCP 工具 `mcp__sokonanoda__check` 直接喂文本），
> 抄回 `failed[].message` **原文** ✓。错误码一律 **`elab-tactic-failed`**（`elab` 阶段）。
>
> ⚠⚠ **本表是「2026-10-10 这一天」的实测快照，不是契约** ✗ ——
> **凡是别的线正在改的行为，写文档时必须重量一次**（见下表与 §7 边界 9）。
> **P3–P5 的 `## 常见错误与出路` 直接从这一列复制**，不许改写、不许按 Lean 的印象写 ✗。

| tactic | 内核规则（真相） | **用户实际看到的**（逐字） | 出处 |
|---|---|---|---|
| `intro` | 目标逐层剥 `Pi`/`Arrow`（`peel_pi_delta`，delta 展开最多 4 层）⇒ lambda 引入；**顺便改名**（`spine::rename_free`）免得目标里留悬空名 | `` `intro` 需要一个函数目标（… -> … 或 forall …），当前目标不是函数 `` | `by.rs:1635` |
| `exact` | 交 `e` 给内核，判它类型与当前目标 **defeq** | `` `exact` 类型不匹配：期望 `a`，实际是 `Prop` ``（**期望/实际是内核 pp 出来的**，逐例不同） | `by.rs:3191` + `mismatch_message`（`by.rs:3000-3010`） |
| `apply` | **目标匹配对齐到判等**（G-109 已落地 2026-10-10）：结论与目标各自把**头递归展开**（多轮 delta，最多 4 轮，逼近 whnf）后按实参位对齐；对不上时**内核再判一次**这条应用是不是本来就证明了目标（defeq）⇒ 两层都不过才报错。**"是不是相等"永远归内核** ✓ | `` `apply` 的目标不匹配：`And.intro` 的结果是 `a ∧ b`，无法对齐当前目标 `True` ``（**逐字未变** ✓，只是**原因**从"不做定义展开"变成"递归展开后仍不对齐"） | `by.rs:2291-2313`（递归展开）/`:2331-2349`（内核兜底 + 报错） |
| `assumption` | ⚠ **今天只判"最新"那条假设**（`judge_strict` 把候选列表塌缩成一个 ⇒ **未修缺陷**；Lean 4 的 `findLocalDeclWithType?` 搜遍全部局部上下文）—— **应有**语义是"找与该目标 defeq 的那条、逐个交内核判" | `` `assumption` 没有找到类型与目标一致的假设 `` | `by.rs:1899`（报错）+ `judge_strict`（候选塌缩） |
| `rfl` | 目标 `Eq α x y` ⇒ 造候选 `Eq.refl.{u} α x`；目标 `Iff A B` ⇒ `Iff.refl`；**两边是否相等一律内核判**（只认 `Judgement::Match`） | 形状不对：`` `rfl` 需要目标是 `Eq α x y` 或 `Iff A B` 形状（`Iff` 只在两边定义上相等时过）；当前目标是 `True`。目标是 `↔` 时可先 `constructor` 再分别证两个方向 ``／两边不等：`` `rfl` 判定失败：两边不相等（期望 …，实际 …） `` | `by.rs:1936` / `by.rs:1962` |
| `match` | 解析成 `Tactic::Exact { expr: Expr::Match }` ⇒ **臂体是项**，递归子与 iota 走**既有 match 降低路径** | **与 `exact` 同一条**（同一个 `Tactic::Exact` 分支） | — |
| `constructor` | 按目标头选**第一个**构造子（`ctor_tactic(0, …)`），等价 `apply <Ind>.<第一个构造子>` | **只有一条**：头不是归纳 —— `` `constructor` 需要目标是**归纳类型**，但当前目标的头 `a` 不在归纳表里（它可能是公理、定义，或者是一个函数目标——先 `intro` 拆开试试） ``（取下标 0 ⇒ 归纳非空就不可能"构造子不够"） | `by.rs:3334-3375` + `:3404-3441`（`nth_constructor`，2026-10-10 实测） |
| `left` | 与 `constructor` **同一台机器**（`ctor_tactic(0, …)`）：取目标归纳的**第 1 个**（`Or` 上是 `Or.inl`）—— **不要求 ≥2 构造子**：单构造子归纳（`And`）上也成功，只把字段留成目标 | **只有"头不是归纳"这一条**（`{what}` = `left`，实测：`` `left` 需要目标是**归纳类型**，但当前目标的头 `a` 不在归纳表里（它可能是公理、定义，或者是一个函数目标——先 `intro` 拆开试试） ``） | 同上 |
| `right` | 同 `left`，取**第 2 个**（`ctor_tactic(1, …)`；`Or` 上是 `Or.inr`） | **两条**：头不是归纳（同上，`{what}` = `right`）／构造子不够：`` `right` 需要目标至少有 2 个构造子，但 `And` 只有 1 个 ``（**只有下标 1 的 `right` 能产生**，实测） | 同上 |
| `use` | 取目标归纳的**第 1 个**构造子（`ctor_tactic(0, …)`）⇒ 交证人位；等价 `apply <ctor>` 后立刻 `exact w`（**复用 `exact_tactic`**，判定完全同源）。⚠ **多构造子的目标也取第 1 个** —— 不是"只支持单构造子" ✗（P3 实测：`use ha` 在 `a ∨ b` 上过 ✓） | 同 `constructor`（`{what}` = `use`）；证人类型不对时另给 `exact` 那条 | `by.rs:1873-1893`（`Tactic::Use`）+ `by.rs:3334-3375`（`ctor_tactic`） |
| `exfalso` | 目标换成 `False`（原目标记在组装里）；等价 `apply False.elim` | ⚠ **自己不会失败**（实测：目标换成 `False` 一定成功）⇒ 失败发生在**之后**那一步（谁去证 `False`） | `by.rs:1853-1875` |
| `cases` | **降低成 `Expr::Match`**：每臂的 tactic 序列各自组装成项 ⇒ 递归子/iota 由既有 match 路径处理（引擎**不手搓 recursor**） | **六条**（实测全部可复现）：不是局部假设名：`` `cases` 的被消去项必须是一个**局部假设名**（例如 `cases h`） ``／读不到类型：`` `cases` 读不到 `h` 的类型 ``／不是归纳值：`` `cases` 的被消去项不是归纳类型的值：`Prop → Prop` ``／头不在归纳表：`` `cases` 只支持**归纳类型**，但 `h : True` 的头 `True` 不在归纳表里 ``／**不支持 dependent elimination**：`` `cases a` 暂不支持**目标依赖该假设**的情形（dependent elimination）：目标 `a` 里提到了 `a` ``／分支名数不对：`` `cases` 分支 `…` 需要 N 个假设名（构造子有 N 个字段），实际给了 M 个 `` | `by.rs:2471`/`:2479`/`:2585`/`:2660`/`:2670`/`:2740` |
| `have` | 目标**不变**、上下文多一条 `h : T`；降低成 `(fun (h : T) => <rest>) t`；值算完**立刻**判一次 `value : T`（defeq） | `` `have h` 的值类型不匹配：期望 `False`，实际是 `True` ``（**带 `have` 后面那个名字**）／`` `have h` 判定失败：洞不在可填写的位置 ``（**只有** `have h : T := by sorry` 这种嵌套写法能产生，P5 实测） | `by.rs:3217` 那条 `mismatch_message`（`what` = `` `have h` 的值类型不匹配 ``）；第二条是 `judge.rs:3681` 经 `Tactic::Have` 的 `Judgement::Error` 分支 |
| `sorry` | **不动作**：当前目标保持开放（**合法状态**，不是错误） | **没有**（它不失败）；"多余 `sorry`"那条判据在 `docs/design/redundant-sorry.md` | — |

**复现片段（每条一行；把它们喂给内核就能看到上表的原文）** ——
`intro`：`example : True := by` + `intro h` · `exact`：`example (a b : Prop) : a := by` +
`exact b` · `apply`：`example : True := by` + `apply And.intro` · `assumption`：
`example (a b : Prop) : a := by` + `assumption` · `rfl`(形状)：`example : True := by` + `rfl` ·
`rfl`(不等)：`example (a b : Prop) : a = b := by` + `rfl` · ctor 族：
`example (a : Prop) : a := by` + `constructor`/`left`/`use a` · `cases`(依赖)：
`example (a : Prop) : a := by` + `cases a` · `cases`(非归纳)：`example (a : Prop) (x : a) : True := by` + `cases x` ·
`have`：`example : True := by` + `have h : False := True.intro` + `exact True.intro` ✓

⚠ **两条从实测里看到的、要在文档里如实交代的**（**不美化、也不隐瞒** ✗）：
1. **`rfl` 的"两边不相等"消息里会出现内核 pp 记法**，例如
   `` … 期望 `Pi (a : Sort(0)), … (((Eq.[1] Sort(0)) 第 1 个绑元（a）) 第 0 个绑元（b）)` … ``
   —— `第 N 个绑元（x）` 对学习者不可读 ⇒ **`rfl.md` 的 `## 常见错误与出路` 要翻译一句**
   （"它把两边的类型原样打印出来了，`第 N 个绑元` 就是在说第几个假设"）✓。
   （**是否要改内核 pp 是另一件事、不归本设计** ✗ —— 同 §7 边界 9 的归属口径：
   行为缺陷属**修复**范畴，文档只负责如实描述当天的行为。）
2. **`exfalso` 自己没有失败模式** ⇒ 它的文档要写清"失败会出现在下一步"，
   不能编一条"`exfalso` 报错"的假消息 ✗。

**⚠ 一条曾经「正在被别的线改动」的行为（已落地，2026-10-10）**：

| tactic | 旧读数（快照）→ **现在** | 谁改的 | 本设计怎么办 |
|---|---|---|---|
| `apply` | 旧读数是"**只做位置 spine 合一**"、目标头不匹配就报 `` `apply` 的目标不匹配：… ``（`by.rs:2248`）—— 现已改为**对齐到判等**（同本节 `apply` 行：递归展开 + 内核兜底，`by.rs:2291-2349`）✓ | **修复线 `d9a701e4` —— 已落地 2026-10-10**（用户澄清：`mem_of_subset_singleton` 里 `apply Eq.refl a` 报"目标不匹配"、期望 `apply` 支持**定义展开**对齐 Lean 4 ⇒ 那是**内核/tactic 行为缺陷**，属**修复**范畴 ✓） | ⭐ **本阶段已按新行为收口**：§1.3 的 `apply` 行、§4.5 的注、R20 全部改写；`apply.md` 由 **P3** 写、动笔前**重新喂过内核**（`apply Eq.refl a` 过 ✓、报错文案**逐字未变** ✓）——**不许照抄旧读数**这条纪律对**任何**后续行为改动仍然成立 ✓ |

**为什么这条单列**：文档写的是"**你按下去会发生什么**"，而那条线**当时正在改**"会发生什么"（**已落地，见上表**）⇒
照抄本表就会交付一篇**在描述旧行为**的文档 ✗。同一条纪律对**任何**后续行为改动都适用
（本表所有读数都有这个性质，`apply` 只是**已知**的那一个）✓。

**三条最容易写错的（文档必须照抄本表，不许按 Lean 直觉写）**：
1. `rfl` **不是**"两边语法相同"，是**内核 defeq**（`by.rs:1942-1951`）；
   `Iff` 也只在两边 defeq 时过（`by.rs:3434-3436`）。
2. `exact` 的"统一"是**内核 defeq**，失败时给的是内核 pp 出的 expected/actual
   两个文本（`by.rs:1956-1965` 是 `rfl` 的同形兄弟）。
3. `intro` 能过**delta 展开后**才成为函数的目标（最多 4 层）⇒ 学习者**不需要**
   先 `unfold`；这条不写，文档就在教一个不必要的仪式 ✗。

### 1.4 LSP 的 hover / 补全 / F12 能力（`crates/lsp/src/lib.rs`，3354 行）

* **capabilities**（`lsp/lib.rs:2253-2290`）：`hover_provider`、`definition_provider`
  （`OneOf::Left(true)`）、`inlay_hint_provider`、`completion_provider` 都已广告 ✓。
* **hover 链**（`lsp/lib.rs:2417+`）是有序的，顺序本身就是契约：`command_line_hover`
  ⇒ **`tactic_goal_hover`** ⇒ `half_expression_goals_hover` ⇒
  `notation_symbol_hover` ⇒ 记法目标名 ⇒ …（每条都有 `SOKO_HOVER_TRACE` 探针）。
  `tactic_goal_hover`（`lsp/lib.rs:1405-1475`）拿 `by_steps` 里**进入**该 tactic 的
  目标快照，渲染成"tactic 行 + goal state"；名字上再加分割线 + 类型行
  （`lsp/lib.rs:1462-1467`，契约在 `docs/protocol.md:303-331`）。
* **关键字闸门的位置**（`lsp/lib.rs:2545-2551`）：`SemanticKind::Keyword` ⇒ `Ok(None)`。
  它排在 `tactic_goal_hover` **之后** ⇒ tactic 关键字（`intro`/`exact`/`rfl`…）
  今天**已经**有 goal state，而 `def`/`theorem`/`fun`/`=>` 这类关键字 hover **静默** ✗。
  **① 只动 tactic 那 14 条**：闸门一个字不改（改它 = 顺手放出一堆关键字 hover，
  那是**另一个需求**，见 §7 边界 7）✓。
* **`match` 的双重身份**（P1/P3 必须先定死）：`match` 同时是**项位关键字**
  （`is_expr_keyword`，`parser.rs:3565-3567`）与 **tactic**。它今天**已经**在 `by`
  块白名单里（`parser.rs:3579`），所以表里**有**它；但 P3 写它的文档时要**同时**
  讲两种用法（`by match` 与值位 `match`），且 `## 内核在背后判什么` 那一节写
  「臂体降成 `Expr::Match`，递归子/iota 走既有 match 路径」（`ast.rs:382-391`）✓。
* **F12**：`goto_definition`（`lsp/lib.rs:2785-2969`）今天有六段前置分支，顺序是
  ① 闭包记法表（`notation_at`）② 内建记法兜底（`builtin_declaration_span`）
  ③ 记法目标名（`notation_target_at` → `project_definition` → `prelude_def_span`）
  ④ `definition_at`（hover span + 声明表）⑤ prelude 名字 ⑥ 本文件声明。
  **tactic 关键字在这些分支里一个都不命中**（它不是名字、不是记法）
  ⇒ `definition_at` 返回 `None` ⇒ 落到 prelude 分支也返回 `None` ⇒ **F12 无声** ✗。
  这与 2026-10-10 刚修的「内建记法第二跳」是**同一类缺陷、同一处插入点**（`lsp/lib.rs:2848-2912`）。
* **落点怎么拿到一个真实文件**：`sokonanoda_front::compile::prelude_source_path()`
  （`crates/front/src/compile/prelude.rs:484-545`）三段兜底 —— ① 仓库里的真源
  （`CARGO_MANIFEST_DIR` 相对，**编译期**路径 ⇒ 发布产物上不存在）② 平台缓存目录
  ③ 系统临时目录（**两条都幂等**：内容一样不重写，避免每次 F12 动 mtime 让编辑器
  反复重载）。**tactic 文档照抄这一条** ⇒ 发布产物与开发树都能落到真文件 ✓。
* **`Url::from_file_path` 是本仓唯一的落点形态**（`prelude.rs:486-489` 明写：
  "零 `TextDocumentContentProvider`、零自定义 scheme"）⇒ 文档**必须是磁盘上的真
  `.md`**；`untitled:` 之类要同时改 LSP 与扩展两侧（含 stub 宿主）✗ 不做。

### 1.5 编辑器能不能打开本地 markdown

* 扩展 `dependencies` 只有 `vscode-languageclient`（`editor/vscode/package.json:368-376`），
  **没有任何 markdown 渲染库**（`marked`/`markdown-it` 都没有）✓ ⇒ **不自造渲染器**。
* 扩展是 `extensionKind: workspace`（`package.json:19-21`）⇒ 远端场景下扩展与 LSP
  同侧，`file:` 落点是**扩展这一侧能打开**的 ✓。
* ⇒ **最小且正确的一跳 = 返回 `file:` 指向一个真 `.md`，让 VS Code 自己的 Markdown
  预览接手**（F12 打开编辑器 + 预览即可）✓。**不在本轮**新增 webview/渲染器
  （那条路要动 CSP、nonce、stub 宿主与两条 e2e，成本与收益不成比例，见 §6 风险 R3）。

### 1.6 文档纪律（新文件要过哪些闸）

| 闸 | 管什么 | 对 `reference/**` 生效吗 |
|---|---|---|
| `scripts/docs-lint.py` ①–⑧ | 活文档总量 / 单文件行数 / 入口行数 / `docs/design` 新增 ≤150 / 垃圾 / 归档索引 / 四层行数 / 过期日期 | **不生效**：`live_docs()` 只收"根 `*.md`"与 `docs/**`（`docs-lint.py:103-108`） |
| `scripts/docs-expiry-check.py` | 每份活文档的过期日（`live_docs()` 同口径，`docs-expiry-check.py:78-88`） | **不生效**（同上） |
| `docs/design/**` 预算 | 新文件 ≤150 行，登记后**只许减不许增**（`docs-lint.py:52,203-215`） | 本文自己受它管 ⇒ §8 的预算表 |

**⚠ 这条"不生效"不是免费通行证** ✗：`reference/` 是**产品内容**，靠 §5 的**自己的**预算闸（每篇 ≤120 行、总量 ≤1200 行）管，且**必须在 CI 里真跑** ✓（AGENTS.md：「咬不住的守卫等于没有」）。

⚠ **再加一个目录，也要说清它为什么不需要过期/预算登记**：§4.9 D10 的 `editor/vscode/docs/tactics/`（随 VSIX 发的**拷贝**）—— 它 ① 不在 `docs/**` 下、② 是**生成目录且 gitignored**（照 `bin/` 的先例）
⇒ **`docs-lint` 与过期机制都扫不到它**，而这是**对的** ✓：它是**派生物**（源是 `reference/tactics/`），生命周由**打包脚本 + `--check` 漂移判据**管
（R22），不该有独立的过期日 ✗ —— **给它登记过期日等于承认它是一份独立文档**，那正是"两份源"的开始 ✗。

## 2. 需求 ↔ 落点（一句话各一条）

| 需求 | 落点 | 用户屏幕上多什么 |
|---|---|---|
| ① hover 显示简短说明 | `tactic_goal_hover` 里追加摘要（插入点见 §4.1 ①） | **全部 14 条** tactic 关键字的 hover 里多一段（goal state **之上**、**仍是同一张卡片**） |
| ② F12 跳文档 | `goto_definition` **新增一段前置分支**（插在 `lsp/lib.rs:2848` 那段之前、`definition_at`（`:2913`）之前） | 光标在**任一** tactic 关键字上按 F12 ⇒ 打开 `reference/tactics/<关键字>.md` 并停在标题行 |
| ③ 完整介绍文档 | 新增 `reference/tactics/**`（**14 篇 + 1 篇索引**） | 每篇 markdown：怎么用 / 什么时候用 / 内核在背后判什么 / 常见错误与出路 / 相关 tactic |
| ④ **范围 = 全部 tactic**（用户 2026-10-10 补充） | **文档覆盖基线** = §1.1 的 14 行清单；阶段按族切（§5 P3–P5） | 没有"某条 tactic 点进去没文档"的漏项 ✓ |
| ⑤ **防漏项机制**（用户 2026-10-10 补充） | §5 P1 的覆盖判据（并集 ↔ 表**双向**比对，进 `soko gate` + CI） | 以后新增 tactic 忘了补文档 ⇒ **CI 判红**，不会静默漏 ✓ |
| ⑥ **随插件打包分发**（用户 2026-10-10 补充） | §4.9 D10：文档**也**以真文件进 VSIX（`editor/vscode/docs/tactics/`）+ 扩展经 `SOKONANODA_DOCS_DIR` 交给 LSP | 装插件即自带完整文档；`unzip` VSIX 能看见那 14 篇；**离线、零外部站点、零额外拉取** ✓ |

## 3. 决策 D1：文档放在 `reference/tactics/`（**新目录**），不放 `docs/`

**理由（三条，都是可核的）**：
1. **身份不同**：它是**产品内容**（`include_str!` 进二进制、随 VSIX 发出去、被 `reference/` 之外的课程与技能引用），而 `docs/` 按 `docs/README.md:110-113` 是
   开发者台账（含 CI 失败记录与内部取舍）。把教学文档塞进开发者台账 ⇒ 两类读者、两套寿命混在一处 ✗。
2. **预算语义不同**：`docs/design/**` 的新文件被 `docs-lint` ④ 卡在 150 行（`docs-lint.py:52`），而这 14 篇是**要长期随语言长大的教学内容**（每加一条
   tactic 就加一篇）⇒ 硬塞进 `docs/design/` 必然长期顶预算、逼出"抬上限"这种被用户点名禁止的动作（`docs-budget.json:_comment`：「**不许靠抬上限达标**」）✗。
3. **不污染用户的工作区**：`docs/` 是仓库里的开发者面；`reference/` 与 `prelude/`（2026-10-08 E1 落的"真源目录"，`prelude/Prelude.sokonanoda`）同级，
   语义清楚：**这两个目录都是"随产品发出去的内容源"** ✓。

**代价与对策**：`reference/**` 不被 `docs-lint` / 过期机制覆盖 ⇒ **必须自己带闸**
（§5 的判据 + §6 的验收），否则就是"躲在闸门外长胖" ✗。

## 4. 决策 D2–D10

### 4.1 D2：hover 上**加一行**，并**有意**改掉那条"逐字节不变"判据

* 加什么（Markdown 纯文本，不引入新围栏、不新起段落）—— **P2 定稿**（原草案的
  `· F12 看全文` 与 §4.5 不一致 ⇒ 已按 §4.5 统一）：`` **`<关键字>`** — <§4.5 那一句>
  · 完整文档：`F12` ``，整行一个段落，已写进 `docs/protocol.md`。**设计上定死的是这两条**：
  ① **插入点** = 现有输出里 **tactic 行之后、goal state 之前**；
  ② **现有内容一个字不动** —— 把新的那几行从输出里删掉，
     剩下的必须与改动**逐字节相同**（`APPLY_KEYWORD_BASELINE` 就是那个"剩下的"）✓。
* **为什么不另起 hover**：hover 链是有序单返回（`lsp/lib.rs:2446-2475`），新起一段
  只会把 goal state **挤掉** ⇒ 回归 ✗。用户要的是"简短说明"，不是"用说明换掉目标状态"。
* ⭐ **冲突判据处置（明写；这是需求的必然结果，不是可选项）**：`hover.rs:1232` 今天断言
  关键字上**整段**等于 `APPLY_KEYWORD_BASELINE`，而需求 ① 要"代码区域 hover 显示简短说明"
  ⇒ 两者不可能同时成立 ⇒ **该断言必须改成上面 ①② 的形状**（保留本意"不许顺手改 goal state
  那份渲染"、换掉"整段逐字节"的口径；P2 已落并更名，见 §5 P2）。反向验证：把新加的那一行
  删掉 ⇒ 新断言必须判红 ✓（AGENTS.md：「守卫必须能咬住已知的历史 bug」）。
  ⚠ **`APPLY_KEYWORD_BASELINE` 不删**（`hover.rs:1092-1105`）：它降级成"**goal state 那一段**"的期望值，继续咬住那份渲染 ✓。
* ⭐ **`sorry` 也给摘要 —— 本轮已定，不再留"P2 二选一"** ✓：用户补充把范围写成"**【所有】**
  tactic …… 一个都不能少"，而 `sorry` 就在 §1.1 的 14 条里 ⇒ **它有一句**（§4.5 末行）。
  **别把两件事混起来** ✗：`NON_NAME_TACTIC_WORDS`（`lsp/lib.rs`）讲的是"**不给它名字类型行**"
  （`#check sorry` 会被内核按 `elab-hole-misplaced` 拒掉），与"**tactic 摘要**"无关。

### 4.2 D3：`front::tactics` 表 = tactic 的单一真相（四层机制，逐层可核）

**表本身**：`crates/front/src/tactics.rs`
```rust
pub struct TacticDoc {
    pub name: &'static str,        // 关键字（= §1.1 基线表的「关键字」列）
    pub summary: &'static str,     // 一句话（hover 摘要 + 补全 detail；§4.5 定稿）
    pub variant: &'static str,     // AST 落点名（判据用；`match` 写 "Exact"）
    /// 编辑器把它归成什么（**具名例外用**，§4.2 L3② / R17）：
    /// 13 条是 `SemanticKind::Keyword`，**`sorry` 是 `SemanticKind::Hole`**。
    /// 用**已有的枚举**（`crate::semantic::SemanticKind`），不另造一个 ✗。
    pub semantic_kind: SemanticKind,
    pub markdown: &'static str,    // include_str!("../../../reference/tactics/<name>.md")
}
pub static TACTICS: &[TacticDoc] = &[ /* 14 项 */ ];
```
⚠ **四个字段都有消费者，别省**：`name`（白名单派生 + 错误文案 + 判据）、
`summary`（hover/补全/索引/正文首句）、`variant`（判据与"覆盖基线"对账）、
`semantic_kind`（L3② 的具名例外）、`markdown`（L2 编译期 + F12 落点）✓。
* 位置在 `front`（不是 `lsp`）：要同时被 parser（白名单 + 错误文案）、
  `semantic`、LSP（hover/F12/补全）消费 ✓。
* `include_str!` 的**路径深度有先例**：`prelude.rs:178` 用
  `"../../../../prelude/Eq.sokonanoda"`（从 `crates/front/src/compile/`）；
  本表在 `crates/front/src/tactics.rs` ⇒ 少一层 = `"../../../reference/tactics/<name>.md"` ✓。
* **体积账**：14 篇 × 约 100 行 × 约 40 B ≈ **50–60 KB**，进 `front` 的每个二进制
  （CLI + LSP + 全部测试）。与 prelude 的三个 `include_str!`（同量级）并列，可忽略 ✓；
  P3 的预算判据（总量 ≤1200 行）同时也是这个数字的上界 ✓。

**四层机制**（每层各挡一类漏项；**没有一层依赖 Rust 反射** —— §12 第 1 条更正）：

| 层 | 机制 | 挡什么 | 成本 |
|---|---|---|---|
| **L1 构造** | `is_tactic_keyword(name)` **派生自** `TACTICS`（`parser.rs:3571-3589` 的 match 换成查表） | ⭐ **"加了 tactic 却没进表"**：表外的关键字不被认作 tactic（`parse_by_block` 直接收空、`tactic_keyword_ahead` 为假）⇒ 功能不可用，而不是"能跑但没文档" ✗→✓ | 一处函数体 |
| **L2 编译期** | `markdown: include_str!(…)` | **"表里有行、文件不在"** ⇒ **编译失败** ✓（比运行时 `open()` 找不到再静默强得多） | 0（宏自带） |
| **L3 判据（Rust）** | ① 手写清单（**14 条字面量**）== `TACTICS` 名字集合（**双向**）；② `KEYWORDS` ∩ 名单 == 名单 ∖ {`sorry`}（**具名例外**，理由见下）；③ **行为判据**：表里每条都**不许掉进**兜底臂（兜底臂今天不可达，见 §12 第 6 条）+ 文案本身由 `parser.rs` 单元测试判；④ 每篇正文的文件名 == `name`、且含 **5 个** `##` 小节 | **"绕开 `is_tactic_keyword` 直接往 `parse_tactic_inner` 里加 `kw == \"…\"` 臂"**（那时 L1 不生效）；④ 挡"文件在但正文是空壳" | 一个测试文件 |
| **L4 源码 lint（Python，编译之外）** | `scripts/tactic-docs-lint.py`：从 `tactics.rs` 抽 `name: "…"`、从 `parser.rs` 的 `parse_tactic_inner` 区间抽 `kw == "…"` 字面量、从 `reference/tactics/*.md` 抽文件名，**三者比对** | **第二意见**：一侧在 Rust 编译/测试侧、一侧在文本侧，**任一侧漂移都红**；也覆盖"忘了跑 cargo 就先提交"的情形 | 一个小脚本（进 `soko gate` + CI） |

**为什么 L3① 要用手写清单**：Rust **没有反射**（无 `strum`/`variant_count`，
本仓 `Cargo.toml` 也没有）⇒ 判据**枚举不出** `Tactic` 的变体名 ✗。
手写清单是**唯一**可行形态，而且**本仓已有同一形状的先例**：
`crates/lsp/src/tests/hover.rs:1374-1389` 就手写了全部 14 个关键字做逐个断言 ✓
（照抄那个形状即可，不发明新写法）。
这道判据咬合的链条是：**手写清单 == `TACTICS`** 且 **`TACTICS` 每行都有正文文件**（L2 + L3④）
⇒ 改了 parser 而没补文档 ⇒ 判红 ⇒ 补文档 ⇒ 文件必须存在且非空壳 ✓。

**为什么 L3② 给 `sorry` 一个具名例外**：`sorry` 在**白名单里**（是真 tactic，
`parser.rs:3583`）但在 `KEYWORDS` 里**故意没有**（它是 `SemanticKind::Hole`，
`semantic.rs:866-867` + 判据 `:999`/`:1419`）✗ 一个"名单 ⊆ KEYWORDS"的粗糙判据
会**假红**。⇒ 例外必须**具名写死**（`TACTICS` 里给 `sorry` 一个字段标
`semantic_kind: Hole`，判据按字段分流）——**不许**写成"跳过所有不在 KEYWORDS 里的项"
（那等于把判据阉掉 ✗）。

**`KEYWORDS` 为什么不派生**：`semantic::keywords()` 的签名是
`-> &'static [&'static str]`，而 `crates/cli/tests/extension.rs:1247-1255` 直接拿它
排序后与 TM 语法比 ⇒ 派生要么改公开签名（牵动 `lsp` 与那条断言），要么引入
`LazyLock`（把一个纯常量表变成运行时构造）✗。**收益为零**（L3② 已经双向钉住）⇒
保持 `const` ✓。

**⑥ 那条真 bug 顺手修**：错误文案改为由 `TACTICS` 拼（`parser.rs:1546-1548`），
判据见 L3③。**这是本设计附带修掉的一个既有用户可见缺陷**（§1.2 ⑥）✓。

**⑦ 那条既有断言链白拿**：`TM 语法 ↔ KEYWORDS` 的判据已经在
（`crates/cli/tests/extension.rs:1240-1261`）⇒ 接上 L3② 后，链变成
`TM 语法 ↔ KEYWORDS ↔ TACTICS` ⇒ **加一条 tactic 必须同时补三处**（TM 词表 /
KEYWORDS / 文档表），否则任一条判红 ✓。**本轮 14 条都已在 TM 语法里** ⇒ 不改那个文件，
但 P1 的判据清单里要写明"这条链已被覆盖"✓。

### 4.3 D4：补全弹窗顺带吃到同一份表（① 的免费收益）

`lsp/lib.rs:3108-3115` 的补全今天 `detail: None` ⇒ tactic 关键字只有名字（**13 条**：`sorry` 不在
`KEYWORDS` ⇒ 今天没有它的补全项，P2 **不**凭空加）。既有了表就补 `detail: Some(summary)` +
`documentation: Some(摘要 + 空行 + 「完整文档：`F12` 跳转」)`；同一数据源 ⇒ hover / 补全 / F12 / 文档**四处永不漂移** ✓（P2 已落）。

### 4.4 D5：F12 的触发条件与 `range`

**触发条件**（两个都满足）：① 光标 token 的 `Ident` **就是**表里某条 tactic 名
（走**语言自己的词法**，不文本扫描 —— 与 `tactic_name_at`（`lsp/lib.rs:1497-1517`）
同一条纪律）；② 该 token 落在**某条 `by` 步骤的 span 里**（`report.decls[].by_steps`，
与 `tactic_goal_hover`（`lsp/lib.rs:1412-1420`）**同一个选择器**）。
②的意义：`match` / `have` 在**项位**也是合法写法（`parser.rs:2564-2567` 的
`is_expr_keyword`），没有②就会把"值位 `match`"也当成 tactic 抢去跳文档 ✗
（`docs/design/notation-subset.md` 那类"同一个词两种身份"的坑，同族先例见
EG-04 的记法目标名）。

**`range` = 落点文档里 `# <关键字>` 标题那个名字 token 的 span**（不是整条 tactic、
不是源文件里光标那个 token 的位置 ✗ —— P6 落地时更正，见 §5 P6 的「设计更正」）。
理由：`Location.range` 是**目标文件**坐标，编辑器据此把光标停在打开的 `.md` 里、
并选中标题名 ⇒ 与"点了哪个词"一一对应；拿源文件的行列去答会把光标停到 `.md` 的
空白行 ✗（实测判红）。
⚠ **与 hover 的取舍相反且两处都要写进 `docs/protocol.md`**：hover 的 `range`
**保持整条 tactic**（`docs/protocol.md:329-331` 已定：在 tactic 任意位置都触发）
—— 两条相反的取舍**都是有意的**，不写清楚下一棒会以为其中一条写错了 ✗。

### 4.5 D6：14 条摘要**定稿**（P2 只负责落地，不再重新措辞）

> 为什么在设计阶段就把这 14 句定下来：它们同时是 **hover 摘要**（P2）、
> **补全弹窗的 `detail`/`documentation`**（P2）、**索引页的一行说明**（P1/P3–P5）
> 与**文档正文的第一句**（P3–P5）—— 一份文本、四处消费（AGENTS.md「单一数据源」）✓。
> 若留到 P2 再写，就会在四个地方各写一遍、然后开始漂移 ✗。

**文风**：照 `skills/sokonanoda-teacher/references/zh-style.md` 的正面规则
（**术语先行**、短句、先事实后判断）+ 红线（翻译腔四套路、路标词、破折号起手、
万能好评词、emoji）✓。读者是**正在证题、卡在某一步**的人 ⇒ 一句话说清
"**这一步在干什么**"，不写背景故事 ✓。

| # | 关键字 | 摘要（**定稿**，≤50 字，照抄即可） | 内核依据（§1.3） |
|---|---|---|---|
| 1 | `intro` | 引入假设：目标 `A → B`（或 `∀ x, …`）时，先假设 `A`，再证 `B`。 | 逐层剥 `Pi`/`Arrow`（含 delta 展开） |
| 2 | `exact` | 交出证明项 `e`；内核判 `e` 的类型与当前目标是否定义相等（defeq）。 | 交内核判 defeq |
| 3 | `apply` | 用一条函数的结论对上目标，它剩下的前提各自变成新目标。 | 目标匹配对齐到判等（G-109 **已落地 2026-10-10**：结论与目标各自多轮展开后再对齐，最后归内核判）⇒ 这一句是**机制级**描述，**仍然成立** ✓ |
| 4 | `assumption` | 在已有假设里找一条与目标定义相等的，直接结束当前目标。 | 应有语义 = 逐条交内核判 defeq；⚠ **今天只判最新那条**（**未修缺陷**，见 §1.3 与 `assumption.md`） |
| 5 | `rfl` | 自反：目标是 `=` 或 `↔`，且两边定义相等时成立。 | 造 `Eq.refl`/`Iff.refl`，内核判 defeq |
| 6 | `match` | 对项做情形分析，臂体写的是项（等价于 `exact (match …)`）。 | 解析成 `Tactic::Exact { Expr::Match }` |
| 7 | `constructor` | 按目标取第一个构造子，它的参数变成新目标。 | `apply <Ind>.<第一个构造子>` |
| 8 | `left` | 选目标归纳的第一个构造子（`A ∨ B` 上就是左边 `A`）。 | 同上 |
| 9 | `right` | 选目标归纳的第二个构造子（`A ∨ B` 上就是右边 `B`）。 | 同上 |
| 10 | `use` | 对 `∃ x, p x` 交出证人 `w`，接着去证 `p w`。 | `apply` 构造子 + 立刻 `exact` |
| 11 | `exfalso` | 把当前目标换成 `False`；原来要证的东西留到后面用。 | `apply False.elim` |
| 12 | `cases` | 对假设做情形分析，每个构造子一个分支。 | 降低成 `Expr::Match` |
| 13 | `have` | 在证明中间先证一条 `h : T`，当前目标不变。 | `(fun (h : T) => <rest>) t` |
| 14 | `sorry` | 占位：目标保持开放。练习没做完的合法状态，不是错误。 | 不动作；合法开放状态 |

**落地形态（P2 照抄，不再决策）**：

* **hover 里的位置** = tactic 行那个围栏块**之后**、`tactic i/n` 行**之前**：
  ```
  ```sokonanoda
  apply Set.ext
  ```
  **`apply`** — 用一条函数的结论对上目标，它剩下的前提各自变成新目标。
  tactic 1/2
  …
  ```
* **`F12` 那半句用纯文本，不做链接** ✗→✓：本仓**任何地方都没用 `isTrusted`**
  （`editor/vscode` 0 命中），而 VS Code 对**不受信任**的 hover markdown 会剥掉
  命令链接 ⇒ 放一个点了没反应的链接就是**说假话** ✗。纯文本写
  「完整文档：`F12`」即可 ✓（真正的入口是用户按 F12，不是 hover 里的装饰）。
* **补全项**（D4）：`detail = 摘要`；`documentation` = 摘要一行 +
  空行 + 「完整文档：`F12` 跳转」✓（Lean 4 在补全里放整段 docString，
  我们放摘要 + 指路：`reference/tactics/**` 是唯一正文，不复制到 wire 里 ✓）。

### 4.6 D7：物化路径 · 与 `clean` 的关系 · 写文档的重编代价

⚠ **档数已从三档扩到五档**（用户 2026-10-10 补充「文档要随插件打包」）⇒
**完整顺序与理由见 §4.9 D10** ✓；下表是**物化那两档**的细节（通道 ② 用）✓：

**物化路径，逐条照抄 prelude**（`prelude.rs:499-545` 的三段兜底）✓：

| 档 | 路径 | 什么时候命中 |
|---|---|---|
| ① 仓库真源（**开发树首选**） | `<仓库>/reference/tactics/<关键字>.md`（`CARGO_MANIFEST_DIR` + `../../../reference/tactics/` —— 与 `prelude.rs:178` 的四层先例同源，少一层） | 检出仓库时 ⇒ F12 落到**你正在编辑的那一份真文件** ✓ |
| ② 平台缓存 | `<cache::root()>/reference/tactics/<关键字>.md`（`cache.rs:81`：`SOKONANODA_CACHE_DIR` 优先，否则 macOS `~/Library/Caches/sokonanoda`） | 发布产物（VSIX / 缓存二进制）✓ |
| ③ 系统临时目录 | `<temp_dir>/sokonanoda-reference/tactics/<关键字>.md` | 缓存被禁（`SOKONANODA_NO_CACHE`）或**不可写**时的兜底 ✓ |

三档都写不进去 ⇒ **返回 `None`，F12 答"没有落点"** —— **绝不编造位置** ✗
（与 prelude 同一条纪律，P6 判据 ③ 钉住）✓。
写入**幂等**：内容一样就不重写（避免每次 F12 动 mtime ⇒ 编辑器反复重载）✓。

**⚠ `clean` 会不会把文档删掉？—— 不会（已实测）** ✓：
`sokonanoda clean` → `cache::clean()`（`cache.rs:377-378`）→ `clean_dir()` = **`<root>/compiled`**，
而 `clean_in`（`cache.rs:403-416`）**只删 `.json` 文件** ⇒
① 文档不在 `compiled/` 下、② 也不是 `.json` ⇒ **`clean` / 扩展的 `Rebuild`（`alt+shift+b`）
都不动它** ✓ —— 与平级的 `<root>/prelude/Prelude.sokonanoda` **同待遇** ✓。
**升级后内容变了怎么办**：幂等检查是"读出来与内嵌字节比"，不等就重写 ✓
（所以不需要 `clean` 参与，也不会留下过期文档 ✗）。

**⚠ 改一篇文档会重编 `front`（已实测）**：`include_str!` 的文件**进 cargo 的 dep-info** ——
`target/debug/libsokonanoda_front.d` 里逐字列着 prelude 那三份源
（`…/src/compile/../../../../prelude/{Eq,L1,Quot}.sokonanoda`）✓。
⇒ 加 14 篇 `include_str!` 之后，**改任何一篇 ⇒ `front` 重编（连带 `lsp`/`cli`）** ✓。
这是**设计上的预期**（文档是二进制的一部分、随产物发出去），但对 P3–P5 的
"写文档"循环有成本含义 ⇒
* 日常反馈用 `scripts/dev-verify.sh`（脚本头自己写着"**冷跑 ≤ 2 秒**"，`dev-verify.sh:13`）✓；
* **P1 要量一次**"改一篇文档 ⇒ 重编 `front` 的墙钟"，并把数字记进 `STATUS.md`
  （AGENTS.md「每条交付带耗时账」）✓ —— 本设计**不预填数字**（没量过就不许写 ✗）。

### 4.7 D8：页面模板 + **一篇填满的样板**（P3–P5 照抄，不再自行设计版式）

**为什么在设计阶段就把版式定死**：14 篇由 P3–P5 三个阶段写；没有样板 ⇒ 14 种版式，
而判据只查 `##` 标题在不在（**查不出"写得不像同一套文档"** ✗）。
样板的每一处都对应一条判据，写的人只要**填空** ✓。

#### 4.7.1 模板（每篇的骨架，判据逐条对得上）

```
# <关键字>                       ← H1 就是关键字本身（判据：文件含 `# <name>`）
                                  ← 紧跟 §4.5 那一句摘要，**逐字**
<!-- P1 骨架：…（填完删掉这一行） -->   ← P1 骨架标记（见下"防真空"一条）
## 怎么用                        ← 语法 + 至少一个 ```sokonanoda 例子
## 什么时候用                    ← 「适用」与「不适用」**各至少一条**
## 内核在背后判什么              ← 照抄 §1.3 的「内核规则」列
## 常见错误与出路                ← 表格，首列 = §1.3 的「用户实际看到的」**逐字**
## 相关                          ← 相对链接到别的页（`exact.md` 这种）
```

**⚠ 一条 P1 实现期补上的机制（防"零例子真空判绿"）**：判据"每个 ```sokonanoda
块都判绿"在**零个**块时**真空通过** ✗（AGENTS.md：「咬不住的守卫等于没有」）。
P1 的骨架**还没有例子** ⇒ 用一条**显式阶段标记**挡住：**标记在 = 正文未写**
（那条判据对该篇不生效）；**P3–P5 每填完一篇就删掉标记** ⇒ 从那一刻起
"至少一个例子 + 逐块喂真内核判绿"对该篇**立刻生效** ✓。
（**反向验证 4** 实测过：删标记而不给例子 ⇒ 判红 ✓。）

**四条版式规则**（都出自已核实的事实）：
1. **H1 == 关键字** ⇒ F12 的 `range` 落在标题行时，屏幕上第一眼就是这条 tactic 的名字 ✓。
2. **第一句 == §4.5 的摘要**（逐字）⇒ hover / 补全 / 索引 / 正文**四处的第一句永远一样** ✓。
3. **`## 常见错误与出路` 的首列逐字复制 §1.3** ⇒ 学习者看到的消息与文档里写的一模一样 ✓
   （**这是 §12 第 5 条更正要保护的属性**）。
4. **每个 ```sokonanoda 块必须能独立判绿**（自带它需要的声明）⇒ 判据可以**逐块**喂内核 ✓
   （不依赖"上一块喂过了"这种隐式状态 ✗）。

#### 4.7.2 样板页：`reference/tactics/intro.md`（**版式样板**；正文文风见 §4.7.4）

> ⚠ 本样板里的**两个** ```sokonanoda 块都**已喂真内核判绿** ✓
> （`example_checked: 1` / `decl_checked: 1 + example_checked: 1`，`failed` 为空）。
> 第二个块是**有意**选的：它演示"`intro` 会先做定义展开"，即 §1.3 关键事实 3 ——
> 而这一条**只有实测才敢写**（实测：`MyImp a a` 这种 `def` 头，`intro ha` 直接能过 ✓）。

````markdown
# intro

引入假设：目标 `A → B`（或 `∀ x, …`）时，先假设 `A`，再证 `B`。

## 怎么用

写法 `intro <名字>…`：一次可以给多个名字，**每个名字剥掉一层**
（`intro ha hb` 等于先 `intro ha`、再 `intro hb`）。

```sokonanoda
example (a b : Prop) : a → b → a := by
  intro ha hb
  exact ha
```

名字随你取。`intro y` 之后目标里的绑定名就是 `y`，不必照声明里的名字写。

## 什么时候用

- **适用**：目标是函数类型（`→`）或全称量化（`∀`）。这是 `by` 块里最常见的开头。
- **不适用**：目标不是函数形状（比如已经是 `a ∧ b`、`∃ x, p x` 或 `a = b`）。
  想拆 `∧` 用 `constructor`，想拆 `∃` 用 `use`，想证等式先试 `rfl`。

## 内核在背后判什么

`intro` 自己不判定，它做两件事：

1. 把目标**剥一层** `Pi`/`Arrow`。剥之前会先做**定义展开**（最多 4 层），
   所以目标的头是一个 `def`、展开之后才是函数类型时，`intro` 照样能过：

   ```sokonanoda
   def MyImp (a b : Prop) : Prop := a → b

   example (a : Prop) : MyImp a a := by
     intro ha
     exact ha
   ```

2. 在证明项里包一层 lambda（对应 Lean 的 `Expr.lam`）。剩下的目标交给后面的 tactic。

## 常见错误与出路

| 你看到的 | 意思是 | 下一步 |
|---|---|---|
| `` `intro` 需要一个函数目标（… -> … 或 forall …），当前目标不是函数 `` | 目标不是 `→`/`∀` 形状 | 先看目标是什么形状：`∧` → `constructor`；`∃` → `use`；`=` → 先试 `rfl` |

## 相关

- [`exact`](exact.md) —— 假设都引完了，用它交出答案
- [`apply`](apply.md) —— 不从前提往下推，而是从结论往上找
- [`constructor`](constructor.md) —— 目标是 `∧` 时拆成两个子目标
````

**行数**：样板 **48 行** ⇒ 距每篇上限（120 行）留一倍余量 ✓；14 篇按这个密度约
**700 行**，也在总量上限（1200）之内 ✓。

#### 4.7.3 索引页 `reference/tactics/README.md` 的形状

````markdown
# tactic 速查

本语言实现的全部 14 条 `by` tactic。一句话说不清的地方，点进每一篇看完整文档 ——
在编辑器里把光标放在 tactic 关键字上按 `F12`，也会直接落到对应那篇。

> 预算（本目录自己的闸，判据在 `crates/front/tests/tactic_docs.rs`）：
> 每篇 ≤120 行 · 本目录 ≤1200 行 · 本索引 ≤80 行。

| tactic | 一句话 | 文档 |
|---|---|---|
| `intro` | 引入假设：目标 `A → B`（或 `∀ x, …`）时，先假设 `A`，再证 `B`。 | [intro](intro.md) |
| …（其余 13 行同构，第二列**逐字**取 §4.5） | | |
````

**判据**（P1 判据 (e)）：14 行的第一列 == `TACTICS[].name`、第二列 == `TACTICS[].summary`，
**逐字**；第三列的链接指向存在的文件 ✓。

> ⚠ **相对链接在 VS Code 的 Markdown 预览里可用** ✓（`exact.md` 这种同目录相对链接），
> 而物化时 14 篇 + 索引**落在同一个目录**（§4.6 D7）⇒ 链接**在发布形态下也成立** ✓
> （不是"只在仓库里能点" ✗）。

#### 4.7.4 ⭐ 正文文风：**说人话、教材式连贯叙述**（用户 2026-10-10 拍板，P3–P5 必须遵守）

用户原话：「P3-P5 写正文务必**说人话、教材式连贯叙述**（完整通顺文章面向学生），
示例逐个喂真内核判绿」✓。这条把 §4.7.2 那个**要点式**样板的性质说清楚了：
**它是"版式"样板**（标题层级、首句、例子围栏、错误表的位置），
**不是"文风"样板** ⇒ 正文要写成**给人读的连贯文章**，不是要点清单 ✗。

| 维度 | 要求 | 反例（不许 ✗） |
|---|---|---|
| **体裁** | **连贯段落**：一段说一件事，段与段有承接（"所以"、"这时"、"如果……那么"） | 通篇 bullet 罗列；把 `## 什么时候用` 写成两个光秃秃的词条 |
| **读者** | 一个**正在证题、卡在某一步**的学生；假设他读过前面几课，但**没读过 Lean 手册** | 面向实现者的术语（"位置 spine 合一"这种内部说法 **要翻译**） |
| **术语** | 术语先行（中文 + 英文对照），紧跟一句白话解释（`skills/sokonanoda-teacher/references/zh-style.md` §四） | 用比喻替代术语；英文缩写不解释 |
| **示例** | 每个例子**先讲它在干什么**，再贴代码；代码后补一句"注意这里……" | 光贴代码不解释；贴完不指认关键那一行 |
| **红线** | `zh-style.md` 的四套路（物理动作写思考 / 抽象名词主语 / 逻辑胶水直译 / 有现成中文不写）、路标词、emoji、感叹号连用 | "接下来让我们看看……"、"这不仅仅是……更是……" |
| **可核部分** | 判据仍按 §4.7.1（5 个小节 + 首句逐字 + 例子判绿）；**文风靠 P3–P5 的自查清单**（`zh-style.md` §四第 6 条：圈五类、圈出即**重写**） | —— |

**为什么文风不写成判据**（诚实交代）：机器判不了"说人话" ✗ ⇒ 这一条只能靠
**写的人自查 + 评审抽查**；判据层能保证的是**结构与例子正确**（可核部分）✓。
**它仍然必须写进设计**：否则 P3–P5 会照 §4.7.2 的要点式样板抄出 14 篇清单 ✗。

### 4.8 D9：本次改动**会碰到 / 不会碰到**的既有判据清单（逐条点名）

> 本仓的行文规矩（`docs/design/command-naming.md` / `rename-inlay.md` 的形状）要求设计里
> 有一节「**会被这次改动碰到的静态契约**」：**点名判据 + `文件:行` + 红线** ✓。
> 本节就是那一节；**P2/P6 动笔前先读它**（免得改完发现"守卫本来就在拦" ✗）。

#### 4.8.1 **会被碰到**的（**必须同轮处理**，否则判红）

| # | 判据 / 契约 | 它现在断言什么 | 处置 |
|---|---|---|---|
| ① | `crates/lsp/src/tests/hover.rs` `hover_on_a_tactic_keyword_adds_only_the_summary_line`（P2 前叫 `…_is_byte_identical_to_the_old_output`） | 关键字上的 hover **整段** == `APPLY_KEYWORD_BASELINE`（`:1258-1261`），且**不含 `---`**（`:1262-1266`） | **改口径**：改成"把新增那几段删掉后与基线逐字节相同"（§4.1 ①②）；**`---` 那条保留原样** —— ⚠ **它顺带变成对摘要文本的约束：摘要里不许出现 `---`**（实测：14 条摘要**都没有** ✓，最长 44 字） |
| ② | 同文件 `:1085-1091` —— 基线自己的**文档注释** | 写着"这条守卫挡的是「关键字上**多加一行**」" | ✅ **P2 已改写**（注释写明：现在挡的是"goal state 那一段被改动 / 摘要行之外多东西 / 摘要里的 `---` 与新增围栏"）|
| ③ | 同文件 `:1109-` `fence_blocks()` + 多处 `fence_blocks(&markup).len()`（如 `:1640`） | 数 hover 里有**几个** ```sokonanoda 围栏块 | **不许新增围栏** ⇒ 摘要**用纯文本**（§4.5 的落地形态已如此定）✓ —— 这条是"纯文本"的**第二个**理由（第一个是 `isTrusted` 缺位让链接没用） |
| ④ | `crates/cli/tests/extension.rs:1240-1261` | **TM 语法词表 == `KEYWORDS` + `sorts` + `forall/∀`** | **不动**（本案不改 `KEYWORDS`、不改 TM 语法）✓ —— 但它是 §1.2 ⑦ 那条链的一半，P1 的判据要覆盖它 |

#### 4.8.2 **明确不会碰到**的（已逐条核过，写在这里免得 P2 白紧张）

| # | 判据 | 结论 |
|---|---|---|
| ⑤ | **补全项的 `detail` / `documentation`** | **没有任何判据**盯着 tactic 关键字的补全形状 —— 全仓 `CompletionItemKind::KEYWORD` 在测试里 **0 命中**；唯一的 `documentation` 断言在 `crates/lsp/src/tests/navigation.rs:82-88`，对象是**声明**（`def two : Nat`）不是关键字 ⇒ **§4.3 D4 是纯加法、不会判红** ✓ |
| ⑥ | `crates/lsp/src/tests/hover.rs:1372-1407`（手写 14 关键字 × `tactic_name_at`） | 断言的是"关键字**不是名字**"（不给类型行）⇒ 与"给摘要"**不冲突**（两件事，§4.1 末条）✓ |
| ⑦ | `crates/front/src/semantic.rs:999` / `:1419`（`sorry` == `Hole`） | 不动 `KEYWORDS` ⇒ 不冲突 ✓ |
| ⑧ | `crates/front/src/semantic.rs:1014` 附近（`by`/`intro`… 的关键字判据） | 不动词表 ⇒ 不冲突 ✓（**P1 落地后要复跑**，见 P1 验收 ④） |
| ⑨ | `textDocument/definition` 的其它分支（记法 / prelude 名字 / 声明表） | 新分支**只对"落在 `by` 步骤 span 里、且是表里那 14 个关键字之一"的 token 生效**（§4.4 触发条件）⇒ 其余路径逐字节不变；P6 判据 ④ 专门钉"不许抢项位" ✓ |
| ⑩ | 编辑器侧（`editor/vscode/**`） | hover/F12 全走 `vscode-languageclient`，扩展**零 provider**（实测 0 命中）⇒ **无客户端判据会碰** ✓ |

⚠ **这张表本身也要维护**：P2/P6 落地时若发现**还有**守卫在拦（本节漏列），
**必须回填本节**（"漏列的守卫"正是本设计要消灭的那类静默失败 ✗）✓。

### 4.9 D10：打包与分发 —— **两条通道、一份源**（用户 2026-10-10 补充）

**用户原话**：「tactic 文档（markdown）要**打包在 VS Code 插件里，随插件自然附带分发**——
就像 `prelude.lean` 一样随插件一起发，用户装插件即自带完整文档，无需额外拉取或访问外部站点」
（建议放 `media/` 或 `resources/`、随 publish 打包；F12 与 hover 从插件内嵌文档读取）。

**先把意图与形态分开对齐** ✓：
* **意图（硬要求）**：「装插件即自带、离线、不访问外部站点」——**两条通道都满足** ✓；
* **形态（用户点名的）**：文档**也**以**真文件**进 VSIX ⇒ **采纳** ✓（见下表通道 ①）。

#### 4.9.1 两条通道，一份源

| 通道 | 形态 | 谁用 | **为什么不能砍掉** |
|---|---|---|---|
| **① 插件内文件**（用户要的） | `editor/vscode/docs/tactics/*.md`，**随 VSIX 打包**（`unzip` 可见的真文件）；扩展把该目录经**环境变量**交给 LSP；F12 落在**插件自己的文件**上 | VS Code 用户 | 用户点名；而且 **F12 落点稳定、无运行时写盘** ⇒ **R10（只读环境物化失败）在插件这条路上直接消失** ✓ |
| **② 二进制内嵌 + 物化**（§4.6 D7，已设计） | `include_str!` 进 LSP + 物化到缓存/临时 | **CLI / opencode / DSH / 任何不经扩展的启动** | 砍掉它 = 这些入口**没有文档** ✗ —— 而 AGENTS.md 明写 opencode 与 DeepSeek Harness 是**一等公民** ⇒ 那是**回归** ✗ |

**一份源**：两条通道**都**来自仓库 `reference/tactics/*.md` ——
① 由**打包脚本拷贝**、② 由 `include_str!` 编译期内嵌 ⇒
「拷贝 == 源」由**判据**保证（R22）✓，**不是**靠"记得同步" ✗。

#### 4.9.2 这比"偏离 prelude"更贴近 prelude

Lean 的 `Init/Prelude.lean` 住在**工具链里**（随 Lean 安装发出去），VS Code 扩展只是**用它**；我们的对应物 = 文档住在**插件自带的 LSP 二进制**里（通道 ②）+ **插件自己的目录**里（通道 ①）—— **两条都在插件内、都不拉网络** ✓。所以这条补充**不推翻** D1/D7，而是**加一条落点** ✓。

#### 4.9.3 解析顺序：D7 的四档 → **五档**（先开发树、后插件、再物化）

| 档 | 路径 | 什么时候命中 |
|---|---|---|
| ① 仓库真源 | `<仓库>/reference/tactics/<kw>.md` | 开发树（`CARGO_MANIFEST_DIR` 只在**编译机**上存在） |
| ② ⭐ **插件目录（新）** | `$SOKONANODA_DOCS_DIR/<kw>.md`（扩展设为 `<extension>/docs/tactics`） | **安装形态**（①不存在）⇒ 天然走这里 ✓ |
| ③ 缓存物化 | `<cache::root()>/reference/tactics/<kw>.md` | 不经扩展的启动（CLI / opencode / DSH）✓ |
| ④ 临时物化 | `<temp>/sokonanoda-reference/tactics/<kw>.md` | 缓存被禁或不可写 ✓ |
| ⑤ | **`None`** | 五档都不成立 ⇒ 答"没有落点"，**绝不编造位置** ✗ |

顺序的两条理由：开发机上 ① 先命中 ⇒ **改文档立即生效**（不必重打包）✓；
安装形态下 ① 不存在 ⇒ ② 命中 ⇒ **F12 落在插件自己的文件上** ✓。
每一档都要求 **`is_file()`** ⇒ 目录在但文件缺 ⇒ 继续往下走（不是死指针 ✗）。

#### 4.9.4 接线：一处小改动（已核过现状）

扩展现在这样起服务器（`editor/vscode/extension.js:2834-2837`）：
```js
serverOptions = { run: { command, transport: TransportKind.stdio },
                  debug: { command, transport: TransportKind.stdio } };
```
**没有 `options.env`** ⇒ 加一项即可（**已核过 `vscode-languageclient` 的实际语义**）：
```js
const env = { SOKONANODA_DOCS_DIR: path.join(context.extensionPath, "docs", "tactics") };
serverOptions = { run:   { command, transport: TransportKind.stdio, options: { env } },
                  debug: { command, transport: TransportKind.stdio, options: { env } } };
```
* ⚠ **不要手写 `...process.env`** ✗：`vscode-languageclient` 的 `getEnvironment()`
  （`editor/vscode/node_modules/vscode-languageclient/lib/node/main.js:188-200`）**自己会先把
  `process.env` 铺底、再用你给的键覆盖** ⇒ `env` 里只写**新增的那一个**就够 ✓，
  写 `...process.env` 反而会让人误以为这个 API 是**替换**语义（不是 ✗）。
* 形状有类型依据：`ServerOptions.run: Executable`，而 `Executable.options?: ExecutableOptions`
  且 `ExecutableOptions.env?: any`（同包 `editor/vscode/node_modules/vscode-languageclient/lib/node/main.d.ts:32-43`）✓。
* `context.extensionPath` 是本仓扩展定位自己的**既有方式**（`extension.js:89/117/132/1828/2203/2208/2668`）✓。
* 与本仓既有的 `SOKONANODA_CACHE_DIR` / `SOKONANODA_LSP_BIN` / `SOKONANODA_NO_CACHE`
  **同一条环境变量纪律** ✓ —— **不新增自定义请求**，§7 边界 1 仍然成立 ✓。

#### 4.9.5 打包：两条已核过的事实 + 目录选择

* `.vscodeignore` 只排除 `.vscode/**`、`.vscode-test/**`、`**/*.vsix`、`.DS_Store`、`src/test/**`、`scripts/**`、`test-*.js` ⇒ 新增的 `docs/` **会**被打包 ✓。
* ⭐ **`.gitignore` 不影响 `vsce`**（实测：VSIX 里有 `extension/bin/darwin-arm64/sokonanoda-lsp`，而 `bin/` 是 gitignored —— `.gitignore:15`）⇒ 生成目录照 `bin/` 的先例：**gitignored + 随包发** ✓ —— 这是本仓**已有**的形态，不是新发明的 ✗。
* **目录用 `editor/vscode/docs/`，不用 `media/`**：`media/` 是 **webview 资源**（`localResourceRoots` 只含它，`extension.js:695`；里面是 `infoview.{html,js,css}`）⇒把产品文档塞进去会让"webview 资产"与"文档"混成一类 ✗（用户给的两个候选里，`media/` 的这个代价要说明；`resources/` 也可以，但 `docs/` 语义最直白）✓。

#### 4.9.6 ⭐ 装配线：**四个接线点** + 为什么拷贝必须是**生成的**（不是提交的）

**先定一个取舍**（这是本节最容易做错的地方）：插件里那 15 个文件
**是打包时生成的、gitignored**（照 `bin/` 的先例），**不是提交进仓库的副本** ✓。理由：

| 方案 | 漏了会怎样 | 判决 |
|---|---|---|
| **生成 + 打包时 stage**（选它） | 打包那一刻**必然**与源一致（stage 就是拷贝）⇒ **不可能陈旧** ✓；风险只剩"某条装配线忘了 stage" ⇒ 那会**缺文件**，而**通道 ② 兜底** ⇒ 用户仍能 F12（落到物化副本），只是"随插件自带"没兑现 ⇒ 由 **VSIX 条目判据**抓 ✓ | ✓ **选它** |
| 提交副本 | 作者改了源、忘了改副本 ⇒ 插件里是**陈旧内容** ✗ ⇒ F12 打开旧文、而 hover 摘要来自新表 ⇒ **用户可见的自相矛盾** ✗（比"缺文件"坏得多） | ✗ |

**四个接线点**（已逐条核过现状；**漏一个就有一类 VSIX 不带文档** ✗）：

| # | 装配线 | 现状 | 怎么接 |
|---|---|---|---|
| ① | 本地 `npm run package:host` | = `node scripts/stage-lsp.js --package`，而该脚本在 `stage-lsp.js:137` **自己**调 `vsce package`（**一个进程**）⇒ **npm script 前置写法不适用** ✗ | 在 **`stage-lsp.js` 内部**调用文档 stage（`require("./stage-docs.js").stage()`），**必须在 `packageVsix` 之前** ✓ |
| ② | 本地 `npm run package` / `package:universal` | `package` = 裸 `vsce package`（不 stage 二进制，靠已有 `bin/`）；`package:universal` = `clean:lsp && vsce package` | 这两条**也**要先跑文档 stage（或让 `package*` 都走 `stage-lsp.js`）✓ |
| ③ | **CI 逐 target**（`release.yml:168-173`） | `node scripts/stage-lsp.js …` **然后单独** `npx vsce package --target …` | 因为 ① 已把 stage 放进 `stage-lsp.js` ⇒ **这一条自动覆盖** ✓（**这就是选"放进脚本内"而不是"再加一个 npm script"的原因** ✓） |
| ④ | **CI universal 回退包**（`release.yml:181`） | 裸 `npx vsce package`，**前面没有 stage 调用** ⚠ | 必须**显式**加一次文档 stage（或改走 `stage-lsp.js`）✓ —— **这一条最容易被漏** ✗ |

**安全网（不依赖上面四点都记得）**：**每个产物都过一遍条目判据** ——
`release.yml` 出 **9 个平台包 + 1 个 universal**，判据要对**每一个** VSIX 数那 15 个条目
（§5 P7 验收 ①）✓。

#### 4.9.7 `--no-build` 下的例外：**文档 stage 无条件跑**

`scripts/vscode-e2e.sh:128-140` 的 `--no-build` **无条件跳过构建与 stage**（脚本自己写明"只在看起来更旧时警告"）⇒ 若文档 stage 跟着被跳过，**P7 的"F12 落点在扩展目录下"那条断言会在 `--no-build` 快路上误红** ✗ ——而 `--no-build` 恰恰是 `docs/E2E.md` §1b 推荐的**单用例快路**（秒级）✓。

**规则**：`--no-build` **只**跳过**二进制**构建与 stage；**文档 stage 无条件跑**（15 次文件拷贝、零编译、毫秒级）✓ —— 这样快路与全量路的判据**同口径** ✓。

#### 4.9.8 hover **不做文件 I/O**（有意决定）

摘要来自**编译进 LSP 的表**（§4.5 D6，零 I/O、零失败模式）⇒ **即使插件目录被删、`SOKONANODA_DOCS_DIR` 没设，hover 照样有摘要** ✓。五档只决定 **F12 的落点** ✓ —— 这把"hover 会不会因为文件找不到而静默"这个失败模式**从设计上消掉** ✓（用户说的"hover 从插件内嵌文档读取"由通道 ②满足：那张表就在插件自带的 LSP 里 ✓）。

## 5. 阶段拆分（每段独立可交付、可验收）

> 顺序即依赖顺序；**P0 通过评审后**才动 P1。每段的"验收"都是**可执行的命令或判据名**，
> 不是"看看对不对"。

### P0 · 设计评审（本轮）
* **交付物**：本文 + `docs-budget.json`/`docs-expiry.json` 登记（新文件入 L2 与 ④ 冻结表）。
* **改文件**：`docs/design/tactic-docs.md`（新）· `scripts/docs-budget.json`（`frozen`
  + `layer_max_lines.L2`；**手改**，不用 `--freeze` —— 它会抹掉 `onboarding` 一节 ✗）·
  `scripts/docs-expiry.json`（`--renew` 写理由）· `docs/ONBOARDING.md` §2（计划索引点名）·
  `docs/TESTING.md`（顺手收掉 3 行冗余空行，把 §2 新增那一行在 L1 层内抵回来 ⇒ **不抬 L1** ✓）。
* **验收**：`python3 scripts/docs-lint.py` ✓（实测 `活文档 305 个 / 5.52 MB · 判据 ①–⑧ 全过`；
  ⚠ **口径**：工具的「活文档 N 个」= **根 `*.md` + `docs/**` 下全部跟踪文件**
  （`docs-lint.py:102-110` + `:78-95`），**不只是 markdown** —— 其中 **63 个是 `.md`**，
  其余是 e2e 日志 / `*.jsonl` 台账 / 复现脚本 ✓。所以这个数**涨了不代表文档变多**
  （本轮它 305→307，是另一会话新增两个 `docs/gaps/repro/G108-*.{js,sh}`）✓）·
  `python3 scripts/docs-expiry-check.py --check` ✓（`无已过期 / 无未登记`）·
  `git status --short -- crates editor skills` **无本会话新增改动** ✓。
* **不做**：任何 `crates/**`、`editor/**`、`skills/**`、`courses/**`、`prelude/**` 的改动 ✗。

### P1 · 机制：`front::tactics` 表 + **全部 14 条**的单一真相 + 四层防漏项机制 ✅ **已落地（2026-10-10）**

**as-built**（设计→实现的落点与**每一条实测读数**；下同，"✅ 已落地"= 验收判据全绿 ✓）：

| 交付物 | 落点 | 读数 |
|---|---|---|
| ① 单一真相表 | `crates/front/src/tactics.rs`（**234 行**，`TacticDoc` 五字段 + `TACTIC_DOCS` 14 项） | 14 条 ✓ |
| ② 白名单派生（L1） | `parser.rs::is_tactic_keyword` ⇒ `crate::tactics::is_tactic` | 4 个调用点行为不变（`cargo test -p sokonanoda-front --lib` **924 passed**）✓ |
| ③ 错误文案由表拼出 | `parser.rs::unknown_tactic_message` + `crate::tactics::whitelist_text()` | 顺手修掉"手写名单漏 `have`"✓（**但那条兜底不可达** ⇒ §12 第 6 条更正） |
| ④ L3 判据 | `crates/front/tests/tactic_docs.rs`（**366 行**，**6** 条判据） | 6 passed ✓ |
| ⑤ L4 独立 lint | `scripts/tactic-docs-lint.py`（**229 行**，`--json` + `--selftest`） | `表 14 · parser 臂 14 · 文档 14` ✓；`--selftest` **8/8** ✓ |
| ⑥ 文档骨架 | `reference/tactics/*.md`（**15 篇**，14 篇骨架 + 索引，共 234 行） | 每篇 15 行、索引 21 行（预算 ≤120/≤1200/≤80）✓ |
| ⑦ 接线 | `scripts/soko`（gate，两步：本体 + `--selftest`）· `.github/workflows/ci.yml`（新 job `tactic-docs-lint`，两个 step） | `ci-yml-lint ✓ 4 个 workflow · 24 个 job` ✓ |
| ⑧ L1 正面验证（派生后**行为不变**） | `cargo test -p sokonanoda-front --lib` · `cargo test -p sokonanoda-cli` · `cargo test -p sokonanoda-lsp --lib` | front lib **924** · **cli 40 个测试二进制全绿**（0 failed）· lsp lib **207** ✓ |

**五条反向验证**（全部现场跑过，逐条记录"它证了什么"）：

| # | 注入的破坏 | 结果 | 证明了什么 |
|---|---|---|---|
| 1 | 往 `parse_tactic_inner` 加一条**只有臂、没进表**的 `hax` | **L4 判红**（`parser 有 hax 的臂，但表里没有它`）；⚠ **六条 Rust 判据全绿** | ⭐ **L4 不是冗余的** —— 这正是 §4.2 预言的"绕开 `is_tactic_keyword` 加臂"那条路，Rust 侧看不见 ✓ |
| 2 | 只往表里加一行、**不建** `.md` | **编译失败**：`couldn't read crates/front/src/../../../reference/tactics/ghost.md` | L2（`include_str!`）在**编译期**就挡住 ✓ |
| 3 | 删掉 `rfl.md` 的 `## 常见错误与出路` | L3 判红：`rfl.md 缺小节 ## 常见错误与出路` | 骨架完备判据咬得住 ✓ |
| 4 | 删掉 `exact.md` 的骨架标记、**不给例子** | L3 判红：`已删掉骨架标记 ⇒ 必须至少有一个 ```sokonanoda 例子` | **防真空判绿**：零例子不许蒙混 ✓ |
| 5 | 把 parser 的 `left` 臂改成永不匹配（表里仍有 `left`） | **L4 判红** + **L3 判红**：`` `left` 在表里，却掉进了「未知 tactic」兜底臂 `` | 两头都咬；且**兜底臂当场变成可达** —— 正是它存在的理由 ✓ |

**顺带记两件实现期的发现**（都在 §12 留档）：
* **那条兜底文案今天不可达**（14 条都有自己的臂 + 3 个调用点都有白名单闸门）
  ⇒ 初稿把"漏 `have`"记成**用户可见 bug** 是**过重**的，实为**不可达的死文本**（§12 第 6 条更正）✓；
* **改 `match` 最后一条臂的写法会触发 rustfmt 重排同 `match` 的兄弟臂**
  （`Assumption` 那条被合并成一行）⇒ 属于 fmt 的连带，已随 `cargo fmt -p sokonanoda-front` 一并落 ✓。

### P2 · hover 摘要（需求 ①，**覆盖全部 14 条**）✅ **已落地（2026-10-10）**

**as-built**（交付物 → 落点 → 读数）：

| 交付物 | 落点 | 读数 |
|---|---|---|
| ① hover 摘要行 | `crates/lsp/src/lib.rs`：`tactic_goal_hover` 里插一行；新助手 `tactic_keyword_at`（认关键字）+ `tactic_summary_line`（**唯一渲染点**，文案只来自表） | 14 条全量 ✓ |
| ② 补全 `detail`/`documentation` | 同文件 `completion()` 的 keywords 循环 | **13** 条（`sorry` 不在 `KEYWORDS` ⇒ 今天没有它的补全项，**不**凭空造）✓ |
| ③ 判据改口径 + 更名 | `crates/lsp/src/tests/hover.rs`：`hover_on_a_tactic_keyword_adds_only_the_summary_line`（`APPLY_KEYWORD_BASELINE` 降级成"goal state 那一段"的期望值，注释已改写） | 绿 ✓ |
| ④ 五条判据 | 同文件：`hover_on_every_tactic_keyword_shows_its_summary`（14 条**逐字** + 双向覆盖）· `hover_on_language_keywords_gets_no_tactic_summary`（边界 7 · 反向）· `tactic_keyword_completion_carries_its_summary_on_the_wire`（**字段存在性**）· `the_hover_summary_line_matches_the_protocol`（文档 ↔ 代码互咬）· `tactic_names_are_prefix_free`（实现前提） | 全绿 ✓ |
| ⑤ wire 契约 + 登记 | `docs/protocol.md` 新增 **§The tactic keyword**（`apply` 那一行**逐字** + `sorry` 具名例外）· `docs/visible-changes.md` +1 行 | protocol **1047 ≤ 1048** · visible-changes **56 = 56**（**同轮等量压缩**：protocol 压掉 4 行旧过程叙述、visible-changes 合并 2 行 ✓）|

**三条 P2 定稿的实现决定 / 修正**（设计没写死或写得含糊的地方）：
1. **摘要只在这一步的"关键字 token"上出现**，不是整条 tactic 的任意位置 —— 光标在名字
   （`Set.ext`）或**项位**关键字（`exact fun … => …` 的 `fun` / `=>`）上 ⇒ **一字不加** ⇒
   边界 7 **在实现层**成立（不是只靠"语言关键字走不到这条分支"）。
2. 认关键字 = **表驱动的精确前缀匹配**（表里哪个名字正好是 `step.span` 的开头 + 后面不是标识符
   字符），**不走词法**：前端没有公开"带记法符号表的词法"（`tokenize_with_symbols` 在私有模块、
   `Lexer::next_token` 私有），而用 `front::tokenize` 时，tactic 文本里出现**已声明符号**
   （`exact a ⊗ b`）会让整片 slice 词法失败 ⇒ 摘要**静默消失** ✗。前提由 `tactic_names_are_prefix_free` 钉住。
3. **措辞四处不一致 ⇒ 已按 §4.5 全篇对齐**：摘要的写法在设计里有**四种**（§4.1 / §4.2 D4 / §4.5 两处）
   ⇒ P2 以 **§4.5（唯一自称「落地形态、不再决策」的一节）**为准，并把 §4.1 / §4.2 的草案**同轮改齐** ✓。

**三条反向验证**（每条都先实测判红、再**还原**；文件已复原 ⇒ 工作树里只剩本轮的实现与判据）：

| # | 注入的破坏 | 判红的判据 | 证明了什么 |
|---|---|---|---|
| 1 | 摘要**不写进输出**（保留调用、去掉 `push_str`） | `hover_on_a_tactic_keyword_adds_only_the_summary_line`（字节判据） | ② 咬得住"少一行"✓ —— ⚠ **第一次的写法（整段删掉）直接编译失败**：两个助手成了 dead code，而本 workspace `-D warnings` ⇒ **构造层比判据层先挡住**（也算一条护栏，但它证不了判据会红）；换成"照常调用、只是不写进输出"才落到判据层 ✓ |
| 2 | 放宽门（语言关键字也给摘要：`hover()` 的关键字闸门之前插一段） | `hover_on_language_keywords_gets_no_tactic_summary` | 边界 7 有守卫，不是承诺 ✓ |
| 3 | 改 `tactic_summary_line` 里一个字符 | `hover_on_every_tactic_keyword_shows_its_summary`（14 条里那一条） | "逐字等于表"不是"含有某句话"✓ |

### P3 · 文档正文（一）：**引入与消去** 5 篇 ✅ **已落地（2026-10-10）**

**as-built**（交付物 → 落点 → 读数）：

| 交付物 | 落点 | 读数 |
|---|---|---|
| ① 五篇正文 | `reference/tactics/{intro,exact,apply,assumption,use}.md`（骨架标记已删） | **59 / 60 / 75 / 53 / 71** 行（每篇 ≤120 ✓）；本目录 **477** 行（≤1200 ✓）；索引 24 行（≤80 ✓） |
| ② 索引 | `reference/tactics/README.md`（**未改**） | 14 行**逐字**等于表（`index_lists_every_tactic_with_its_summary` 绿 ✓）；无状态列 ✓ |
| ③ 记法规则覆盖 `.md` | `scripts/notation-lint.py`：`markdown_blocks()` + `scan_code_lines()` + `DEFAULT_ROOTS` 加 `reference/tactics` | `--root reference/tactics` **15 篇零旧写法** ✓；`--selftest` **11/11**（新增 3 条 md 用例：块内命中 + 行号 = 块内行 + 两条不误红）✓；扫描面 301 → **325** 个文件 |
| ④ `apply` 六处更正 | 本文 §0 第 11 条 · §1.3 表行 · §1.3 表下那条 · §4.5 注 · §5 P3 判据 ⑦ · R20 | 新行为 `apply Eq.refl a` 过（`decl.checked:4` / exit 0 ✓）；失败文案**逐字未变** ✓ —— 两读都来自 `scripts/soko version --json` 的 **`0.87.3` · repo-build**（同一份构建 ✓） |
| ⑤ 例子喂真内核 | `cargo test -p sokonanoda-front --test tactic_docs` | **6 passed**（含 ① 的 **11 个块逐块 0 诊断** ✓） |

**四条反向验证**（每条先实测判红、再还原；工作树里只剩本轮的正文与 lint）：

| # | 注入的破坏 | 判红的判据（原文） | 证明了什么 |
|---|---|---|---|
| 1 | 删掉 `exact.md` 的 `## 相关` | `every_tactic_doc_file_exists_and_has_all_sections`：``…/exact.md 缺小节 `## 相关` `` | ① 骨架完备对**已填**的页仍生效 ✓ |
| 2 | 删掉 `use.md` 例子里那两行 `inductive`/`ctor` | `pages_marked_done_have_kernel_green_examples`：`…/use.md 的第 1 个例子**内核判红**` + 内核原文 `` `use` 需要目标是**归纳类型**…头 `Pair` 不在归纳表里 `` | ② "例子喂真内核"真的会咬（不是文本比对）✓ |
| 3 | 往 `intro.md` 的围栏块里写一行 `Eq.{1} Prop a b`，同时在**散文**里写 `` `P -> Q` `` | `notation-lint.py`：`reference/tactics/intro.md (code 0 / comment 1)` · `13:8 [comment] Eq.{…} → a = b`，**散文那处不报** ✓ | ③ 新入口够得着文档例子，且不误红散文 ✓ |
| 4 | 删掉 `assumption.md` 的**全部**围栏块（它的骨架标记早已删 ⇒ 正是"已删标记 + 零例子"） | `pages_marked_done_have_kernel_green_examples`：``…/assumption.md 已删掉骨架标记 ⇒ 必须至少有一个 ```sokonanoda 例子（零例子真空判绿 ✗）`` | **防真空判绿**仍咬得住（P1 反向验证 4 的同形复验）✓ |

* **交付物 / 骨架 / 改文件 / 验收**（P3 · P4 · P5 **共用**）：每篇 5 小节正文（版式 §4.7 D8）+ 索引
  `README.md`（形状 §4.7.3，**三阶段都没改**）；判据 ①–⑦ = 骨架完备 · 逐块喂真内核 · 记法 lint 入口 ·
  三条预算 · 首句逐字 · 测试绿 · 行为快照纪律；**P4/P5 沿用同一组**，各自的读数见各段的 as-built 表 ✓。

**三条发现**（都写进正文或本表，不留在会话里）：
* ⭐ **`use` 取第 1 个构造子**（`ctor_tactic(0, …)`）、**`assumption` 只检查最新那条假设**
  （`judge_strict` 把候选塌缩成一个，**既有缺陷、未修** ⇒ 出路：把要用的假设放到最后、或 `exact` 点名）
  ⇒ §1.3 的 `use` 行已改准、`assumption.md` 按**当天行为**写并**明写这是缺陷** ✓。
* §1.1/§1.3 里成批 `by.rs:` 行号被 G-109 推漂（快照性质）；本阶段只改与上面**同一处**的行号，**不整体重编号** ✓。

### P4 · 文档正文（二）：**构造与选择** 5 篇 ✅ **已落地（2026-10-10）**

**as-built**（交付物 → 落点 → 读数；交付物 / 骨架 / 改文件 / 验收 = **P3 的共用块**，判据 ①–⑦ 同）：

| 交付物 | 落点 | 读数 |
|---|---|---|
| ① 五篇正文 | `reference/tactics/{constructor,left,right,cases,match}.md`（骨架标记已删） | **60 / 53 / 62 / 67 / 67** 行（每篇 ≤120 ✓）；本目录 **687** 行（≤1200 ✓）；索引 24 行（≤80 ✓） |
| ② 索引 | `reference/tactics/README.md`（**未改**） | 14 行**逐字**等于表（`index_lists_every_tactic_with_its_summary` 绿 ✓）；无状态列 ✓ |
| ③ 记法规则覆盖 `.md` | `scripts/notation-lint.py`（**未改** —— P3 已建入口，P4 白拿 ✓） | `--root reference/tactics` **15 篇零旧写法** ✓；`--selftest` **11/11** ✓ |
| ④ 例子喂真内核 | `cargo test -p sokonanoda-front --test tactic_docs` | **6 passed**（含 **11 个块逐块 0 诊断** ✓：五篇各 2/2/2/2/3 块） |
| ⑤ 同批其余闸 | 同 P5 ⑤（`tactic-docs-lint` · `fmt --check` · `docs-lint` · `status-lint` · `dev-verify`） | 全绿 ✓；`dev-verify`：`冷 0.09s passes=13 ⇒ 改一行 0.05s passes=3` ✓ |

**三条反向验证**（每条先实测判红、再还原；**还原后 sha256 与注入前逐字节相同** ✓；注入脚本 `trap … EXIT` ⇒ 崩溃也还原）：

| # | 注入的破坏 | 判红的判据（原文） | 证明了什么 |
|---|---|---|---|
| 1 | 删掉 `cases.md` 的 `## 相关` | `every_tactic_doc_file_exists_and_has_all_sections`：``…/cases.md 缺小节 `## 相关` `` | ① 骨架完备对**已填**的页仍生效 ✓ |
| 2 | 删掉 `right.md` 的**全部**围栏块（标记早已删 ⇒ 正是"已删标记 + 零例子"） | `pages_marked_done_have_kernel_green_examples`：``…/right.md 已删掉骨架标记 ⇒ 必须至少有一个 ```sokonanoda 例子（零例子真空判绿 ✗）`` | **防真空判绿**仍咬得住（P1 反向验证 4 的同形复验）✓ |
| 3 | 删掉 `right.md` 里 `Tri` 例子需要的整个 `inductive` 声明 | 同上判据：`…/right.md 的第 2 个例子**内核判红**` + 内核原文 `` `right` 需要目标是**归纳类型**，但当前目标的头 `Tri` 不在归纳表里 `` | ② "例子喂真内核"真的会咬（不是文本比对）✓ |

**设计更正 / 发现**（父会话点名的两条 + 我抓到的两条；§1.3 已同轮改准，**行数不变** ✓）：
* ⭐ **§1.3 的 `left` 行写错两处**：它写"目标头归纳有 **≥2 构造子**时取第 1 个"，但 `left` =
  `ctor_tactic(0, …)` ⇒ **单构造子归纳（`And`）上照样成功**（实测 `left` 在 `a ∧ b` 上过，只把字段留成目标）；
  它还把「至少 N 个构造子」那条消息挂在 `left`/`constructor` 名下，而那条**只有 `right`（下标 1）能产生**
  ⇒ `left.md`/`right.md` 的错误表**分开写**、§1.3 三行已改准 ✓。
* **§1.3 的 `cases` 六条消息逐条复现、全部可达** ✓（`cases And.left h` 直接命中「被消去项必须是局部假设名」）
  ⇒ `cases.md` 六行表**一条不缺、一条不加** ✓。
* ⭐ **`right` 按【下标】取构造子，不认名字**：反证 3 的第一次注入是删掉 `ctor Tri.second` ——
  **例子照样判绿**（`Tri.third` 滑到下标 1）⇒ 这条负面结果正面印证"下标 1"语义 ✓。
* **§5 P4 那句「回显 `exact (match …)`」今天没有用户可见落点**：hover 的 tactic 围栏块取**源码 span**
  （`lsp/lib.rs:1424-1427`），`render_tactic` 的 `exact (match …)` 只喂 `Expr::By` 渲染 ⇒ 实测能看见的是
  **报错前缀 `exact`**（`match` 三条失败消息全部如此）⇒ `match.md` 按"内部就是一步 `exact` + 报错前缀"写 ✓。
* **`by.rs:` 行号整体漂移**（G-109 插入代码）：本阶段按**内容**定位、只写**实测过**的读数
  （§1.3 的 `constructor`/`left`/`right`/`cases` 四行已换新号），**不整体重编号** ✓（同 P3 处置）。

### P5 · 文档正文（三）：**上下文与占位** 4 篇 ✅ **已落地（2026-10-10）**

* **覆盖**：`have` · `exfalso` · `rfl` · `sorry`（`rfl`/`sorry` 属于"要先把概念讲清"才写得对的那两篇）。
* **交付物 / 骨架 / 改文件 / 验收**：**P3 的共用块**（判据 ①–⑦ 同）；**额外验收**：`rfl.md` 讲 **defeq**
  （不是"字符串相同"）+ `Iff` 边界；`sorry.md` 写"**合法开放状态**"并与 `redundant-sorry.md` 划清界限 ✓。

**as-built**（交付物 → 落点 → 读数）：

| 交付物 | 落点 | 读数 |
|---|---|---|
| ① 四篇正文 | `reference/tactics/{have,exfalso,rfl,sorry}.md`（骨架标记已删） | **70 / 54 / 77 / 88** 行（每篇 ≤120 ✓）；**14 篇合计 916** 行（+ 索引 24 = 本目录 940 行 ≤1200 ✓） |
| ② 索引 | `reference/tactics/README.md`（**未改**） | 14 行**逐字**等于表（`index_lists_every_tactic_with_its_summary` 绿 ✓）；无状态列 ✓ |
| ③ 记法规则覆盖 `.md` | `scripts/notation-lint.py`（**未改** —— P3 已建入口，P5 白拿 ✓） | `--root reference/tactics` **15 篇零旧写法** ✓；`--selftest` **11/11** ✓；**整树 6 处全在** untracked 的 `courses/set-theory/units/zz-scratch-user-snippet.sokonanoda`（他人草稿，P3/P4 已证基线）✓ |
| ④ 例子喂真内核 | `cargo test -p sokonanoda-front --test tactic_docs` | **6 passed**（含 **11 个块逐块 0 诊断** ✓：四篇各 3/1/4/3 块） |
| ⑤ 同批其余闸 | `tactic-docs-lint.py`（`表 14 · parser 臂 14 · 文档 14` ✓）· `cargo fmt …--check`（exit 0 ✓）· `docs-lint.py`（`活文档 310 个 / 5.68 MB` ✓）· `status-lint.py` ✓ · `dev-verify.sh`（`冷 0.10s passes=13 ⇒ 改一行 0.05s passes=3` ✓） | 全绿 ✓ |

**三条反向验证**（每条先实测判红、再还原；**还原后 sha256 与注入前逐字节相同** ✓；注入脚本 `trap … EXIT INT TERM`、还原动作放最前 ⇒ 崩溃也还原）：

| # | 注入的破坏 | 判红的判据（原文） | 证明了什么 |
|---|---|---|---|
| 1 | 删掉 `rfl.md` 的 `## 相关` | `every_tactic_doc_file_exists_and_has_all_sections`：``…/rfl.md 缺小节 `## 相关` `` | ① 骨架完备对**已填**的页仍生效 ✓ |
| 2 | 删掉 `sorry.md` 的骨架标记 + **全部**围栏块（正是"已删标记 + 零例子"） | `pages_marked_done_have_kernel_green_examples`：``…/sorry.md 已删掉骨架标记 ⇒ 必须至少有一个 ```sokonanoda 例子（零例子真空判绿 ✗）`` | **防真空判绿**仍咬得住（P1 反证 4 的同形复验）✓ |
| 3 | 删掉 `sorry.md` 里 `Box` 例子需要的整个 `inductive` 声明 | 同上判据：`…/sorry.md 的第 3 个例子**内核判红**` + 内核原文 ``unknown identifier `Box` `` | ② "例子喂真内核"真的会咬（不是文本比对）✓ |

**设计更正 / 发现**（§1.3 已同轮改准，**行数不变** ✓）：
* ⭐ **`have` 的「判定失败」路径可达，且是 P5 新测出来的**（§1.3 只列了它的**值类型不匹配**）：
  `have h : a := by sorry` ⇒ `` `have h` 判定失败：洞不在可填写的位置 ``（`judge.rs:3681`）——
  **只有嵌套 `by` 写法能产生**（值是项时那个洞有期望类型、判绿；两读同一份构建）✓。
* ⭐ **`sorry` 的"多余"有两条路，只有一条会发警告**：`theorem … := f h` + 尾随 `sorry` ⇒ **有**
  `redundant-sorry`；`Box.mk True.intro` + 尾随 `sorry` ⇒ `exercise_open:1` 且**零警告**（候选只认
  **直接实参**，`docs/design/redundant-sorry.md` §7 的边界）⇒ 正文按"**「还是 open」≠「你还没证出来」**"写 ✓。
* ⭐ **`exfalso` 的错误表只能收「下一步」的消息**：它自己零失败模式（实测：换目标一定成功、
  `exercise_open` 才是它的常态）⇒ 表里两条分别是 `exact` 的报错与"目标停在 `False`"这个**状态**，
  正文明写"这是下一步的报错，不是它自己的"✓ —— §1.3 那条"不许编它的报错"由此落地 ✓。
* ⭐ **`rfl` 的三条边界全部实测**：`f = f`（`α → α`）✓ · `A ↔ A` ✓ · `MyP a ↔ a`（`def` 头）✓ ·
  `a ↔ b` ✗ · `a ↔ a ∧ True` ✗（两边展开后仍不同）；判定是**内核 defeq**（`by.rs:1983` 只认
  `Judgement::Match`）⇒ `Iff` 的相等与 `Eq` 同源、同样归内核 ✓。⚠ **"两边不相等"消息确实泄漏内核
  pp 记号**（`Pi (a : Sort(0)), … 第 N 个绑元（a）`）：§1.3 只留稳定前缀 + 省略号，而屏幕上是**完整
  的目标 λ 项** ⇒ 表格首列记完整原文、表下如实翻译（`第 N 个绑元` = 第几个绑元、`Sort(0)` = `Prop`
  的内部写法）—— §1.3 与 §4.7.1 规则 3 之间的一处**口径歧义**，按"诚实文档"取向解决 ✓。

### P6 · F12 跳转（需求 ②，**覆盖全部 14 条**）✅ **已落地（2026-10-10）**
* **交付物**：`goto_definition` 新增一段分支（位置见 §2 表）+
  `front::tactics::doc_path()` 的**五档解析**（§4.9 D10 的 ①②③④⑤：
  仓库真源 → **`SOKONANODA_DOCS_DIR`（插件目录）** → 缓存物化 → 临时物化 → `None`；
  物化细节与 `clean` 的关系照 §4.6 D7）✓。
* **改文件**：`crates/front/src/tactics.rs`（`doc_path()`）·
  `crates/lsp/src/lib.rs`（一个分支）· `crates/lsp/src/tests/navigation.rs`（新判据）·
  `docs/protocol.md`。
* **验收（**必须绑用户动作** ✗，AGENTS.md 第 0 条 (a)）**：
  ① `goto_definition_on_every_tactic_keyword_lands_in_its_reference_file`：
  **14 条逐个**（不是只挑 `intro`/`exact`/`rfl` —— 那正是用户点名的三条，
  只测它们等于没测横向 ✗）；每条光标放在**用户实际点的那个字符**
  （关键字的**头一个字符**与**最后一个字符**各测一次 —— E27 的教训是用
  "能跑通的位置"代替"用户点的位置" ✗）；断言 ① 落点是**真文件**、
  ② 文件内容含 `# <关键字>`、③ `range.start.line` **就是那一行**
  （不是原地跳）、④ `range` == **落点文档里**那个名字 token 的 span（D5 已更正：
  `Location.range` 是目标文件坐标）、
  ⑤ 落点的**文件名与基线表（§1.1）一致**（防"全跳到同一篇"的假绿 ✗）；
  ② **反向验证**：删掉那个分支 ⇒ 判红 ✓；
  ③ **`file:` 可用性**：断言落盘路径**存在且可读**（物化失败必须**不编造位置** ⇒
  返回 `None` 而不是给一个死指针 ✗）；
  ④ **不许抢项位**：值位 `match` / `have`（不在任何 `by` 步骤里）上按 F12
  **必须仍走既有链**（与 §4.4 触发条件 ② 一一对应）✓；
  ⑥ ⭐ **插件目录那一档（② 档）单独判**（§4.9 D10）：给 `SOKONANODA_DOCS_DIR`
  指向一个临时目录（里面放假文档）⇒ 断言 F12 **落在那个目录里**（不是缓存、不是仓库），
  且内容与 `TACTICS[i].markdown` **逐字节相同** ✓；**反向判据**：那目录里的文件**不存在**时
  ⇒ 必须**继续往下走**（落到 ③ 档），**不是**返回死指针 ✗；
  ⑦ ⭐ **那 14 篇正文若在本阶段之前又变过**（R20/R21）：动笔前把 §1.3 的复现片段**重新喂一次**
  内核，按**当次读数**落笔，**不许照抄快照** ✗（⚠ 本条原为"`apply.md` 对着当天内核写"，
  `apply` 那条线已落地 ⇒ **该判据已移交 P3** ✓）。
* **不做**：不新增自定义 scheme / `TextDocumentContentProvider` ✗（§1.4 末条）。

**as-built**（交付物 → 落点 → 读数）：

| 交付物 | 落点 | 读数 |
|---|---|---|
| ① 五档解析 | `front/src/tactics.rs::doc_path()`（**+77 行**；P1 的表与 `doc()` 一字未动 ✓） | 五档逐档 `is_file()` 守卫 ✓ |
| ② 一个分支 | `lsp/src/lib.rs`（**+85 行**）：`tactic_doc_at()`（两条闸门）+ `doc_heading_token()` + `goto_definition` 那段 | **能力声明未动** ✓；只在两条闸门同时成立时早返回 ✓ |
| ③ 四条判据 | `lsp/src/tests/navigation.rs`（**+516 行**：4 条判据 + 夹具 + 三个守卫） | navigation **25 → 29** · lsp 全量 **214 → 218** ✓ |
| ④ 协议契约 | `docs/protocol.md` 新增 §The tactic keyword: F12 lands in its document | **1048 ≤ 1048**（同轮压回 **−20** 行）✓ |
| ⑤ 文档正文 / 索引 | **一字未改**（`reference/tactics/**`） | `cargo test -p sokonanoda-front --test tactic_docs` **6 passed** ✓ |

**三条反向验证**（每条先实测判红、再还原；**还原后 sha256 与注入前逐字节相同** ✓；注入脚本一律 `set -u; cp "$F" "$BAK"; trap restore EXIT INT TERM`（还原动作 = `cp "$BAK" "$F"` 放最前）后跑 `.py` 替换件 ⇒ 崩了也还原）：

| # | 注入的破坏 | 判红的判据（原文） | 证明了什么 |
|---|---|---|---|
| 1 | 分支**照常算完、只是不返回**（`return` → `let _`） | ①：`` `intro` 的第 1 个字符` 上 F12 必须给一个**标量位置**，实际 = None `` | ① 咬得住"分支没了"✓。⚠ **第一次的写法（整段删掉）没落到判据层**：两个助手成 dead code、`-D warnings` ⇒ **编译失败**（`inj1.log` 零测试失败）—— 与 P2 反证 1 同形，故改成"保留调用、去掉 return" |
| 2 | `doc_path` 的 ② 档**去掉 `is_file()`**（直接答 `$SOKONANODA_DOCS_DIR/<name>.md`） | ⑥：`` 插件那份已经不存在了 ⇒ 绝不许把**不存在的路径**当答案（死指针 ✗） `` | ⑥ 的反向半咬得住（**死指针**当场判红）✓ |
| 3 | 分支之前插"有失败的声明（= 有诊断）⇒ `return Ok(None)`" | ⑦：`` `apply（文件里有诊断）` 上 F12 必须给一个**标量位置**，实际 = None ``（同轮 G-108 的旧判据也一起判红 ✓） | ⑦ 咬得住（G-108 那条线在新分支上成立）✓。⚠ 第一次写的是 `docs.diagnostics()` —— 那个方法不在 `MutexGuard<Docs>` 上 ⇒ **编译失败**（`inj3.log`），改成按 `report.decls[].status` 判 |

**设计更正 / 发现**（四条，都写进活文档而不是留在会话里）：
* ⭐ **§4.4 D5 的 `range` 写错了**：原文写"`range` == 关键字 token 的 span"，而
  `Location.range` 是**目标文件**坐标 ⇒ 按**源文件**坐标答，编辑器会把光标停在
  `.md` 的第 1 行空白处（实测：判据 ①ⓒ 当场判红 ✓）。**已改准**：`range` = 落点
  文档 `# <关键字>` 标题里那个名字 token（`doc_heading_token()` 从 markdown 求；
  找不到 ⇒ 整跳回落既有链，**不编造** ✓）；§5 P6 判据 ④ 与 `docs/protocol.md` 同轮改齐 ✓。
* ⭐ **② 档在开发树里不可观测**（① 恒命中，这是 §4.9.3 **有意**定的顺序）⇒ 判据 ⑥ 的
  正面半只能把 ① 档那份真源**临时改名藏起来**（`HiddenRepoDoc`：`Drop` 第一件事还原 +
  还原后核对内容逐字节等于 `TACTIC_DOCS[i].markdown`）。⚠ 进程内的 `DOCS_LOCK` 只串行
  **同进程**判据；两个并发 `cargo test` 进程仍可能互见改名窗口（CI 每个 target 只跑一次 ⇒ 可接受，留档）。
* **判据 ④ 的 `is_none()` 是"断言当前行为"**：值位 `match` 今天走既有链答 `null` ✓，
  若将来那条链给它一个落点，本判据会**误红**（与 P2 的 hover 基线同族的脆弱性）⇒
  到那天按 P2 的办法改成"与没有新分支时逐字节相同" ✓。
* **`by_steps` 只记"跑过且成立"的步**：`intro` 用在不匹配的目标上 ⇒ 该声明**零
  `by_steps`**（`soko/stateAt` 答 `step:-1 / total:0`）⇒ F12 无处可落。判据夹具因此必须
  **每条 tactic 一条真能过的目标**（与 `hover.rs::EVERY_TACTIC_SOURCE` 同一份夹具），
  不能拿"随便一条目标 + 关键字"当夹具 ✗。

### P7 · 收口：编辑器 / 协议 / 文档 / skills 同步
* **交付物（含用户 2026-10-10 补充的打包分发）**：① `editor/vscode/scripts/stage-docs.js` —— 把
  `reference/tactics/*.md` **逐字节**拷进 `editor/vscode/docs/tactics/`（幂等；`--check` **只比对不写**、
  漂移即 exit 1 —— 同 `test-project-first-screen.js --check` 的既有纪律 ✓）；② `.gitignore` 加
  `editor/vscode/docs/`（照 `bin/` 的**生成目录 + 随包发**先例）✓；③ **四个接线点全接**（§4.9.6，
  **漏一个就有一类 VSIX 不带文档**）：`stage()` 进 `stage-lsp.js` **内部、`packageVsix` 之前**
  （`--package` 与 CI 逐 target 自动覆盖）＋显式补 `package` / `package:universal` / **CI universal
  回退包**（`release.yml:181`，最容易漏的一点 ⚠）＋ `scripts/vscode-e2e.sh`（**`--no-build` 时也跑**）✓；
  ④ `extension.js` 的 `serverOptions` 带 `options.env.SOKONANODA_DOCS_DIR`（§4.9.4 的形状）✓；
  ⑤ 扩展侧**其余零代码改动**，同步 `editor/vscode/README.md` + `CHANGELOG.md` + `package.json`
  （**版本号只在发版批次动**，`docs/vscode-dev-guide.md` §2）· `skills/sokonanoda-teacher/SKILL.md`
  （**不要再用"tactic 白名单"人肉名单**，指向 `reference/tactics/`）· `docs/visible-changes.md` ·
  `STATUS.md` · `REQUIREMENTS.md` §9（两条新要求：范围 = 全部 tactic；要有防漏项机制）。
* **e2e（真 VS Code + CDP DOM，`docs/E2E.md` §8）**：新增一条用例 —— **在编辑器里对 tactic 关键字按 F12
  ⇒ 断言真的打开了那个 `.md` 文档**（用户可见结果，不是"LSP 返回了 Location"）；**这是 ① ② 两样唯一的
  端到端判据**（front 验表与例子、LSP 验 wire、e2e 验用户看见的东西 ✓）。⚠ 至少覆盖**三条**（引入族
  `intro`、糖 `constructor`、容易抢项位的 `match`），其余交给 LSP 层逐条判据 ✓。
* **验收**：① **打包判据（机械）**：VSIX 里**真的有**那 15 个条目（`unzip -l <vsix> | grep
  extension/docs/tactics/` ⇒ **14 篇 + 索引**，逐名核对）✓ —— 把"随插件分发"变成**可判的事实** ✗→✓；
  ② **漂移判据**：`node editor/vscode/scripts/stage-docs.js --check` exit 0（**反向验证**：手改插件里那份
  一个字节 ⇒ 判红 ✓）；③ **e2e**：断言打开的**落点在扩展目录下**（`…/docs/tactics/<kw>.md`）——
  "随插件分发"的**用户可见**判据 ✓（与①合起来是**一条用例的两个断言**，不重复跑宿主）；
  ④ `scripts/soko gate --fast` ✓ →（批次收尾）`scripts/soko gate` ✓ →
  `SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh` 一条台账 ✓。
* **P1 留给 P7 的一条**（`reference/tactics/` 进不进 `docs/README.md` 的「关联目录」）：**不进** ✓ ——
  那份地图列的是开发者台账，`prelude/` 同样不列；`reference/tactics/` 是**产品内容**（F12 落点），
  理由与结论写进下面的 as-built ✓。

**P7 as-built（2026-10-10，发布版本 `0.88.0`）** ✓：①`stage-docs.js`（15 篇逐字节镜像 + `--check`；**多出来的文件也算漂移** ⇒ 目录是生成物、与源同形）②四个接线点全接：`stage-lsp.js` 的 `main()` 里**无条件**先 `stageDocs.stage()`（不只在 `--package` —— e2e 的常规路也只调这条命令 stage 二进制）· `package`/`package:universal` 显式前置 · **`release.yml` 的 universal 回退包显式补一次**，并给 9 个平台包 + universal **每个产物**加了"15 个条目逐名 + 与 `reference/tactics/` 逐字节相同"的冒烟判据 ③`vscode-e2e.sh` 在 `if/else` **之后**无条件跑文档 stage ⇒ `--no-build` 快路与全量路同口径 ④`extension.js` 的 `serverOptions` 加 `options.env`（只写新增那一个键，不写 `...process.env`）。
**读数**：host VSIX（darwin-arm64）`unzip -l` 的 **15 个条目逐名核对** ✓ · `--check` exit 0（反向：改 1 字节 ⇒ exit 1，改完 `cmp` 还原）· 真宿主 e2e `P7：…按 F12…` 三条（`intro`/`constructor`/`match`）**770ms 绿**：落点 = `extensionPath/docs/tactics/<kw>.md`（**反向**：删 `SOKONANODA_DOCS_DIR` ⇒ 判红，落点变成物化副本 `…/T/sokonanoda-reference/tactics/intro.md`；删 `stage()` ⇒ 打出的 VSIX 无条目）✓ · `reference/tactics/` **不进** `docs/README.md` 地图（与 `prelude/` 同为产品内容，不是开发者台账）。
**设计更正 / 发现（4 条）**：① §5 P7 的 e2e 写"CDP DOM §8"，但**记台账的是 `vscode-test` 层**（§8 的驱动不写台账）⇒ 用例落在 `extension.test.js`，动作 = `editor.action.revealDefinition`（**F12 绑的就是它**）、断言 = 文档真的打开；② §4.9.3 的 ① 档是**编译期**路径 ⇒ 开发树里 ② 档不可观测，**e2e 也要临时藏起仓库真源**（P6 判据同款；JS 侧 `renameSync` + `finally` 还原 + 开跑前自愈残留）；③ `editor/vscode/README.md` 有一句被 P2 改假（"tactic 关键字上什么都不加"）⇒ P7 一并改准，另加"每篇文档随插件离线可读"一条；④ ⭐ **全量跑逮到的新事实**：`editor.action.*` 是**编辑器命令**，`when` 含 `editorTextFocus` ⇒ **宿主窗口没有 OS 焦点时静默不动**（实测 `window.state.focused=false` 时连 `cursorRight` 都不动光标，而 `executeDefinitionProvider` 照常答对 ✓）⇒ 导航那一半必须"有焦点断言真按键、没焦点退到打开落点并留日志"，两半都在（`docs/CI-FAILURES.md` 2026-10-10 条）。

## 6. 风险与对策（评审时逐条过；**26 条**）

| # | 风险 | 对策 |
|---|---|---|
| R1 | **F12 落点找不到文件**（发布产物里没有 `reference/`） | 用 `include_str!`（编译期内嵌）+ 三段兜底物化（§1.4），与 prelude **同一条已验证的路**；物化失败 ⇒ `None`，**不编造位置** |
| R2 | **文档放 `reference/` 后没人管**（`docs-lint` / 过期机制都不覆盖） | 三条自己的闸：① 覆盖判据（§5 P1，防漏项）② 骨架 + 例子喂内核（§5 P3–P5，每篇都过）③ 预算上限（每篇 ≤120 / 总量 ≤1200 / 索引 ≤80）；**都要进 `scripts/soko gate` 与 CI**（判据在 `crates/front/tests/tactic_docs.rs`）✓ |
| R3 | **用户期待"文档在编辑器里渲染成网页"**（我们只给 VS Code 原生 markdown 预览） | **明确声明边界**：本轮交付"打开真 `.md` + 预览"；若要自定义渲染（Infoview 风格文档面板）是**另一条线**（要动 CSP/nonce/stub 宿主/两条 e2e），建议单独立项、不在本设计里承诺 ✗ |
| R4 | **hover 变长把 goal state 挤掉** | 摘要**插在 tactic 行之后、goal state 之前**且**不新起段落**；判据钉"把新增那几行删掉后与 `APPLY_KEYWORD_BASELINE` 逐字节相同"（§4.1 ②） |
| R5 | **两处 wire 契约文档都在 0 余量的 L1 层**：`docs/protocol.md` **1048/1048**、`docs/visible-changes.md` **56/56**（冻结值 = 当前实测值）⇒ P2/P4 想加行，`docs-lint` ⑦ 立刻判红 ✗ | **对策（也是本设计对文档纪律的表态）**：**同轮做等量压缩**，不许抬 L1 上限 ✗。具体：`docs/protocol.md` §Tactic goal-state hover（`:285-333`）里属于"**将来会变**"的部分（摘要行措辞、F12 分支的**动机与阶段计划**）压缩成 **≤6 行 + 指向本文**（`docs/design/tactic-docs.md` 才是计划的唯一权威，`protocol.md` 只留**今天的 wire 契约** ✓）；F12 那一节同理。`docs/visible-changes.md` 的 +2 行从它自己的旧条目里压回来 ✓ |
| R6 | **例子判红**（文档里的代码过不了内核） | §5 P3–P5 的共同判据 ② 把**每个** ```sokonanoda 例子喂 `parse`+`compile`（0 诊断）；写正文前先 `scripts/soko grade <临时文件>` 跑一遍 |
| R7 | **14 篇文档与 `skills/sokonanoda-teacher` 已有的 tactic 说法分叉** | §5 P7 把 SKILL.md 的"tactic 白名单（全量）"段（`skills/sokonanoda-teacher/SKILL.md:286` 附近）改成**指向 `reference/tactics/`**（**同一份真相**）；`crates/cli/tests/skill.rs` 的既有守卫继续管入口不漂移 |
| R8 | **`with` / `sorry` 是不是 tactic** 的边界说法不一 | §1.1 + §4.1 定死：`with` **不在基线里**（`cases … with` 的连接词，`parser.rs:3569-3589` 本来就没有它）；`sorry` **在基线里、也给摘要**（用户补充的「一个都不能少」已包含它），它与 `NON_NAME_TACTIC_WORDS`（不给**名字类型行**）是两件不同的事 ✓ |
| R9 | **`docs/design/by-tactics.md` 已删但仍有 9 处引用**（`crates/front/src/lib.rs:11`、`by.rs` 4 处、`judge.rs:335`、`report.rs:65`、`check/mod.rs` 2 处、`crates/cli/tests/extension.rs:117`），另有 `docs/design/tactic-hover.md` 被 `ROADMAP.md:403` 引用 | 这**不在本需求范围内**（`docs/README.md:101-104` 的全局规则已覆盖："历史由 git 追溯"）；但 P3 的文本**可以**顺手提供"tactic 引擎在哪、按什么判"的活指针 ⇒ 建议 P5 把那批引用改成指向 `docs/design/tactic-docs.md` §1.4（**单独立项、不混进本批** ✗）。本文的**命名刻意避开这两个死名字**（叫 `tactic-docs.md`）✓ |
| R10 | **物化写盘在只读环境失败**（受限沙箱 / 只读 HOME / 只读缓存） | 照抄 `prelude_source_path` 的三段兜底（仓库真源 → 缓存 → 临时目录），**四处都写不进 ⇒ `None`**；F12 的判据里**必须**有一条「答 `None` 而不是给死指针」（§5 P6 验收 ③）✓ |
| R12 | ⭐ **范围扩到全部 tactic 后，工作量与"完整性"同时放大**（14 篇正文 + 14 条 hover + 14 条 F12）；若只做完 `intro`/`exact`/`rfl` 就报"完成"，用户看到的仍是"大部分 tactic 没有文档" ✗ | **阶段按族切**（§5 P3/P4/P5 各 5+5+4 篇）⇒ 每一族**独立可交付**；**验收按 §1.1 的 14 条基线逐条核**（判据是"并集 ⊆ 表"且"表 ⊆ 并集"，不是抽查）✓；**任何一族没做完就不许报"需求 ③ 完成"** ✗ |
| R13 | **`match` 被"重复计数"或"被漏计"**：它既在 parser 白名单里、又长得像 `exact`（§1.2 第 ⑤ 行） | 基线表 §1.1 **按"关键字"取值**（14 条，`match` 单独一行）；P1 判据的输入是 §1.1 的关键字列，**不是**枚举名 ⇒ 重复/漏计两个方向都被判据挡掉 ✓ |
| R14 | **用户清单里的 `exact?` 被当成"已实现"写进文档** | §1.1 已实证：`exact?` **本语言没有**（只有 `judge.rs:10` 一句注释）；P3–P5 的验收③（骨架 + 内容）里加一条：文档正文**不许出现**基线表之外的关键字当"支持"（人工评审 + §1.1 是唯一权威）✓ |
| R16 | ⭐ **"取 `Tactic` 枚举的变体名做判据"在 Rust 里实现不了**（无反射、无 `strum`/`variant_count`；本仓 `Cargo.toml` 也没有）⇒ 初稿的 P1 判据 ① 是**不可实现**的 ✗ | **已改**（§4.2 L1/L3）：机制改成 **L1 派生（构造上不可能漏）+ L3 手写清单**（本仓已有同形先例 `hover.rs:1374-1389`）+ **L4 独立源码 lint**。**不再声称能"枚举变体"** ✓ —— 见 §12 第 1 条 |
| R17 | **`sorry` 会让粗糙判据假红**：它在白名单里（真 tactic），但**故意不在 `KEYWORDS`**（是 `SemanticKind::Hole`，`semantic.rs:866-867`） | §4.2 L3② 给**具名例外**（表里带 `semantic_kind` 字段并按字段分流）；**禁止**写成"跳过所有不在 KEYWORDS 的项"（那等于阉掉判据 ✗）—— 见 §12 第 2 条 |
| R18 | ⭐ **文档例子逃过"课程一律写记法"的硬规则**：`notation-lint.py` 只扫 `*.sokonanoda`，够不着 `.md` 围栏块（AGENTS.md 硬规则 3） | §5 P3 判据 ③：给 `notation-lint.py` 加 `.md` 入口 + `reference/tactics` 根 ⇒ 同一条命令管课程与文档 ✓ —— 见 §12 第 3 条 |
| R25 | ⭐ **装配线漏一个接线点 ⇒ 某类 VSIX 不带文档**（§4.9.6 的四个点里，**④ CI universal 回退包**（`release.yml:181` 裸 `vsce package`）最容易漏）⇒ 「装插件即自带」对**那个产物**是假话 ✗ | ① **把 stage 放进 `stage-lsp.js` 内**（覆盖 ①③ 两条路径，而不是再加一个 npm script —— 因为 `--package` 在**同一个进程里**就调了 `vsce`）✓；② ④ 显式补一次 ✓；③ **安全网**：**每个产物**都过条目判据（9 个平台包 + universal **逐一**数那 15 个条目，§5 P7 验收 ①）✓ —— 即"不靠四个点都记得"✗ |
| R26 | **`--no-build` 快路下文档没 stage** ⇒ P7 的"F12 落点在扩展目录下"**误红** ✗ （而 `--no-build` 正是 `docs/E2E.md` §1b 推荐的秒级单用例路） | §4.9.7：`--no-build` **只**跳过二进制构建与 stage；**文档 stage 无条件跑**（15 次拷贝、零编译）⇒ 快路与全量路**同口径** ✓ |
| R22 | ⭐ **插件里那份拷贝会与源漂移**（`reference/tactics/` 改了、忘了重跑打包脚本）⇒ 插件自带的文档与 hover 摘要/索引**说的不是一回事** ✗ | §5 P7 的 `stage-docs.js --check`（只比对不写、漂移 exit 1）挂进 `soko gate` + CI + 打包线 ✓；**反向验证**：手改插件里那份一个字节 ⇒ 判红 ✓。⚠ **不靠"记得同步"** ✗ —— 那是本设计要消灭的病（§1.2 六份副本）✓ |
| R23 | **LSP 拿不到 `SOKONANODA_DOCS_DIR`**（opencode / DSH / CLI / 用户在非扩展环境直跑）⇒ 若把插件目录当**唯一**来源，这些入口**没有文档** ✗（回归） | **两条通道**（§4.9 D10）：通道 ② 的 `include_str!` 内嵌是**兜底且自包含**的 ⇒ 五档解析里 ②（插件目录）不成立就落到 ③④（物化）✓；P6 判据 ⑥ 专门钉「目录在但文件缺 ⇒ 继续往下走」✓ |
| R24 | **VSIX 里出现两份同样的文档**（`bin/` 里 LSP 内嵌的 + `docs/tactics/` 的真文件）⇒ 体积 +「到底哪份是权威」的歧义 ✗ | 体积：14 篇 ≈ **60 KB**，对 **16 MB** 的 VSIX 可忽略 ✓；权威性：**源唯一**（仓库 `reference/tactics/`），两份都是**派生物**且由 R22 的判据保证一致 ⇒ 歧义只在"没有源"时才会被感知，而那时两份必然相同 ✓ （**判据**：P6 判据 ⑥ 断言插件那份与 `TACTICS[i].markdown` 逐字节相同 ✓）|
| R20 | ⭐ **`apply` 的行为曾被另一条线改**（修复线 `d9a701e4`：`apply` 的目标匹配要做定义展开对齐 Lean 4）⇒ 本文 §1.3/§4.5 对 `apply` 的描述一度是**旧行为**，照抄就会交付一篇**描述旧行为**的文档 ✗ | **已落地 2026-10-10**：§1.3 的 `apply` 行、表下那条、§4.5 的注、§0 第 11 条**同轮改齐**；**P3 判据 ⑦**（原挂 P6）：写 `apply.md` 前**必须重新喂一次内核**，以当天行为为准 —— P3 已按此办理（新行为过 ✓、报错文案逐字未变 ✓）；**§7 边界 9** 明写这条修复**不归本文档**（不写需求、不立判据、不动判定路径）✓ |
| R21 | **同类风险普遍存在**：本设计的所有行为读数（§1.3 的 12 条消息、§4.5 的摘要依据）都是**某一天的快照**，别的线随时可能改掉其中任何一条 ⇒ 若把快照当契约，交付的文档会悄悄变成假话 ✗ | 把「**照抄 §1.3**」降级成「**以落地当天的实测为准**」：P3–P5 的判据 ②（例子逐块喂内核）已经强制"例子必须真绿"，再加一条**人为规则**（§4.7.1 规则 3 的"逐字复制"限定在**当天重测过**的前提下）✓ —— **判据能咬住"例子错"，咬不住"描述旧行为"** ✗，所以这一条只能靠流程 + 本文的显式声明 ✓ |
| R19 | **"未知 tactic" 文案漏 `have`（既有用户可见 bug）且今天零判据覆盖** ⇒ 同类漂移随时会再发生 | §4.2 让文案**由表拼出**（⑥ 派生）+ P1 判据 (c) 断言含全部 14 条。⚠ **要不要在本轮修**：**要**（一行改动 + 一条判据，属于"顺手修掉同类漏项"，不是新功能 ✗）—— 见 §12 第 4 条 |
| R15 | **`reference/**` 完全在 `docs-lint` 与过期机制之外** ⇒ 长期可能变成"没人管的文档堆" ✗ | §1.6 已把这条写成**明账**；P1 的判据把 `reference/tactics/**` 纳入 CI（覆盖 + 骨架 + 例子 + 预算）⇒ 它**有自己的闸**，而且比 `docs-lint` 更严（例子必须过内核 ✓）|
| R11 | **`docs/protocol.md` 已有未修的计数漂移**（`:396` 写 "five custom requests"，实际注册 7 条，`lsp/lib.rs:3344-3350`） | P5 顺手改掉这一句，并**同类横排**：`docs/protocol.md` 里所有"几条/几个"的数字都核一遍（AGENTS.md 第 0 条 (b)：同类问题不许只修单点）✓ —— 但这属于**文档纠错**，不与本需求的契约新增混为一谈 ✓ |

## 7. 明确不做（边界）

1. **不加新的 LSP capability / 自定义请求** ✓（`definition_provider` 已广告，需求 ② 是分支不是能力）。
2. **不做自定义 markdown 渲染 / 文档 webview**（R3）。
3. **不改 tactic 的任何判定语义**（`by.rs` 只在 P1 动关键字名单那一处，判定路径一字不改）。
4. **不碰 kernel**（本设计零内核改动）。
5. **不给 `apply`/`cases` 等写"完整 Lean 语义"**：文档只写**本语言今天真实支持的子集**，与错误文案逐条对齐（AGENTS.md 硬规则 3：教学语法是真实 Lean 4 的**子集**）。
6. **不在本轮动版本号**（发版是独立批次，`docs/vscode-dev-guide.md` §2）。
7. **不动关键字闸门**（`lsp/lib.rs:2545-2551`）：`def`/`theorem`/`fun`/`=>` 这些**语言关键字**的 hover今天静默，本设计**不顺手放开** ✗ —— 那是"给语言关键字也写文档"的另一个需求（表要更大、正文要另写 20 篇）。若用户要，**单独立项**。
8. **不给 Infoview 加"点关键字看文档"**：`extension.js:1473-1500` 的 `gotoDefinition()` 在服务端答 `null`时**静默降级成 `revealRange`**（`extension.js:1481-1496`）⇒ 走那条路要么改它的降级行为、要么加一条它会静默骗人的用例。P4 交付的是 **F12（用户点名的那条）**；Infoview 那条**记进 §6 R6 的边界** ✓。

9. ⭐ **不实现 `apply` 的目标匹配修复**（**归属：修复线 `d9a701e4`**，用户 2026-10-10 澄清）：`mem_of_subset_singleton` 里 `apply Eq.refl a` 报"目标不匹配"、期望 `apply` 支持**定义展开**对齐 Lean 4 —— 那是**内核/tactic 行为缺陷**，属**修复**范畴 ✗。本次交付的是**文档体系**（hover 摘要 /
   F12 跳转 / 14 篇介绍 / 防漏项门禁）：**不把它写进需求、不立判据、不改 `by.rs` 的判定路径** ✓。`apply` 的**文档照写**，但**以落地当天的实际行为为准**（§1.3 的快照规则）✓。
10. **不做任何"在线文档 / 外部站点 / 首次运行拉取"**（用户 2026-10-10 明确）：文档**只**从插件与二进制里来（§4.9 D10 两条通道），**没有网络调用、没有 CDN、没有"缺文档就下载"** ✗ —— 这与仓库既有的版本锁定纪律（禁 `releases/latest`）同向 ✓。
## 8. 预算账（本文自己也要过闸）

* 本文：`docs/design/tactic-docs.md`，**新文件** ⇒ `docs-lint` ④ 默认上限 **150** 行；本文按 §5 的**八个阶段（P0–P7）** + §6 **二十六条**风险 + §9/§10/§11 三节调研，**实测 1312 行**（P2 落地后 1311；P3 的 as-built 新增 33 行 ⇒ **同轮压缩 §12 等量**、1310；P4 新增 27 行 ⇒
  **同轮压缩 P3 那条已跑完的验收块 34 行**、1303；P5 把骨架压成 4 行、as-built 新增 36 行 ⇒**同轮压缩 P3/P4/P7 的散文与 §7 的换行 36 行**（**整行删/并**，读数行与反证行一行未少）⇒ 实测 **1312**；**P6** 的 as-built 新增 **37** 行 ⇒ **同轮压缩 §1.6/§3/§7/§4.9/§8/§10.4/§11 的换行 48 行**（同一条纪律：整行删/并，读数行与反证行一行未少）⇒ 实测 **1301**）⇒
  P0 已**手改 `scripts/docs-budget.json` 的 `frozen` 显式登记 1312**（评审可见 ✓），并按例外①把 `onboarding.layer_max_lines.L2` **13789 → 15101** 跟着走一次，随后**只许减不许增** ✓。
  ⚠ **同轮在 L1 层内抵回来**：`docs/ONBOARDING.md` §2 新增一行索引 ⇒ 从 `docs/TESTING.md` 收掉 3 行冗余空行，**L1 上限不动**（仍 5959）✓ —— "新增**文件**才动基线，给既有文件加行
  一律自己压回来"这条纪律没有破例 ✓。
* `reference/tactics/**`：**不在这两套闸的口径内**（§1.6）⇒ 预算由 **P1 的覆盖判据 + P3–P5 的共同判据 ③** 自己钉（每篇 ≤120 / 总量 ≤1200 / 索引 ≤80，数字与判据同处一份测试文件）✓。

## 9. Lean 4 对照（机制级摘要；完整取证见 §11）

> AGENTS.md 的「卡住先读 Lean 4」要求**动手前**先交一份对照；本设计**三条**都要动
> 判定面之外的东西（hover 文本 / `definition` 返回值 / 文档正文），所以对照放在这里 ✓。

| 环节 | Lean 4 怎么做 | 本仓怎么做 | 差异与决定 |
|---|---|---|---|
| tactic 文档正文住哪 | 与语法声明同处的 doc string（`Tactic/Doc.lean:137-148`），靠 `parserExtension` 的 `tactic` 类目**推导**出清单（`:149+`） | **独立 `.md` 源文件**（`reference/tactics/`），`include_str!` 内嵌 + 手写表 + **双向判据** | **偏离是有意的**：读者是学习者（要"适用场景/常见错误"这类教学段落），不是 API 参考；本仓 parser 没有类目注册表 ⇒ 推导不出清单，只能靠判据防漂移 |
| hover 上显示 tactic 说明 | **不做** ✗（`Info.docString?` 只认项信息，`InfoUtils.lean:329-333`） | **加一行摘要**到既有目标状态 hover（`tactic_goal_hover`） | ⭐ **净增量**，且方向一致：**不替换**目标状态、只**追加**一行 ⇒ 两边都保住"hover 首先是目标状态" ✓ |
| 补全里带 tactic 文档 | **做** ✓：`allTacticDocs` → 补全项的 `documentation`（markdown、`detail? = none`，`CompletionCollectors.lean:600-611`） | 同构（补 `detail` + `documentation`，§4.3 D4） | **对齐** ✓ |
| F12 跳到 tactic 文档 | **不做** ✗：`locationLinksOfInfo` 不含 `TacticInfo` ⇒ `intro` 上 F12 无落点（`GoTo.lean:301-335`） | **做**（`goto_definition` 新增分支） | ⭐ **有意偏离、且是用户点名的**：多给一条入口；代价是"关键字跳文档"成了本仓独有语义 ⇒ 必须写进 `docs/protocol.md` ✓ |

## 10. 调研结论：编辑器侧（已核，2026-10-10）

### 10.1 LSP 能力面（逐条核过 `lsp/lib.rs:2234-2297`）

已广告 ✓：`textDocumentSync`(FULL) · `hover` · `definition` · `documentHighlight` · `selectionRange` ·
`documentSymbol` · `documentLink`(resolve:false) · `rename`(prepare) · `references` · `inlayHint`(resolve:false) ·
`codeLens`(resolve:false) · `codeAction` · `completion`(resolve:false) · `foldingRange` · `semanticTokens`。
**未广告** ✗：`declaration` · `typeDefinition` · `implementation` · `signatureHelp` · `formatting` ·
`workspaceSymbol` · `executeCommand` · `linkedEditing` · `workspace/*`。
自定义请求实际注册 **7 条**（`lsp/lib.rs:3344-3350`：`soko/{goals,goalAt,nextHole,hints,stateAt,project,version}`）。

> ⚠ **顺带抓到的既有漂移**：`docs/protocol.md:396` 仍写 "the server answers **five**
> custom requests"，而注册的是 7 条，且**没有判据**盯着这个数字 ✗。P5 顺手改掉这一句
> （**同类问题横向排查**：`docs/protocol.md` 里所有"几条"的数字都要核一遍）✓。

### 10.2 扩展侧：没有客户端 provider，markdown 交给 VS Code 原生渲染

* 客户端**零** hover/definition provider（`HoverProvider|registerHover|provideHover|
  registerDefinitionProvider` 全仓 0 命中）⇒ hover 与 F12 **全走 `vscode-languageclient`**
  （`editor/vscode/extension.js:2838-2842`，`documentSelector` = `{language:"sokonanoda",
  scheme:"file"}`，无 middleware）✓。
* 唯一客户端定义助手是 `gotoDefinition()`（`extension.js:1473-1500`，走
  `vscode.executeDefinitionProvider`）；**服务端答 `null` 时它静默降级成 `revealRange`**
  （`extension.js:1481-1496`，E27 事故的现场）⇒ **这会把"F12 坏了"伪装成"跳了一下"** ✗。
  ⇒ **P4 的 e2e 必须断言"文档真的打开了"**，不能只断言"没降级"（§5 P4 验收 ① ）。
* **唯一的 webview 是 Infoview**（`package.json:186-193`），而它**按设计只吃
  `textContent`**（`media/infoview.js:6,26-33,62-96`）⇒ **今天没有任何 markdown 渲染路径**；
  `localResourceRoots` 只有 `<extension>/media`（`extension.js:695`）。
* `markdown-it@14.3.1` 确实躺在 `editor/vscode/node_modules/` 里，但它是
  `@vscode/vsce` 的**传递 dev 依赖**（`package-lock.json` 标 `dev:true`），
  **不在 `package.json`、也不在 VSIX 里** ⇒ **不能拿它当渲染方案** ✗（只能当"探针"用，
  `docs/gaps/ledger.jsonl` G-99 正是这么用过它）。
* `vscode.MarkdownString` 只用在不信任的 tooltip（`extension.js:511-515`/`1434`/`1462`），
  无 `isTrusted`；`vscode.markdown` API 0 命中。
* ⇒ **结论**：把文档渲染成网页 = 要新写一套 CSP 安全的渲染器 + 改
  `localResourceRoots` + 动 stub 宿主与两条 e2e；**收益（比 VS Code 原生预览好看）
  远小于成本** ⇒ §6 R3 的边界决定成立 ✓。

### 10.3 交付形态的三种候选（选 (a)）

| 候选 | 证据 | 判决 |
|---|---|---|
| **(a) `include_str!` 内嵌 + 物化到缓存（同 prelude）** | `prelude.rs:499-544` 三段兜底、幂等写、写不进 ⇒ `None` | **选它** ✓：发布产物（VSIX/缓存二进制）里也能用；开发树走仓库真源、能直接改 ✓ |
| (b) 指向仓库 `docs/**` | `CARGO_MANIFEST_DIR` 是**编译期**路径（`prelude.rs:505-506` 已记这条坑） | ✗ 发布产物上必然不存在 |
| (c) 文档塞进 `editor/vscode/media/` 随 VSIX 发 | `.vscodeignore` + 实测 zip 列表：**VSIX 一个仓库 `.md` 都不带** | ✗ 要把 `reference/` 复制进扩展目录（多一份产物、多一套同步），且 `include_str!` 已经把这件事做得更干净 |

### 10.4 订阅到的两个既有事实（写进 P4/P5 的边界）

1. `textDocument/documentLink` **已实现但只服务 `import` 行**（`lsp/lib.rs:2759-2783`；设计 `docs/design/import-links.md:45` 明确"import 行的 F12 不做"）⇒ 文档入口**不走 documentLink** ✗（那是另一条线，会与 F12 的落点语义打架）。
2. `docs/design/tactic-hover.md` 与 `docs/design/by-tactics.md` **都已删**（`dae75759`），但 `ROADMAP.md:403` / 9 处代码注释仍引用 ⇒ §6 R9。
   本文的**命名**刻意避开这两个名字（叫 `tactic-docs.md`），免得与死指针混淆 ✓。

## 11. 调研结论：Lean 4 侧（源码级，`~/Documents/lean/lean4`）

> **对照纪律**（AGENTS.md「卡住 ⇒ 先读 Lean 4 源码，按它对齐」）：下面**逐条对齐机制**、
> **不抄代码**；差异写清**为什么偏离** ✓。取证方式 = 读本机源码 `git` 锁定的那份 ✓。

| 环节 | Lean 4 的机制（源码证据） | 本设计怎么做 | 差异与决定 |
|---|---|---|---|
| **tactic 文档正文住哪** | 住**语法声明旁**：`TacticDoc.docString` 字段（`src/Lean/Elab/Tactic/Doc.lean:137-148`），由 `allTacticDocs`（同文件 `:149+`）从 `parserExtension` 的 **`tactic` 类目**遍历出全部 tactic，`userName` = 该语法第一个 token、`docString` = 环境里那条声明自己的 doc string | 住**独立 `.md` 源文件**（`reference/tactics/`），表里 `include_str!` 内嵌 | **有意偏离**：读者是学习者，要的是"适用场景 / 常见错误与出路"这类**教学段落**，不是 API 参考 ⇒ 正文独立成篇更合适。代价（文档与实现分叉）= 用 §5 P1（名单双向判据）+ P3 判据 ④（文档里的例子**喂真内核**）补 ✓ |
| **tactic 清单从哪来** | **从注册表推导**，不是手写名单（`allTacticDocs` 遍历 `categories.find? \`tactic`；`alternativeOfTactic` 跳过别名） | 手写一张表 + **双向判据**钉住 parser 白名单（§4.2 D3） | **不得不同**：我们的 parser 是手写递归下降、没有 `parserExtension` 那种类目注册表 ⇒ 推导不出，只能用判据防漂移。**这也正是本仓今天"名单硬编码两遍"的病根** ⇒ P1 收成一份后，判据是唯一防线 ✓ |
| **tactic 的 hover** | **没有**：hover 的 doc string 只从 `Info.docString?` 取（`src/Lean/Server/InfoUtils.lean:329-333`），而它只认 `TermInfo`/`DelabTermInfo` 一类**项**信息；`ofTacticInfo` 只被用来定位 `stx`（`InfoUtils.lean:173,425-477`）⇒ 光标在 `intro` 上得到的是**目标状态**（tactic 语义），**不是**它的 doc string | **加一行摘要**到既有的"目标状态"hover 里（`tactic_goal_hover`） | ⭐ **这是本设计对 Lean 4 的净增量**，而且**方向一致**：Lean 4 也没把 doc string 塞进 hover，我们**不替换**目标状态、只**追加一行**（§4.1）⇒ 两边都保住"hover 首先是目标状态"这条语义 ✓ |
| **tactic 的补全** | **有，且带文档**：`allTacticDocs` 直接映射成补全项，`documentation? = docString`（markdown）、`kind = keyword`、`detail? = none`（`src/Lean/Server/Completion/CompletionCollectors.lean:600-611`） | 同构：补全项补 `detail: Some(summary)` + `documentation`（§4.3 D4） | **对齐** ✓：连"不发 `detail`"这个取舍都可以照抄（我们发 summary 是因为我们的表里就有，比 Lean 多一个字段不算偏离语义） |
| **F12（go to definition）到 tactic 文档** | **Lean 4 不做** ✗：`locationLinksOfInfo` 的 `match` 只覆盖 `TermInfo`/`DelabTermInfo`/`FieldInfo`/`OptionInfo`/`CommandInfo`/`ErrorNameInfo`/`DocElabInfo`，**其余一律 `pure #[]`**（`src/Lean/Server/GoTo.lean:301-335`）；`TacticInfo` **不在其中** ⇒ `intro` 上按 F12 **没有落点** | **做**（`goto_definition` 新增分支，§5 P4） | ⭐ **有意偏离、且是本需求点名的**：Lean 4 的 tactic 文档只从补全与参考手册进得去；用户明确要"F12 跳到对应文档"⇒ 我们**多给一条入口**。代价：这是**本仓独有的语义**（"声明名跳声明 / tactic 关键字跳文档"两种落点混在同一个 `definition` 里）⇒ **必须写进 `docs/protocol.md`**（P4 交付物）并在判据里钉死 ✓ |
| **`tactic` doc role 里的名字** | `addConstInfo s found.internalName`（`src/Lean/Elab/DocString/Builtin.lean:321-337`）—— **只在 doc string 的代码块里**给 tactic 名字建 const info | **不抄** ✗（我们没有 doc role 机制） | 这是 Lean 4 内部"文档里写 `` `intro` `` 能点"的实现，与本需求的"代码区 F12"是两件事 ⇒ 不在本轮 ✓ |

**参考手册的正文**：Lean 4 的 tactic 文档**同时**出现在 (a) 补全与 (b) 由 doc string生成的参考手册（`lean-lang.org/doc/reference/` 的 tactics 章）；本仓**没有**手册生成链
⇒ `reference/tactics/**` 就是唯一正文，它**同时**服务 F12 落点与人工阅读 ✓（这条差异让"正文只有一份"变成硬约束，§6 R8 的"单一生产者"由此而来）。

*（注：`leanprover/lean4/doc/` 下只有 `make/`+`dev/`+`std/` 等，**没有**参考手册生成器—— 手册在 `leanprover/doc-gen4`，本机未检出 ⇒ "手册是否由 docgen 逐字生成"这条
**未验证**，落地 P3 前若要引用它必须补证。上表所有结论都不依赖它 ✓。）*

## 12. 设计自审：初稿里被自己推翻的**六条**（留档更正）

> 立此存照的规矩来自 `docs/design/notation-subset.md` / `by-prefix-reuse.md` 的先例：
> **留着错的结论比没有更坏** ✗ ⇒ 初稿写错的地方不删、就地记明"错在哪、为什么、
> 改成什么" ✓。四条都是本轮（设计阶段、只读代码）**核出来的**。

> **实现阶段又抓到 6 条**（§1.3 的 `use`/`assumption`/`left`/`cases`/`have`/`rfl` 行各有错或不全）⇒ 逐条记在
> **§5 P3/P4/P5 各段 as-built 的「设计更正 / 发现」块**里（就地改准、行数不变）✓。

1. **"取 `Tactic` 枚举变体名与表比对" —— 不可实现** ✗→✓。
   初稿把防漏项机制写成"取枚举全部变体名 + 白名单 + `render_tactic` 臂的**并集**"，
   但 **Rust 没有反射**（本仓无 `strum`/`variant_count`）⇒ 判据枚举不出变体 ✗。
   **改成**：`is_tactic_keyword` **派生自表**（构造上不可能"有 tactic 无文档"，
   因为 `parse_by_block`/`tactic_keyword_ahead` 走同一个函数）+ 手写清单判据
   （本仓先例 `crates/lsp/src/tests/hover.rs:1374-1389`）+ 独立的 Python 源码 lint ✓。
   **保留的正确部分**：基线表 = 14 条（§1.1）、"只比枚举会漏 `match`"的论断仍然成立 ✓。
2. **"`KEYWORDS` 覆盖 14 条" —— 是错的，实际 13** ✗→✓。
   初稿把 `semantic::KEYWORDS` 记成"14，靠注释人肉同步"。实测：`KEYWORDS` 39 项里
   tactic 只有 **13**，`sorry` **不在**其中 —— 而这是**有意设计**：`sorry` 被归成
   `SemanticKind::Hole`（`semantic.rs:866-867`，判据 `:999`/`:1419`）✓。
   ⇒ 初稿"两者同集合"的说法对 13/14 这件事是**含糊的**，会让实现者写出假红判据 ✗。
   **改成**：具名例外（§4.2 L3②）。
3. **"文档例子喂内核就够" —— 漏了记法规则** ✗→✓。
   `notation-lint.py` 的扫描根是 `courses/set-theory` / `course` / `playground.sokonanoda`
   且只收 `*.sokonanoda`（`notation-lint.py:57-61`）⇒ `.md` 里的例子**完全不受**
   "课程一律写记法"（AGENTS.md 硬规则 3）约束，可能一边判绿一边教旧写法 ✗。
   **改成**：P3 判据 ③ 扩展那个 lint 的入口与扫描根 ✓。
4. **"错误文案"只当资料，没当判据** ✗→✓。
   初稿把 `parser.rs:1546-1548` 的"未知 tactic"文案当作"写文档时照抄的素材"，
   没意识到它**本身就是 tactic 清单的第 6 份副本，而且已经漂了**（漏 `have`，
   全仓零断言）⇒ 学习者打错字时看到的支持清单是**不完整的** ✗。
   **改成**：文案由表拼出 + P1 判据 (c) ✓，并作为**附带修掉的既有缺陷**记进
   `docs/visible-changes.md`（P7）✓。

5. ⭐ **§1.3 的"错误文案"列是【转述】而不是【逐字】—— 那正是要写进产品的东西** ✗→✓。
   初稿那一列写的是我的概括（`apply` 写「目标不匹配」一类、`constructor` 写
   「找不到构造子」一类、`assumption` 干脆引了**另一个函数**的文案
   `proof.rs:35`，而 `assumption` **tactic** 的真实文案在 `by.rs:1899`）✗。
   **危害等级最高的一条**：这一列是 P3–P5 的 `## 常见错误与出路` **唯一来源**，
   转述会被逐字抄进**给学习者看的文档** ⇒ 学习者按图索骥找不到那条消息 ✗。
   **改成**：把 12 条失败路径**逐条喂真内核**、抄回 `failed[].message` 原文（§1.3 现在的表，
   含复现片段）✓ —— 顺带查到两件照抄不到的事实：`rfl` 的失败消息里带内核 pp 记法
   （`第 N 个绑元（x）`）、`exfalso` **自己没有失败模式**。

6. ⭐⭐ **"错误文案漏 `have`"是【不可达的死文本】，不是"用户可见 bug" —— 初稿把严重度写重了**
   ✗→✓（**P1 实现期实测抓到**）。初稿把 `parser.rs:1546-1548` 那条文案的漏项记成
   **用户可见缺陷**（"学习者打错一条 tactic 时看到的支持清单不完整"），并在 P1/P7 的
   交付物里写成"顺手**修掉**一条既有缺陷"。
   **实测推翻了它**：`parse_tactic()` 只有 **3 个**调用点
   （`parse_by_block` / `parse_tactic_sequence_in_arm` / `parse_nested_tactic_sequence`），
   **每个前面都有 `tactic_keyword_ahead()` 闸门**，而它走 `is_tactic_keyword`（= 表成员）
   ⇒ 进得来的一定在表里；表里 **14 条在 `parse_tactic_inner` 里都有自己的臂**
   ⇒ 兜底臂**取不到**。打错一条 tactic（`hax`）走的是"空 `by` 块 + 未知命令"
   那条路（实测报 `expected a .sokonanoda command, found Token { … Ident("hax") }`），
   **根本到不了**那条文案。
   **两处后果**（都已改）：
   ① **工序**：写判据时才发现"从外面断言那句话"**断言不到** ⇒ 判据 3 从"断言文案"
      改成**行为判据**（`every_tactic_in_the_table_has_a_real_parser_arm`：表里每条
      都不许掉进兜底臂 + 表外关键字不许走 tactic 解析），文案本身改由
      `parser.rs` 的**单元测试**直接判（具名函数 `unknown_tactic_message`）✓；
   ② **措辞**：文案仍**由表拼出**（删掉手写那份是正确的），但它的价值从"修一个可见缺陷"
      降级为"**给一种被 L4 禁止的漂移（表里有行、parser 没臂）准备兜底提示**"，
      而**第 5 条反向验证证明它当场可达** ✓。
   **教训**（值得写下来）：**"不可达"这件事本身要实测**（数调用点的前置闸门 + 数臂）
   —— 只看"名单少了一项"就断言"用户会看到错的清单"，是把**静态不一致**当成了
   **用户可见后果** ✗。

### 12.1 本轮**没有**推翻、仍需在实现阶段复核的三条

* **`include_str!` 的路径深度**：按 `prelude.rs:178` 的 4 层先例推算为 3 层（`crates/front/src/tactics.rs` → 仓库根）✓ —— 实现时第一件事就是让它编译过（L2 自带验证）。
* **物化后的 `file:` 落点能在 VS Code 里打开**：与**已发布且已在用**的"F12 跳 prelude 源"（`prelude.rs:499-544` + `lsp/lib.rs:2816-2823`）同类 ✓；端到端确认在 P7 的 e2e（`docs/E2E.md` §8）✓。
* **`is_tactic_keyword` 的 4 个调用点**（`parser.rs:1274`/`:1332`/`:1373`/`:1398`）改成查表后行为不变 ⇒ P1 验收 ④ 用**全绿**证明，不用嘴说 ✓。

### 12.2 本文所有 `文件:行` 引用的**取证方式**（可复现）

一条脚本（**不靠眼睛** ✓）：把所有 `` `path:NNN[-MMM]` `` 抽出来 → 解析到真实文件（裸名按 `git ls-files`
的 basename 唯一匹配；`lib.rs` 已**全部**限定成 `lsp/lib.rs` 或 `front/lib.rs` 以消歧 —— 仓里有三个 ✗）
→ 断言**文件存在**且**每个行号 ≤ 该文件总行数** ✓。
**读数**：96 个引用 / 161 个行号 **全部在范围内** ✓；`crates/lsp/src/lib.rs` 实测 **3354 行**，与 §1.4 标题一致 ✓。
⚠ 它只验"引用落在文件里"、**不验"那一行是不是那个东西"** ✗ —— 后者靠写时逐行 `sed -n` 读过，以及
**实现阶段的第一件事**：P1 一编译、P2/P6 一跑判据，错的行号会立刻暴露 ✓。

### 12.3 本文**三张 14 行表**的一致性（用 L4 的同一手法自查）

本文有三处独立手写的 tactic 表：**§1.1** 覆盖基线 · **§1.3** 语义与错误文案 · **§4.5** 14 条摘要 —— 一处漏
一条或写错名，设计就自相矛盾 ✗（而"六份副本靠人同步"正是本设计要消灭的病，§1.2）。
**手法 = L4 的预览**：正则抽三张表的关键字列 → 断言 **都是 14 行** · **三个集合相等** · **编号都是 1..14** ·
**§1.1 的文档文件列逐行等于 `<关键字>.md`** ✓。
**读数**：**通过 ✓**（三张表都是同样 14 条）。⭐ **它当场咬到一次**：§1.3 原先把 `left`/`right` 合成**一行**
（13 行）⇒ 判据立刻报"§1.3 不是 14 行" ⇒ 已拆成两行 ✓ —— **这条读数本身就是 L4 可行性的证据** ✓。
⚠ **已知假阳性**：它与**节范围**绑定，而一节里可能出现**第二张表**（§1.3 表下那张"快照/归属"表，首列又是
`apply`）⇒ 会报"§1.3 有 15 行" ✗；**修法 = 只看每节的【第一张连续表】**（已改 ✓）。⚠ 这条限制**不影响
P1 的真判据**：L4 比的是 `parser.rs` 里 `kw == "…"` 的**源码字面量**，不是 markdown 表格 ✓。

### 12.4 §1.3 的 12 条失败消息：**逐条喂真内核**抄回来的（可复现）

**方法**：不读源码猜文案，而是把"能让那条 tactic 失败"的最小片段**喂给真内核**，抄回 `failed[].message`
—— 与判卷**同一条通道**（`scripts/soko grade`，或 MCP 工具 `mcp__sokonanoda__check` 直接喂文本、**不落盘**）✓；
复现片段逐条列在 §1.3 末尾（12 条，含 `rfl` 的"形状不对"与"两边不等"两条分支）✓。
**读数**：12 条全部拿到**逐字原文** ✓；错误码一律 `elab-tactic-failed` ✓。**顺带查实两条**（只有实测才会看见）：
① `rfl` 的"两边不相等"消息里含内核 pp 记法（`第 N 个绑元（a）`）⇒ 文档要翻译一句 ✓；② `exfalso`
**没有自己的失败模式**（换目标一定成功）⇒ 文档**不许编**一条它的报错 ✓。
**为什么单列**：这是"**判定走 kernel、禁文本比对**"（AGENTS.md 硬规则 4）在**文档写作**上的同一条纪律 ——
"你会看到什么"也必须来自内核，不能来自印象 ✗（**P3–P5 已照办**：正文的失败文案逐条重喂过 ✓）。

### 12.5 本文**自称的数字与字段**也要自洽（第三道机械自查）

**方法**（一条脚本，把**声明**与**被声明的对象**对起来，都机械可判）：① §6 表头写的风险条数 == `| R<n> |`
行数 == §8 里那句中文数字，且编号 **1..N 连续**；② §1.2 表头"六份表示 + 一条断言链" == 表里 **7 行**；
③ `TACTICS` 结构体的**字段集合** == 散文里提到的那几个、且每个都有消费者；④ 阶段标题恰好 **P0..P7**（各一次）
== §8 的"八个阶段"；⑤ §0 条数 == 表头写的中文数字且编号连续；⑥ 每个 `§x.y` 引用都能找到对应标题
（悬空引用 = 死指针 ✗）；⑦ §12 的"六条更正" == 该节（`### 12.1` 之前）的编号项数 ✓。
**读数**：**全部一致 ✓**。⚠ **它当场咬到一次**：`TACTICS` 的结构体草图里**漏了 `semantic_kind` 字段**，
而全文有**三处**（§4.2 的 L3②、§5 P1 的交付物、R17）都在用它 ⇒ P1 会照错的形状建表 ✗ ⇒ 已补齐 ✓
（§12.3 那道自查也有过一次假阳性 ⇒ 两道**各咬到一次**，说明它们**真会红** ✓）。
