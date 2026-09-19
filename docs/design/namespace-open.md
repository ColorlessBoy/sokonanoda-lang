# namespace / open 子集（G-05）——源级命名空间 + 可省略前缀

> 状态：**已落地（as-built）**。台账 `docs/gaps/ledger.jsonl` 的 G-05；复现件
> `docs/gaps/repro/G05-namespace-open.sokonanoda`。硬规则依据：`REQUIREMENTS.md`
> §2 第 3 条（语法增量 = 课程 + 测试 + 白名单三件套）、第 4 条（判定走内核）、
> 第 1 条（内核冻结快照）。本文是白名单的**边界文档**：`docs/architecture.md`
> §4.1 只列命令清单，语义规则在这里。记法（G-04）是本文的直接先例，两篇的
> 「文件内作用域」「不是声明」两条边界同款。
>
> **两刀**：第一刀（0.60.0）= §1–§6 的 `namespace`/`end`/`open` 三条命令；
> **第二刀**（§N7–§N9，2026-09-19）= `open` 的三条子句、`open … in <命令>`、
> `export`、遮蔽警告，外加对 `section`/`variable` 的**评估结论**（§N9：做不动，
> 有实测）。§7 已按「已落 / 已澄清 / 做不动」逐条改写。

## 0. 一句话

`namespace Foo … end Foo` 给区间内的**声明名**自动加前缀（全局名不变），
`open Foo` 把 `Foo.` 加进**可省略前缀**集合（只影响引用解析，不改任何名字）。
三条命令都**不是声明**：不产生事件、不进声明表、不参与判卷计数——与
`import` / 记法命令同族（源级作用域命令）。

## 1. 语法边界（v1 语义规则，逐条可测）

### N1 命令形状

白名单只加**三个拼写**：

```
namespace <Ident>      -- 开一个命名空间块
end <Ident>            -- 闭最近的 namespace（名字必须写出）
open <Ident>           -- 把 <Ident>. 加进可省略前缀集合
```

- `<Ident>` 允许**点分**（`A.B`），与声明名同一套词法（`Set.mem` 今天就是
  一个 `Ident` token）。
- 三条命令都**独占一行**、都在命令位（不能在表达式里出现；`is_reserved_command`
  把它们挡在表达式之外，与 `def`/`notation` 同款）。
- 嵌套：`namespace A` + `namespace B` ⇒ 内层全名前缀 `A.B`；
  `namespace A.B` 一次写成 ⇒ 同样是 `A.B`（两条路径共用 `join_ns`，逐字一致）。
- `end <Ident>` 的**名字必须与最近的未闭合 `namespace` 同名**（写出来的名字，
  或它展开后的全前缀都算同名，见 N2），否则给**专用 parse 错误码**
  `parse-namespace-mismatch` + 人话 hint。
- 文件结束时还有未闭合的 `namespace` ⇒ **专用 parse 错误码**
  `parse-namespace-unclosed` + hint，span 指回那条 `namespace`。
- 名字缺失/不是标识符（`namespace` 后面直接换行、`end` 后面没有名字）⇒
  `parse-namespace-shape` + hint（不落进通用 `unexpected-token`）。**关键字也
  不算名字**：`namespace def x : Type := Prop`（漏写名字）报形状错，而不是把
  `def` 当命名空间名、再在下一行报一个看不懂的错误。

### N2 `end` 的同名判据

每个未闭合的 `namespace` 记两份名字：**写出来的**（`A.B`）与**累积全前缀**
（外层 `C` 里的 `namespace A.B` ⇒ `C.A.B`）。`end X` 在 `X == 写出名` 或
`X == 全前缀` 时算同名。理由：`namespace A` + `namespace B` 的自然收尾是
`end B`，而 `namespace A.B` 的自然收尾是 `end A.B`——两者都该收。Lean 的
`end` 也是「最后一段或全名」都认；我们比 Lean **严格**的一点是**必须写名字**
（Lean 允许裸 `end`，见 §6 差异 1）。

### N3 声明：命名空间前缀是**解析期**决定的名字的一部分

`namespace A` 内 `def mem …` ⇒ 声明名 `A.mem`（**全局名**，写进内核的就是它）。
声明名本身已含点时**拼接**：`namespace A` 内 `def Set.mem …` ⇒ `A.Set.mem`
（与 Lean 同）。归纳块同法：`namespace Set` 内 `inductive Foo` ⇒ `Set.Foo`，
构造子照 R1 规则取规范名 `Set.Foo.mk`（G-02 的 `canonical_ctor_name` 不变）。

前缀在 **parser** 里落定（而不是 elab），因为名字是**语法作用域**的函数：
`Command::Def{name}` 一出门就是全局名，于是 `top_level_def_spans`、闭包级重名
检查（`import-name-collision`）、hover/goto 回填、`references::decl_name_span`
的「最后一段回退」全部**零改动**地看到同一个名字。elab 期再改名会造出第二份
名字真相（硬规则 4 的同款禁令）。

### N4 引用：解析顺序（elab 期，按源码顺序驱动）

维护两条状态：**命名空间栈**（累积全前缀，从外到内）与 **open 集合**（按
出现顺序）。引用 `x` 时依次尝试：

1. **当前命名空间链，从内到外**：`A.B.x` → `A.x`（最长前缀优先）；
2. **精确 `x`**（根命名空间）；
3. **`open` 的前缀**，按 `open` 出现顺序（先开的先试）；
4. 都没有 ⇒ 既有 `elab-unknown-identifier`（hint 里补一句「若它在该命名空间
   里，检查前缀或加 open」）。

命中即止：第一个在 `known` 表里能解析的候选胜出。候选的解析沿用既有
`KnownName` 三态（`Decl` 自身 / 唯一裸名别名 → 规范名 / 歧义 → 报错），所以
命名空间解析与 G-02 的构造子别名**天然共存**（`Set.Foo.mk` 与裸名 `mk` 同一
条路径）。`#check` / `#reduce` / `#print` / 记法目标名 / `UniverseApp` 全走这
一个函数（`resolve_known` / `resolve_known_constant`），没有第二处解析。

`open` **只影响解析**：不重命名任何东西，也不改内核里的名字。`open` 之后新
声明的名字同样享受（`open` 是集合，不是对已有声明的快照）——与 Lean 同。

### N5 作用域边界：文件内（不跨 `import`）

- `namespace` 的作用域是**词法块**：`namespace … end` 之间。parser 在文件尾
  校验闭合，所以一个单元的命令序列里命名空间栈在单元边界必然为空。
- `open` 的作用域是**文件**：闭包编译把多个单元拼成一条扁平命令序，所以 elab
  在**单元切换处清空 open 集合**（命名空间栈同时归零），依赖模块的 `open`
  不会泄漏进入口文件。
- **但被导入模块的全局名本来就可见**（本语言的 `import` 不做模块限定、闭包级
  `known` 是扁平的），所以入口文件里 `open Set` 对**依赖模块声明的** `Set.mem`
  **必须有效**——这是最常用的用法，也是 N5 唯一需要额外钉住的一条。

### N6 三条命令不是声明

不产生 `decl.checked` / `exercise.open` / `expr.typed` / `expr.reduced`，不进
`top_level_def_spans`、不进 `GoalTemplates`、不参与闭包级重名检查、不参与判卷
计数——与 `Command::Import` / `Command::Notation` 同族（N6 与记法设计的 N6
逐字同款）。文件里只有 `namespace`/`end`/`open` 时判卷结果是「0 声明、0 错误」。

## 2. 教学契约（验收的语言）

判卷只认内核：**同一份声明在「写前缀」与「不写前缀」两种写法下必须产生同一
批事件**（名字都是全名）。测试钉死这一条（front 单测比较 `event_shapes`），
不做文本比对。

错误一律走既有稳定码 + 人话 hint：

| 现场 | 码 | hint 说什么 |
|---|---|---|
| `end B` 与最近的 `namespace A` 不匹配 | `parse-namespace-mismatch` | 写出期望的 `end A` |
| `end` 没有对应的 `namespace` | `parse-namespace-mismatch` | 这一行多余/检查是否漏了 `namespace` |
| 文件结束仍有未闭合 `namespace` | `parse-namespace-unclosed` | 在文件尾补 `end Foo` |
| `namespace`/`end`/`open` 后面没有名字 | `parse-namespace-shape` | 三条命令的形状 |
| 引用找不到 | `elab-unknown-identifier`（既有） | 补一句「若它在该命名空间里，检查前缀或加 open」 |

## 3. 命令 / AST / 分发的落点（as-built）

| 层 | 落点 |
|---|---|
| 词法 | **零改动**（`namespace`/`end`/`open` 是普通 `Ident`） |
| AST | `Command::{Namespace, End, Open} { name, span }`（三个变体，都不是声明） |
| parser | `Parser::namespaces`（未闭合栈：写出名 + 全前缀 + span）；`parse_file` 末尾校验闭合；`is_reserved_command` 加 `namespace`/`open`（`end` 早就在） |
| elab 作用域 | `compile/scope.rs` 的 `NamespaceScope`（栈 + open 集合 + 候选生成 + `join_ns`） |
| 声明改名 | parser 里对 `def`/`theorem`/`axiom`/`inductive` 的声明名加前缀（`scope::join_ns`） |
| 引用解析 | `elab.rs::resolve_known` / `resolve_known_constant` 加 `&NamespaceScope` 参数；`ElabCtx` 加 `ns` 字段 |
| 走查 | `Walk::run` 的三条命令臂：`push` / `pop` / `open`；单元切换处 `NamespaceScope::reset` |
| 判卷合成 | `judge.rs` 的前缀解析改用 `parse_fragment`（**容忍**未闭合 `namespace`：前缀本来就是文件的一个片段，合成的 `#check`/合成声明必须落在**仍然打开的**命名空间里） |

`ElabCtx` 的 `ns` 字段与 `inductives` 同款（借用、只读）；`install_inductive_block`
多收一个 `&NamespaceScope`（它在内部自建 `ElabCtx`）。

## 4. 实测边界（本轮真跑出来的，不是推理）

1. **未闭合 `namespace` 会打断判卷合成**。`judge.rs` 的 `parse_prefix` /
   `judge_infer` 都是「前缀源码 + 合成命令」再走完整流水线；`by` 块所在声明
   只要在 `namespace Foo` 里，前缀就带着一个未闭合的 `namespace`。**实测**：
   严格校验下 `def f := by exact …` 在前缀解析阶段就红（`parse-namespace-unclosed`）。
   修法：新增 `parser::parse_fragment`（只跳过文件尾闭合校验，栈**不**自动收），
   判卷合成路径专用。**实测**：改用 fragment 后 `by` 块在命名空间里正常判卷，
   且合成的 `#check`/合成声明落在仍然打开的命名空间内（引用按 N4 解析）。
2. **判卷合成声明的名字不走 parser**（`Command::Def` 在 Rust 里直接构造），
   所以 `_soko_judge_k` **不会**被加前缀——这是**好事**：`judgement_of` 按
   `_soko_judge_k` 查状态，名字必须稳定。
3. **`open` 泄漏**：判卷合成的文件是「依赖文本 + 本文件前缀」，依赖里的 `open`
   行在合成文件里是**活的**。合成文件里能解析、而真身 elab 不能解析的名字，
   只会多报一条 `elab-unknown-identifier`（不会假绿：真身 elab 仍要过），
   所以是**教学噪声**级别的边界，不是可靠性缺口（§6 差异 4）。
4. **`def Set (α : Type)` 这类「类型本身」不能放进 `namespace Set`**：放进去它
   就变成 `Set.Set`，外部 `Set α` 全部失效。课程库因此保留
   `def Set …` 在命名空间**外面**（Mathlib 也是这么排的：`Set` 在根）。
5. **`namespace` 内的 `iota` 规则按源名匹配**（`iota mk := …` 里的 `mk` 对的是
   本块构造子的源名），不需要前缀，也不受前缀影响（既有行为，本轮实测未变）。
6. **`apply` 按文本对齐，是本轮唯一被 `namespace` 打破的既有实现**（实测，
   见本节第 7 条）——`exact` / `assumption` / `rfl` 都走 `judge_terms`（内核判定），
   命名空间里**开箱即用**；`apply` 的 `unify_spine` 拿内核 pp 的 codomain
   与源 AST 的目标做**文本**比较，源里的 `mem` 对不上内核的 `A.mem`。
7. **`apply` 的修法（as-built）**：**用了 `namespace`/`open` 的文件**里，`by`
   块的**根目标先过一遍内核 pp**（`judge::judge_render_type`：合成
   `#check fun (x : <目标>) => x`，取回 `(x : T) -> T` 再剥一层 ⇒ `T` 的规范
   文本）。于是 `apply` 两侧同源（都来自内核 pp），`exact`/`apply` 在命名空间里
   判定一致（front 单测 `a_by_block_inside_a_namespace_resolves_short_names`）。
   没碰命名空间的文件**完全跳过**这一步（零额外开销、行为逐字不变）——
   `unit_uses_namespaces` 在 `Walk::run` 里按单元算一次。规范化失败一律退回源
   AST（绝不因为"规范化失败"把好文件判红）。
8. **依赖模块的 `open` 不泄漏**（N5）已由 CLI e2e 负向钉住：
   `open_does_not_leak_out_of_the_module_that_wrote_it`——依赖文件尾写
   `open Set`，入口不写 `open` 就引用 `mem` ⇒ 必须 `elab-unknown-identifier`
   （若泄漏会假绿）。

## 5. 影响面（事件计数 / golden）

- 三条命令**零事件**：既有语料（`examples/`、课程）里没有这三条命令，所以
  **新增诊断 0 条、计数逐字不变**（实测：`cargo test --workspace` 全绿 +
  课程门禁计数不变，见 §8）。
- 课程迁移（`courses/set-theory/lib/Set.sokonanoda` 包进 `namespace Set`）**不改
  任何全局名**，所以外部引用一行都不用改；课程计数改前/改后逐字相同（§8）。

## 6. 与 Lean 的已知差异（v1 明说，不假装对齐）

1. **`end` 必须写名字**。Lean 4 允许裸 `end`（reference manual 的 `end` 与
   `end …` 是两条语法）。v1 只收 `end <Ident>`，裸 `end` 报
   `parse-namespace-shape` + hint（教学上更省事：错配一眼可见）。
2. **没有 `section`**。Lean 的 `namespace`/`section` 共用一套 scope 命令；v1
   只有 `namespace`（`variable`/`section` 都不做）。
3. **`open` 的解析位次**：本语言是「命名空间链 → 精确 → open（按 open 顺序）」。
   Lean 的确切位次（尤其是「根名」与「open 名」谁先）本轮**没有取证**（无官方
   工具链可用，硬规则 2），所以按本项目自己定死的顺序实现并在此明说；两者在
   课程语料上不产生可观察差异（没有重名遮蔽的用例）。
4. **判卷合成环境里依赖的 `open` 是活的**（§4.3）；真身 elab 仍按 N5 文件内
   作用域判，所以只会多报错，不会假绿。
5. **`namespace` 不是模块系统**：它不产生模块、不参与 `import` 解析、不跨文件
   传播（N5）。Lean 里 `namespace` 与文件/模块是正交的两件事，本语言同样正交
   ——只是本语言的 `import` 不做模块限定，所以「文件内」这条边界必须靠单元
   切换处的 reset 实现。
6. **REPL 里 `namespace` 要写完 `end` 才生效**：REPL 每行都把**整个 buffer**
   重新 `parse`（严格模式），所以单敲 `namespace Foo` 会先报一条
   `parse-namespace-unclosed`，敲完 `end Foo` 之后整个块一起生效（help 文本
   承诺的「同文件」因此逐字成立，代价是中间态会有一条错误）。
7. **子句互斥 + 先过滤后改名**（第二刀）：Lean 能写
   `open Foo (a b) renaming a => c` 这类**组合**子句，两条规则的先后（先过滤后
   改名 vs 反过来）本轮**没有取证**（硬规则 2），所以 v1 只收**一条**子句，
   语义钉成「先过滤、后改名」，改名是**替换**（原短名不再是候选）。
8. **`renaming` 的目标名占位**：`open Foo renaming a => b` 之后 `b` 的候选是
   `Foo.a`，`Foo.b`（若真有）不再作为 `b` 的候选。理由与 N4「第一个命中即止」
   同款：一条 open 对一个短名只给一个候选，避免在候选表里再造一层歧义。
9. **遮蔽警告只看本文件**（§N8.3）：Lean 的 `open` 遮蔽规则比「取第一个」复杂
   得多，本语言按 N4 固定顺序解析、并给一条 `open-shadowed-name` warning 说明
   「其实指向谁」。依赖模块声明的候选不在语法 pass 的视野里，所以那条路径
   不报警（记账在 §N8.3，不假装覆盖）。
10. **`export` 是 `open` 的传播版，不是别名声明**：Lean 的 `export` 会在当前
    命名空间里造**别名声明**；本语言没有模块系统，造别名等于第二份名字真相
    （硬规则 4 的同款禁令），所以钉成「文件内与 `open` 逐字相同 + 跨 `import`
    重放」（§N7.3）。
11. **`open scoped Foo in <命令>` 不做**（§N7.4）：记法生效表是 parse 期全局
    副作用，回滚要克隆整张表；给专用形状错，不静默。

## 7. 第一刀未做项 —— 第二刀逐条销账

> 原文与理由**保留**在下面第一列（历史）；第二刀（0.60.x，as-built 见 §9）逐条
> 给结论：**已落**（做掉了）/ **已澄清**（本来就不需要做）/ **做不动**（做了实测，
> 理由写清，不硬做）。

| # | 第一刀未做项（原文理由保留） | 第二刀结论 |
|---|---|---|
| 1 | `open Foo in <cmd>`（局部 open） | **已落（§N7.2）**：`Command::OpenIn`，`Walk` 用 mark/rollback 撤销；被包住的命令限**叶子命令**（声明 / `#check`/`#reduce`/`#print`），`import`/`namespace`/`end`/`open`/`export`/记法命令各给专用形状错。合成前缀补一行**源码原文**的 open 头（`by` 引擎看同一个作用域）。嵌套 `open A in open B in <叶子>` 可以。`open scoped … in …` **不做**（理由见 §N7.4） |
| 2 | `open Foo hiding …` / `open Foo renaming …` / `open Foo (a b)`（only） | **已落（§N7.1）**：三条**互斥**子句（组合的先后顺序未取证，见 §6 差异 7）；语义钉成「**先过滤、后改名**」，改名是**替换**（原短名不再是候选）。子句里的名字必须是**短名**（带点报形状错） |
| 3 | `open scoped`、`export`、`attribute` | **`open scoped` 已落（第三刀 §14.3）**：`open scoped Foo` 只开记法、不开名字前缀（`scoped: true` 不进 `ns.open`），本文只做**接线确认**（§N7.3）。**`export` 已落（§N7.3）**：文件内与 `open` 逐字相同 + **跨 `import`**（单元切换重放导出表）。**`attribute` 不做**（本语言没有属性系统，也没有内核可挂的 reducibility/instance 元数据；`abbrev` 已经是「与 `def` 同语义的拼写」——见 `docs/design/abbrev.md`，加属性只会造一个没有观察面的语法） |
| 4 | `section` / `variable` / `include` | **做不动（§N9，有实测）**：`variable (α : Type)` 的 auto-bound 要落在「隐式参数自动插入」上，而本语言的应用是**逐位显式**的（`#check id Nat` ⇒ `Nat -> Nat`，实测），落不出 Lean 的体验；`include` 是 Lean 3 的遗产，本语言无对应物，**不做** |
| 5 | `namespace` 跨文件传播、把 `namespace` 当模块系统 | **已澄清：无需做**。`namespace` 本来就是**全局名字前缀**（`Command::Def{name}` 一出门就是 `A.mem`，进内核的就是它），闭包级 `known` 是扁平的 ⇒ 被导入模块声明的 `A.mem` 在入口里**本来就可见**（`open A` 只是让短名可用）。Lean 的 `namespace` 同样是**文件内**的词法作用域、与模块系统正交；「跨文件传播」在 Lean 里由 `import` + `open`/`export` 承担，本语言已经有了（`export` 是第二刀补上的那一半）。把它做成模块系统会与既有 `import` 闭包**打架**（两套模块边界），且没有任何教学收益 |
| 6 | 别名/遮蔽的**警告** | **已落（§N8）**：`open-shadowed-name` warning（不是 error），两条判据——两个 `open` 给同一个短名 / 短名与**根上的**同名声明撞车；有 hint；走既有 warning 通道（`warning.rs` + `decl` 事件流）。**边界明说**：语法级 pass **只看本文件**，依赖模块声明的候选不参与（§N8.3） |
| 7 | `namespace` 内的 `#print` 反向补全（编辑器补全列表按全名给出，不做前缀折叠） | **不做（维持原判）**：补全列表的**数据源**是 `known` 表的全名键（`crates/front/src/suggest.rs`），做前缀折叠等于在补全层造第二份名字真相（硬规则 4 的同款禁令）；收益（少打几个字）不值。`#print` 本身在命名空间里**已经**按 N4 解析（`#print mem` 在 `namespace Set` 里打印 `Set.mem`，第一刀就有） |

## N7. 第二刀的语法与语义（逐条可测）

### N7.1 三条互斥子句

```
open Foo (a b)              -- only：只让 a、b 两个短名进来
open Foo hiding a b         -- 除 a、b 之外都进来
open Foo renaming a => b    -- a 改叫 b（a 随之不再是候选），逗号可列多组
```

- **先过滤、后改名**：`only`/`hiding` 决定哪些声明短名能进来，`renaming` 再把
  进来的那些换个可见名。`renaming` 的目标名**占位**：`renaming a => b` 之后
  `b` 的候选是 `Foo.a`，`Foo.b`（若真有）不再作为 `b` 的候选——一条 open 对一个
  短名只给一个候选，与「第一个命中即止」同款。
- 子句里的名字是**短名**（不带点）：`open Foo (A.b)` 报
  `parse-namespace-shape`（候选会变成 `Foo.A.b`，那不是「省略前缀」而是另一回事）。
- 三条**互斥**（`open Foo (a b) renaming a => c` 报形状错）：组合时「先过滤还是
  先改名」在 Lean 里的确切规则本轮**没有取证**（硬规则 2 禁用官方工具链），
  教学语法不落没取证的语义（§6 差异 7）。
- 重复 `open` 幂等：**逐字相同**的条目只记一次；不同子句各占一个位次，按出现
  顺序参与解析（N4 的 ③）。

### N7.2 `open Foo … in <命令>`（局部）

- 只对**紧跟的那一条命令**生效，命令结束即撤销（`NamespaceScope::opens_mark` /
  `rollback_opens`；幂等性保证「本来开着的同一个前缀」不会被误删）。
- 被包住的命令限**叶子命令**：`def`/`theorem`/`axiom`/`example`/`inductive`/
  `#check`/`#reduce`/`#print`（嵌套的 `open … in` 也算，内层已按同规则校验）。
  `import`（置顶规则）、`namespace`/`end`（块结构会跨出 `in` 的作用域）、
  `open`/`export`（作用域命令对一条命令没有意义）、记法命令（不是声明、不解析
  引用）各给 `parse-namespace-shape` + 人话。
- **被包住的声明照样是声明**：`ast::effective_commands` 把它展开给所有声明级
  pass（`top_level_def_spans`、`GoalTemplates`、语义着色、warning）——否则
  hover/goto 回填、`sorry` 的期望类型、重名检查会静默漏掉它。作用域级 pass
  （`Walk`）**不**展开，`OpenIn` 自己负责压/弹。
- 合成前缀补一行 open 的**源码原文**（`header` span 切出来，不含 `in`）：
  `by` 引擎的根目标规范化与 `judge_terms` 都是「前缀源码 + 合成命令」再走一遍
  流水线，前缀里没有这一行，短名在那里解析不了（退回源 AST 是安全的，但
  `apply` 的文本对齐会失准，见 §4.6）。

### N7.3 `open scoped` 接线 + `export`

- `open scoped Foo`（第三刀已落）：**只**把 `Foo` 作用域下的 `scoped` 记法搬进
  生效表，**不**打开名字前缀（`Walk` 的 `scoped: true` 臂不进 `ns.open`）。
  本文确认这条接线在第二刀之后**未变**（`Command::Open` 的 `scoped` 分支照旧）。
- `export Foo [<子句>]`：**文件内**与 `open` **逐字相同**（同一个候选表、同一个
  位次、同一份子句语义）；额外记进 `Walk::exports`，单元切换（`reset`）时重放
  ⇒ **跨 `import`**：导入本文件的入口在**文件头**就能用这些短名。
  - 为什么这样钉：本语言没有模块系统，`export` 在 Lean 里的完整语义（在当前
    命名空间里造别名声明）需要「别名声明」这一层，而那会造出**第二份名字真相**
    （硬规则 4 的同款禁令）。钉成「`open` 的传播版」之后，内核里的名字一个都
    没变、事件计数一个都没变，唯一的可观察差异是**作用域边界**（`open` 不跨
    `import`，`export` 跨）——这正是 Lean 里两者最重要的差别，也是教学上最需要
    讲清的一条。
  - 导出是**位置性**的：`export` 之后的命令（本文件）与**后续单元**才享受；
    依赖按拓扑序排在入口之前，所以入口看到的是依赖文件里**所有** `export`。
  - **边界（实测）**：闭包是**扁平**的（`known` 本来就跨模块可见、`import` 不做
    模块限定），所以重放是「所有已经走完的单元」，同级的**兄弟模块**也会看到
    彼此的 `export`。这不是新引入的洞：那些全局名**本来就**能点名跨模块引用
    （N5），`export` 只是让短名也跟着可见；而且每个文件**单独判卷**时闭包里
    没有兄弟，所以「B 没 import A 就用了 A 的短名」在 `grade B.sokonanoda` 上
    照样报错——不会假绿到判卷层。要做成「只重放本单元的真依赖」得把依赖图喂进
    `Walk`（`project/graph.rs` 有、`run` 没有），本轮不做，在此记账。
  - `export` 不吃 `in`（导出是长期声明，不是一条命令的临时作用域）：
    `export Foo in …` 报形状错并提示改用 `open Foo in <命令>`。

### N7.4 明确不做（第二刀也没做）

- **`open scoped Foo in <命令>`**：记法生效表是 **parse 期全局副作用**
  （`notations` / `scoped_pending` / `opened_scopes` 三张表），回滚要克隆整张表；
  而名字 open 的 `in` 是 elab 期集合，代价为零。收益（把 `open scoped` 限定到
  一条命令）在课程语料里为零，所以明说**不做**并给专用形状错（不是静默错）。
- **`open Foo (a b) renaming a => c` 之类的组合子句**：见 N7.1（未取证）。
- **子句里名字的「存在性检查」**：`open Foo (typo)` 今天不报错（候选永远解析
  不到 ⇒ 用到时才是 `elab-unknown-identifier`）。理由：`open` 是**集合**，可以
  写在声明**之前**（N4），所以「此刻 `known` 里没有」不等于「这个名字不存在」；
  要做只能在文件尾做一次收尾检查，而它看不见依赖模块（语法级 pass 的文件内
  边界，同 N8.3），会给出**假阳性**。

## N8. 遮蔽警告（`open-shadowed-name`）

### N8.1 判据

一条 `open` / `export` 让某个短名有了**第二个候选**时给一条 **warning**（不是
error，判定与退出码不受影响）：

1. **两个 `open` 给同一个短名** ⇒ 后开的被先开的挡住（候选顺序 ③「按 `open`
   出现顺序」）；
2. **短名与根上的同名声明撞车** ⇒ 根名赢（候选顺序 ②「精确名」在 `open` 之前），
   这条 `open` 对这个短名等于没写。

消息**点名双方**（`x` 会解析到 `A.x`（不是 `B.x`）），hint 给三条出路：写全前缀、
`hiding`、`only`/`renaming`。span 收窄到 `open` 那行的名字 token。

### N8.2 通道

语法级 warning（`crates/front/src/compile/warning.rs` 的
`collect_open_shadow_warnings`，由 `collect_warnings` 每次 update 在整文件上
重算）⇒ 与 `reserved-declaration-name` 同一族：**不进会话快照**
（`is_kernel_verified() == false`），batch 与编辑器两条通道都看得到。
**不是** `Walk` 期警告：`out.warnings` 里非内核终审的条目在 session 路径会被
丢掉（`session.rs::snapshot_from` 只收 `is_kernel_verified()` 的），走那条路会
出现「CLI 报、编辑器不报」的分裂。

### N8.3 边界（明说）

- **只看本文件**：语法 pass 手里只有 `FolFile`，依赖模块声明的候选不在里面，
  所以 `open Set`（`Set` 来自 `lib/Set.sokonanoda`）不会在这里被判遮蔽。
  要覆盖它得把 elab 的 `known` 表喂进 warning pass（`kernel_phase` 有、单文档
  session 没有），那会让 CLI 与编辑器**判据不一致**——比漏报更糟，所以本轮
  统一按文件内口径，并在此记账。
- **命名空间链不参与**：`namespace A` 里 `open A`（候选 ① 与 ③ 都指向 `A.x`）
  不报警——那不是「两个候选」，是同一条候选出现两次。
- **`open … in <命令>` 不报警**：局部 open 是**有意为之**的作用域（"我就想在这
  一条命令里用 `A.x`"），报警是噪声。

## N9. `section` / `variable`：**做不动**（实测结论，不是推理）

任务给的判据是「`variable (α : Type)` 之后，后续声明里出现的自由变量自动变成
binder（Lean 的 auto-bound）」。本轮**实测**了三件事，结论是做不动：

1. **本语言的应用是逐位显式的，没有隐式参数插入/元变量推断**（实测，用本仓库
   二进制跑出来的）：
   ```
   def id {α : Type} (x : α) : α := x
   #check id Nat        ⇒  id Nat: Nat -> Nat      -- α 被**显式**吃掉
   #check id Nat Nat    ⇒  id Nat Nat: Nat
   def u8 : Nat := id8 Nat   ⇒  内核拒绝：期望 `Nat`，实际是 `Pi (x : Nat), Nat`
   ```
   也就是说 `{α : Type}` 今天只是**渲染风格**（`BinderKind::Implicit` 只在
   pp/judge 里换括号），应用位置上它照样占一个实参位。auto-bound 的**全部价值**
   就在「调用点不用写 α」——而那正是这里缺的能力。
2. 于是只剩两条路，都不可接受：**落成显式 binder**（`def f (x : α) : α := x` ⇒
   `(α : Type) → α → α`，每个调用点都要多写一个类型实参，与 Lean 的
   `variable` 体验相反）；**落成 `{…}` 风格**（渲染成隐式、用的时候仍要显式传，
   是**四不像**，而且教学语法接受了一个在 Lean 里行为不同的拼写——硬规则 3
   要求教学语法是 Lean 4 的**子集**，子集必须与 Lean 同义）。
3. Lean 的 auto-bound 到底按 `()` / `{}` 哪个 binder info 落，本轮**没有取证**
   （硬规则 2 禁用官方 Lean 工具链；本会话 web 检索也不可用）。在「没有隐式
   参数」+「语义未取证」两件事同时成立时落一个新语法，风险是**教错 Lean**，
   收益是少写几个 binder——不划算。

顺带记一笔**不做**的另外半条：`section` 若只做「块 + 变量表作用域」而不做
auto-bound，就是 `namespace` 的同义词（本语言的 `namespace` 已经是纯词法块），
没有新语义；`include` 是 Lean 3 的遗产，本语言没有对应物。两者都留在「不做」。


## 8. 测试与验收（as-built，2026-09-19）

三层都落地：

- **front 单测**：`crates/front/src/parser.rs` 11 条（三条命令的 AST 形状、嵌套/
  点分前缀、拼接、`end` 错配、裸 `end`、无对应 `namespace`、未闭合、片段模式、
  保留字）+ `crates/front/src/compile/scope.rs` 3 条（候选顺序/嵌套与点分一致/
  open 是集合与 reset）+ `crates/front/src/compile/tests.rs` 12 条（两种写法事件
  一致、解析顺序 ①②③、open 顺序、`open` 之后新声明、未知标识符 hint、G-02 构造子
  共存、命名空间里的 `by`（`exact` + `apply`）、零事件零声明、`#print`/`#check`、
  hover 解析到全局名）。
- **CLI e2e**：`crates/cli/tests/namespace.rs` 5 条（真文件 + `import` + `open`
  对被导入模块有效、**open 不跨 import 泄漏**、三条命令零事件、`end` 错配、
  未闭合，后两条钉退出码 1 + 专用码）。
- **语料/课程**：`courses/set-theory/lib/Set.sokonanoda` 已包进 `namespace Set`。

验收命令与退出码（全部实跑）：

| 命令 | 退出码 |
|---|---|
| `cargo test --workspace --locked` | 0 |
| `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check` | 0 |
| `cargo clippy -p sokonanoda-front -p sokonanoda-cli --all-targets --locked` | 0 |
| `python3 courses/set-theory/tools/check.py` | 0（36 目标 · 329 checked · 99 open · **0 判负**） |
| `python3 courses/set-theory/tools/check.py --selftest` | 0 |
| `python3 scripts/gap.py check` | 0（G-05 行：fixed / 已判卷通过） |
| `python3 scripts/gap.py selftest` | 0 |
| `node editor/vscode/test-extension-host.js` | 0（14/14） |

课程计数**改前/改后逐字相同**（36 / 329 / 99 / 0）——因为全局名一个都没变；
`--json` 的 `summary` 两次一致。新增诊断 0 条（既有语料里没有这三条命令）。

## 9. 第二刀 as-built（2026-09-19）

### 9.1 落点

| 层 | 落点 |
|---|---|
| AST | `OpenFilter { only, hiding, renaming }`（`ast.rs`，**公开类型**，`lib.rs` re-export）；`Command::Open` 加 `filter` 字段；新变体 `Command::OpenIn { name, filter, inner, header, span }` 与 `Command::Export { name, filter, span }`；`Command::wrapped_command()` + `ast::effective_commands(file)`（声明级 pass 的展开器） |
| 词法 | **零改动**（`hiding`/`renaming`/`in`/`export` 都是普通 `Ident`；`export` 进 `is_reserved_command`，`hiding`/`renaming`/`in` **不**保留——它们是上下文关键字，还能当普通名字用） |
| parser | `parse_open_filter`（三条互斥子句 + `short_name_ahead` 的列表边界）、`expect_short_name`（只收短名）、`parse_open_command` 的 `in` 分支（`is_open_in_body` 限叶子命令，嵌套 `OpenIn` 允许）、`parse_export_command`（不吃 `in`）；形状错统一走新的 `namespace_shape_error`（码仍是既有的 `parse-namespace-shape`） |
| 作用域 | `compile/scope.rs` 的 `OpenEntry { prefix, filter }` + `candidate(name)`（改名目标优先、`visible_short` 过滤）；`opens_mark`/`rollback_opens`（局部 open 的撤销）；open 集合从 `Vec<String>` 变成 `Vec<OpenEntry>`（**逐字相同**的条目去重） |
| 走查 | `Walk::run` 的 `match` 抽成 `Walk::command`（唯一原因：`OpenIn` 要把被包住的命令按同一个 `CmdCtx` 再走一遍）；`Walk::open_in`（压 open → 补一行源码原文的 open 头到合成前缀 → 走 inner → 回滚）；`Walk::exports` 在单元切换（`reset`）后重放 ⇒ `export` 跨 `import` |
| 警告 | `warning.rs` 的 `WarningKind::OpenShadowedName`（码 `open-shadowed-name`，`is_kernel_verified() == false`）+ `collect_open_shadow_warnings`（语法级、只看本文件；`decl` 事件流里带 hint） |
| 声明级 pass | `top_level_def_spans` / `GoalTemplates`（`new_for` / `file_owns_eq` / `file_owns_l1` / `probe_sub_goal_types`）/ `semantic`（着色）/ `warning` 一律改读 `effective_commands`——局部 open 包住的声明**照样是声明**（否则 hover 回填、`sorry` 的期望类型、保留名警告会静默漏掉它） |
| 文档 | CLI help（`crates/cli/src/help.rs`）、`docs/architecture.md`（§4.1 命令清单 + 新段落）、`diagnostic.rs` 的 `parse-namespace-shape` hint（补上新形状） |

### 9.2 实测边界（本轮真跑出来的）

1. **`hiding` 的列表边界要靠关键字判定**：`while peek is Ident` 会把下一条命令的
   `def` 当成列表成员（`open Foo hiding x` + 换行 + `def …` ⇒ 报「子句里要跟短名，
   found `def`」）。修法：`short_name_ahead`（`!is_namespace_name_keyword(name) &&
   name != "in" && name != "hiding" && name != "renaming"`）。**实测**：修后
   `open Foo hiding x\ndef …` 正确解析成两条命令；而 `open Foo hiding a renaming
   b => c`（组合子句）落到 `reject_a_second_clause` 的**专用**形状错——**实测**：
   不加这个终止符时 `renaming` 会被当成一个短名默默吞掉、`=> c` 再报通用
   `unexpected-token`（误导）。
2. **`open Foo in <声明>` 必须进声明级 pass**：`OpenIn` 是包装节点，不展开的话
   `top_level_def_spans` 里没有它、`GoalTemplates` 里也没有它 ⇒ hover/goto 回填
   与 `sorry` 的期望类型会漏（实测：展开前 `def use : Type := inside` 的
   `inside` 解析不到定义 span）。修法：`ast::effective_commands`。
3. **`by` 引擎的合成前缀要补 open 头**：`open A in def f … := by …` 里 `by` 的
   合成文件是「前缀源码 + 合成命令」，前缀里没有那行 open（`prefix_src` 只切到
   `open` 之前）。修法：`OpenIn.header`（源码 span，不含 `in`）+ `Walk::open_in`
   把它按**源码原文**追加到前缀（子句逐字保真）。**实测**：`open A in def y :
   Type := by exact x` 判卷通过。
4. **导出表的顺序依赖拓扑序**：单元切换时重放的是**已经走完的**单元的导出。
   闭包按「先依赖、后入口」拼 `flat`（`check/mod.rs`），所以入口在文件头就拿到
   依赖的全部 `export`。**实测**：`Lib` 里 `export Set`，入口不写任何 open 就能
   用 `mem`（`export_reaches_the_importing_file_while_open_does_not`）。
5. **遮蔽警告的通道选择**：先试了 `Walk` 期 `out.push_warning`，但 session 路径
   只收 `is_kernel_verified()` 的 warning（`session.rs::snapshot_from`）⇒ 会出现
   「CLI 报、编辑器不报」。所以落在**语法级** `collect_warnings`（每次 update 在
   整文件上重算），两条通道一致。**实测**：CLI `--json` 有 `warning` 事件、退出码
   仍 0。
6. **`open … in` 的作用域对 `#check` 同样成立**（不是只对声明）：`open A in
   #check x` 的 `x` 走 `ElabCtx.ns`（`check`/`reduce` 臂本来就传 `self.ns`），
   `#print` 走 `resolve_known` 的同一张候选表。
7. **编辑器词表不在本轮**：`export`（新命令关键字）与 `hiding`/`renaming`/`in`
   （上下文关键字）**没有**进 `front::semantic::KEYWORDS`——KEYWORDS 与
   `editor/vscode` 的 TM 语法词表必须**同一轮**改
   （`crates/cli/tests/extension.rs::tm_grammar_keywords_follow_the_single_source`
   是守护），而本轮禁改 `editor/vscode/package.json` 的版本号（与第三刀的
   `binder_notation`/`scoped` 同一处置）。`namespace`/`open` 本来也不在
   KEYWORDS 里，所以第二刀一字未改，交给主线同一轮同步。
8. **`by` 引擎的局部 open 有专门的回归测试**：`a_by_block_inside_a_local_open_sees_the_same_scope`
   ——**做过变异验证**：把 `Walk::open_in` 的前缀补行去掉，这条测试立刻红
   （`apply` 报 `unknown identifier \`mem\``），补回来即绿。
9. **兄弟模块互相看得到 `export`**（实测：`A` 导出、`B` 不 import `A` 却用了
   `x`，`import A` + `import B` 的入口判绿）。理由与记账见 §N7.3 的边界条：
   扁平闭包下全局名本来就跨模块可见，且**单文件判卷**（`grade B.sokonanoda`）
   闭包里没有兄弟 ⇒ 该报的错照样报，不是判卷层的假绿。

### 9.3 测试与验收（全部实跑）

- **front 单测**：parser 6 条（子句 AST 形状、列表边界、`OpenIn` 的 header/span、
  嵌套、`export`、11 个形状错样本）+ scope 2 条（子句候选表、局部 open 回滚）+
  `compile/tests.rs` 11 条（子句正/反例、改名拿走原短名、`open … in` 局部性、
  包住的声明仍是声明（hover）、局部 open 里的 `by`（`apply`，**做过变异验证**）、
  `export` 文件内、遮蔽 warning（两个 open / 与根撞车 / 不撞不报）、零事件）。
- **CLI e2e**：`crates/cli/tests/namespace.rs` 5 条（`open … in` 局部性、
  三条子句与点名同判 + `only` 反例、`export` 跨 `import`（对照既有 `open` 不跨）、
  遮蔽 warning 退出码 0、5 个形状错退出码 1 + 专用码）。
- **语料/课程**：课程语料**没有**用第二刀的任何新语法（`grep` 实测），所以计数
  逐项不动。

| 命令 | 退出码 | 备注 |
|---|---|---|
| `cargo test --workspace --locked` | 0 | 全绿（含第三刀的测试） |
| `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check` | 0 | |
| `cargo clippy -p sokonanoda-front -p sokonanoda-cli --all-targets --locked` | 0 | |
| `python3 courses/set-theory/tools/check.py --bin <abs>/target/debug/sokonanoda` | 0 | **36 目标 · 329 checked · 99 open · 0 判负**（改前 = 第三刀 §14.7 记录的同四个数 ⇒ **逐项相同**） |
| `python3 courses/set-theory/tools/check.py --selftest` | 0 | |
| `python3 scripts/gap.py check` | 0 | G-05 行 fixed / 已判卷通过 |
| `node editor/vscode/test-extension-host.js` | 0 | 14/14 |

**新增诊断 0 条、新增 warning 0 条**（课程语料没有第二刀语法）；`export` 只改
**作用域边界**、不改任何全局名，所以 `decl.checked` 计数与事件名逐字不变。
