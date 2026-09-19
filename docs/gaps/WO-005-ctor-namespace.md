# WO-005 构造子没有命名空间且全局唯一（G-02）

> 依据标注约定：`文件:行` = 本 WO 作者本次实读（0.58.0 工作树）；「**实测**」= 本次实跑、
> 输出已抄进正文；「**待确认**」= 没跑/没读到，实现时必须先钉一条测试再动手。
> 本文件只是工作单：本次派工**未改任何源码**，也未改 `docs/gaps/ledger.jsonl`。

## 用户可见症状 / 最小复现

- 复现命令（可直接粘贴，模块根 = 仓库根，无需 `--root`）：

  ```bash
  scripts/soko grade docs/gaps/repro/G02-ctor-namespace.sokonanoda
  ```

- 今天的实际输出（**实测**，exit 1，关键行照抄）：

  ```text
  {"human":"checked declaration P1","name":"P1","type":"decl.checked"}
  {"code":"elab-duplicate-declaration","message":"duplicate declaration mk",
   "span":{"start":{"line":20,...}},"stage":"elab",...}
  {"code":"elab-unknown-identifier","message":"unknown identifier `P1.mk`",
   "span":{"start":{"line":24,...}},"stage":"elab",...}
  ```

  复现件里只有 24 行：`P1`/`P2` 两个归纳块各写 `ctor mk`，第 24 行用 `P1.mk` 构造。
  判卷摘要（**实测** `scripts/soko query check --file …`）：
  `counts.decl_checked = 1`、`failed` 两条、`warnings` 空、退出码 1。

- 症状的两半（都与台账 `today` 一致）：

  1. **没有前缀名**：`ctor mk` 装进环境的是裸名 `mk`（`crates/front/src/compile/elab.rs:303-306`
     取 `c.name`，`elab.rs:404` `known.insert(ctor.name.clone(), …)`），所以 `P1.mk` 报
     `unknown identifier`；
  2. **全局唯一**：第二个 `ctor mk` 在 `builder.add_declar` 上撞名字，映射成
     `ErrorKind::ElabDuplicateDeclaration`（`elab.rs:400-402`），**整个 `P2` 块不安装**
     （实测输出里只有 `checked declaration P1`，没有 `P2`）。

- 反证（**实测**，说明缺的只是前端"加前缀"这一步，不是内核不认点）：

  ```text
  ① inductive P1 / ctor P1.mk  →  2 checked（`P1.mk` 可解析、可判卷）
  ② inductive P1 / ctor P1.mk  +  def … := mk A a  →  1 checked +
     failed: unknown identifier `mk`
  ```

  即：**显式写全前缀名今天就能跑**（因为 ctor 名只是一个字符串，
  `crates/kernel/src/builder.rs:90-100` 的 `name_from_str` 按 `.` 切段建层次名），
  但那条路**没有任何裸名兼容**；反过来，`ctor mk` 那条路**没有前缀名**。缺口 = 两条路要合并。

- 顺带实测（**不属本 WO**，供另记账，待确认）：`#reduce Nat.succ (Nat.succ Nat.zero)` 的
  `expr.reduced.text` 掉了右括号（实测输出 `"text":"Nat.succ (Nat.succ Nat.zero"`），
  与本次修复无关，疑似 `#reduce` 命令 span 少吞一个 token。

## 期望行为

- **官方 Lean 4**：`inductive Pair (A B : Type) where | mk : A → B → Pair A B` 之后，构造子自动
  进入类型自己的命名空间，叫 `Pair.mk`；`Prod.mk` / `Subtype.mk` / `Or.inl` / `Exists.intro`
  可以同时存在、互不冲突（Lean 里到处这么写）。裸 `mk` 在 Lean 里**不可解析**（除非
  `open` 了那个命名空间）——Lean 没有类型导向的 `mk` 解析。
- **本教学子集的边界**（写清"哪些不做"，防止被顺手扩大）：
  - 不做 `namespace` / `open`（那是 G-05，独立缺口）；
  - 不做 `inductive … where |` 语法（教学语法只认 `ctor` 块，parser 不动）；
  - 不做 `..`（匿名构造子）与类型导向解析；
  - **裸名别名是本子集的一条"扩展"，不是 Lean 语义**：为了让既有 12 单元 + 入门课
    11 单元 + `examples/` 不因这一刀同时改内容（仓库惯例「一次一刀」见
    `docs/design/teaching-project.md:327-328`，P1 验收明写"既有 11 单元 golden 不回归"，
    `docs/design/teaching-project.md:382-384`），本轮**保留裸名作为解析别名**。
    这条扩展必须在 `docs/architecture.md` 与 CHANGELOG 里点名，并给出日落计划（见"不做的事"）。
- **验收判据**（台账 `expected_lean` 的落地版）：
  `scripts/soko grade docs/gaps/repro/G02-ctor-namespace.sokonanoda` **exit 0、0 诊断**，
  两个归纳块 + 一个 `def` 全部 `decl.checked`，且 `P1.mk`、`P2.mk` 都能解析；
  同时裸名 `mk` 在**唯一**时仍可解析（兼容），在**同名冲突**时按 R2 处理。

## 范围

### 第 0 步：设计先行（仓库硬规则）

先写 `docs/design/ctor-namespace.md`：规范名规则（R1）、别名规则（R2）、源级匹配规则（R3）、
歧义错误码、迁移计划与日落版本。设计没落纸不要动 `elab.rs`。

### 语言层改动点（实读位置 + 要改什么）

| 位置 | 现状（实读） | 要改什么 |
|---|---|---|
| `crates/front/src/compile/elab.rs:303-306` | `ctor_names` = `builder.name_from_str(&c.name)`（源名直通） | 按 R1 生成**安装名**（规范名）数组；同时保留源名数组（match/iota/hover 仍按源名走） |
| `elab.rs:318-333` | `add_inductive(… Arc::from([ind_name]), Arc::from(ctor_names))` | 传入安装名（内核 `all_ctor_names` 必须与 `Declar::Constructor` 一致） |
| `elab.rs:377-402` | `Declar::Constructor { info: { name: ctor_name } }` | 用安装名；错误 span 仍指 `ctor.span` |
| `elab.rs:404` | `known.insert(ctor.name.clone(), …)`（裸名进解析表） | 规范名进表；裸名按 R2 作为**别名键**（`known` 的键 ⇒ 别名命名空间） |
| `elab.rs:2845-2849` | `derive_recursor` 派生 iota 规则用 `ctor.name.clone()` | **必须换成安装名**：内核断言 `rule.ctor_name == ctor.name`（`crates/kernel/src/inductive.rs:1590`），漏改=派生的 rec 一律对不上规则 |
| `elab.rs:428-441` | 显式 `iota` 规则按**源名**匹配（`position(|c| c.name == rule.ctor_name)`） | 不动（R3）；但实现时补一条"显式 rec + 前缀 ctor"的测试 |
| `elab.rs:850-885`（`Expr::Ident`） | `known.get(name)` 后 **`builder.name_from_str(name)`**（用源文本建常量） | **别名必须解析到规范名**：只往 `known` 里加一个裸名键会造出第二个内核常量（`mk` ≠ `P1.mk`），必须把"源名 → 规范名"带进解析（改 `known` 的值类型为 `{canonical, universes}`，或加平行 `canon` 表；`elab.rs:513/553/590/628/781` 五处签名同步） |
| `elab.rs:890-915`（`Expr::UniverseApp`） | 同上的点名前缀问题（`Name` 域） | 同上 |
| `elab.rs:873-884` | `ResolvedTarget::Declaration { name: name.clone() }`（源名） | 填**规范名**，否则 hover/goto 回填（下一行）找不到定义 |
| `crates/front/src/compile/check/kernel_phase.rs:388-401` | 按 `ResolvedTarget::Declaration.name` 去 `top_level_def_spans_over` 里查 span | 与上一条同源：表键换规范名后自动跟随，但**必须有 hover/goto 回归**（裸名 + 前缀名两种写法各一条） |
| `crates/front/src/compile/check/mod.rs:619-651`（`top_level_def_spans`） | 第 636-638 行把 **ctor 源名**塞进闭包唯一性表 | 换成规范名（否则跨模块的第二个 `mk` 仍被下面那道闸拦下） |
| `crates/front/src/project/mod.rs:325-361`（`check_name_collisions`） | 用上表做**闭包级唯一性**，重复报 `import-name-collision`（`project/mod.rs:222` 调用） | 不改判定逻辑；但这是"语言层修好、项目层照旧报错"的**第二道闸**，必须补跑项目用例（见验收） |
| `crates/front/src/compile/goals.rs:77-109` | refine 模板的 `CtorTemplate.name` / `funcs` 键取源名 | 模板文本（学员看到的 `refine …` 骨架）与 `docs/protocol.md:308-313` 的契约对齐：**显示规范名**，解析靠别名兜底 |
| `crates/front/src/semantic.rs:158-187`、`578-596` | `declaration_kinds` 用 `ctor.name`（源名）建表；`classify_ident` 查 `names.decls` / `names.ctors` | 让 `Prod.mk` 这类点名前缀 token 归到 `CtorUse`（protocol 词汇不变，仍是 `ctor_use`，见 `docs/protocol.md:355-362`）；`semantic.rs:930-945` 已有 `("succ", CtorUse)` 风格断言，照抄加一条 |
| `crates/lsp/src/lib.rs:1389-1400` | 补全 = 关键字 + sort + `PRELUDE_NAMES` + 文档声明名 | 若声明名列表来自 AST 源名，决策"补全给规范名还是给裸名"（建议规范名；`PRELUDE_NAMES` 已是点名形态） |

### 已核对、**不需要**改的点（防止顺手扩大）

- `crates/front/src/compile/prelude.rs:153-171`、`211-229`：prelude 的 `Nat.zero`/`Nat.succ`/
  `Bool.true`/`Bool.false` **本来就是点名前缀**，且 `docs/architecture.md:316-318` 明写这三个
  Bool 名字是内核 name cache 的预留槽位 —— R1 必须"已带点则不重复加前缀"，prelude 一行不改。
- `check/mod.rs:352-366`（`user_top_level_names`）只收**类型名**喂 Eq prelude 闸，不收 ctor 名；
  `check/mod.rs:475-505` 的 Nat/Bool 装载闸按 `Command::InductiveBlock{name}` 判 —— 都不受 ctor 改名影响。
- `project/mod.rs:418-423`（`owns_inductive`）取点名**根**再比对，所以依赖模块自带
  `inductive Nat`（⇒ ctor 规范名 `Nat.zero`/`Nat.succ` ∈ `PRELUDE_OWNED`，`project/mod.rs:52-62`）
  仍会被豁免，**不会**冒新的 `import-prelude-conflict` —— 已核对，别顺手改它。
- `elab.rs:428-441` 的 `iota` 源名匹配、`Expr::Match` 的 canonical 化（`elab.rs:1299-1330`）：
  分支名按源名走，本轮不动（R3）。

### 是否动内核

**预期否。** 证据（实读）：

- `crates/kernel/src/builder.rs:90-100` `name_from_str` 已按 `.` 切段、逐段建 `Name::Str/Num`；
  `pretty_printer.rs:940-951` 同法 —— 层次名是内核既有能力，不是新语义；
- prelude 的 `Nat.zero`/`Bool.true` 今天就走这条路（`prelude.rs:153-171`、`225-229`），
  并且能被 `#check`/`#reduce`/`match` 正常消费；
- 内核 `mk_name_cache`（`crates/kernel/src/util.rs:1046-1074`）只按名字查 intern 表打标记，
  不要求声明来自 prelude。

若实现中发现必须动内核（例如某个断言要求 ctor 名与归纳名同段），**停手**：按
`docs/architecture.md:337-360`（§6 改动清单）流程加"只加不改语义"的内核条目 + 三层回归，
并回来更新本 WO 的范围表。

### 兼容策略（本轮落地口径）

- **R1 规范名（进内核 = 打印名）**：源 `ctor` 名**已含 `.`** ⇒ 原样（保护 prelude 与既有显式写法）；
  否则 ⇒ `"{归纳名}.{ctor名}"`。
- **R2 裸名别名（只在解析层，绝不进内核）**：闭包内**唯一**的裸名 ⇒ 解析到规范名
  （表键 = 裸名，值 = 规范名；`known` 与 `canon` 同步维护）；**重复**的裸名 ⇒ 裸名不可解析。
  二选一并在设计文档里定死：报 `unknown identifier`（复用 `elab-unknown-identifier`）
  或新码 `elab-ambiguous-ctor-alias`（**若选新码**：必须同轮进 `docs/protocol.md` 错误码表 +
  `ErrorKind` 稳定码测试 `crates/front/src/compile/tests.rs:753-816`
  （`every_error_kind_has_stable_code_and_hint`）与 `tests.rs:1431`
  （`protocol_doc_lists_every_error_code`，它会直接抓住"加了码没写协议"））。
  **待确认**：`known` 是闭包级扁平表（`check/mod.rs:473` 一次 `run_pass` 建一张），
  别名因此天然是闭包级、无模块作用域 —— 与今天 `check_name_collisions` 的扁平口径一致，
  设计文档要写清这一点（模块作用域属 G-05）。
- **R3 源级写法不动**：`match` 分支（`| none =>`）、显式 `iota zero :=`、注释/文案继续按**源名**；
  但**一旦课程把 `ctor zero` 改写成 `ctor Nat.zero`（迁移），同文件的 `iota`/`match` 也必须
  跟着改点名前缀** —— 这是迁移轮的注意点，写进设计文档的迁移清单。

### 同轮改动文件清单

**A. 语言层（本轮必改）**：上表 15 行位置（`elab.rs` / `check/mod.rs` / `check/kernel_phase.rs` /
`project/mod.rs`（只加测试）/ `goals.rs` / `semantic.rs` / `lsp/src/lib.rs`（视补全决策））
+ 新建 `docs/design/ctor-namespace.md`。

**B. 测试文本（本轮必改 19 处，表共 20 行 —— 差的那行 `parser.rs:1803` 明确不动）**：
`crates/cli/tests/cli.rs`（5）、`crates/cli/tests/examples.rs`（1）、
`crates/front/src/compile/tests.rs`（13，含
`match_source_inductive_nat_still_uses_bare_ctors`（`tests.rs:4213-4239`）改名/重钉）。

**C. 课程内容：本轮 0 个文件**（靠 R2 别名继续绿）。这是本 WO 的**主验收判据**：
`python3 courses/set-theory/tools/check.py` 与 `cargo test` 在**课程内容一字未改**的前提下全绿。
若实现时发现别名撑不住（例如 `goal` 文本与学员书写漂移导致某题判负），**不要在这一刀里改课程**，
而是回到设计文档评估：那属于"强制改名分支"。

**D. 强制改名分支（若 R2 被否，同轮必须一起改的文件，共 37 个 `.sokonanoda`）** ——
先记在这里，附标识符，便于评估工作量（统计口径：仓库内全部 `.sokonanoda`、`docs/` 下的复现件除外；
去掉纯注释行后按词边界匹配**裸**构造子名，负向后顾排除 `Nat.zero` 这类点名前缀；本 WO 作者脚本实算）：

| 区 | 文件数 | 涉及构造子 |
|---|---|---|
| `course/`（入门课 11 单元 + solutions + en/ + shared/） | 23 | `inl` `inr` `zero` `succ` `none` `some` `vnil` `vcons` `red` `green` `le_refl` `le_succ` |
| `courses/set-theory/`（卷 I，12 单元） | 13 | `inl` `inr` `prod_mk` `aa` `bb` `zero` `succ` |
| `examples/py-nat.sokonanoda` | 1 | `zero` `succ` |

迁移顺序建议（下游单开一张 WO，见"不做的事"）：`courses/set-theory/lib/Prod.sokonanoda:45`
（该文件 `:23-25` 已经自己预约了"G-02 修好后机械改名回 `Prod.mk`"）→ `units/unit05-*` +
`units/solutions/unit05-solution.sokonanoda` → `inl/inr` 家族（`lib/Logic.sokonanoda:62-63` 起）
→ `aa/bb`（unit08）→ 最后 `examples/` 与 `course/`（后者牵动双 GOLDEN 与 23 个文件的门禁）。

## 不做的事（明确排除，防顺手扩大）

1. 不做 `namespace` / `open` / `open scoped`（G-05）——本轮只到"类型前缀"这一层；
2. 不做类型导向的 `mk` 解析、不做 `..`、不做 `where | ` 语法糖；
3. **不做别名日落**：R2 至少保留到下一个 minor；日落与 37 文件迁移一起开 WO-005b；
4. **不改任何课程内容**（A/B 清单以外），不改 `course/` 与 `courses/set-theory/` 的画布/解答；
5. 不动 prelude 的装载闸与 `Nat`/`Bool`/`Eq` 三名；不动 `iota` 源名匹配；不动 `match` 分支解析；
6. 不动 kernel 目录（若真需要，按 §6 流程另说，见上）；
7. 不把 `#reduce`/`#check` 的 `text` 字段行为（含前面记的括号小瑕疵）掺进这一刀。

## 验收（三层）

### front 单测（`crates/front/src/compile/tests.rs`，参照既有 `:563 explicit_inductive_block_compiles`、`:447 non_recursive_inductive_block_compiles_and_reduces` 的写法）

1. **前缀名 + 共存**：复现件同构源码（两个块各 `ctor mk`、一个 def 用 `P1.mk`/`P2.mk`）⇒ `errors` 空、三个声明 checked；
2. **别名**：`ctor mk` 之后裸 `mk` 可解析（唯一时）；两个 `mk` 之后裸 `mk` 给出约定错误（R2 选定的码），而两个前缀名照常；
3. **不重复加前缀**：`ctor Nat.zero`（或 `ctor Foo.bar`）保持原名，`P1.mk` 不变成 `P1.P1.mk`；
4. **派生 rec 的 iota 名**：无显式 `rec` 的块 + `#reduce`（`elab.rs:2845-2849` 的坑）；再补一条**显式 `rec` + `iota` 用源名**的对照；
5. **改钉 `tests.rs:4213-4239`**：`match_source_inductive_nat_still_uses_bare_ctors` 的语义变了
   （不再是"裸名 ctor"），改名/改写并**实测**归约文本（见"影响面"第 3 条，很可能变 `NatLit`）。
6. **项目级（`crates/front/src/project/tests.rs:260-282` 旁边新增）**：A/B 两个模块各声明
   `inductive A/B` + `ctor mk`，入口 import 两者 ⇒ **不得**出现 `import-name-collision`
   （这是 `check/mod.rs:619-651` + `project/mod.rs:325-361` 的第二道闸）；
7. **hover/goto（若 front 有 hover 回归位）**：裸名与前缀名两种写法的使用点都能回填到定义 span
   （`check/kernel_phase.rs:388-401`）。

### CLI e2e（`crates/cli/tests/cli.rs`，`run(src)` 默认走 stdin/临时文件）

1. 新增 `cli_ctor_names_are_namespaced`：复现件同构源码 ⇒ exit 0、stdout 有
   `checked declaration P1`/`P2`、`P1.mk` 出现在 `#check` 输出；
2. 兼容 e2e：`course/unit7-induction-recursion-2.sokonanoda`（`ctor none`/`some`）
   与 `examples/py-nat.sokonanoda` **原文不改**仍 exit 0（别名路径）；
3. 直接跑复现件本身（可作为新 e2e 的一行）：`crates/cli/tests/cli.rs` 里用
   `concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/gaps/repro/G02-ctor-namespace.sokonanoda")`
   ⇒ exit 0（复现件从此同时是"缺口即测试"）。

### 课程用例

`courses/set-theory/units/unit05-pairs-products.sokonanoda` 的**练习 3**（第 72-76 行注释 +
第 76 行 `theorem prod_mk_inj`，L 类：配对定理）——它的目标里带着
`prod_mk A B a b`（第 74 行 hint 原文），而 `prod_mk` 正是
`courses/set-theory/lib/Prod.sokonanoda:45`（`ctor prod_mk (a : A) (b : B) : Prod A B`）
这个**假唯一名**（该文件 `:15-25` 就是 G-02 的现场说明）。

- **本轮判据**：课程内容零改动，`python3 courses/set-theory/tools/check.py` 仍
  **34 个目标 · 296 checked · 93 open · 0 判负、exit 0**（本 WO 作者实测基线，2026-09-18 工作树）；
- 附带跑 `courses/set-theory/units/solutions/unit05-solution.sokonanoda`（8 checked）确认别名在
  解答里也成立；
- 迁移轮（WO-005b）判据：`lib/Prod.sokonanoda` 改 `Prod.mk` 后，把 `prod_mk` 的出现全部机械
  改名（按行计数：`lib/Prod.sokonanoda` 12、`units/unit05-pairs-products.sokonanoda` 22、
  `units/solutions/unit05-solution.sokonanoda` 16，含注释与 hint 文案），该单元判负仍为 0。

### 影响面

1. **事件计数：不变（已实测）**。构造子**不产生**自己的 `decl.checked` 事件（实测：
   `inductive Two / ctor aa / ctor bb / def t` ⇒ 只有 `Two`、`t` 两条 checked，
   `ctor` 行没有对应事件）⇒ 本刀不改 checked/open/reduced 的**数量**。
2. **双 GOLDEN 不需要同步**（本轮）：`crates/cli/tests/course.rs:86-98`（每单元
   `(decl.checked, exercise.open, expr.reduced)`）与 `crates/cli/tests/course_status.rs:68-80`
   （四项 + failed=0）都只比**计数**；11 个单元今天全绿、构造子改名不改变这些计数。
   两份 GOLDEN 讲的是**入门课 `course/`**（`unit1…unit11`），**不是** `courses/set-theory/`
   （后者目前只有 `tools/check.py` 一道门禁，仓库内没有 Rust 侧 golden 接线 —— 实读
   `crates/` 全树 grep `set-theory` 无命中）。
   ⇒ 结论：**计数 golden 保持原值**；谁要是顺手改了这两张表，说明计数真变了，那才是回归。
3. **但要改的是"文本断言"，且其中一批不是"改前缀"而是"改归约形态"（高风险，必须先实测）**：
   源文件自带 `inductive Nat` 时，ctor 一旦叫 `Nat.zero`/`Nat.succ`，内核 name cache 会给
   `Nat.succ` 打 `NatRed::Succ`（机制：`crates/kernel/src/util.rs:1046-1074` 按名字独立 stamp；
   文档：`docs/architecture.md:310-313` 的"原生 Nat"），`apply` 会把一元链折成 `NatLit`。
   **实测对照**（同构源码，只差 ctor 名）：

   ```text
   裸名：inductive Nat / ctor zero / ctor succ  ⇒  #reduce add two two
         => succ (succ (succ (succ zero)))          （既有断言就是这么写的）
   点名：inductive Nat / ctor Nat.zero / ctor Nat.succ ⇒
         · `-- sokonanoda:prelude none`，链经过定义（two := Nat.succ one）
           ⇒  #reduce two  =>  Nat.succ 1              （混合表示，与 architecture §5.4 记的一致）
         · prelude Full（默认），two := Nat.succ (Nat.succ Nat.zero)
           ⇒  #reduce two  =>  2                       （全折成 NatLit）
         · 两次都能 `#check Nat.succ` ⇒ `Nat -> Nat`
   ```

   两次点名形态的**输出还不一样**（`Nat.succ 1` vs `2`，差别在"链是否经过一层 def"）——
   这正是下面"必须先实测"的理由：**不要把文本断言按想象改**。

   ⇒ `succ (succ …)` 那一族断言**不能**机械改成 `Nat.succ (Nat.succ …)`，要先跑一遍看是
   `4` / `2` 还是混合形 `Nat.succ 3`（`docs/architecture.md:313` 记过混合表示）。逐条清单：

   | 文件:行 | 断言 | 预期处置 |
   |---|---|---|
   | `crates/cli/tests/cli.rs:502` | `add two two => succ (succ (succ (succ zero)))` | 实测后重钉（NatLit/混合） |
   | `crates/cli/tests/cli.rs:1091` | `=> ff`（自带 `inductive Bool`） | 预计 `=> Bool.ff`（Bool 无 NatRed，纯改名）**待确认** |
   | `crates/cli/tests/cli.rs:1180` | `swap red => green` | 预计 `=> Color.green` **待确认** |
   | `crates/cli/tests/cli.rs:1258` | `addM two three => succ (…)` | 同 502，实测后重钉 |
   | `crates/cli/tests/cli.rs:1518` | `fromOption (some Nat (Nat.succ Nat.zero)) …` | 预计 `Option.some …`（`some` 是源 ctor）**待确认** |
   | `crates/cli/tests/examples.rs:74` | py-nat 的 `add two two => succ (…)` | 同 502；`examples/py-nat.sokonanoda` 原文不改 |
   | `crates/front/src/compile/tests.rs:520,527` | `text == "ff"` / `"tt"` | 预计 `Bool.ff`/`Bool.tt` |
   | `tests.rs:555,628,648,1577` | `succ (succ …)` / `succ zero` | 同 502，实测后重钉 |
   | `tests.rs:3719,3765` | `text == "green"` | 预计 `Color.green` |
   | `tests.rs:4049` | `text == "ff"` | 预计 `Bool.ff` |
   | `tests.rs:4234` | `succ (succ (succ zero))`（自带 Nat 的 match 测试） | 改名 + 实测后重钉 |
   | `tests.rs:4496,4540,4609` | `contains("succ")` / `contains("vnil")` | **子串断言，改前缀后仍真**，只需确认 |
   | `crates/front/src/parser.rs:1803` | AST 名字 `green` | 不动（parser 不看前缀） |

4. **用户可见面（同轮文档义务的触发条件）**：`#check`/`#reduce`/hover/Infoview 的目标文本、
   refine 骨架、补全列表、LSP 语义高亮都会换成规范名 —— 属于**用户可见改动**，
   见下一节；事件字段名/`code` 不变（`docs/protocol.md:355-362` 的词汇表不变）。

## 文档同步清单

- `docs/design/ctor-namespace.md`（**新建**，设计先行）+ `docs/architecture.md`
  §4.2（elab 一节）/§5.4（prelude 点名 + name cache，`295-318`）：写清"规范名 = 前缀名、
  裸名 = 别名（扩展）、别名闭包级、日落计划"；
- `docs/protocol.md`：**契约词汇不变**（`ctor_name`/`ctor_use` 仍在 `:355-362`）；
  仅当 R2 选了**新错误码**时，同轮把码加进错误码表（`:100-140` 一带）与
  `crates/front/src/compile/tests.rs:753-816`（"每个 ErrorKind 都有稳定码 + hint"）+
  `tests.rs:1431`（协议文档必须列全每个码）两族断言；
- `skills/`：`skills/sokonanoda-teacher/SKILL.md:130`（教学例写的是 `ctor vnil`/`vcons`）
  必须同轮更新为规范名或注明别名；`skills/sokonanoda-dev/SKILL.md` 若提到构造子写法同步；
  `.agents/skills/` 薄入口不动（`crates/cli/tests/dsh.rs` 守漂移）；
- `editor/vscode/`：本次实读未见构造子字样（grep `prod_mk`/`ctor` 无命中），但**打印文本变了
  = 用户可见改动** ⇒ `editor/vscode/CHANGELOG.md` + `package.json` **版本 bump**（与
  `Cargo.toml:6` 两处一起，见 `docs/RELEASE.md`）；README 若提到 `#reduce` 文本同步；
- `AGENTS.md`：无新硬规则 ⇒ 只在"硬规则速记"之外无需改（**明确写"不改"**，避免顺手扩写）；
- `docs/HANDOVER.md` + `STATUS.md`：按收尾义务同轮更新（`STATUS.md` 只留最近 3 轮）；
- `REQUIREMENTS.md` §9：追加一条带日期（用户可见：打印名变化 + 裸名别名这条**子集扩展**）；
- `docs/gaps/ledger.jsonl`：WO 落盘后把 G-02 的 `wo` 填成
  `docs/gaps/WO-005-ctor-namespace.md`、`status` 改 `wo-filed`
  （口径见 `docs/design/teaching-project.md:282-283`）。**本次派工只交 WO 文本，未动台账**；
- `courses/set-theory/README.md` 的"现状"表：仅迁移轮改。

## 门禁

```bash
scripts/soko gate                       # fmt + clippy + test + playground 锚点（注意缓存版本一致，否则 exit 3）
cargo test --workspace --locked         # 全量（含 crates/cli/tests/{cli,course,course_status,examples}.rs）
python3 courses/set-theory/tools/check.py   # 课程用例基线：34 目标 · 296 checked · 93 open · 0 判负
scripts/soko grade docs/gaps/repro/G02-ctor-namespace.sokonanoda   # 必须 exit 0
python3 scripts/gap.py check            # 修好后 G-02 会红一行（提醒关账），其余必须全绿
```

- 内核目录若被意外改动：`cargo fmt --all` 是**禁止**的（会重排冻结快照），只 fmt 教学 crates
  或直接 `scripts/soko gate`（`AGENTS.md` 命令节）；
- `docs/gaps/repro/` 下别的复现（G-01/G-03/G-04/G-05/G-08/G-09/G-13/G-14/G-15）**不允许**因为
  这一刀改变判定 —— `gap.py check` 是总闸。

## 关账

```bash
python3 scripts/gap.py close G-02 --version 0.59.0    # 版本号 = 本轮 bump 后的新版本
python3 scripts/gap.py check                          # 应回"全部与台账一致"
```

- 关账前置：复现件**已干净判卷**（`.sokonanoda` 类复现的判据 = 干净判卷 + 有 checked 声明，
  `scripts/gap.py:71-80`）——修好但没关账时，`gap.py check` 会把 G-02 标成
  "已判卷通过 ← 台账写的是「仍有失败」，请更新"（`scripts/gap.py:182-189`），这就是提醒；
- `.sh` 类复现才有"exit 1 = 行为变了"的自断言；G-02 是 `.sokonanoda` 复现，等价信号就是上面那条红行；
- 关账后把迁移任务登记为 WO-005b（`courses/set-theory/lib/Prod.sokonanoda:23-25` 的预约改名
  + 37 文件清单），并在 `docs/design/ctor-namespace.md` 写日落版本。
