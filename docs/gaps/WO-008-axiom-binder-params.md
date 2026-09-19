# WO-008 axiom 不吃 binder 参数表（G-13）

> 台账行：`docs/gaps/ledger.jsonl` 第 8 行（`id=G-13`，`kind=language`，`severity=painful`，
> `status=open`，`wo=null`）。本 WO 落地后应把该行 `wo` 指向本文件、`status` 改 `wo-filed`
> （见文末「关账」）。
> 本 WO 只描述**做法与验收**；写它的这一轮**没有改任何源码、复现件或课程文件**，只创建了本文件。

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴）：
  `bash docs/gaps/repro/G13-axiom-binder-params.sh`
- 今天的实际输出（2026-09-18 于版本 `0.58.0` 实跑，`exit 0` = 缺口仍在；只贴关键行）：

  ```text
  == ① binder 形式（照 Lean 4 习惯写）==
  {"code":"unexpected-token","message":"expected axiom type, found LParen",
   "span":{"start":{"column":11,"line":1},"end":{"column":12,"line":1}},"stage":"parse","type":"diagnostic"}
     → unexpected-token（预期）：yes
  == ② 柯里化形式（同一语义）==
  {"human":"checked declaration Foo","name":"Foo","type":"decl.checked"}
  == ③ 对照：def 接受 binder（同一台二进制）==
  {"human":"checked declaration Bar","name":"Bar","type":"decl.checked"}
  == ④ 加重情节：同一份 binder 文件走 query check（G-10）==
  {"data":{"counts":{"decl_checked":0,…其余计数全零…},"failed":[],"warnings":[]},"ok":true,…}
  结论：G-13 仍在（axiom 拒 binder、柯里化可用、def 接受 binder）——与台账一致。
  ```

  即：`grade` 因 parse 错误 exit 1，而**同一份文件**走 `query check` 报 `ok:true` + 全零计数
  ——那是 **G-10 / WO-003** 在掩盖它（`docs/design/set-theory-syllabus.md:122-123`
  已把「凡 `decl_checked` 突降为 0 一律用 `grade` 复核」写成课程红线）。G-10 与本缺口
  独立修复，本 WO 不改 `query check`。

- 复现现场的最初来源：调研探针 `docs/notes/settheory-survey/repro/P1-axiom-params.sokonanoda`
  （同目录 `P1b-axiom-curried.sokonanoda` 是对照），结论写在
  `docs/notes/settheory-survey/set-theory-teaching-survey.zh.md:291-295`。

- **通过 / 失败边界对照**（同一台 `scripts/soko` 二进制实测，9 种写法；第 4 列是修完后的
  预期，实现时请直接用这张表做先后对拍）：

  | # | 写法 | 今天（0.58.0 实测） | 修后预期 |
  |---|---|---|---|
  | 1 | `axiom Foo (α : Type) : Prop` | exit 1 `expected axiom type, found LParen` @1:11 | **checked** |
  | 2 | `axiom Foo (α β : Type) : Prop` | exit 1 `found LParen` | **checked**（多名字组，走 `push_binders`） |
  | 3 | `axiom Foo {α : Type} : Prop` | exit 1 `found LBrace` | **checked**（隐式 binder 组） |
  | 4 | `axiom Foo {u} (α : Sort u) : Prop` | exit 1 `found LParen` | **checked**（宇宙参数 + binder 混排） |
  | 5 | `axiom Foo (α) : Prop` | exit 1 `found LParen` | 仍**拒绝**，文案变成「声明 binder 需要显式类型…」 |
  | 6 | `axiom Foo (α : Type) -> Prop`（缺冒号） | exit 1 `found LParen` @1:11 | 仍**拒绝**，位置移到 `->`、`found Arrow` |
  | 7 | `axiom Foo : (α : Type) -> Prop` | exit 0 checked | **不许回归**（checked） |
  | 8 | `axiom Foo {u} : (α : Sort u) -> Prop` | exit 0 checked | **不许回归**（checked） |
  | 9 | `axiom Foo : Prop` | exit 0 checked | **不许回归**（无 binder 路径逐字不变） |

  第 5、6 行的两处**报错文案/位置变化是改进**（更早指到真正的错处），但必须写进 CHANGELOG 式
  的一轮记录里；仓库内**没有任何测试或 golden 依赖旧文案**：全仓 grep
  `expected axiom type` 只命中台账/设计/调研文档（`docs/gaps/ledger.jsonl`、
  `docs/design/teaching-project.md:126`、`docs/design/set-theory-syllabus.md:113`、
  `docs/notes/settheory-survey/…:293`、复现脚本注释），**零测试命中**。

- 现存 axiom 的写法审计（写本 WO 时全仓扫描 `**/*.sokonanoda`，280 条 `axiom` 行）：
  **279 条已是柯里化或无参**，唯一一条 binder 形式就是上面那个故意失败的调研探针
  （`docs/notes/settheory-survey/repro/P1-axiom-params.sokonanoda:2`，位于 `docs/`，
  不进任何测试/门禁）。⇒ 这是**纯增量**改动，见「范围」的兼容策略。

## 期望行为

- **官方 Lean 4**：`axiom`（`constant` 的命令层拼写）与 `def`/`theorem`/`example` 一样接受
  声明级 binder 参数表，`axiom Foo (α : Type) : Prop` 与 `axiom Foo : (α : Type) -> Prop`
  语义完全等价（binder 只是把类型写成 Π 望远镜的糖）。依据：台账 `expected_lean` 字段
  （`docs/gaps/ledger.jsonl` G-13 行）、`docs/design/set-theory-syllabus.md:112-114`
  （「`def`/`theorem` 允许参数表，`axiom` 被拒是本语言最容易绊倒课程作者的一条」）、
  `docs/design/decl-binders.md:82`（当年设计时把 `axiom` 列为**边界**）。
  说明：本会话 `web_search` 不可用（无 API key），**没有**联网核对官方手册原文
  ⇒ 「引用官方手册某一页」待确认；但「与柯里化形式等价」这一条本仓已可自证：
  复现 ② 实测 `axiom Foo : (α : Type) -> Prop` 判卷通过，且 `build_axiom` 只吃
  `ty`（`crates/front/src/compile/elab.rs:623-642`），糖只改 Expr、不改内核路径。
- **本教学子集的边界**（以下都**不做**，见下一节）：
  - binder 组必须显式写类型（`(a : Prop)`）；无类型的 `(a)` 仍报 parse 错误——与 `def`
    完全同款，由 `parse_decl_binders`（`crates/front/src/parser.rs:221-237`，无类型分支
    `:228-232`）免费继承。
  - 只支持声明位 binder 组与宇宙参数 `{u}`；**不做** `variable` / `section` / auto-bound 隐式
    泛化（与 `docs/design/decl-binders.md:84` 的既有边界一致）。
  - 重名 binder、binder 与宇宙参数同名**不新增检查**：实测 `def f (a : Prop) (a : Prop) : Prop := a`
    今天就是 checked（内核接受遮蔽），所以 axiom 也一视同仁。（`docs/design/decl-binders.md:85`
    写的「重复 binder 名 → parse 错误」与实测不符 ⇒ 该行是**陈旧描述（待确认）**，
    不要在实现里补出这条检查。）
  - 显示面：`#print` 对非依赖 Π 会丢 binder 名——实测 `axiom Foo : (α : Type) -> Prop`
    `#print` 出 `axiom Foo : Type 0 -> Prop`。两种拼写产生**同一 AST**，所以打印形状不变，
    不构成新的不对称；验收不要拿 `#print` 文本当判据（硬规则 4：判定走 kernel）。

## 范围

改动点只有一处（parser 的 `axiom` 分支），下游一行不用动。

- `crates/front/src/parser.rs`
  - `parse_axiom`（`:370-383`）：现在是 `名字 → parse_universe_params()（:373）→
    expect_colon("axiom type")（:374）→ parse_expr()（:375）`。改成与 `parse_def`
    （`:166-184`，binder 在 `:170`）/`parse_theorem`（`:186-204`，binder 在 `:190`）同序：
    `parse_universe_params()` **之后**插 `parse_decl_binders()`，再 `expect_colon`、`parse_expr`。
  - binder 折成类型望远镜：`wrap_decl_binders`（`:34-51`）要求**类型 + 值**两个 Expr，
    而 axiom 无值位 ⇒ 建议抽一个只折类型的辅助函数（例如
    `fn wrap_type_binders(binders: Vec<Binder>, ty: Expr) -> Expr`，空 binder 原样返回），
    并让 `wrap_decl_binders` 复用它，保持「降级逻辑只有一处」。
  - **span 纪律**：`Command::Axiom.span`（`:376`）今天是 `Span::new(start, ty.span().end)`；
    折成 Forall 后 end 仍等于 body end（`wrap_decl_binders` 的既有语义 `:38`），所以 span
    不变。实现时不要在 span 里带上/丢掉 binder；内核拒绝诊断 span 漂移是另一案
    （台账 G-15，另写 WO），本 WO 不碰 `crates/front/src/compile/check/kernel_phase.rs`。
  - 复用全部现成机制，零新语法机制：`parse_decl_binders`（`:221-237`）、
    `push_binders`（`:1126-1141`，多名字组 `(α β : Type)` 展开）、`named_group_ahead`
    （`:789-813`）、`multi_name_group_ahead`（`:1145-1154`）、`universe_params_ahead`
    （`:387-405`，负责 `{u}` 与 `{α : Type}` 的消歧——顺序必须是**宇宙参数在前**）。
- 下游零改动（读码依据，实现时用来复核「不用顺手改」）：
  - `crates/front/src/compile/check/walk.rs:132-138`（`Command::Axiom` → `self.axiom`）、
    `:531-560`（`axiom()` → `build_axiom`）；
  - `crates/front/src/compile/elab.rs:623-642`（`build_axiom` 只 elaborate `ty`，
    产出 `Declar::Axiom { info: DeclarInfo { name, uparams, ty } }`）；
  - `crates/front/src/compile/goals.rs:127-140`（axiom 模板用 `peel_type`（`:561`）
    从 `ty` 剥 binder —— 柯里化写法今天就在走这条）；
  - `crates/front/src/semantic.rs:417-427`（只 `walk_expr(ty)`）、
    `crates/front/src/ast.rs:231`（`Command::Axiom { name, universe, ty, span }` 字段不变）。
- **是否动内核：预期否**（kernel 是冻结快照；`Declar::Axiom` 的形状、`add_declar`、
  宇宙参数收集 `collect_uparams`（`elab.rs:655-662`）都不变）。如果最后发现需要动
  `crates/kernel/**`，**立即停手**回到课程线重新设计——那意味着上面的等价性判断错了。
- 兼容策略（本缺口不牵动既有课程，但要把「为什么不用动」写死）：
  - **纯增量**：只把原先被拒的写法变合法，**不改名、不删写法、不改 AST 形状**；
    279 条既有 axiom（课程/示例/画布）解析路径逐字不变 ⇒
    `crates/cli/tests/course.rs:86-97` 的 11 条 `(checked, open, reduced)` GOLDEN 与
    `crates/cli/tests/course_status.rs:68-79` 的四元组 GOLDEN **本 WO 不需要动**。
  - **本 WO 不改任何课程/画布/示例文件**（含 `playground.sokonanoda` 的既有 axiom）。
    唯一允许的课程侧改动是 `playground.sokonanoda` §0.2（`:34-40`，现在只教
    「`axiom 名字 : 类型`」）**加一行注释**说明也能写参数表：注释不进事件计数，
    playground 锚点（`crates/cli/src/env/mod.rs:285-301`）仍过。
  - **可选的后续「可读性迁移」轮（不在本 WO，列出来是为了防止本轮顺手扩大）**：把下列
    柯里化 axiom 改成 binder 形式——`courses/set-theory/lib/Set.sokonanoda:45-46`（`Set.ext`）、
    `courses/set-theory/lib/Exists.sokonanoda:68-70`（`Exists` 三件套）、
    `courses/set-theory/lib/Rel.sokonanoda:51`（`Rel.ext`）、
    `courses/set-theory/lib/Logic.sokonanoda:20,52-55`、
    `examples/py-fol-core.sokonanoda:17,41-42,56-58`、
    `playground.sokonanoda:99,104,109-111,115-117`。那轮必须重跑课程判卷并重测锚点。

## 不做的事（明确排除，防顺手扩大）

1. **不动内核**（`crates/kernel/**` 一行不改）；不需要新增内核测试。
2. **不改 G-10**（`query check` 假绿）——那是 WO-003；本 WO 的复现脚本第 ④ 段在修完后会
   自然变绿（因为文件不再有 parse 错误），但**不要**因此去动 query 通道。
3. **不改 G-15**（内核错误 span 漂移）与 `CompileOutput.warning_cmds`
   （`crates/front/src/compile/event.rs:39,63`；0.58.0 的 warning 归因平行数组）——两条独立线。
4. **不改 G-14**（只允许一个宇宙参数 binder）：`axiom Foo {u v} (α : Sort u) : Prop` 仍按 G-14
   处理，本 WO 不扩宇宙参数语法。
5. **不给 axiom 加值位/`:=`/`variable`/`section`/auto-bound/未注解 binder 推断**；
   无类型 `(a)` 继续报错。
6. **不批量重写课程库里已有的柯里化 axiom**（见上「可选的后续迁移轮」）；不为了让某个文件
   「好看一点」而改课程内容。
7. **不新增 event / 错误码 / 协议字段**，因此 `docs/protocol.md` 的对外契约不变。

## 验收（三层）

修完的先后对拍请直接用「通过/失败边界对照」那张 9 行表；判据一律走 kernel（硬规则 4）。

- **front 单测**
  - `crates/front/src/parser.rs` 测试模块（范式：`example_with_decl_binders_parses` `:1383`、
    `type_with_level_parses_as_sort_succ` `:1401`）：新增 `axiom_with_decl_binders_parses`——
    解析表中 1/2/3/4 号，断言 `Command::Axiom { ty: Expr::Forall { binders, body, .. }, .. }`，
    binder 的 `name` / `style`（`Explicit`/`Implicit`）/ 个数正确；
    再断言第 1 行与第 7 行**产出等价 AST**（`Expr` 已 `derive PartialEq`，`crates/front/src/ast.rs:13`；
    若 `span` 有差就归一化后比较 binder + body，具体口径由实现者定并写进测试注释）。
  - 新增 `axiom_untyped_decl_binder_still_errors`（5 号）：断言 parse 失败且文案含「显式类型」。
  - `crates/front/src/compile/tests.rs`：在 `checks_axiom_with_two_universe_params`
    （`:244-253`）旁新增 `checks_axiom_with_decl_binders`（1/2/3/4 号 → `compile_fol` 零
    `errors`）；并照 `decl_binders_match_arrow_style_outcomes`（`:1180-1199`）写
    `axiom_decl_binders_match_arrow_style_outcomes`：同一文件里 binder 与箭头两种 axiom
    各一条，再用一条 `theorem` 消费它们，断言三条都 `Checked`（比结果，不比文本）。
  - 既有 `type_with_level_parses_as_sort_succ`（`:1401`）与全部 `decl_binders_*`
    （`:1116-1212`）**不许改**。
- **CLI e2e**（`crates/cli/tests/cli.rs`，范式 `cli_decl_binders_compile_and_open` `:247-268`、
  `cli_untyped_decl_binder_is_a_parse_error` `:269-274`）
  - `cli_axiom_decl_binders_compile`：stdin 送「4 号写法 + 一条 `:= sorry` 的 theorem +
    一条用该 axiom 的闭合定理」，断言 exit 0、stdout 有 `checked declaration Foo`、
    `exercise open`，stderr 无 `error[parse]`。
  - `cli_axiom_untyped_decl_binder_is_a_parse_error`：`axiom Foo (α) : Prop` → 非 0 +
    stderr 含「显式类型」。
  - 同测试里保留 7/8 号（柯里化 + 宇宙参数）防回归。
- **课程用例**（`courses/set-theory/`，12 单元）
  - 引用：`courses/set-theory/units/unit04-extensionality-identities.sokonanoda` 的练习 1
    `union_comm`（`:70-72`）——它的证明必须调用带参数的公理 `Set.ext`
    （`courses/set-theory/lib/Set.sokonanoda:45-46`），正是本缺口卡住的那族写法；
    同类还有 `courses/set-theory/units/unit09-equinumerosity.sokonanoda` 系列里
    用 `Exists.elim`（`courses/set-theory/lib/Exists.sokonanoda:68-70`）的练习
    `Set.Equiv.symm`（`:113`）/`Set.Equiv.trans`（`:123`）。
  - 判卷命令与**本次实跑基线**（改动前，务必逐字比对）：
    `python3 courses/set-theory/tools/check.py` → 末行
    `34 个目标 —— 296 checked · 93 open · 0 个被判负`，exit 0；
    单单元 `scripts/soko grade courses/set-theory/units/unit04-extensionality-identities.sokonanoda`
    → exit 0（3 `decl.checked` / 8 `exercise.open`）。
    修完这两个数字**必须一模一样**（本 WO 不改课程文件）；变了说明解析或归因被动过 ⇒ 停手排查。
  - 硬规则 3（新增语法 = 课程 + 测试 + 白名单三件套）的「课程」这一件：同轮在
    `playground.sokonanoda` §0.2（`:34-40`）的格式说明处补一句注释，教「axiom 也能像
    def/theorem 一样先写参数表」；`skills/sokonanoda-teacher/references/curriculum.md:10`
    的「声明级 binder」一栏同步提一句（老师要知道这条已解锁）。
- **影响面**
  - 事件计数：既有画布/课程**全为 0 变化**（唯一改动是 playground 注释）。
  - 双 GOLDEN：`crates/cli/tests/course.rs:86-97` 与 `crates/cli/tests/course_status.rs:68-79`
    **不需要同步**；若实现者发现它们红了，说明你动了不改动的东西（本 WO 的既定结论是「不动」）。
  - 复现脚本：修完后 `bash docs/gaps/repro/G13-axiom-binder-params.sh` 应翻成 **exit 1**
    （脚本自身语义：行为变了）；exit 0 说明没修好。**不要**为了让脚本变绿去改脚本。

## 文档同步清单

- `docs/design/decl-binders.md`：§3.3 边界 `:82`（「`axiom`（无值位）、`inductive` 块不加声明
  binder」）改为「axiom 的**类型**位接受 binder 组（无值位，故不产生 Lambda）；inductive 参数
  早已支持」；§4 测试计划（`:100` 起）补 axiom 两例；`:88` 的「重复 binder 名/与宇宙参数重名
  → parse 错误」标注为**与实测不符**（见「期望行为」），别照抄。
- `docs/design/set-theory-syllabus.md`：§2 红线第 1 条 `:112-114` 删除或改为「已修（0.xx.0）」；
  §5 联动表 `:218` 的「柯里化；已在 WO 候选池」改为「已修」。
- `docs/design/teaching-project.md`：§4 缺口表 `:126` 与附录清单 `:494` 的 G-13 行状态更新
  （若 `scripts/gap.py close` 不自动改这两处，手工同步）。
- `docs/notes/settheory-survey/set-theory-teaching-survey.zh.md:291-295`：硬约束第 1 条标注已修。
- `docs/TESTING.md`：`:20`（elab 行）与 binder 相关行补新测试名（测试地图义务）。
- `docs/protocol.md`：对外契约不变 ⇒ 默认**不改**；可选在 `:206-209`
  （「Declarations may still carry Lean-style binders」）补一句「包括 `axiom` 的类型位」。
- `skills/`：`skills/sokonanoda-teacher/SKILL.md:113-114` 的能力速查 +
  `references/curriculum.md:10` 同步；改完确认 `.agents/skills/` 薄入口不漂移
  （`crates/cli/tests/dsh.rs`、`skill.rs` 会挡）。
- `editor/vscode/`：默认**不改**——`syntaxes/sokonanoda.tmLanguage.json:52,63` 的关键词正则
  已通用匹配 `axiom <name>`，binder 走既有括号规则。若实际改动了 `editor/vscode/` 下**任何**
  文件，必须同轮 bump 扩展版本（`docs/vscode-dev-guide.md` 版本纪律）。
- 收尾同轮：`AGENTS.md` / `docs/HANDOVER.md`（待办里若有 G-13 条目一并划掉）/ `STATUS.md`
  （最新轮，旧轮归档）/ `REQUIREMENTS.md` §9 追加一条（用户可见语法变化）。
- 版本与发布：bump `Cargo.toml:6`（现 `0.58.0`）两处版本 → push → auto-tag
  （`docs/RELEASE.md`）；CHANGELOG 式记录写进 `STATUS.md` 本轮。

## 门禁

- `scripts/soko gate`（= `fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp --check`
  + `clippy --workspace --all-targets` + `test --workspace --locked` + playground 锚点；
  实现在 `crates/cli/src/env/mod.rs:244-305`）。**注意** `:245-255`：锚点要求「运行中二进制
  版本 == 仓库版本」，bump 后先 `scripts/soko update`（或
  `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`），
  否则 gate 直接 exit 3（是缓存过期，不是源码 bug）。
- 全量 `cargo test --workspace --locked`。
- 课程侧：`python3 courses/set-theory/tools/check.py`（34 目标 / 296 checked / 93 open /
  0 负，逐字不变）。
- 复现：`bash docs/gaps/repro/G13-axiom-binder-params.sh`（应 exit 1）。

## 关账

```bash
python3 scripts/gap.py close G-13 --version <新版本>
```

- `close` 会先复跑复现；**仍复现（exit 0）则拒绝关账**——所以必须等复现翻成 exit 1 再跑。
- 台账维护：`docs/gaps/ledger.jsonl` G-13 行填 `fixed_in=<新版本>`、`status=fixed`；
  **本 WO 写出的这一轮没有改台账**（交付范围只允许新建本文件），实现者落地时把
  `wo` 补成 `docs/gaps/WO-008-axiom-binder-params.md`。
- 关账后回看「通过/失败边界对照」表：9 行全部落在「修后预期」列，特别是 7/8/9 三行
  **没有回归**。
