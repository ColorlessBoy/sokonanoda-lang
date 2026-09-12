# sokonanoda-lang 架构与内核深度理解

> 读者对象：刚接手本仓库的 agent / 工程师。
> 本文回答三个问题：**这是什么**、**代码在哪、各管什么**、**一条 `.sokonanoda` 文件如何变成"被完整内核检查过的声明"**。
> 配套文档：`STATUS.md`（当前状态与进度日志，先读）、`ROADMAP.md`（里程碑与原则）、
> `docs/protocol.md`（反馈/事件协议）、`docs/notes/inductive.md`（归纳类型语义）、
> `docs/notes/research.md`（外部调研）、`docs/design/infrastructure.md`（基础设施方案脑暴）。

---

## 1. 30 秒版

`sokonanoda-lang` 是一个**独立、自包含的 Lean 4 教学编译器栈**，跑在 Rust 上：

- **底层**是完整移植的 [sokonanoda](https://github.com/intgrah/sokonanoda) 内核（Lean 4 类型检查器，含 inductive / quot / proof irrelevance / 完整 conv/eval / pretty printer），几乎不改动；
- **上层**是受限的教学方言 `.sokonanoda`（语法白名单 = 课程），有 lexer/parser/小型 elaborator/CLI/REPL/`#prove` 草稿；
- 产品目标是「用户 + code agent 看着同一个 `.sokonanoda` 文件协作学证明」，但 L0 阶段**只做编译器层**，不依赖 VS Code、不依赖任何 agent、不调用任何官方 Lean 工具链。

一句话：**完整内核能力 + 受限教学通道 + 可被 agent 消费的结构化反馈**。

```
 .sokonanoda 源码
   │  parse（front）
   ▼
 AST(FolFile/Command/Expr)     带 span
   │  elab（front → builder）
   ▼
 kernel Declar 序列（arena 内，按依赖顺序）
   │  EnvBuilder.finish()
   ▼
 ExportFile（完整环境 + 名字/层级/表达式 intern 表）
   │  try_check_declar / infer_closed_type / reduce_closed / with_pp
   ▼
 CheckEvent[] + CompileError[]      ← 文本行（人）/ JSON Lines（agent）
```

---

## 2. 仓库地图

```text
sokonanoda-lang/
├── Cargo.toml              # workspace：kernel / front / cli
├── README.md               # 英文快速上手
├── ROADMAP.md              # 中文里程碑 + 执行清单（当前状态入口）
├── NOTICE.md               # 上游归属（sokonanoda@7b51784，Apache-2.0）
├── .github/workflows/ci.yml  # 离线 cargo test + 课程语料 + JSON 事件自检
├── docs/
│   ├── architecture.md     # 本文
│   ├── protocol.md         # 文本/JSON 事件协议（CLI 已实现 --json）
│   ├── inductive.md        # inductive/ctor/rec/iota 语义
│   ├── research.md         # 教学型证明语言外部调研
│   └── design-infrastructure.md  # 基础设施补全设计脑暴
├── crates/
│   ├── kernel/             # 完整 sokonanoda 内核快照 + 少量教学适配（见 §6）
│   │   ├── src/{expr,level,name,value,env,util,parser,...}.rs
│   │   ├── src/builder.rs  # 【新增】EnvBuilder：内存中造声明 → ExportFile
│   │   ├── src/quote.rs    # 【改动】infer_closed_type / reduce_closed
│   │   ├── tests/arena.rs  # Lean Kernel Arena 集成测试（需环境变量）
│   │   ├── tests/memory_api.rs  # M0 内存 API 验收
│   │   └── test_resources/ # 上游 NDJSON fixtures（部分测试的输入）
│   ├── front/              # .sokonanoda 前端
│   │   └── src/{lib.rs,compile.rs,proof.rs}
│   ├── cli/                # `sokonanoda` 二进制（文件检查 / repl / --json）
│   │   ├── src/main.rs
│   │   └── tests/{cli.rs,examples.rs}
│   └── lsp/                # `sokonanoda-lsp`：tower-lsp 服务器（诊断/hover/符号/练习状态）
├── editor/vscode/          # 实验性 VS Code 客户端（薄壳，未打包）
└── examples/               # 入库课程文件（lesson-01/02、fol-basics、py-fol-core、py-nat）
```

测试数量（2026-09-06，`cargo test --workspace` 全绿）：

| 层 | 数量 | 位置 |
|---|---|---|
| kernel 单元测试 | 41（2 ignored：缺 fixture） | `crates/kernel/src/*` 内 `#[cfg(test)]` |
| arena 集成测试 | 1（需 `LEAN_KERNEL_ARENA`） | `crates/kernel/tests/arena.rs` |
| 内存 API 测试 | 1 | `crates/kernel/tests/memory_api.rs` |
| front 单元测试 | 49 | `crates/front/src/{lib,compile,proof}.rs` |
| CLI 端到端 | 21 | `crates/cli/tests/cli.rs` |
| 课程语料 | 1（遍历全部 examples） | `crates/cli/tests/examples.rs` |
| LSP（文档服务层） | 手动 smoke（诊断/hover/符号/code action） | `crates/lsp` |

---

## 3. 不可动摇的原则（来自 ROADMAP，代码与之对齐）

1. **kernel 保持完整**：受限的是前端教学语法，不是内核能力。
2. **无官方工具依赖**：运行/构建/测试都不调 `lean`/`lake`/`lean4export`/`leanc`/`elan`；所有语料入库。
3. **教学语法是真实 Lean 4 的子集**：在 `.sokonanoda` 里学会的写法放进官方 Lean 依然合法。
4. **语法白名单即课程**：parser 只认课程引入过的语法点；新增语法必须伴随课程单元。
5. **分层推进**：L0（编译器）→ L1（服务）→ L2（编辑器）→ L3（agent 协作），每层只依赖下一层公开接口。
6. **TDD 与重复测试**：单元、kernel 端到端、CLI 三层重复覆盖同一行为。
7. **反馈即功能**：类型、化简、打印、环境、错误都结构化输出，人和模型都能无文档驱动工具。

---

## 4. 一条 `.sokonanoda` 文件的旅程

### 4.1 词法/语法（`crates/front/src/lib.rs`）

- `Lexer`：手工字符扫描，产出 `TokenKind`（`Ident/Num/Hole/Colon/ColonEq/Arrow/Plus/FatArrow/Forall/At/括号/逗号/Eof`）。标识符允许 ASCII 字母/`_`/非 ASCII（≥0x80），续字符还含 `' ! ? .`；`#check` 这类命令被 lex 成 `#` 前缀的 Ident。
- `--` 是行注释；`???` 是 Hole（未完成练习）。
- `Parser` → `FolFile { commands: Vec<Command> }`。命令：`def` / `theorem` / `example` / `axiom` / `inductive ... end` 块 / `#check` / `#reduce` / `#print`。
- 表达式 AST（`Expr`）：`Sort(Prop/Type/Sort n/Level u)`（源码里的 `Type n` 解析成 `Sort (n+1)`，是 Lean 记法的糖）、`Ident`、`UniverseApp name.{u,...}`、`Num`、`Hole`、`App`、`Lambda`、`Forall`、`Arrow`、`Plus`。
- 值位关键字：`by <tactic 序列>`（`Expr::By`，进内核前由 `crates/front/src/by.rs` 降级为 lambda）与 `intro`（`Expr::Intro`，进内核前由 `crates/front/src/compile/intro.rs` 全剥成 `fun … => sorry`；值不进内核，填洞后仍由内核终审）。
- **命名箭头**：`(x : A) -> B` = 带 binder 的 `forall`；`{x : A} -> B` = 隐式 binder 的 forall；`A -> B -> C` = 匿名 binder 右结合 Pi。`A -> B` 与 `fun (x : A) => ...` 的 binder 都必须**带显式类型**（elaborator 尚未做 binder 类型推断，见 §8 待办）。
- span 全程保留（offset/line/column），诊断带行列。

### 4.2 Elaboration（`crates/front/src/compile.rs`）

`compile_fol(file) -> CompileOutput { events: Vec<CheckEvent>, errors: Vec<CompileError> }`：

1. 开一个 `stumpalo::Arena`，建 `EnvBuilder`（kernel 的 `builder.rs`）。
2. 若文件里没有显式 `inductive Nat ... end` 块，就 `install_prelude` 内置最小 Nat 基元（见 §5.4）。
3. 顺序处理每条命令：
   - `def/theorem/example/axiom` → `build_*` 把 AST elaborate 成 kernel `Declar`，`builder.add_declar` 入表（记录每条声明在环境里的索引），随后 push `PendingOp`。
   - `#check/#reduce` → 先 elaborate 表达式并记住 `decl_before`（当前声明数），稍后用 `EnvLimit::ByIndex(decl_before)` 检查，保证 `#check` 只看到它之前的声明。
   - `inductive ... end` → `install_inductive_block`：先加 `Inductive`，再逐个加 `Constructor`，有 `rec` 则加 `Recursor`（带 `RecRule` 列表，每条 iota 规则按构造子索引绑定）。
   - `example : T := ???` → 不建声明，直接产出 `CheckEvent::ExerciseOpen`。
4. 若 elaboration 阶段已有错误，直接返回（不碰 kernel）。
5. 否则 `builder.finish()` 得到 `ExportFile`，设 `pp_options.proofs = true`（打印证明项本体而不是 `_`），然后逐个执行 PendingOp：
   - 声明：`env.try_check_declar(&declar)`（panic 包装成 `Result<(), CheckError>`），成功产 `decl.checked` / `example.checked`。
   - `#check`：`tc.infer_closed_type(expr)` + `pp.pp_expr(ty)` → `TypeChecked{text}`。
   - `#reduce`：`tc.reduce_closed(expr)`（kernel 新增的 deep reduce）+ pretty print → `Reduced{text}`。
   - `#print`：`env.with_pp(pp.pp_declar(ptr))` → `Printed{name,text}` 或 "unknown declaration" 错误。

### 4.3 事件与错误（协议的第一版实现）

`CheckEvent`：`DeclarationChecked/ExampleChecked/TypeChecked/Reduced/Printed/ExerciseOpen`。

`CompileError { message, span, kind: ErrorKind }`；`ErrorKind` 细分为 `elab-unknown-identifier`、`elab-universe-arity`、`kernel-rejected` 等稳定 code，每个 kind 带一条教学 `hint()`（中文，直接贴在编辑器诊断里）。词法/语法错误是 `front::Diagnostic`（stage=parse，`unexpected-token`/`unexpected-eof`，也有 hint）。CLI 人类视图输出 `line:col: error[code]: message`；`--json` 输出 JSON Lines（见 `docs/protocol.md`）。

### 4.4 CLI / REPL（`crates/cli/src/main.rs`）

- `sokonanoda <file>` / `sokonanoda -`：批处理检查，人类可读输出。
- `sokonanoda --json <file>`：每条事件一行 JSON（agent/service 视图）。
- `sokonanoda repl`：逐行累积 buffer，整体重新 `parse + compile_fol`（最小"增量"模型 = 声明累加）；支持 `#check/#reduce/#print/#env/#help`。
- `#prove <goal>`：进入证明草稿（见 §5.5），`intro/exact/apply/assumption/lambda/done`。
- `sokonanoda-lsp`：编辑器路径的**唯一反馈通道**（文件无 `#` 命令）——publishDiagnostics、hover（表达式类型 / `???` 的目标）、documentSymbol、codeLens（练习状态）、quick-fix `intro`（把 `???` 变成 `fun (x : T) => ???`）。

---

## 5. kernel 内部（`crates/kernel`）——它为什么快、各模块做什么

### 5.1 总体设计：arena + hash-consing + 闭包求值

上游 sokonanoda 的核心卖点是**用闭包替换显式替换**：β 归约不再 O(n) 遍历替换，而是 O(1) 环境扩展；再叠加大量 hash-consing / 指针相等 / 缓存。在 Lean Kernel Arena 基准里相对官方内核约 **10–100x**（详见外层 `lean-kernel-arena-benchmark.md`）。

- **分配**：`stumpalo::Arena`（全局 arena）+ `bumpalo::Bump`（会话 bump），全局 `mimalloc`，小向量 `smallvec`，哈希 `hashbrown + rustc-hash/fx`。
- **interner（Dag）**：`Name/Level/Expr/BigUint/String` 全部按结构 hash-cons，指针相等 ≈ 结构相等。`ExprPtr` 把 **自由变量个数编码进指针 tag**，许多遍历能立刻短路。
- **Expr AST**（`expr.rs`）：`Var/Sort/Const/App/Pi/Lambda/Let/Proj/NatLit/StringLit`，多数带 `hash` 与 `fv_mask`（自由变量位图，用于环境剪枝）。`BinderStyle`（default/implicit/strictImplicit/instImplicit）**只影响打印**，不影响类型检查。
- **Level AST**（`level.rs`）：`Zero/Succ/Max/IMax/Param`；`simplify`、`leq`、`subst` 是内核自身实现（不自带 SAT/求解器）。
- **Name AST**（`name.rs`）：`Anon/Str/Num` 层级名；`NameNode` 附 `decl_idx`（环境里的槽位）与 `nat_red` 标记（见 5.4）。
- **声明**（`env.rs`）：`Declar::{Axiom, Quot, Theorem, Definition, Opaque, Inductive, Constructor, Recursor}`；`InductiveData/ConstructorData/RecursorData/RecRule` 描述归纳块。环境是 `FxIndexMap<NamePtr, Declar>`，靠 `EnvLimit::{Empty, ByIndex, ByName, PpUnlimited}` 用 `cutoff` 控制"可见前缀"，因此**可先整表解析、再按线程切可见切片**。

### 5.2 值模型与求值（`value.rs` + `eval.rs`）

值（`Value`）与表达式分离，全部 arena 化：

- `Env` = 值链（`env_extend` O(1)）；`Ctx` = 类型链（记录 binder 类型，供推断模式用）；`Closure { env, ctx, body }` 捕获环境。
- `Spine` = 消除器链表（`App(v)` / `Proj{ty,idx}`），头 + spine 表示「未展开的刚体」。
- `RigidHead`：`BVar/Axiom/Ctor/Recursor/QuotConst/Inductive`；`Unfold`：需要 delta 展开的定义头（含 `OnceCell` 记忆 head value）。
- `Value::{Rigid, Unfold, Lam, Pi, Sort, NatLit, StrLit, Thunk}`；`Thunk` 惰性求值表达式，`Unfold` 惰性展开定义。
- **eval 主循环**按 Expr 分发（`eval_no_cache`）：闭项查 `(env, expr)` 记忆化缓存；`App` 先求函数 WHNF 再 `apply`；`apply_closure` = 环境扩展 + 求值（推断模式用 `ctx_extend`）。
- **delta**：`unfold_value_go`（1437 起）——对 `Unfold` 头先试**原生 Nat 快路径**（见 5.4），不行再取 head value、把 spine 逐 elim 应用。对未满参数或可延迟情形有 `nat_red_defer`。
- **iota**：`iota_value`（1496）带正缓存 `iota_cache` 与负缓存 `iota_stuck`；`fire_recursor`/`fire_quot`/`try_k_reduce`/`try_struct_eta_reduce`/`nat_rec_natlit` 覆盖 recursor 在构造子、K 规则（小消除）、结构 eta、nat 字面量上的归约。
- **WHNF 记忆**：`whnf_head`/`note_whnf`/`store_lookup` + `global_value_cache`（结构身份键）；探测预算 `PROBE_CAP` 防指数爆炸（见 `conv.rs` 注释）。
- **教学新增** `deep_reduce`（`eval.rs` 957 行附近，lang 特有）：WHNF 后递归进入构造子/消除子参数，让 `s (Nat.rec ...)` 这类"构造子参数里的递归"也能继续算——`#reduce` 用；常规 kernel 检查只需 WHNF。

### 5.3 类型检查（`infer.rs` / `conv.rs` / `quote.rs`）

- `TypeChecker { ctx, env, tc_cache, arena, declar_info, nat_extension }` 是检查器主体；`TcCtx` 负责 arena 内的名字/层级/表达式构造与读写、各类缓存（type/conv/whnf/iota/subst/inst…）。
- `infer_value(flag, depth, env, ctx, e)`：单向类型推断 + 检查，`InferFlag::{InferOnly, Check}` 控制是否做昂贵的一致性检查（如 lambda 的 binder 类型、App 实参类型）。结果带 `type_cache`（按 `(env,e)` 键）+ `CheckScope` 判断是否可复用。
- `check_declar_info_v`（声明类型本身是 Sort、无重复宇宙参数、theorem 类型必须落在 Prop）、`check_def_like_v`（值的类型与声明类型 `def_eq`）。
- **conv（定义相等）**：基于求值后的 WHNF 对齐 + 按需 delta 展开，利用 relevance 签名（`relevance.rs` 的 `Sig`，证明/被忽略参数可跳过）、指针相等、正/负转换缓存、probe 预算。宇宙 `leq`/`eq_antisymm` 用 `level.rs` 的决策逻辑。
- **归纳/商检查器**：`inductive.rs`（正向性、构造子参数/索引、重建 recursor 规则并与导出规则比对、大消除/K/证明无关边界、mutual 块、嵌套特化）；`quot.rs`（`Quot` 声明族）。临时环境扩展（`new_w_temp_ext`）服务于嵌套归纳检查。
- **quote**：值 → 表达式（`quote.rs`）。教学 API 就加在这里：
  - `infer_closed_type(e)` = 空环境推断 + WHNF + `quote_weak`（`#check` 的 kernel 原语）；
  - `reduce_closed(e)` = `value_of` + `deep_reduce` + `quote`（`#reduce` 的 kernel 原语）。
  - `infer_under_binders(&[binder_ty], e)` = 在给定 binder 类型序列（由外到内）下推断
    开放项 `e` 的类型并 quote 回来——这是编辑 hover 类型图的 kernel 原语。
- **pretty printer**（`pretty_printer.rs`）：把表达式/声明打印成可读文本；lang 版本默认 **ASCII `->`**（与教学文件一致），打印 proof term 而非 `_`。

### 5.4 内置 prelude 与"原生 Nat"技巧（重要！）

`compile.rs::install_prelude` 在文件没有显式 `inductive Nat` 时装入最小可信基元：

```text
inductive Nat : Type                 （add_inductive，无构造子）
axiom     Nat.zero : Nat             （名字特判）
def       Nat.succ : Nat -> Nat := Nat.succ      ← 占位自引用体
def       Nat.add  : Nat -> Nat -> Nat := Nat.add ← 占位自引用体
```

要点与坑：

- 这些声明**从不被 try_check_declar 重查**（不进 PendingOp），是"受信任的预置"；注释明确写了这一点。
- `EnvBuilder::finish()` 会 `dag.mk_name_cache(anon)`：在 intern 表里按名字找到 `Nat.succ/Nat.add/...`，给 `NameNode` 打 `NatRed` 标记。此后求值遇到这些头时，`unfold_value_go` 先走 `do_nat_red` **原生大整数运算**（`num-bigint`），而不是展开成递归定义。这就是 `#reduce 1 + 2 => 3` 的来源，也解释了为什么要以 `Definition`（Unfoldable）而非 `Axiom` 形式放置。
- `Expr::Num` 在 front 被 elaborate 成 `NatLit`（bignum 指针）；`a + b` 是 `Nat.add a b` 的语法糖。
- **风险/待对齐**：`Nat.succ`/`Nat.add` 的体是"自引用占位"，语义上等价于公理 + 原生快路径；上游真身是正常递归定义。教学 prelude 必须保证这些名字**只在有实参时被原生快路径接管**、裸名字（如 `#reduce Nat.add`）不会被 delta 无限展开——当前实测 `#reduce Nat.add => Nat.add` 可终止，但这是要长期盯住的边界（见 `docs/design/infrastructure.md` 的 prelude 工作流）。
- `nat_extension` 由 `Config::default()` 默认打开；`StringLit` 类似（`string_extension`）。

### 5.5 `#prove`：tactic 只是"帮你搭 lambda"（`crates/front/src/proof.rs`）

`ProofState { goal, goal_source, binders, solution }` 完全不碰内核：

- `intro name`：剥掉最外层 `Forall`/`Arrow` binder，把 binder 存进 `binders`，goal 变成 body；
- `exact term` / `apply term`（目前同义）：把解析出的项放进 `solution`（洞）；
- `assumption`：在 `binders` 里找一个类型与当前 goal 文本相同的假设（**文本比对**，教学草案级的简化）；
- `lambda_text()`：把 binders 反向包回 `fun (b : T) =>` 前缀，未完成处显示 `???`；
- REPL 里 `done` 把 `example : <goal> := <lambda>` 追加进 buffer 重编译，**由完整内核判定**。

设计意图：让学生先看懂"证明就是构造 lambda/证明项"，再接触 tactic 语感。

---

## 6. 内核相对上游快照的改动清单（务必先读）

快照基线 `sokonanoda@7b51784`（见 `NOTICE.md`）。lang 内核**只加不改语义**，改动如下：

| 文件 | 改动 | 用途 |
|---|---|---|
| `builder.rs` | **新增** `EnvBuilder` | 在 arena 内构造声明、最后一次性 `finish()` 成 `ExportFile`，绕开 `TcCtx` 自引用生命周期 |
| `util.rs` | `ExportFile::empty`、`Config::default`、`CheckError`、`try_check_declar`（panic→Result）、`alloc_string/alloc_bignum/name_from_str` 公开 | 内存 API（M0 验收） |
| `quote.rs` | `infer_closed_type` / `reduce_closed` | `#check` / `#reduce` 的 kernel 原语 |
| `eval.rs` | `deep_reduce` | 教学 `#reduce` 完整归约 |
| `tc.rs` | `ctx` 公开 + `TypeChecker::with_pp` | front 直接读写内核 arena |
| `pretty_printer.rs` | `→` 改 ASCII `->`；`name_from_str` 公开 | 教学文本一致 |
| `conv.rs` + `infer.rs`/`inductive.rs`/`tc.rs` | def_eq 失败分支的 panic 消息改为稳定格式 `def_eq mismatch expected: <E> \| actual: <A>`（保留 `def_eq failed` 前缀；两侧值经 quote + debug 打印，各截断到 200 字符）；仅改动失败/冷路径，热循环不变 | 内核拒绝能给教学文案「类型不匹配：期望 X，实际是 Y」（front 解析该标记，见 `front/src/compile/error.rs`） |
| `conv.rs` | **soundness 修复（2026-09-07，lang）**：`unify_direct` 的 Pi/Lam body-expr 快路径增加 `closure_ctxs_compatible` 守卫——eval 闭包（`ctx: None`）与 infer 闭包（`mk_infer`）对同一 interned body 表达式语义不同（`$0` vs `Sort 1`），混用时 `(A : Sort 1) -> A` 这类不可居住类型会被 `fun (A : Sort 1) => A` 误判可居住。热路径仅增加一个闭包语义判别分支；perf 冒烟与全量测试无回归。回归测试在 `tests/memory_api.rs` 与 CLI e2e | 判定（kernel 信任边界）正确性：tactic judge、练习判定都依赖 def_eq 不出假阳性 |
| `lib.rs` | `deny(clippy::cast_possible_truncation)` → `warn`（上游代码自身未过此 lint；冻结快照原则，教学 crates 的严格 lint 门禁经各自 `[lints]` 表实现） | lint 配置，非语义 |
| `conv.rs` + `inductive.rs` | **冷路径消息分诊（2026-09-07 第十三轮）**：`is_prop_type` 的 `.expect("expected a sort")` 改为带 `got:` 渲染、措辞区分站点的 `expected a sort in conversion`（分类器前缀不变，教学码同为 `kernel-expected-sort`）；`inductive.rs` 两处学习者可触发的裸 `assert_eq!`（iota 规则顺序/数量）改稳定 panic 消息并新增家族 `kernel-rec-rule-mismatch`。三层回归：kernel memory_api + front 分类 + CLI e2e | 错误分类学余项；冷路径，热循环零改动 |
| `Cargo.toml` | bin 改名 `sokonanoda-kernel`；`stumpalo 0.5.1` | workspace 集成 |
| `tests/memory_api.rs` | **新增** | 无导出文件的内存检查验收 |
| `builder.rs` | `add_inductive` 返回构建的 `Declar`；新增 `begin/end_inductive_block` 与 `mutual_block_sizes` 记账 | 归纳块可被 kernel 判定（I8a check-then-add） |
| `quote.rs` + `tc.rs` + `pretty_printer.rs` | **hover 显示层（2026-09-09，第十七轮）**：`infer_under_binders` 去掉 `force_all`（`Not a` 保持折叠，quote 自行 force thunk；唯一消费者是 front 的 `resolve_hovers`）；pp 新增 `seed_binder_names(&[String])` + `TypeChecker::with_pp_scoped`——把 scope binder 名字（外层在前）预置进 `binder_names`，开项打印的松散变量直接还原真名（代数验证含 telescope 域 lift 的所有位置：`S-1-j` 恒成立）。热路径零改动 | 悬停「表达式 : 真名类型」；`$N` 索引不再泄漏（设计见 `docs/design/hover-brackets.md`） |
| `pretty_printer.rs` | **hover 开项短路（2026-09-10，第二十四轮）**：`is_implicit_fun` 对含松散变量的 fun 项直接返回 `false`——空 context 推断开项（Var 头 `p a`、依赖实参 `Eq α a`）会对松散变量 panic（`infer: loose bvar` / `eval: loose bvar`），hover/`#check` 的类型文本被 front 的 catch_unwind 吞空。纯显示层，闭项行为逐字节不变；三层回归（kernel `memory_api` + front hover + CLI/LSP） | hover `Eq.subst.{1}` 显示完整签名；`#check (Eq.subst.{1})` 不再假报 kernel-rejected（设计见 `docs/design/goal-func-spine.md`） |

上游"解析 NDJSON 导出文件 → 完整检查"路径原样保留（`parser.rs` + `Config::to_export_file` + `check_all_declars`），arena 集成测试 `tests/arena.rs` 也在（需要 `LEAN_KERNEL_ARENA` 指向用例根目录才运行）。

---

## 7. 测试金字塔与回归策略

- 每个语法点/错误模式先在 front 写单元测试（`compile.rs` 的 `#[cfg(test)]` 里有大量 py-fol/py-nat 移植断言），再在 CLI 端到端覆盖，最近再加了"所有 examples 必须能整文件通过"的语料测试。
- 判定练习**靠 kernel，不靠文本比对**（唯一例外：`proof.rs::assumption` 的草案级文本比对，待替换）。
- `tests/arena.rs` 需要环境变量；上游两个缺 fixture 的测试在 kernel 内被隔离/ignored（不是跳过内核能力，而是缺少 NDJSON fixture，重建是独立任务）。

---

## 8. 给其他 agent 的注意点（gotchas）

0. **内核交互必须包 catch_unwind**（`quiet_catch`/`resolve_hovers` 模式）：
   内核以 panic 报拒绝/内部错误，不包会崩掉编译/LSP；且 panic hook 是
   进程全局的，`quiet_catch` **不可嵌套**。经验台账见 `docs/LESSONS.md`。
0b. **front 与内核的归纳块协作契约（第十三轮起）**：内核给每个归纳块
   自算 `is_recursive`（构造子 telescope binder 类型是否提到归纳名——含
   result 箭头链的 domain）并断言 front 传入值一致；内核还要求每块注册
   `Recursor` 声明（`<ind>.rec`，每构造子一条 iota 规则）。front 侧镜像：
   `elab.rs` 从源码 AST 同规则计算 `is_recursive`；缺 `rec` 的块在**入环境
   之前**报 `elab-missing-inductive-rec`（check-then-add 语义保持）。
1. **arena 生命周期**：`EnvBuilder`/`ExportFile`/`ExprPtr` 都挂在同一个 `stumpalo::Arena` 上，arena 必须活得比任何检查会话久；front 在 `compile_fol` 内开 arena 并一次跑完所有 PendingOp。Session（`front/src/session.rs`）每次 update 都开新 arena——跨 update 只复用渲染后的快照（DeclState/hover/事件文本），不复用内核对象。
2. **kernel 拒绝 = panic → Result**：内核仍用 `assert!` panic 报拒绝（如 `def_eq failed`），`try_check_declar` 用 `catch_unwind` 包装成 `CheckError::Rejected/Internal`。conv 失败的 def_eq 消息带 `expected/actual`，front 解析填充 `CompileError.expected/actual`（I9 已闭环）；更细粒度的 kernel 错误仍是后续任务（见 design doc）。
3. **elab 仍受限**：binder 可由声明类型推断（I6），但未做 `match`、`let`、结构/类型类、notation/macro。
4. **语法白名单是边界**：想加语法，先加课程 + 测试；`???` 只允许出现在声明（def/theorem/example）的值位。
5. **不用官方工具链**：CI 与本地一律 `cargo`；不要引入 `lean`/`lake`/`lean4export`。
6. **新错误要带 stage/code 与 span**：CLI 已按 `error[stage]:` 输出，`--json` 是 agent 视图；改输出格式要同步 `docs/protocol.md` 与 `crates/cli/tests/cli.rs`。
7. **打印偏好**：教学文本 ASCII `->`；`pp_options.proofs=true` 由 `compile_fol` 设置（否则打印会把证明项压成 `_`）。
8. **tactic/编辑器判定走 `front::judge`**（合成完整声明交完整 kernel 裁决），不要新增文本比对；`proof.rs::assumption` 的文本比对实现已删除。建议生成（`front::suggest`：exact/rfl/refine/intro）与逐洞判定（`judge_hole_fill`：把洞替换候选后整份声明交 kernel）都只是结构生成 + kernel 终审。
9. **kernel lint**：`lib.rs` 的 `cast_possible_truncation` 已降为 warn（上游代码自身未过）；clippy 严格门禁在各教学 crate 的 `[lints.rust] warnings = "deny"`，CI 的 fmt 门禁只覆盖教学 crates（kernel 的 rustfmt.toml 需要 nightly）。

---

## 9. 术语表

| 术语 | 含义 |
|---|---|
| kernel / 内核 | 可信的、被完整迁移的类型检查器（crates/kernel） |
| front / 前端 | `.sokonanoda` 词法/语法/小型 elaborator/练习判定（crates/front） |
| Declar | 内核声明（axiom/def/theorem/inductive/ctor/recursor/quot/opaque） |
| Elaboration | 表面语法 → kernel 项（名字解析、universe 参数解析、binder 折叠） |
| iota | recursor 在构造子上的计算规则（pattern matching 的计算语义） |
| delta | 定义展开（def/theorem 体替换） |
| conv / def_eq | 定义相等/可转换性（类型检查核心关系） |
| WHNF | 弱头范式；常规检查只需 WHNF，`#reduce` 要 deep normal form |
| EnvLimit | 环境可见前缀控制（Empty/ByIndex/ByName/PpUnlimited） |
| CheckEvent | 前端产出的事件（decl.checked / expr.typed / exercise.open …） |
| `???` | 练习"未完成"洞：合法文件状态，产出 `exercise.open` |
| `#prove` | 教学 tactic 草稿：tactic 语句只在搭 lambda，最终仍由 kernel 判定 |
| NDJSON | lean4export 的导出格式（上游内核的检查输入；教学栈不走它） |

---

## 10. 快速上手指南

> 面向**贡献者**（需要 Rust）。用户/agent 的零工具链路径见根 `README.md`
> 「Use it」与 `skills/sokonanoda-teacher`（Release 二进制 / 平台插件）。

```bash
cargo test --workspace                       # 全部测试
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json examples/fol-basics.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda repl     # #check / #reduce / #prove
cargo run -q -p sokonanoda-lsp --bin sokonanoda-lsp      # LSP（editor/vscode 里使用）
```

想给某个语法点加测试：先在 `crates/front/src/compile.rs`（或 `lib.rs`）加单元测试 → 在 `crates/cli/tests/cli.rs` 加端到端 → 需要的话新增/改 `examples/lesson-XX.sokonanoda`（examples.rs 会自动跑它）。
