# 值位 `intro` 关键字 + 展开补全（2026-09-12 设计；同日实现）

> as-built（2026-09-12 实现轮）：降低模块落在
> `crates/front/src/compile/intro.rs`（不在 `check.rs`，避免巨石继续膨胀）；
> 命名 helper 提取为 `proof::fresh_name`，与 `suggest::restart_skeleton`
> 同源；补全项 label 为 `intro（展开为 fun 骨架）`（`filter_text = intro`，
> 门控命中时不重复给裸关键字项）；两个前置修正（§3 P0a/P0b）先行落地。
> 验收见 §11（已全部通过：front 255 / lsp 96 / cli 48 / course 4 测试全绿）。

> 触发（用户原话）：「copilot 会直接给一个补全，但是答案太硬邦邦了，像
> 作弊器。我觉得我们插件也应该实现一个简单的补全……类似 intro 的功能，
> 和 by 同等级的关键词，比如 `intro`」。目标体验：
>
> ```lean
> theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a := intro
> ```
>
> VS Code 建议把 `intro` 原地替换成
> `fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => sorry`。
>
> 用户拍板：**一次全剥**（等价 Lean `intros`）；匿名 binder 名沿用现有生成器
> 约定 **`x` / `x2`**；本轮**只落设计**。实现轮按三件套（语法白名单 + 课程 +
> 测试）执行。

## 1. 体验目标

两条路径都成立、互补：

1. **直接写 `intro` 就是合法答案**：声明进入 Open（练习态），无红波浪线；
   `intro` token 本身是一个洞，编辑器在它后面按既有 inlay 机制显示剩余目标
   （`:= intro` 上直接看到 `: And b a`），于是「写 lambda 看不到后续目标」的
   原始痛点也一并缓解。
2. **输入 `intro` 时补全弹出**：一项「intro → fun 骨架」，接受后 token 原地
   替换为显式骨架。骨架由声明类型**唯一确定**（结构展开，不是搜索、不是 AI
   猜答案）：Copilot 替学习者写正文，这里只替他写壳。

展开与否都是学习者的选择：接受 = 看清 `intro` 的含义；不接受 = 继续用关键字，
两边都由完整内核终审填洞结果。

## 2. 语法与语义

### 2.1 语法（与 `by` 同级，值位专属）

- `intro` 只在 `parse_value`（`crates/front/src/parser.rs:103-111`）识别，
  与 `by` 同一个挂点：`def` / `theorem` / `example` 的值位，且整个值就是它。
- 新 AST 节点 `Expr::Intro { span }`，放在 `Expr::By` 旁
  （`crates/front/src/ast.rs:60-65`，`span()` 补 `ast.rs:68-84`）。
- **不动 lexer**：全仓只有 `forall` 是保留词（`crates/front/src/token.rs:242`）；
  `And.intro` / `Or.intro` 是单个 `Ident`（`.` 是标识符字符，
  `token.rs:269-271`），不受影响；`by intro a` 里的 tactic 解析路径
  （`parser.rs:143-201`）原样保留。
- 值位带尾随 token（如 `:= intro a`）= 既有「unexpected token」parse 错误，
  v1 不做带名字的 `intro a b h`（见 §9）。

### 2.2 语义（全剥，降低为显式 lambda + 洞）

`intro` 的求值发生在 front 层（`check.rs`），**不碰内核**：

- 取声明类型 `ty`（已 parse 的 AST），沿最外层 Pi 望远镜逐层剥：每个
  `Forall` binder（逐个，多名字组拆成多层）或 `Arrow` 域生成一个显式
  `Expr::Lambda` binder，直到余下的 codomain 不再是 Pi/Arrow；
- 末端是 `Expr::Hole { span: intro token 的 span }`；
- 于是值被**降低**为普通的部分作答 `fun … => sorry`，完整复用既有
  open-exercise 流水线：`open_goal`（`compile/goals.rs:635-705`）算
  goal/binders/holes，`PendingOp::OpenExercise` → `DeclState::Open`，
  inlay、code action、`soko/goals`/`nextGoal`/`stateAt` 全部自动生效；
- 声明类型照旧被 elaborate 只为 `ty_text`（`check.rs:330-348` 等）；
  值永远不进 elab/kernel，填洞后的候选仍由内核合成声明终审
  （REQUIREMENTS §2 第 8 条）。

命名规则（与既有生成器一致）：

- 声明的 `Forall` 带名 binder → 用原名；
- 匿名层（`Arrow`，以及无名的 `Forall`）→ 基名 `x`，撞名追加序号
  （`x` → `x2` → …）。该规则已存在于 `suggest.rs:247-265`
  （`restart_skeleton`），实现时提取成共享 helper，两边同源；
- 隐式 `{…}` binder 保留花括号风格（`proof.rs:282-292` 的
  `render_binder` 已支持）。

非函数目标（`theorem t : True := intro`）→ **教学错误**，不静默当 `sorry`：
值位 `intro` 需要目标至少一层函数。新增稳定 code
`elab-intro-not-a-function`（stage elab），hint 指向「先看目标最外层箭头；
不是函数就直接写答案或 `sorry`」。`docs/protocol.md` 的错误码清单与
`protocol_doc_lists_every_error_code`（`compile/tests.rs:1104`）同步更新。

### 2.3 骨架文本（`DeclState.intro_skeleton`）

front 在降低成功时同时产出显式骨架字符串
（`render_expr(lowered)`，末端洞渲染为 `sorry`），随 `PendingOp::OpenExercise`
→ `DeclState` 新增字段 `intro_skeleton: Option<String>` 传给 LSP：

- 单一事实源：LSP 请求期零解析、零文本比对；session 快照自动缓存（String
  无需 remap，洞 span 见 §5 前置修正）；
- 只有「值恰好是 `intro`」的 Open 声明才有该字段，天然完成补全门控。

## 3. 实现路径（建议）

1. **前置修正 P0a：`render_expr` 函数位括号**
   （`crates/front/src/proof.rs:228-230, 270-279`）。现状把应用作函数位置的
   括号也补上，渲染出 `(And a) b`；骨架、binders、goal 都会带上，且与
   内核 pp 的 `ty_text`（`kernel/pretty_printer.rs:631`，打印 `And a b`）
   不一致。修法：函数位置只对 lambda/forall/arrow（及无法作函数的 plus）加
   括号，参数位置保持现状。同步更新被钉断言：
   `compile/tests.rs:2093`、`:2131`、`:2613`、`:2623`、
   `crates/lsp/src/lib.rs:1996`、`docs/protocol.md:228`。
2. **parser**：`Expr::Intro` + `parse_value` 分支 + 全部 exhaustive match
   （`render_expr`、`elab.rs:600` 附近补防御 arm、`goals.rs:201` 的
   `expr_has_hole` 视 Intro 为洞、`semantic.rs:245/301` 归类 Keyword）。
3. **降低（lowering）**：`check.rs` 三个值位分支（Def/Theorem/Example，
   `check.rs:298/442/651` 附近）在 `open_goal` 之前调用新 helper
   `lower_intro_val(ty, val) -> Result<Option<(Expr, String)>, CompileError>`，
   与 `lower_by_val`（`check.rs:122-135`）并列；错误走既有 failed 路径。
4. **数据贯通**：`OpenExercise`（`check.rs:40-59`，构造点
   `349-362/484-497/689-702`）与 `DeclState`（`report.rs:69-116`，字面量
   `check.rs:928/977/1035/1251`）加 `intro_skeleton`。
5. **语义高亮**：`semantic::KEYWORDS`（`semantic.rs:42-63`）加 `intro`，
   `by` 块里的 tactic `intro` 也顺势高亮；`And.intro` 不受影响。
6. **补全**（见 §4）。
7. **课程与文档**（见 §7）。

备选（不推荐）：在 `goal_under_binders` 里加 Intro 分支直接算 binder/goal。
省一步降低，但骨架与洞的生成逻辑会分叉，且 `intro` 无法与「部分作答的
lambda 链」共享同一套 walk 回归。降低法把复杂度压在一点，收益是整个 Open
机制免费复用。

## 4. 补全协议（LSP）

现有通道：`textDocument/completion`（`crates/lsp/src/lib.rs:954-1029`）目前
只返回 label/kind/detail、无 `textEdit`；能力声明在 `lib.rs:657-663`；
客户端 `vscode-languageclient` 自动转发，**VS Code 扩展零改动**（静态契约
测试 `crates/cli/tests/extension.rs` 无需动）。

新增一个门控分支：

- **门控**：光标落在某声明内（`render::decl_at`，`render.rs:298`），该声明
  `status == Open` 且 `intro_skeleton.is_some()`，且光标 offset 落在
  `d.holes[0]`（= `intro` token span）内。
- **返回恰好一项**：
  - `label` = `intro`，`filterText` = `intro`（保证已敲前缀能匹配；v1 只在
    整词 `intro` 时提供，前缀触发留 v2）；
  - `textEdit` = 把 `d.holes[0]` 替换为 `intro_skeleton`（LSP 官方建议用
    `textEdit` 而非 `insertText`）；
  - `insertTextFormat` = `PlainText`：不做 tab-stop/联动占位符，避免
    「魔法感」；
  - `kind` = `KEYWORD`；`documentation` = markdown，展示完整骨架并说明
    「值位 `intro` = 一次引入剩余全部 binder（等价 Lean `intros`）」；
  - `sortText` 置顶（如 `0`），`preselect` 视实现轮体验定（单项且门控窄，
    建议 true）。
- **与既有动作共存**：既有一次一层的 `intro` 快捷修复
  （`actions.rs:124-139`）目标为 Pi 的剩余 goal；`intro` 全剥后剩余 goal 非
  Pi，两者不会同时出现。`restart_skeleton`（`suggest.rs:208-275`，失败声明
  整值替换）保持不动，名字规则与 §2.2 同源。

## 5. 前置修正 P0b：session 洞 span 重映射

`Session::remap_prefix`（`session.rs:271-313`）今天只 remap `state.span`、
`by_steps`、hovers、events、errors；`DeclState.holes` / `sub_goals[].span`
**没有** remap（既有潜伏缺陷）。`intro` token 是洞，注释编辑后的零重编译
路径必须让它的坐标保持正确（否则 inlay / 补全 textEdit / nextHole 全部
指偏）。修：在 `remap_prefix` 补 `holes` 与 `sub_goals`，并加回归测试
（对照 `session.rs:801` 的 by_steps 重映射测试）。

另注意 `judge_hole_fill` 有「洞位源码必须恰为 `sorry`」的字面守卫
（`judge.rs:316-322`）。v1 的 `intro` 主洞走 `judge_terms`（按 spec 判定，
不查源码文本），且 `sub_goals` 为空，不触发；若 v2 允许 `intro` 进构造子
spine，需先放宽该守卫。

## 6. 业界对照（调研摘要）

- **Agda**：`C-c C-r` refine——洞里为空时「插入 lambda 或构造子（若选择
  唯一）」；确定性、用户触发。ref：`tools/emacs-mode` 文档。
- **Coq/Rocq**：`refine` 允许 term 里留洞并生成子目标；交互入口是 tactic/
  命令而非自动补全。ref：Rocq reference manual（refine/intro）。
- **Lean 4**：`intro`/`intros` 只在 tactic 位；确定性骨架（Batteries 的
  instance/match 骨架、`try?` 的 "Try this"）走 **code action + 引擎预验证**，
  补全留给名字。ref：Lean tactics 文档、vscode-lean4 manual。
- **LSP 3.17**：推荐 `textEdit` 而非 `insertText`；`filterText`/`sortText`
  控制排序过滤；补全噪音是已知问题，需服务端按语法上下文门控。

结论：确定性展开 + 用户确认是成熟范式；本设计在「值位关键字可独立成立」
这一点上比上述工具更贴近教学（关键字本身就是合法答案）。

## 7. 测试计划（三层）

**front**

- parser：`:= intro` → `Expr::Intro`；`by intro a` 仍是 tactic；`And.intro`
  不受影响；`:= intro a` 报 parse 错误；`render_expr(Intro)` 回写 `intro`；
- 降低 + walk：`and_swap` 例 → Open、goal `And b a`（P0a 后）、binders
  `a:Prop, b:Prop, x:And a b`、holes = intro token、`intro_skeleton` 恰为
  `fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => sorry`；
- 命名：`def f : Nat -> Nat -> Nat := intro` → `x`/`x2`；声明 `x` 撞名 → `x2`；
- 非函数目标：`elab-intro-not-a-function` 诊断 + Failed，不产生 Open；
- 内核纪律：`intro` 声明不增加 `kernel_checks`、不污染 env（对照
  `open_exercise_does_not_pollute_env`）；trusted prefix 跳过路径同样识别。

**CLI e2e**

- 含 `:= intro` 的文档 `--json`：`exercise.open`、零 error（非函数目标时有
  对应 diagnostic code）。

**LSP / 协议**

- completion：门控命中时恰好一项，`textEdit.range` 恰为 `intro` token，
  `new_text` 等于骨架；`And.intro`、`by` 块内、已判定声明均不返回该项；
- inlay：`intro` 后显示 `: And b a`（复用 `holes`/`goal`，无需新代码）；
- `soko/goals` / `nextHole` / `stateAt` 对 `intro` 声明的表现（洞列表、
  跳转、剩余目标）；
- session：注释编辑零重编译后 `holes` 坐标正确（P0b 回归）。

**课程 / 契约**

- `course/` 单元（中文 + en 镜像 + 解答钥匙）与 golden 计数；
- 白名单文档（parser whitelist / `semantic::KEYWORDS` / protocol 错误码）；
- `protocol_doc_lists_every_error_code`、skill 词汇封闭守卫。

## 8. 课程与文档（三件套）

- 课程：在 unit6-by-tactics 加「两种写法对照」一节并配练习——
  `:= intro`（值位，一次全剥，穷举是 named binder）vs
  `:= by intro a; …`（tactic，一次一层，自己起名）；英文镜像重写、非逐字
  翻译（沿用课程双语纪律）；
- `docs/protocol.md`：新错误码；补全行为若定为对外契约则加一小节；
- `docs/README.md` 设计索引加本文；`docs/TESTING.md` 测试地图同步。

## 9. 非目标（v1）与 v2 前瞻

- 不做：嵌套 `intro`（`fun … => intro`、构造子实参位）、`intro a b h` 指定
  名字、前缀触发补全、批量「全部展开」命令、任何 AI/搜索式建议；
- v2 候选：前缀触发（`int` 即提示）、练习树上的「展开 intro」入口（需要新
  command + manifest + 契约测试）、值位 `intro` 的 code action、
  `intros` 别名（消除与 tactic `intro` 的层数歧义）。

## 10. 风险

- **同名遮蔽**：`def x : T := intro` 在有同名 axiom/def `intro` 时会被关键字
  截胡——与 `by` 同款既有语义（`parser.rs:104-111`），仓库无实例；文档需
  明确「值位 `intro` 是关键字」。
- **层数语义落差**：tactic `intro` 一层 vs 值位 `intro` 全剥，课程必须对照
  讲清，否则初学者会困惑。
- **warning 文案**：LSP 的 sorry warning 按 `holes` 非空触发
  （`lsp/lib.rs:216-241`），`intro` 声明会得到同一句 "uses 'sorry'"。
  实现轮二选一：接受（它确实是未解洞），或把文案改成通用的「有未填的洞」。
- **文件膨胀**：`goals.rs`（705 行）与 `check.rs`（1354 行）已超/接近模块
  上限，新代码放 `check.rs` 的降低 helper + `suggest.rs` 抽共享命名
  helper，避免继续堆进 `goals.rs`。

## 11. 验收（实现轮）

- `:= intro` 在 CLI 与 LSP 里均为合法 Open，`--json` 事件、诊断、
  `intro_skeleton`、inlay、补全项全部符合 §4/§7；
- `by`/`And.intro`/既有测试与课程零回归；fmt/clippy/gate 全绿；
- 课程中文 + en + 钥匙与 golden 计数通过；
- STATUS / REQUIREMENTS §9 / protocol/测试地图同步。

## 12. 追加轮（2026-09-12，0.17.0）：词尾命中 + hover 按钮 + 等价契约

> 触发：用户反馈 `playground.sokonanoda:201`「同一行输入 `intro` 正常，
> 换行就没用了（那行会太宽）」，并加两个要求——hover 加一个直接替换
> `intro` 的按钮（等价 Tab 补全）；`intro` 不被替换也直接等价于 `fun`
> 表达式。

### 12.1 命中区间：不是换行，是光标停在词后

旧判据是洞的**闭区间字节范围**（§4 的门控写的是「offset 落在 `d.holes[0]`
内」）。学习者敲完关键字那一刻，光标在 token **末尾之后**：要么正好贴在
词尾（`<= end` 覆盖），要么顺手多打一个空格（落到区间外）。折行书写让
「停在词尾」成为常态，所以症状看起来像「换行就坏」，实际同一行
`:= intro ` 也坏——第 36/37 轮只修了补全在词尾那一格，hover 与尾随空白
都没跟上。

现判据（hover 与补全**共用**）：

- `trailing_same_line_ws(text, end, offset)`：`offset > end`，且
  `text[end..offset]` 全是空格/制表符（**不含换行**）；
- `intro_hit` = 洞闭区间 **或** 尾随同行空白；
- `intro_at` 再对声明 span 应用同一条尾随空白规则（`:= intro ` 时光标可能
  已经在声明之后），返回 `(洞, 骨架)`，hover 与补全都从这里取——单一事实源
  仍是 `DeclState`，编辑器零扫描、零重算。

跨行**故意不算**：光标落到下一行（新声明或空行）不该再弹，回归
`intro_expansion_is_not_offered_on_a_later_line` 守护。

### 12.2 hover 展开按钮（`command:sokonanoda.expandIntro`）

hover markdown 追加一条命令链接，载荷是服务端算好的
`{uri, range, newText}`——与补全项的 `textEdit` 同源，因此**不会分叉**：

- 编码：`percent_encode_component`（`encodeURIComponent` 语义），额外把
  `(` `)` 也编成 `%28` `%29`——载荷嵌在 `](command:…?)` 里，骨架常含
  `fun (x : Prop) => …`，裸 `)` 会被 markdown 链接解析提前收尾；
- 客户端：`sokonanoda.expandIntro` 只做一次 `WorkspaceEdit`（uri + range +
  newText），不扫文本；
- **受信 markdown**：LSP hover 默认 `isTrusted = false`，命令链接点了没反应。
  客户端 `LanguageClient` 选项用 `markdown: { isTrusted: { enabledCommands:
  ["sokonanoda.expandIntro"] } }`——白名单只放行这一个命令；
- `package.json` 声明该命令并在命令面板隐藏（`when: false`，纯 hover 目标）；
- 静态契约 `manifest_declares_commands_that_extension_registers` 自动覆盖
  「package.json 声明 ↔ extension.js 注册」的一致性。

### 12.3 「不替换也等价」= 契约，不是文案

§1 第 1 条本来就把「直接写 `intro` 就是合法答案」列为体验目标，但没有测试
钉住，容易被后续改动悄悄破坏。追加两条 front 契约：

- `intro_is_equivalent_to_typing_the_skeleton_out_by_hand`：把
  `intro_skeleton` 原样粘回去 vs 写 `intro`，必须同 `status`、同 `goal`、
  同 `binders`、同洞数（差别只有洞的 span 与骨架字段本身）；
- `value_intro_is_layout_independent`：同页 / 换行 / 尾随空格三种排版，
  判定与骨架**一字不差**。

hover 文案同步改成明说「不替换也完全等价」（LSP 测试断言该措辞存在）。

### 12.4 仍然不做（§9 不变）

嵌套 `intro`（`fun (a : Prop) => intro`）**不是**「不替换」的一种形态：
值位关键字只在 `parse_value` 识别（§2.1），lambda 体内的 `intro` 是普通
标识符，会得到既有的 `unknown identifier`。本轮不动这条边界。
