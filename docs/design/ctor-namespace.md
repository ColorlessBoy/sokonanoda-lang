# 设计：构造子的命名空间（R1 规范名 + R2 裸名别名，G-02 / WO-005）

> 触发：台账 G-02（blocker）——构造子以**裸名**进环境且**全项目唯一**，于是
> `inductive Pair … / ctor mk` 之后只有裸 `mk`，`Pair.mk` 报
> `unknown identifier`；两个 `ctor mk` 直接撞成 `duplicate declaration mk`。
> 官方 Lean 4 里构造子自动进入类型自己的命名空间（`Pair.mk` / `Or.inl` /
> `Subtype.mk` / `Exists.intro` 并存不冲突）。
> **kernel 冻结**：本设计只动前端（elab / check / semantic / references）与文档。

## 1. 目标与非目标

**做**：源 `ctor` 名进内核时带类型前缀（R1）；裸名继续可解析（R2，**子集扩展**）；
源级写法（`match` 分支、显式 `iota`、注释）不动（R3）。

**不做**（防顺手扩大，逐条与 WO「不做的事」对齐）：

1. 不做 `namespace` / `open`（G-05）——本轮只到"类型前缀"这一层；
2. 不做类型导向的 `mk` 解析、不做 `..`、不做 `where |` 语法糖（parser 不动）；
3. 不做别名日落（R2 至少保留到下一个 minor；迁移与日落单开 WO-005b）；
4. 不改任何课程内容（`course/` 与 `courses/set-theory/` 一字不改，靠 R2 继续绿）；
5. 不动 prelude 的装载闸与 `Nat`/`Bool`/`Eq` 三名；不动 `iota` 源名匹配；
6. 不动 `crates/kernel/`（一个字节都不改）；
7. 不把 `#reduce`/`#check` 的 `text` 字段行为掺进这一刀；
8. 不扩构造子补全（构造子不进 `DeclState`，补全列表本轮不扩）。

## 2. R1：规范名（安装名 = 内核名 = 打印名）

```text
canonical(Ind, ctor) = ctor 已含 '.' ? ctor : "Ind.ctor"
```

- **已含点则原样**：保护 prelude（`Nat.zero`/`Nat.succ`/`Bool.true`/`Bool.false`
  本来就是点名前缀，`docs/architecture.md` §5.4 记着内核 name cache 的预留槽位）
  与既有的显式写法（`examples/py-nat.sokonanoda`、`course/unit9` 的
  `ctor even_zero : Even Nat.zero` 这类**结果类型**里写点名不算，ctor 名本身
  是裸的 `even_zero`，会变成 `Even.even_zero`）；
- 规范名是**唯一的声明身份**：进 `known`、进内核 `Declar::Constructor`、
  进 `all_ctor_names`、进派生 recursor 的 iota 规则
  （内核断言 `rule.ctor_name == ctor.name`，`crates/kernel/src/inductive.rs:1590`）、
  进 `top_level_def_spans`（闭包唯一性 + hover/goto 回填）、进 `ResolvedTarget`；
- **打印名随之改变**：`#check` / `#reduce` / `#print` / hover / Infoview 的目标
  文本、refine 骨架都会显示规范名 —— 这是**用户可见改动**（文档同步见 §7）。

### 2.1 与内核 name cache 的相互作用（高风险，已实测）

内核 `mk_name_cache`（`crates/kernel/src/util.rs:1046-1074`）按名字给
`Nat.succ` 打 `NatRed::Succ`；源文件自带 `inductive Nat` 时，ctor 一旦叫
`Nat.zero`/`Nat.succ`，`apply` 会把一元链折成 `NatLit`。**实测对照**
（`sokonanoda --json`，同构源码只差 ctor 名）：

| 源 | `#reduce` | 修前 | 修后（规范名） |
|---|---|---|---|
| `ctor zero`/`succ`（显式 rec）| `add two two` | `succ (succ (succ (succ zero)))` | `Nat.succ (Nat.succ (Nat.succ 1))` |
| `ctor Nat.zero`/`Nat.succ`（显式 rec）| `Nat.rec.… (succ zero)` | `succ zero` | `1` |
| 同上 | `Nat.succ Nat.zero` | `succ zero` | `1` |
| 同上（match 版 `addS`）| `addS (succ (succ zero)) (succ zero)` | `succ (succ (succ zero))` | `2` |
| `ctor Bool.tt`/`ff` | `not tt` | `ff` | `Bool.ff`（Bool 无 NatRed，纯改名） |
| `ctor Color.red`/`green` | `swap red` | `green` | `Color.green`（纯改名） |
| `ctor Option.some` | `fromOption (some Nat 1) 0` | `1` | `1`（不变） |

⇒ **文本断言一律实测后重钉**，不做机械替换（混合表示 `Nat.succ 1` 与 `NatLit 2`
的差别在"链是否经过一层 def"，`docs/architecture.md` §5.4 记过这个现象）。

### 2.2 事件计数不变（已实测）

构造子**不产生**自己的 `decl.checked` 事件（一块归纳只发一条 checked），
`#reduce`/`#check` 的条数也不变 ⇒ 双 GOLDEN（`crates/cli/tests/course.rs`、
`course_status.rs`）与 `courses/set-theory/tools/check.py` 的计数基线**不改**。
谁顺手改了这些计数表，说明计数真变了，那才是回归。

**基线口径订正（实测）**：WO-005 正文写的「34 目标 · 296 checked · 93 open」
是 2026-09-18 工作树的数字，之后课程继续生长（`unit03` 等 09-19 还有改动）。
**2026-09-19 实测基线 = 34 目标 · 315 checked · 96 open · 0 判负**
（`courses/set-theory/README.md:67` 记的也是这个）。本刀用**同一棵树**做了对照：
把 `crates/` 回退到 HEAD 重新构建二进制，跑出的同样是 315/96/0 —— 即这 19 个
checked 的差额来自 WO 落盘之后的课程内容，**与本刀无关**。

## 3. R2：裸名别名（只在解析层，绝不进内核）

**子集扩展**（不是 Lean 语义；Lean 里裸 `mk` 不可解析，除非 `open` 了那个命名空间）。
动机：既有 12 单元 + 入门课 11 单元 + `examples/` 全用裸名，一次一刀
（仓库惯例见 `docs/design/teaching-project.md:327-328`）。

规则（`known` 表，闭包级扁平 —— 与今天 `check_name_collisions` 的扁平口径一致，
模块作用域属 G-05）：

| 情形 | 结果 |
|---|---|
| 裸名在闭包里**唯一** | 解析到规范名（表键 = 裸名，值 = 规范名） |
| 裸名被**两个**构造子占用 | **不可解析**，报 `elab-ambiguous-ctor-alias`（新码） |
| 裸名被一个**真实声明**（def/theorem/axiom/inductive/rec）占用 | 真实声明优先；构造子别名不覆盖它 |
| 规范名本身 | 永远可解析 |

- 三种值用 `KnownName` 枚举表示：`Decl { universes }` / `Alias { canonical }` /
  `Ambiguous { candidates }`（`crates/front/src/compile/elab.rs`）；
- **别名必须解析到规范名**：只往 `known` 加一个裸名键会造出第二个内核常量
  （`mk` ≠ `P1.mk`），所以 `Expr::Ident` / `Expr::UniverseApp` / `#print`
  一律用 `known` 的规范名建 `mk_const` / `name_from_str`；
- 别名是**闭包级、无模块作用域**：被 import 的模块里的唯一裸名在入口同样可解析
  （今天的扁平行为不变）；
- 别名**不进内核**：内核里只有规范名一个常量（`#print mk` 打印规范名）。

## 4. R3：源级写法不动

- `match` 分支（`| none =>`）、显式 `iota zero :=`、注释/文案继续按**源名**匹配；
- 模式解析（`ctor_index`）接受**两种拼写**：源名（`prod_mk`）与规范名（`Prod.prod_mk`），
  外加"该归纳内唯一的裸后缀"（既有行为）；错误消息同时给出两种拼写；
- **迁移轮的注意点**：一旦课程把 `ctor zero` 改写成 `ctor Nat.zero`（WO-005b），
  同文件的 `iota`/`match` 分支**也必须跟着写点名前缀**——`iota` 规则按源名匹配，
  源名变了（`zero` → `Nat.zero`）就不再命中；`match` 的裸分支名则因为
  "唯一裸后缀"兜底仍可能命中，但**不要依赖它**。写进迁移清单（§8）。

## 5. 落地（as-built，逐点对应 WO 的范围表）

| 位置 | 改动 |
|---|---|
| `elab.rs::install_inductive_block` | `canonical_ctor_name` 生成安装名数组；`add_inductive` 的 `all_ctor_names`、`Declar::Constructor`、`known`、`InductiveInfo` 全用规范名 |
| `elab.rs::derive_recursor` | 新增 `ctor_names: &[String]` 参数：iota 规则名与 minor 里生成的 `C params fields` 项都用规范名（漏改 = 派生的 rec 对不上规则，内核断言） |
| `elab.rs::Expr::Ident` / `Expr::UniverseApp` | `known` 查 `KnownName`：`Ambiguous` → 新码；否则用规范名建常量与 `ResolvedTarget::Declaration` |
| `elab.rs::ctor_index` / `MatchCtor` | `MatchCtor` 增 `canonical`；模式按源名**或**规范名匹配（R3） |
| `check/mod.rs::top_level_def_spans` | ctor 键换规范名（闭包唯一性 + hover/goto 回填 + `import-name-collision` 的第二道闸自动跟随） |
| `check/walk.rs` | `known` 值类型改 `KnownName`；`#print` 走别名（否则 `#print mk` 从"能打印"回归成 unknown declaration） |
| `prelude.rs` | 三处 `known.insert` 改 `KnownName::decl`（prelude 名字已含点，零语义变化） |
| `goals.rs` | `CtorTemplate` 增 `canonical_name`（refine 骨架显示规范名）；`funcs` 同时按源名与规范名登记（裸名歧义时不登记裸名键）；ctor spine 匹配接受两种拼写 |
| `semantic.rs` | `declaration_kinds` 收规范名；文档 token 表 `ctors` 同时收源名与规范名（点号 token 归 `ctor_use`） |
| `references.rs::decl_name_span` | 规范名找不到定义 token 时回退匹配**最后一段**（`ctor mk` 里的 `mk`）——仍是词法精确匹配，不是文本扫描。**实读时漏了这条**：不补它，ctor 的 prepareRename/goto/references 会静默失效（回归测试：`references::tests::decl_name_span_finds_the_source_token_of_a_canonical_ctor_name` + LSP 三条） |
| `lsp/src/lib.rs` | **不改**：补全列表来自 `report.decls`，构造子不进 `DeclState`（§1 第 8 条） |

**内核零改动**：`builder.name_from_str` 已按 `.` 切段建层次名
（`crates/kernel/src/builder.rs:90-100`），prelude 的 `Nat.zero`/`Bool.true`
今天就走这条路 —— 层次名是内核既有能力，不是新语义。

## 6. 验收（三层，判据 = 内核）

1. **front 单测**（`crates/front/src/compile/tests.rs`）：前缀名 + 共存（复现件同构）、
   别名（唯一可解析 / 重复报 `elab-ambiguous-ctor-alias`）、不重复加前缀
   （`ctor Nat.zero` 保持）、派生 rec 的 iota 名 + 显式 rec 源名对照、
   改钉 `match_source_inductive_nat_still_uses_bare_ctors`、`#print` 别名；
2. **项目级**（`crates/front/src/project/tests.rs`）：A/B 两模块各 `ctor mk`，
   入口 import 两者 ⇒ **不得**出现 `import-name-collision`；
3. **CLI e2e**（`crates/cli/tests/cli.rs`）：新增 `cli_ctor_names_are_namespaced`
   + 直接跑复现件 `docs/gaps/repro/G02-ctor-namespace.sokonanoda` ⇒ exit 0；
   兼容 e2e：`course/unit7-*` 与 `examples/py-nat.sokonanoda` 原文不改仍 exit 0；
4. **课程语料**：`python3 courses/set-theory/tools/check.py`（34 目标 · 315 checked ·
   96 open · 0 判负，实测；见 §2.2 的口径订正）+ 入门课双 GOLDEN 计数不变
   —— **课程内容一字不改**。

## 7. 文档同步

- 本文件（设计先行）+ `docs/architecture.md` §4.2/§5.4（规范名 / 别名 / 日落）；
- `docs/protocol.md`：契约词汇不变（`ctor_name`/`ctor_use` 仍在），新增错误码
  `elab-ambiguous-ctor-alias` 进错误码表；
- `skills/sokonanoda-teacher/SKILL.md`（构造子写法）+ `skills/sokonanoda-dev/SKILL.md`；
- `editor/vscode/CHANGELOG.md`（打印文本变化 = 用户可见改动；版本 bump 由主线统一做）；
- `REQUIREMENTS.md` §9 追加一条（打印名变化 + 裸名别名这条**子集扩展**）；
- `STATUS.md` / `docs/HANDOVER.md` 按收尾义务。

## 8. 迁移与日落（WO-005b，本设计只登记）

- R2 别名**至少保留到下一个 minor**；日落与 37 个 `.sokonanoda` 的机械改名一起开
  WO-005b（清单见 `docs/gaps/WO-005-ctor-namespace.md`「D. 强制改名分支」）；
- 迁移顺序：`courses/set-theory/lib/Prod.sokonanoda:45`（该文件 `:23-25` 自己预约了
  "G-02 修好后机械改名回 `Prod.mk`"）→ `units/unit05-*` + 解答 → `inl/inr` 家族
  （`lib/Logic.sokonanoda:62-63` 起）→ `aa/bb`（unit08）→ `examples/` 与 `course/`
  （后者牵动双 GOLDEN 与 23 个文件的门禁）；
- 日落 = 删掉别名插入（`known` 只留规范名）并把 `elab-ambiguous-ctor-alias`
  降级为历史码；届时 §3 的表整张作废。
