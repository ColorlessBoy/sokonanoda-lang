# WO-009 一个声明只允许一个宇宙层级 binder（G-14）

> 台账行：`docs/gaps/ledger.jsonl` **第 1 行**（`id=G-14`，`kind=language`，`severity=painful`，
> `status=open`，`wo=null`，`wo_planned=null`，`repro=docs/gaps/repro/G14-single-universe-binder.sokonanoda`）。
> 本 WO 落地后应把该行 `wo` 指向本文件、`status` 改 `wo-filed`（见文末「关账」）。
> 本 WO 只描述**做法与验收**；写它的这一轮**没有改任何源码、复现件、测试或课程文件**，
> 只创建了本文件。文内所有「实测」均指本轮在仓库根用 `scripts/soko grade …`（启动器解析到
> 仓库构建 `target/release/sokonanoda`，**自报 0.58.0** = `Cargo.toml:6`；探针文件写在
> `/tmp`，未入库）跑出来的**原始输出**，不是推断。

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴）：
  `scripts/soko grade docs/gaps/repro/G14-single-universe-binder.sokonanoda`
  （复现件是 `.sokonanoda`，不是 `.sh` ⇒ **没有** `bash docs/gaps/repro/…` 形式；
  `docs/gaps/README.md` 的「手工复跑」清单里也是这个命令。）
- 今天的实际输出（0.58.0 实跑，**exit 1**；只贴关键行，全文只有这一条诊断）：

  ```text
  {"code":"unexpected-token","message":"声明 binder 需要显式类型，例如 (a : Prop)；不支持无类型的 (a) 写法, found LBrace",
   "span":{"start":{"column":19,"line":17,"offset":989},"end":{"column":20,"line":17,"offset":990}},
   "stage":"parse","type":"diagnostic"}
  ```

  复现件共两行声明：`:17 def id_two_levels {u v} …`（报在 `{`，1:19）、
  `:19 def id_split_levels {u} {v} …`（第二组 `{`）。**parse 在第一条就停**，所以第二条
  要等第一条修好才会露出来——实现时请把两行拆成两个文件分别对拍，别被「只报一条」骗了。
- 台账机械判定（今天实跑）：`python3 scripts/gap.py check` → `G-14  open  sokonanoda  仍有失败`
  （判据见 `scripts/gap.py:71-80`：`exit==0 && 有 decl.checked && 无 diagnostic` 才算 clean；
  `.sokonanoda` 复现**故意不看** `exercise.open`）。全表仍打印「全部与台账一致。」exit 0 ——
  这是**正常状态**，不是 CI 红。
- 台账 `where` 的描述需要一处更正（有实测依据）：`where.area` 写「`{u}` 分支只吃一个名字」，
  但实测 **`def f {u, v}` 是 checked**（下表第 2 行）——`parse_universe_params`
  （`crates/front/src/parser.rs:407-430`）本来就吃逗号分隔的多名字。真正的限制是
  **「只吃一个花括号组，且组内名字必须用逗号分隔」**：`{u v}` 连组都进不去
  （`universe_params_ahead` `:387-405` 见到 `u` 后面不是 `,`/`}` 就返回 false），
  `{u} {v}` 则是第二组没处放。这条更正很重要——它决定了修法是**糖**而不是新机制。
- 台账 `today` 字段里写的声明名是 `id2`，复现件里叫 `id_two_levels`；行为描述一致，
  只是命名对不上，实现者不必据此改复现件。

### 通过 / 失败边界对照（同一台 `scripts/soko`／0.58.0 实测；第 4 列是修完后必须落到的位置）

**这张表就是本 WO 的钥匙**：实现时逐行对拍，比读任何描述都快。

| # | 写法（完整可判卷语句） | 今天（0.58.0 实测） | 修后预期 |
|---|---|---|---|
| 1 | `def f {u} (α : Sort u) (a : α) : α := a` | checked | checked（不许回归） |
| 2 | `def f {u, v} (α : Sort u) (β : Sort v) (a : α) : α := a` | **checked** | checked（不许回归；证明 AST/elab/kernel **早已**支持多宇宙参数） |
| 3 | `def f {u v} (α : Sort u) (β : Sort v) (a : α) : α := a` | exit 1 `unexpected-token`「声明 binder 需要显式类型…found LBrace」**@1:7**（`{`） | **checked** |
| 4 | `def f {u} {v} (α : Sort u) (β : Sort v) (a : α) : α := a` | exit 1 同文案 **@1:11**（第二个 `{`） | **checked** |
| 5 | `def f {u} {u} (x : Prop) : Prop := x` | exit 1「显式类型」@1:11 | exit 1 **`duplicate universe parameter \`u\``**（去重必须**跨组**生效） |
| 6 | `def f {u} {α : Type} (a : α) : α := a` | checked | checked（宇宙组后接隐式 binder 组不许回归） |
| 7 | `def f {α β : Type} (x : Prop) : Prop := x` | checked | checked（**有冒号 → binder 组**，永远不许被当宇宙参数） |
| 8 | `theorem t {u, v} (α : Sort u) (β : Sort v) (a : α) : Eq.{u} α a a := Eq.refl.{u} α a` | checked | checked |
| 9 | 同 8 但写 `{u v}` | exit 1 同文案 @1:11 | **checked**（theorem 与 def 同一路径） |
| 10 | `axiom A {u, v} : Sort u` | checked | checked |
| 11 | `axiom A {u} {v} : Sort u` | exit 1 **`expected axiom type, found LBrace`** @1:13 | **checked**（axiom 走 `parse_universe_params` `:373`，别只修 def） |
| 12 | `def f {a b} (x : Prop) : Prop := x`（无冒号、名字非 `u`/`v`） | exit 1「显式类型」@1:7 | **checked（新行为：2 个未使用的宇宙参数）**——必须钉测试 + 文档承认，见「不做的事」6 |
| 13 | `def f (a) : Prop := Prop`（**圆括号**无类型） | exit 1「显式类型」 | **逐字不变**（`crates/cli/tests/cli.rs:269-274`、`crates/front/src/parser.rs:1360-1366` 都钉着它） |
| 14 | `def f (α : Sort u) (a : α) : α := a`（`u` 没声明） | exit 1 `elab-unknown-universe-level`（hint：用 `{u}` 声明它） | 逐字不变（不做 auto-bound） |
| 15 | `inductive Foo {u} : Sort u` ＋ `ctor mk : Foo` ＋ `end` | exit 1「inductive 参数 需要显式类型…」 | **不变**（inductive 连单个 `{u}` 都不支持，是**另一条**缺口，本 WO 不碰） |
| 16 | `example {u v} (α : Sort u) (a : α) : α := a`（`example {u} …` 同样） | exit 1「声明 binder…」 | exit 1，但文案必须是明确的「example 不支持宇宙参数」（见「范围」4 的决策点） |
| 17 | `inductive Foo (A B : Prop) : Prop` ＋ `ctor mk (a : A) (b : B) : Foo A B` ＋ `end` | checked | checked（H6-C 的多名字组不许回归；`docs/architecture.md:130-133`） |
| 18 | 2 宇宙参数的声明 + `#check f.{1}` | exit 1 `elab-universe-arity`：`constant \`f\` expects 2 universe argument(s), got 1` | 不变（**后续迁移轮的硬约束**，见「兼容策略 C」） |
| 19 | `def f {u} (α : Sort u) (a : α) : α := a` ＋ `def g (A : Type) (a : A) : A := f A a`（调用点不写 `.{1}`） | exit 1 kernel-rejected「类型不匹配：期望 \`Sort(0)\`，实际是 \`Sort(1)\`」 | 不变（**宇宙层级没有推断**，调用点必须显式写层；`crates/front/src/compile/tests.rs:215` 同款） |
| 20 | `def f.{u} (α : Sort u) : α -> α := fun a => a` | **checked，但声明的名字是 `f.`**（`{u}` 被吃成宇宙组） | 不变 —— canonical Lean 拼写 `def f.{u, v}` 本语言**既不支持也不报错**，是一条**新缺口**（建议课程线另立一条，本 WO 不做） |

第 19 与 18 两行是给「想顺手把 `congrArg` 改成跨宇宙」的人看的：**那件事不是文本替换**。

## 期望行为

- **本语言内部的地面真相（本仓可自证，优先于外部记忆）**
  - 设计白名单**本来**就把多名字宇宙参数写成合法语法：`docs/design/decl-binders.md:33-40`
    的语法块是 `def name {u, v} (binders) : T := v`，并写明「`{u}` / `{u, v}`（只有名字、
    逗号分隔）是宇宙参数」；`docs/design/agent-query-channel.md:522` 也记「⑦ 宇宙参数
    `{u, v}` 已是多名字」。
  - 真实用例早就这么写：`examples/py-fol-core.sokonanoda:13` 的
    `def Function.comp {u, v, w} : {α : Sort u} -> {β : Sort v} -> {δ : Sort w} -> …`，
    而它在测试里（`crates/cli/tests/cli.rs:443-460` `cli_checks_ported_py_fol_core`）判卷通过。
  ⇒ **G-14 是「设计已有多宇宙参数、实现只认逗号单组」的落差**：`{u v}` 是同一组的空格拼写，
    `{u} {v}` 是同一构造连排两组。修法因此是**纯语法糖**，语义侧今天就能跑：
    本轮用等价拼写实测了迁移轮要用的那条引理——
    `def congrArgUV {u, v} (α : Sort u) (β : Sort v) (f : α -> β) (a b : α) (h : Eq.{u} α a b) : Eq.{v} β (f a) (f b) := Eq.subst.{u} α (fun (x : α) => Eq.{v} β (f a) (f x)) a b h (Eq.refl.{v} β (f a))`
    → **`{"human":"checked declaration congrArgUV", …}` exit 0**。
    即：**parser 之外一行都不用动**（AST 早有 `universe: Vec<String>`，kernel 早收多参数）。
- **官方 Lean 4 里是什么行为：待确认**（本轮**没有**联网核对：`web_search` 报
  「no API key for DEEPSEEK_API_KEY」，与 `docs/gaps/WO-008-axiom-binder-params.md:71-73`
  记录的环境限制同款；官方工具链按硬规则 2 禁用，`lean`/`lake` 一律不许调）。
  - 台账 `expected_lean` 已断言 `def f {u v} (α : Sort u) (β : Sort v) : …` 合法，
    本 WO 以它为准；旁证是 Mathlib 真实的 `congrArg` / `Function.comp` 都是两三个宇宙参数
    （本仓 `examples/py-fol-core.sokonanoda:13,69` 就是它们的移植）。
  - **实现者的核对义务（待确认项，别默默放过）**：发布前查一次 Lean 4 手册的
    Universe Levels / `declId` 一节，确认 ①`def f {u v}` 与 ②`def f {u} {v}` 在官方 Lean 里
    是否都合法，把结论写进 `STATUS.md` 本轮。**硬规则 3 的风险点**：教学语法必须是真实
    Lean 4 的**子集**——若官方其实只接受 `{u, v}`（或只接受 `def f.{u, v}`），那接受
    `{u v}` 就是**超集**，两条出路（**由课程线裁决，不由实现者独断**）：
    (a) 照做 + 在 `docs/design/decl-binders.md` 明写「本语言的扩展拼写」，并回填台账
    `expected_lean`；(b) 只收 `{u, v}` + `{u} {v}`，把复现件第一条改成 `{u, v}` 并同步台账
    ——(b) 会牵动复现件语义，**必须先经过课程线**，本 WO 的默认是 (a)。
- **本教学子集的边界（修完后的统一规则，一句话）**：
  声明头 `:` 之前的 `{ident+}`（**无冒号**）一律是**宇宙参数组**，可出现在任意位置、可连排；
  `{ident+ : T}`（有冒号）一律是**隐式 binder 组**。宇宙参数仍必须显式声明
  （不做 auto-bound），仍必须显式写层地调用（不做层级推断）。
- 以下都**不做**：`def f.{u, v}` 声明位拼写、`example` 的宇宙参数、`inductive`/`ctor` 的
  宇宙参数、`universe` 命令、`variable`/`section`、未声明宇宙变量自动绑定、层级推断。

## 范围

改动点集中在 `crates/front/src/parser.rs` 的**声明头**解析；下游（elab/kernel/AST/协议）零改动。

- `crates/front/src/parser.rs`
  - `universe_params_ahead`（`:387-405`）：今天是「`{` → ident →（`,` ident）* → `}`」。
    改成「`{` → ident+ → `}`」（**允许空格分隔**，逗号仍接受）；**仍然要求紧跟 `}`**，
    于是 `{α β : Type}`（有冒号）不会被误判——`named_group_ahead`（`:789-813`）与它
    的先后顺序保持**宇宙参数在前**（`:385-386` 的注释要同步改）。
  - `parse_universe_params`（`:407-430`）：循环吃 ident，`,` 与空格都当分隔；**去重检查
    必须留在同一处**（`:415-417`），这样表第 5 行「跨组重复」免费得到。
  - `parse_decl_binders`（`:221-237`）：现在是「`{`/`(` + `named_group_ahead` → binder 组，
    否则报「需要显式类型」」。改成：遇到**宇宙形状**的 `{ident+}` 就调
    `parse_universe_params()` 把名字**并进同一份 `universe` Vec**（连排多组）；
    其余分支不动。建议签名改成
    `fn parse_decl_binders(&mut self, universe: &mut Vec<String>) -> Result<Vec<Binder>>`
    （或返回 `(Vec<String>, Vec<Binder>)` 让调用点合并）——**别把宇宙名字塞进 `Binder`**，
    那会污染 `wrap_decl_binders`（`:34-51`）与 `by` 引擎的 `initial_binders` 语义。
  - 四个调用点：`parse_def` `:169`、`parse_theorem` `:189`、`parse_axiom` `:373`、
    `parse_rec` `:511`（`rec` 在 inductive 块内，表里没单列，但同路径要一致）。
  - **`Example` 是决策点**：`parse_example`（`:206-216`）**没有** `parse_universe_params`，
    且 `Command::Example`（`crates/front/src/ast.rs:226-230`）**没有 `universe` 字段**
    （`Def` `:214`／`Theorem` `:221`／`Axiom` `:233`／`RecDecl` `:273` 都有）。
    今天 `example {u} …` 报的是误导性的「需要显式类型」（表 16 行）。改造后
    `parse_decl_binders` 会顺带吞掉宇宙组，**必须显式决定**：推荐让 example 分支对
    非空宇宙组报**明确文案**（例如「example 不支持宇宙参数；请改用 theorem/def」），
    并加测试钉死；不要让它静默吞掉（那样 `example {u} (α : Sort u) …` 会变成
    「宇宙参数被丢弃但 `Sort u` 又报未声明」，比今天更难懂）。
- 下游零改动（读码依据，用来复核「不用顺手改」）：
  - `wrap_decl_binders`（`:34-51`）只看 binder；
  - `crates/front/src/compile/elab.rs:655-662` `collect_uparams(builder, univ, universe: &[String])`
    ——吃的是**名字切片**，与它们来自几组无关；
  - `crates/front/src/compile/warning.rs`：`WarningKind` 只有
    `ReservedDeclarationName` / `ImportHasOpenExercises` / `RedundantSorry` ——
    **没有**「未使用宇宙参数」警告，这直接决定表第 12 行的后果（见「不做的事」6）。
- **是否动内核：预期否**（`crates/kernel/**` 一行不改）。理由不是信念而是实测：
  表第 2 行 `{u, v}` 今天 checked、本轮 `congrArgUV`（跨宇宙）用 `{u, v}` 也 checked
  ⇒ 从 parser 往下整条链路已经支持「多个宇宙参数 + 类型分处不同宇宙」。如果实现中发现
  需要动内核，**立即停手**回课程线重新设计——那说明这个等价性判断错了。

### 兼容策略（三段，必须写进实现轮的报告）

**A. 语法修复本身：纯增量，默认不改任何课程内容。**
证据：全仓 105 个 `**/*.sokonanoda` 文件扫描（跳过 `--` 注释行）里，**只有复现件**
（`docs/gaps/repro/G14-single-universe-binder.sokonanoda:17,19`）用到了 `{u v}` / `{u} {v}`
这两种形状，命中数 2；含 `{u` 的行共 86 行（含注释），其余全是「单名单组」或
「逗号多名单组」。⇒ 既有课程/示例/画布的解析路径逐字不变。两个 GOLDEN
（`crates/cli/tests/course.rs:86-97` 的 11 个三元组、`crates/cli/tests/course_status.rs:68-79`
的 11 个四元组）**不需要同步**；若它们红了，说明你动了不该动的东西。
注意它们的 `COURSE_MANIFEST` 是 `course/course.json`（`course_status.rs:13`）——
**`courses/set-theory` 根本没进这两个 GOLDEN**（`crates/cli/tests/*.rs` 里 grep
`set-theory` 零命中），set-theory 的门禁是 `python3 courses/set-theory/tools/check.py`。

**B. 同轮必须做的课程件（硬规则 3 的「课程」那一件，唯一允许的计数变化）。**
在 `courses/set-theory/lib/Logic.sokonanoda` 的 L-05 段（`:33-38`）**新增一条跨宇宙引理**，
用新语法写，**不动既有 `congrArg`**（避免 A 段之外的连锁改动）：

```lean
-- L-05b：跨宇宙 congrArg —— G-14 修好后可以写 `{u v}`（此前只能塞进同一宇宙）。
def congrArgUV {u v} (α : Sort u) (β : Sort v) (f : α -> β) (a b : α)
    (h : Eq.{u} α a b) : Eq.{v} β (f a) (f b) :=
  Eq.subst.{u} α (fun (x : α) => Eq.{v} β (f a) (f x)) a b h (Eq.refl.{v} β (f a))
```

（这段的**语义**本轮已用 `{u, v}` 拼写实测 checked；名字 `congrArgUV` 是**临时名**，
迁移轮并入 `congrArg` 时删除——若课程线已另有命名，以课程线为准。）
同轮把 `:34-35` 的「只能同宇宙层级…都解析不了（台账 G-14）」注释改成「已修（0.xx.0）」，
并把 `courses/set-theory/lib/Exists.sokonanoda:46` 的同类注释一并更新。
**预期数字**：`scripts/soko grade courses/set-theory/lib/Logic.sokonanoda` 从 **26 checked**
变 **27 checked**（本轮实测基线 26）；`python3 courses/set-theory/tools/check.py` 从
`34 个目标 —— 296 checked · 93 open · 0 个被判负` 变 **`297 checked · 93 open · 0 个被判负`**
（exit 0）。**除这两处外，课程数字必须逐字不变**（单单元基线见「验收」）。
若课程线明确要求本轮不加这条引理，则替代课程件 = `playground.sokonanoda` §0.2 语法说明处
加一行**真实演示 decl**（并保证 gate 锚点仍 PASS）；**但两者必做其一**，不接受「只改语法」。
另外 `courses/set-theory/gaps/README.md:16` 把 G-14 从「待修清单」移到已修一节。

**C. 明确排除、但要给下一轮留好清单：把 `congrArg` 本身升级成跨宇宙（迁移轮，不在本 WO）。**
它是 ledger `blocks` 真正要的东西，但**不能**和语法修复挤在一刀里，因为**宇宙层级没有推断**
（表 19 行：不写 `.{1}` 的调用点会被 kernel 拒），所以升级 `congrArg {u}` → `{u v}`
会让**所有显式写层的调用点报 `elab-universe-arity`**（表 18 行），必须逐点重新定层：
`Set α -> Prop` 的像写 `.{1, 0}`，`Type -> Type` 的写 `.{1, 1}`——**不是文本替换**。
迁移轮需要同轮改动的文件（**共 10 个文件**：1 处定义 + 15 处调用点，调用点分布在 9 个文件）：

| 文件 | 位置 | 备注 |
|---|---|---|
| `courses/set-theory/lib/Logic.sokonanoda` | `:36-38`（定义） | 升级为 `{u v}`；删除临时的 `congrArgUV` |
| `courses/set-theory/units/unit06-relations.sokonanoda` | `:189`（另 `:12` 是注释清单） | `.{1}` → `.{1, 1}` |
| `courses/set-theory/units/unit08-images-preimages.sokonanoda` | `:210` | 逐点定层 |
| `courses/set-theory/units/unit10-cantor.sokonanoda` | `:75`、`:80` | `(Set α) → Prop` ⇒ `.{1, 0}` |
| `courses/set-theory/units/solutions/unit06-solution.sokonanoda` | `:290` | |
| `courses/set-theory/units/solutions/unit07-solution.sokonanoda` | `:45`、`:54`、`:87` | |
| `courses/set-theory/units/solutions/unit08-solution.sokonanoda` | `:265` | |
| `courses/set-theory/units/solutions/unit09-solution.sokonanoda` | `:194`、`:210` | |
| `courses/set-theory/units/solutions/unit10-solution.sokonanoda` | `:78`、`:106` | `(Set α) → Prop` ⇒ `.{1, 0}` |
| `courses/set-theory/units/solutions/unit12-solution.sokonanoda` | `:370`、`:415` | |

`examples/py-fol-core.sokonanoda:69` 里的 `congrArg` 是**该示例文件自带的 Prop 版**
（`{u}` + `β : Prop`），与课程库不是一个声明、也不 import 课程库 ⇒ 迁移轮**不受影响**，
但迁移轮要顺手确认它没被误改。**本 WO 不改上表任何一行**（那是课程线的一刀）。

## 不做的事（明确排除，防顺手扩大）

1. **不动内核**（`crates/kernel/**` 一行不改），不需要新内核测试。
2. **不做 `def f.{u, v}` 声明位拼写**。本轮实测 `def f.{u}` 会被解析成名字 `f.` **并静默
   checked**（表 20 行）——这是 canonical Lean 拼写的陷阱，属于**另一条缺口**
   （建议课程线登记，例如 G-16），本 WO 只在文档里记一笔，不顺手修（它要动 `declId`
   的词法/名字规则，风险与收益都不在 G-14 的范围里）。
3. **不给 `inductive` / `ctor` 加宇宙参数**（表 15 行：连单 `{u}` 都不支持；
   `parse_inductive_binders` `:474-490` 是另一条路径；`docs/architecture.md:385` 把
   「宇宙多态参数」列为未做，与实测一致）。也不碰 H6-C 的多名字 binder 组（表 17 行只做回归）。
4. **不给 `example` 加宇宙参数**（表 16 行；`Command::Example` 没有 `universe` 字段）。
   只能在「报明确文案」与「维持今天文案」之间二选一，且必须加测试钉住；推荐前者。
5. **不做 auto-bound / 未声明宇宙变量推断 / `universe` 命令 / `variable`+`section`**：
   表 14 行保持 `elab-unknown-universe-level` 逐字不变。
6. **不新增 warning kind**。「`{a b}` 从 parse 错误变成 2 个未使用宇宙参数」（表 12 行）
   是本修法**不可避免**的副作用（`{u v}` 与 `{a b}` 在文法上**同形**，除非按名字特判，
   那是荒谬的）；补救手段是加一条 `unused-universe-parameter` 警告，但那是**新协议字段**
   （`WarningKind` → `docs/protocol.md` 的 warning code 列表 + golden 观感），
   属于另立一条 WO 的事。本 WO 只要求：加测试钉住新行为 + 在 `docs/design/decl-binders.md`
   的边界一节写明这条取舍，保住 `(a)` 圆括号那条教学文案（表 13 行）。
7. **不做迁移轮**（兼容策略 C 的 15 处调用点）——一次一刀（`docs/design/teaching-project.md:326-329`）。
8. **不碰 G-10 / G-15 / `CompileOutput.warning_cmds`**（`crates/front/src/compile/event.rs:39,63`
   的归因平行数组）：三条独立线，本 WO 不许「顺手对齐」。
9. **不改复现件、不改台账、不改 `scripts/gap.py`**（即使发现 `close` 的护栏只覆盖 `.sh`
   复现——见「关账」——也另立一条 infra 缺口，不在本 WO 里扩）。
10. **不为了让 `query check` 变好看而改它**（G-10 是 WO-003 的事）；本 WO 的判据一律走
    `grade` 的退出码（`courses/set-theory/tools/check.py` 的既有纪律）。

## 验收（三层）

判据一律走 kernel/退出码（硬规则 4），**不许文本比对**；先后对拍直接用上面那张 20 行表。

- **front 单测**（`crates/front/src/parser.rs` 测试模块，范式 `:1369-1380`
  `universe_params_before_decl_binders_parse`、`:1476-1483` `def_parses_two_universe_params`、
  `:1486-1492` `duplicate_universe_param_is_rejected`）
  - `universe_params_accept_space_separated_names`：解析表 3 行，断言
    `Command::Def { universe, .. }` 的 `universe == ["u","v"]`，`ty` 仍是 2 个 binder 的
    `Forall`（与表 2 行**产出等价 AST**；`Expr` 已 `derive PartialEq`，`ast.rs:13`；
    span 有差就归一化后比 binder + body，口径写进测试注释）。
  - `universe_params_accept_repeated_brace_groups`：表 4 行 → 同一个 `universe == ["u","v"]`。
  - `duplicate_universe_param_across_groups_is_rejected`：表 5 行 → parse 失败且文案含
    `duplicate universe parameter`。
  - `named_group_with_colon_is_still_a_binder`：表 7 行 + `{u} {α : Type}`（表 6 行）→
    binder 组不被误吞（回归钉子）。
  - `example_cannot_declare_universe_params`：表 16 行 → parse 失败且文案是你选定的那句。
  - `untyped_paren_decl_binder_is_a_parse_error`（既有 `:1360-1366`）**不许改**。
  - `crates/front/src/compile/tests.rs`：在 `:192 checks_universe_polymorphic_id_declaration`
    与 `:244 checks_axiom_with_two_universe_params` 旁新增
    `checks_space_separated_and_split_universe_params`（表 3/4/6/9/11 → `compile_fol` 零 errors），
    并复用 `:255 rejects_undeclared_universe_variable`、`:266 rejects_wrong_number_of_universe_arguments`
    作为不变式回归。
- **CLI e2e**（`crates/cli/tests/cli.rs`，范式 `:417 cli_checks_universe_polymorphic_declarations`）
  - `cli_space_separated_universe_params_compile`：stdin 送「表 3 行 + 表 9 行 + 表 11 行 +
    一条消费它们的闭合 `theorem`」，断言 exit 0、stdout 有 3 条 `checked declaration`、
    stderr 无 `error[parse]`。
  - `cli_split_universe_groups_compile`：表 4 行同款。
  - 同测试保留表 1/2/10/13/14 作为防回归；`:443-460 cli_checks_ported_py_fol_core`
    （`{u, v, w}` 真实大文件）**必须继续绿**。
- **课程用例**（`courses/set-theory/…`，12 单元）
  - 引用（本缺口卡住的那族，真实文件 + 练习名）：
    `courses/set-theory/units/unit07-functions.sokonanoda` 的**练习 3 `surjective_comp`**
    （`:114`）与**练习 4 `leftInverse_injective`**（`:127`）——它们的解答
    （`courses/set-theory/units/solutions/unit07-solution.sokonanoda:45`、`:54`）正是
    `congrArg.{1} β γ g (f x) (f y) hxy` 这一族调用；库件是
    `courses/set-theory/lib/Logic.sokonanoda:36-38` 的同宇宙 `congrArg`
    （`:34-35` 注释直接写着「`{u v}` 与 `{u} {v}` 都解析不了（台账 G-14）」）。
    同类跨类型调用还在 `courses/set-theory/units/unit09-equinumerosity.sokonanoda` 的解答
    （`courses/set-theory/units/solutions/unit09-solution.sokonanoda:194/210`）。
  - **改动前的基线（本轮实跑，修完必须逐字比对）**：
    ```text
    python3 courses/set-theory/tools/check.py
      → 34 个目标 —— 296 checked · 93 open · 0 个被判负   (exit 0)
    scripts/soko grade courses/set-theory/units/unit07-functions.sokonanoda
      → 3 decl.checked / 9 exercise.open                   (exit 0)
    scripts/soko grade courses/set-theory/lib/Logic.sokonanoda
      → 26 decl.checked                                    (exit 0)
    ```
    修完后只有 `Logic.sokonanoda` 与全课程总数按「兼容策略 B」**+1**（27 / 297），
    unit07 与其余单元**一个数字都不许动**；动了说明 parse 或归因被动过 ⇒ 停手排查。
  - 硬规则 3 的三件套：**测试** = 上面两节；**课程** = 兼容策略 B 的那条引理；
    **白名单** = `docs/design/decl-binders.md:33-40`。
  - **验收复核命令（修完后必跑，注意与 WO-008 的差别）**：
    `python3 scripts/gap.py check` 应从 `G-14  open  sokonanoda  仍有失败` 变成
    **`G-14  wo-filed  sokonanoda  clean`**（判据 `scripts/gap.py:71-80`）。
    复现件是 `.sokonanoda` ⇒ **没有**「翻成 exit 1」这件事（那是 `.sh` 复现的约定，
    见 `docs/gaps/README.md` 的退出码表）；`scripts/soko grade docs/gaps/repro/G14-…`
    修完后应 **exit 0 且有两条 `decl.checked`**。
- **影响面**
  - 事件计数：**既有文件全为 0 变化**（兼容策略 A 的全仓扫描）；唯一变化是策略 B 的
    课程库 **+1 `decl.checked`**。⇒ **双 GOLDEN 不动**（`crates/cli/tests/course.rs:86-97`、
    `crates/cli/tests/course_status.rs:68-79`；且它们只覆盖 `course/course.json`）。
  - gate 锚点：`crates/cli/src/env/mod.rs:285-301`（读 `playground.sokonanoda` 并
    `check_source`）——按策略 B 走则**锚点本身不改**，仍必须 PASS；按替代方案加演示 decl
    则要确认锚点仍 PASS。
  - 协议/诊断：**不新增** event / 错误码 / warning code；唯一文案变化是表 16 行
    （`example` 的宇宙参数）与新出现的 `duplicate universe parameter`（表 5 行，
    复用既有文案，`parser.rs:416`）⇒ `docs/protocol.md` 的对外契约不变。

## 文档同步清单

- `docs/design/decl-binders.md`：`:33-40` 语法块与「与宇宙参数消歧」那条**必改**
  （写成「`{u, v}` / `{u v}` / `{u} {v}` 等价；有冒号才是 binder 组」）；
  `:43` 与 `:82` 的边界一节补「表 12 行的取舍」（`{a b}` 现在是宇宙参数）。
  这是本 WO 的**白名单**件，漏了等于语法没进白名单。
- `docs/architecture.md`：`:146`（声明级 binder 那段）补一句宇宙参数组的三种拼写；
  `:385` 的「宇宙多态参数」保持未做（inductive 侧），但建议加半句「声明位已支持
  `{u v}`（G-14，0.xx.0）」以免读者误读。
- `docs/design/course-stdlib.md`：`:49`（`Eq.symm`/`Eq.trans`/`congrArg` 那行的
  「**受 G-14 限制**：当前只能同宇宙」）与 `:100`（缺口表 G-14 行）改成「已修；
  跨宇宙迁移见 WO-009 兼容策略 C」。
- `docs/design/teaching-project.md`：附录缺口表 `:495` 的 G-14 行标状态；
  §6.4 的「可执行台账」不用改。
- `docs/design/set-theory-syllabus.md:140`（只是指针，改成「已修」即可）；
  `docs/gaps/spike/README.md:59`（G-14 行）与 `:48` 的 C 类表——**注意**这两处引用的
  `docs/gaps/spike/lib/Logic.sokonanoda` 在仓库里**已经不存在**（`docs/gaps/spike/` 下只有
  `README.md`）⇒ 顺手把死指针改指 `courses/set-theory/lib/Logic.sokonanoda`。
- `courses/set-theory/lib/Logic.sokonanoda:34-35` 与 `lib/Exists.sokonanoda:46` 的注释
  （策略 B 同轮）；`courses/set-theory/gaps/README.md:16` 的待修清单。
- `docs/TESTING.md:19`（parse 行）：把「宇宙参数 `{u, v}`」扩成三种拼写 + 补新测试名
  （测试地图义务）。
- `skills/`：`skills/sokonanoda-teacher/SKILL.md:114`（能力速查里的「宇宙参数」）与
  `skills/sokonanoda-teacher/references/curriculum.md:14`（宇宙那一栏）各提一句；
  改完确认 `.agents/skills/` 薄入口不漂移（`crates/cli/tests/dsh.rs`、`skill.rs` 会挡）。
- `editor/vscode/`：**默认不改**——`syntaxes/sokonanoda.tmLanguage.json` 里
  `宇宙参数|Sort u|universe` **零命中**（无宇宙专用规则），新拼写走既有大括号/关键字规则。
  若最终改动了 `editor/vscode/` 下**任何**文件，必须同轮 bump 扩展版本
  （`docs/vscode-dev-guide.md` 版本纪律）。
- `docs/protocol.md`：默认**不改**（无新 event/错误码/warning code）。
- 收尾同轮：`AGENTS.md` / `docs/HANDOVER.md`（若待办里有 G-14 条目一并划掉）/
  `STATUS.md`（最新轮，旧轮归档）/ `REQUIREMENTS.md` §9（`:124` 起，追加一条**用户可见
  语法变化**：宇宙参数可以写 `{u v}` 与 `{u} {v}`）。
- 版本与发布：bump `Cargo.toml:6`（现 `0.58.0`）两处版本 → push → auto-tag
  （`docs/RELEASE.md`）；CHANGELOG 式记录写进 `STATUS.md` 本轮，并**附上「官方 Lean 4
  是否接受这两种拼写」的核对结论**（见「期望行为」的待确认项）。

## 门禁

- `scripts/soko gate`（= fmt -p front/cli/lsp `--check` + `clippy --workspace --all-targets`
  + `test --workspace --locked` + playground 锚点；实现在 `crates/cli/src/env/mod.rs:244-305`）。
  **注意** bump 版本后要先 `scripts/soko update`（或
  `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`），
  否则锚点因「运行中二进制版本 ≠ 仓库版本」直接 exit 3（缓存过期，不是源码 bug）。
- 全量 `cargo test --workspace --locked`。
- 课程侧：`python3 courses/set-theory/tools/check.py` → `34 个目标 —— 297 checked · 93 open · 0 个被判负`
  （策略 B 执行后；若课程线否决 B，则必须是 296/93/0 逐字不变），exit 0。
- 复现件：`scripts/soko grade docs/gaps/repro/G14-single-universe-binder.sokonanoda` → exit 0、
  两条 `decl.checked`、无 diagnostic；`python3 scripts/gap.py check` → G-14 行 `clean`。

## 关账

```bash
python3 scripts/gap.py close G-14 --version <新版本>
```

- **先自己复核，别信 `close` 的护栏**：`cmd_close`（`scripts/gap.py:198-215`）只在
  `kind == "script" and code == 0` 时拒绝关账——G-14 的复现是 `.sokonanoda`
  （`kind == "sokonanoda"`），**护栏不覆盖它**：即使缺口仍在，`close` 也会照写
  `fixed_in`。所以关账前必须手工确认
  `python3 scripts/gap.py check` 打印的是 `G-14 … clean`。（给 `cmd_close` 补
  `.sokonanoda` 分支是**另一条 infra 缺口**，按「不做的事」9 不在本 WO 里改。）
- 台账维护：`docs/gaps/ledger.jsonl` 第 1 行填 `fixed_in=<新版本>`、`status=fixed`、
  `wo=docs/gaps/WO-009-single-universe-binder.md`；**本 WO 写出的这一轮没有改台账**
  （交付范围只允许新建本文件）。
- 关账后回看那张 20 行表：**3/4/9/11 行必须翻成 checked**，**5 行翻成
  `duplicate universe parameter`**，**12 行落到 checked（新行为：未使用宇宙参数）**、
  **16 行落到你选定的那句 `example` 文案**，**1/2/6/7/8/10/13/14/15/17/18/19/20
  行逐字不动**。任何一行偏离 ⇒ 回滚重做，别改表。
- 下一步（不在本 WO）：把兼容策略 C 的表交给课程线排「`congrArg` 跨宇宙迁移轮」；
  并把表 20 行发现的新缺口（`def f.{u, v}` 静默变名 `f.`）登记进台账。
