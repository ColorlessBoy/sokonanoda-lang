# namespace / open 子集（G-05）——源级命名空间 + 可省略前缀

> 状态：**已落地（as-built）**。台账 `docs/gaps/ledger.jsonl` 的 G-05；复现件
> `docs/gaps/repro/G05-namespace-open.sokonanoda`。硬规则依据：`REQUIREMENTS.md`
> §2 第 3 条（语法增量 = 课程 + 测试 + 白名单三件套）、第 4 条（判定走内核）、
> 第 1 条（内核冻结快照）。本文是白名单的**边界文档**：`docs/architecture.md`
> §4.1 只列命令清单，语义规则在这里。记法（G-04）是本文的直接先例，两篇的
> 「文件内作用域」「不是声明」两条边界同款。

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

## 7. 明确不做（第二刀及以后）

- `open Foo in <cmd>`（局部 open）；
- `open Foo hiding …` / `open Foo renaming …` / `open Foo (a b)`（only）；
- `open scoped`、`export`、`attribute`；
- `section` / `variable` / `include`；
- `namespace` 跨文件传播、把 `namespace` 当模块系统；
- 别名/遮蔽的**警告**（`open` 把两个同名候选都放进来时，本语言按 N4 顺序取
  第一个，不报歧义——Lean 的 open 遮蔽规则比这复杂，v1 不假装对齐）；
- `namespace` 内的 `#print` 反向补全（编辑器补全列表按全名给出，不做前缀折叠）。

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
