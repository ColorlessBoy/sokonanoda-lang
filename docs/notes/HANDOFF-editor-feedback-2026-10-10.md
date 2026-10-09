# HANDOFF / PROMPT —— 四条编辑器反馈的实现交接（2026-10-10）

> **这份文件是什么**：用户 2026-10-09/10 反馈的**四条**编辑器体验问题，已由本轮逐条实测定位。
> 下面是**可直接交给执行者（agent 或人）的完整 prompt**——每条含「用户原话 / 实测读数 /
> 根因（文件:行）/ 目标行为 / 判据 / 边界」。执行者不需要再读这份文件以外的东西才能开工，
> 但**必须先读仓库的 `AGENTS.md` → `docs/ONBOARDING.md`**（纪律与接手路径以那两份为准）。
>
> **过期**：本件是**临时交接件**，四条做完即应删除或归档（登记见 `scripts/docs-expiry.json`）。

---

## 0. 环境与纪律（先读，别跳）

- 仓库 `sokonanoda-lang`。工具链一律 `scripts/soko …`（版本锁定、幂等、零 cargo）；先 `scripts/soko doctor --json`（0=就绪）。
- 改完 Rust 源码要重编：`cargo build -p sokonanoda-cli -p sokonanoda-lsp`（LSP 探针走 `scripts/soko lsp`）。
- **禁止**：调 Lean 官方工具链（lean/lake/lean4export/leanc/elan）；为使用仓库装 Rust；`cargo fmt --all`。
- 验证节奏（AGENTS「CI 节奏：批次制」）：日常 = `scripts/dev-verify.sh`（0.3 秒）+ 单点
  `cargo test -p sokonanoda-front --lib` / `--test prelude_mirror` / `-p sokonanoda-lsp --test <单个>`；
  环节收尾 = `scripts/soko gate --fast`；**不许**日常跑 `cargo test --workspace` / 整本课程 `check.py` / 语料对拍。
- 长命令：`timeout N <cmd> > /tmp/<有意义的名字>.log 2>&1 &` + `tail -20` 轮询；别阻塞干等、别把原始输出灌进上下文。
- 硬规则：判定永远走 kernel，**禁止文本比对**；`crates/kernel/**` 一行都不许动；
  **改行为必须同轮改判据**（「改行为不改判据 = 没做完」）；每条用户可见改动先回答
  「屏幕上会多/少什么？那条断言在哪一层？」；**同类问题横向排查、不许修单点**；
  守卫必须能咬住已知历史 bug（做反向验证）；卡住 ⇒ 先读本机 Lean 4 源码对齐
  （`~/Documents/lean/lean4/src/Lean/**`），**没有对照不许改判定代码**。
- 工作区有**未提交**的课程改动（`courses/set-theory/lib/Set.sokonanoda`、`courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda`、
  未跟踪的 `courses/set-theory/units/zz-scratch-user-snippet.sokonanoda`）——**不是你的任务**；
  `git add` 前逐行看 `git status --short`，别用 `-A` 扫进去。
- 仓库根有一份本轮量读数用的临时探针 `probe-lsp-feedback.mjs`（未入库）：`node probe-lsp-feedback.mjs`
  一次跑完反馈 1/3 的读数。收尾时按 `docs/gaps/repro/` 退出码约定（0=缺口仍在 / ≠0=已修）转正或删除。

---

## 1. 反馈一：`=` 没有 F12；指令注释里的目标名也不能跳

**用户原话**：
> `"a = b"` 里的等于号没有跳转，`"sokonanoda:builtin-notation"` 没有。另外，这个走得特殊逻辑吗？
> 因为 prelude 里的 `-- sokonanoda:builtin-notation "∧" => And` 是注释，点击 `And` 无法跳转。

**实测读数**（真 LSP；复跑 `node probe-lsp-feedback.mjs` 应得同样读数）：

| 位置 | `textDocument/definition` | `textDocument/hover` |
|---|---|---|
| `a = b` 的 `=`（有 import 的项目 / 单文件都一样） | `null` ✗ | 含一句假话（见下） |
| `a ∧ b` 的 `∧`（对照） | 落 `prelude/Prelude.sokonanoda` 第 65 行 `-- sokonanoda:builtin-notation "∧" => And`（列 0–43）✓ | **同样**说「没有源码声明，`F12` 无处可跳」✗ 自相矛盾 |
| prelude 里该行注释中的 `And` | `null` ✗ | `null` ✗ |
| prelude 里该行字符串中的 `∧` | `null` ✗ | — |

`=` 的 hover 逐字：
```
`=` —— 记法符号

内建记法（内核 prelude）：**没有源码声明**，`F12` 无处可跳

展开成 `Eq`

`Eq : {α : Sort u} -> α -> α -> Prop`

输入：直接打 `=`（语言没有为它约定缩写）

`a = b : ∀ {α : Type 0}, α → α → Prop`
```

**根因（三条，互相独立，别只修一条）**：

1. `=` 是**三处故意特例**的叠加：
   - `crates/front/src/parser.rs:3502` `LEXER_NATIVE_SYMBOLS = &["="]` + `:3494 lexer_builtin_symbols()`
     把 `=` 从**喂词法的符号表**剔除（真理由：词法最长匹配，`=` 进符号表会把 `=>` 吃成 `=`+`>`，R-2 有实测）；
   - `crates/front/src/notation.rs:107 builtin_directive_span()` 在 prelude 里找不到 `=` 的指令行
     ⇒ `span = Span::default()`（offset 0）；
   - `crates/lsp/src/lib.rs:2306` 的 `if module.is_none() && span.start.offset != 0` 把 offset 0 的内建
     **直接放过** ⇒ F12 无声返回（既不报错也不跳）。
   - ⚠ **关键结论**：prelude 注释（`prelude/Prelude.sokonanoda:63-64`、`prelude/L1.sokonanoda:59-60`）
     把「喂词法的符号表」与「注释登记行」**混为一谈**。`-- sokonanoda:builtin-notation` 是**注释**，
     只被 `builtin_directive_span()` 做**纯文本查找**、**不喂词法**（`lexer_builtin_symbols()` 是硬编码剔除）
     ⇒ **给 `=` 补一行登记不会影响 `=>` 的分词**。真正不许动的只有 `lexer_builtin_symbols()` /
     `LEXER_NATIVE_SYMBOLS` / `BUILTIN_NOTATIONS` 的**符号表内容**。
2. 指令注释里的目标名不可跳：`crates/front/src/notation_input.rs:832 notation_target_at()` 是**词法**扫描
   （在 token 流里找 `infix*`/`prefix`/`postfix`/`notation`/`binder_notation` + Str + 之后第一个 Ident），
   **注释不产生 token** ⇒ 永远认不出注释里的目标名 ⇒ 三条消费者全 `null`：F12（`crates/lsp/src/lib.rs:2340-2355`）、
   hover（`:2052-2071`）、documentHighlight（`:2435+`）。
   - 这与仓库白纸黑字写的**两跳模型**矛盾：`docs/gaps/repro/G23-notation-navigation.js:189` 写
     「『`∈` → 记法声明 → 定义』明确建模成**两跳**（第二跳是 E10 的活）」。第一跳 E10 做了，
     **第二跳在指令注释这条路上没做**。
3. hover 文案是 T-D20 时代的旧话（`crates/lsp/src/lib.rs:1610`），E10 之后对 `∧ ∨ ↔ ¬ ≠` 已不成立
   ⇒ 现在在 5 个符号上是**假话**；判据 `crates/lsp/src/tests/hover.rs:543` 还在断言它。

**目标行为（要决策、要写进文档）**：

1. `=` 上 F12 必须有落点。**推荐**与另外 5 个内建记法**对称**：在 prelude 登记区补一行
   `-- sokonanoda:builtin-notation "=" => Eq`，让 `builtin_directive_span` 找到它 ⇒ F12 落那一行；
   同轮把「为什么 `=` 不喂词法」的注释改写成准确版本（区分「注释登记」与「喂词法的符号表」）。
   - 备选（若你有硬理由否决对称方案）：内建记法没有指令行时，F12 退到 `prelude_def_span(target)`
     （`=` → prelude 第 1 行 `axiom Eq`）。选它就必须在 `docs/design/notation-subset.md` 写清**为什么偏离对称**。
   - 无论哪条：**不许**改词法符号表内容；`=>` 的分词必须仍逐字节正确（跑 `crates/cli/tests/notation.rs:1844` 与 R-2 相关用例）。
2. 指令注释里的目标名要能跳（第二跳）：让 `notation_target_at`（或同族新入口）认出
   `-- sokonanoda:builtin-notation "<符号>" => <目标名>` 这种**注释形态**的目标名 span，
   然后**复用已有**的目标名分支（`lib.rs:2340-2355` 等）。
   - 落点：`And` → prelude 里 `inductive And` 那一行；`Eq` → `axiom Eq` 那一行。
     名字解析走**已有真相通道**（`project_definition` / `prelude_def_span`），不许文本比对。
   - **同族横排（一次改齐，逐条给「做 / 不做 + 理由」）**：
     - `-- sokonanoda:builtin-sugar "{a}" => Set.singleton`、`"{a, b}" => Set.pair`：目标是**卷 I 库常量**，
       在 prelude 文件自己的闭包里**解析不到** ⇒ 诚实 `null`，但 **hover 要说明原因**，别静默；
     - `-- sokonanoda:builtin-rust "Nat / …"`：`prelude_def_span` 对它们**如实**是 `None`（E2 已钉）
       ⇒ 保持诚实 `null` + 说明；
     - 记法声明**字符串里的符号**（`"∧"`）现在是 `null`（`docs/gaps/ledger.jsonl` 的 G-55 notes ②
       已记为「建议另立一条」）⇒ 本轮**要么做、要么写明不做 + 理由**，不许不表态。
3. hover 文案与事实一致：有指令行的内建记法，删掉/改写「没有源码声明，`F12` 无处可跳」；
   确实没落点的保留诚实说明。`crates/lsp/src/tests/hover.rs:543` 跟着行为一起改。

**判据（三层 + 用户动作 + 反向验证）**：

- front（真相）：`notation_target_at`（或新入口）对**注释形态**的识别单测——给一行
  `-- sokonanoda:builtin-notation "∧" => And`，断言在 `And` 的 offset 上返回 `("And", span)`；
  反向：offset 落在注释别的词上 ⇒ `None`。
- 判据同步：`crates/front/src/notation.rs:244-277` 的
  `builtin_notations_carry_their_directive_line_as_the_declaration_site` 今天**明确豁免 `=`**（`:248-252`），
  `crates/front/tests/prelude_mirror.rs:199-202` 同样跳过 ⇒ 若补登记行，两条一起改成**覆盖 6 条**。
- LSP（wire）：`=` 上 definition **非 null** 且**落点行号**正确；注释里的 `And` 上 definition **非 null**、
  落点行 == `inductive And` 那一行。**必须断言落点行号**——E05/G-37 的教训：只断言「非 null」会放过
  **原地跳**（账面绿、实际坏）。既有 E10 用例 `crates/lsp/src/tests/navigation.rs:678` 是模板。
- 用户动作（硬规则 0(a)）：判据必须打在**用户实际点的字符位置**上（`=` 字符上、注释里 `And` 的字符上），
  不是「能跑通的位置」。
- 反向验证：撤掉实现 / 删掉登记行 ⇒ 判据当场判红，报错与判红时逐字一致。

---

## 2. 反馈二：prelude 自己编译不过（`Classical.byContradiction`）

**用户原话**：
> 「prelude」文件自己都编译不过自己，为什么没有解决掉 `Classical.byContradiction` 会报错

**实测读数**（先复现，再动手）：

```
scripts/soko grade --json prelude/Prelude.sokonanoda        # exit 1
scripts/soko query check --file prelude/Prelude.sokonanoda  # decl_checked: 42 / failed: 1
```
诊断逐字：
```
{"code":"kernel-rejected",
 "message":"类型不匹配：期望 `Sort(0)`，实际是 `((Or.[] 第 1 个绑元（p）) (Not.[] 第 1 个绑元（p）))`",
 "start_line":34, "end_line":36}
```
分文件：`prelude/Eq.sokonanoda` 3 checked / 0 诊断 ✓ · `prelude/L1.sokonanoda` 33 / 1 ✗（同一处，start_line 30）
· `prelude/Quot.sokonanoda` 6 / 0 ✓。

**根因链（四条，报告里要写全）**：

1. 出错的是 `prelude/L1.sokonanoda:30-32`：`Or.elim` 的**动机位 `c` 漏了**，`Classical.em p` 被塞进了 `f` 的位置。
   正确写法（本轮已在自足夹具上验过判绿，你要在真 prelude 上重验）：
   `Or.elim p (Not p) p (fun (hp : p) => hp) (fun (hnp : Not p) => False.elim p (h hnp)) (Classical.em p)`
2. prelude 是**受信任安装**（`crates/front/src/compile/prelude.rs:3`「安装后不再被内核重查（不进 PendingOp）」；
   `:830` `build_def(...).expect("L1 prelude definition elaborates")`）⇒ **只 elaborate、不 kernel-check**
   ⇒ 类型错的**体**被静默装进环境；因为**类型**是对的，所有使用点照常判绿（`#check` / `#print` / 课程
   `unit104-solution` 全绿）⇒ **单靠「用一次 `Classical.byContradiction`」的判据永远咬不住它**。
3. 仓库里**没有任何判据**把 `prelude/*.sokonanoda` 当**普通文档**判一遍：`crates/front/tests/prelude_mirror.rs`
   只钉「三段拼接 == 视图 == `prelude_source()`」的逐字节相等；`crates/front/tests/prelude_shape.rs`
   钉课程闭包的 prelude 形状；`scripts/soko gate` 六步里没有一步判 prelude 源。
4. 历史与过期注释：`Classical.byContradiction` 是 `660b9ccd`（2026-10-02，G-74）作为 `prelude.rs` 里的
   **源级字符串**加进来时就写错的；`e6bdc6bc`（2026-10-08，E1）把真相搬进 `prelude/*.sokonanoda` 时
   原样搬过来 ⇒ **从落地那天起就没编译过**，只是没人判它。而 `crates/front/src/compile/prelude.rs:491-492`
   至今写着「实测前提：这份源作为普通文档编译是**干净**的（35 条声明 / 0 诊断 ✓）」——**今天已是假话**
   （43 条声明 / 1 条诊断），这句注释本身就是过期证据。

**目标行为 / 交付**：

1. 修 `prelude/L1.sokonanoda`。**L1 是真相源**，`prelude/Prelude.sokonanoda` 是**生成视图**
   ⇒ 顺序别反（先改 L1，再同步视图；`prelude_mirror.rs` 会挡住不一致）。
   - **只修体**：签名、名字、让位族（B10 的 `names`/`deps`）**一字不动**（改签名会漂全课程 `--json`）。
2. 补一条**会判红的守卫**（这才是本条真正的交付）：把 prelude 的**生效源**（`prelude_source()`，
   即编辑器 F12 打开的那一份）当普通文档判一遍，断言 **0 诊断 + 声明计数**（计数别写死太小，今天 43）。
   建议放 `crates/front/tests/prelude_mirror.rs`（已有 prelude 三件套守卫、进 `cargo test` ⇒ 自动进
   `scripts/soko gate` 与 CI）。
   - **反向验证**：把那行改回错的 ⇒ 守卫必须判红（守卫要能咬住这个**已知历史 bug**）。
   - 顺带把 `prelude.rs:491-492` 那句写死的「35 条声明 / 0 诊断」改成跟着判据走的说法（别再写死数字）。
3. **边界要如实写进交付说明**：本轮**不**改「prelude 受信任安装、不重查」这条设计（`prelude.rs:3`）——
   那是独立的设计决定；但要在 `docs/` 记下「受信任安装 ⇒ 体的类型错误只能靠**判 prelude 源**这条守卫抓」，
   并说明 `install_l1_command` 的 `.expect(...)` 只保证 elaborate 成功、**不保证内核接受**。

---

## 3. 反馈三：tactic 里 hover 要加「名字本身的类型」

**用户原话**：
> `apply Set.ext` 这种 tactic 下，我鼠标在 `Set.ext` 上时，我希望 hover 的内容除了当前的 goal state，
> 分割线后再加上 `Set.ext` 本身的类型内容

**实测读数**（真 LSP；用户现场 = `courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda:65`，
库里的 `Set.ext` 在 `courses/set-theory/lib/Set.sokonanoda:108`）：hover 只给 tactic + `tactic 1/2` + goal state，
**没有 `Set.ext` 的类型**；hover 在 `apply` 关键字上输出**逐字相同**。

**根因**：`crates/lsp/src/lib.rs:1398 tactic_goal_hover()` 对「光标落在任何 `by` step span 内」
**无条件早退**（`:2028`，位于 hover 链**最前面**），返回的 `range` 是**整条 tactic**；
它只读 `by_steps` 的 goal 快照，**不解析 tactic 里的名字**。而 `Set.ext` 的签名本来就在手上：
`judge_type_of_constant`（`crates/front/src/judge.rs`；`notation_symbol_hover` 已在用，`lib.rs:1636`）
能给出 `Set.ext : {α : Type} → (A B : Set α) → (∀ (x : α), x ∈ A ↔ x ∈ B) → A = B`。

**目标行为**：

- 光标在 tactic 里的**名字**上（`apply Set.ext` 的 `Set.ext`、`exact h` 的 `h`…）⇒ hover = 现有内容
  （tactic + `tactic i/n` + goal state）**+ 一条分割线 + 该名字的类型行**（`` `Set.ext : {α : Type} → …` ``，
  折记法、与其它 hover 同一观感）。
- 光标在 **tactic 关键字 / 其余位置**上 ⇒ 行为**逐字节不变**（别把 goal state 弄丢、别改顺序）。
- 名字解析要**诚实**：拿不到干净类型（含 `$N` 松散变量、unknown identifier）⇒ **不编那一行**
  （与 `lib.rs:1638-1642` 同一条纪律）；不是名字（数字、括号、字符串）⇒ 不加。
- 分割线形式**定一次**并写进 `docs/protocol.md`——该文档 `:285`「Tactic goal-state hover (0.27.0)」
  现在写的是「range = tactic 的 span、只有 goal state」，改行为必须同轮改它。
- **要决策并写明**：hover 的 `range` 还是不是整条 tactic？（今天 wire 契约就是它；改成名字的 span
  会与「点 `apply` 关键字也给 goal state」冲突 ⇒ 写清取舍。）

**判据（三层 + 反向）**：

- 接缝/真相层：`apply <常量>` 夹具，断言 hover markdown **同时**含 goal state 块与目标名签名行、
  且**分割线存在**。⚠ 别只断言「有 `Set.ext` 字样」——goal state 里本来就可能出现 `Set.ext`，
  那种判据会被自己骗过（硬规则 0(a)：断言用户实际看到的）。
- LSP（wire）：`crates/lsp/src/tests/hover.rs` 加用例，光标**正好落在 `Set.ext` 的字符上**
  （用户实际点的位置）；**反向**：光标在 `apply` 关键字上 ⇒ 断言**没有**类型行（防「整条 tactic 一律加」）。
- 渲染层（真宿主）：Infoview/悬停若走同一渲染，补一条「看得见」的判据（`editor/vscode/test-webview.js`
  或 `scripts/vscode-e2e.sh` 用例；e2e 按批次收尾跑一次）。
- 反向验证：撤掉新分支 ⇒ 新判据判红，且报错与判红时一致。

---

## 4. 反馈四：`rfl` 在 `↔` 形状的目标上不可用（Lean 4 对齐）

**用户原话**：
> rfl 这个 tactic 好像不太好用，这类都不能使用 rfl。不过 rfl 确实比较隐蔽，不一定适合教学。
> ```
> theorem mem_empty_iff_false {α : Type} (a : α) :
>     a ∈ ∅ ↔ False := by
>   constructor
>   intro h
>   exact h
>   intro h
>   exact h
> ```

**实测读数**（自足夹具；用户形状 = 课程库 `courses/set-theory/lib/Set.sokonanoda:167`）：

| 写法 | 读数 |
|---|---|
| `theorem … : a ∈ ∅ ↔ False := by rfl` | ✗ `elab-tactic-failed`「`rfl` 需要一个 `Eq α x y` 形状的目标」 |
| `theorem … : a ∈ ∅ ↔ False := by exact Iff.refl` | ✓ **判绿**（今天的可行绕法） |
| `theorem (a : Prop) : a ↔ a := by rfl` | ✗ 同上 |
| `theorem (a : Prop) : a = a := by rfl` | ✓ |
| `theorem (a b : Prop) (h : a ↔ b) : a ↔ b := by exact Iff.refl` | ✗ `exact` 类型不匹配（**这条必须继续红**） |
| `#check Iff.rfl` / `#check Iff.refl` | 前者 `unknown identifier Iff.rfl` ✗ / 后者存在 ✓ |

**Lean 4 对照（先交这 3–6 行，再动手）**：

- `~/Documents/lean/lean4/src/Init/Core.lean:1025-1029`：`@[refl] theorem Iff.refl (a : Prop) : a ↔ a`、
  `protected theorem Iff.rfl {a : Prop} : a ↔ a`，以及
  **`macro_rules | `(tactic| rfl) => `(tactic| exact Iff.rfl)`** —— 即 **Lean 4 的 `rfl` 字面上就是
  `exact Iff.rfl`**（`exact` 走 defeq 统一）⇒ `a ∈ ∅ ↔ False` 这种两边 defeq 的 `Iff` 目标**直接过**。
- `~/Documents/lean/lean4/src/Lean/Meta/Tactic/Rfl.lean`：另有 `@[refl]` 属性 + `applyRfl` 扩展
  （`whnfR`、不解关系本身；`Eq` 是特例）⇒ 任何 `@[refl]` 标记的自反关系都吃 `rfl`。
- `~/Documents/lean/mathlib4/Mathlib/Data/Set/Basic.lean:428-429`：
  `theorem mem_empty_iff_false (x : α) : x ∈ (∅ : Set α) ↔ False := Iff.rfl`
  —— **用户这条定理在 Mathlib 里的原样证明就是 `Iff.rfl`**（`@[refl] theorem Set.Subset.refl`
  在同文件 `:264-265`，所以 `A ⊆ A` 在 Lean 4 里也吃 `rfl`）。
- **我们**：`crates/front/src/by.rs:1911/1929` 的 `rfl_candidate` 只认 `Eq` 头 ⇒ 判红；`Iff.rfl` 这个名字不存在。

**决策（二选一 + 可选第三项，都要写进文档）**：

1. **推荐 = 对齐 Lean 4**：让 `rfl` 接受 `Iff` 头的目标，候选 `Iff.refl`
   （`prelude/Prelude.sokonanoda:46`，`{A : Prop} : Iff A A`），**仍由内核裁决**（`judge_terms` 同一通道），
   闭合走与今天同一处（`nodes[cur].kind = NodeKind::Closed(…)`）。
   - 边界：**只在两边 defeq 时过**（内核说了算）；**不许**把不 defeq 的 `a ↔ b` 弄绿。
   - 实现指路（不是规定）：`by.rs:1911` 的归一化路径已经会拿内核 pp 的规范形态
     （`canonical_goal_with_spec`）⇒ 在 `Eq` 之外加 `Iff` 头分支（**AST 直接构造**候选，别回读文本——
     `by.rs:3413` 的教训）；`by.rs:1929` 的错误文案要跟着改（现在是「需要一个 `Eq α x y` 形状的目标」）。
   - **横向排查（同类消费者）**：`crates/front/src/suggest.rs:472 eq_refl_candidate`（quick-fix 的 rfl 建议）
     ——`Iff` 头要不要也给建议？判据 `code_action_failed_eq_decl_offers_kernel_verified_rfl_first`；
     若不给，写清理由。
   - 判据：`crates/front/src/compile/tests.rs` 加「`rfl` 在 defeq 的 `Iff` 目标上判绿」+
     「`rfl` 在不 defeq 的 `Iff` 目标上仍判红」；`:4683 rfl_on_non_eq_goal_is_a_tactic_error` 的夹具
     是裸 Prop 目标（不是 `↔`）⇒ 它**仍应绿**，但**测试名/注释要跟着改**（它断言的是「非 Eq 就报错」这条口径）；
     `docs/TESTING.md:52` 的「rfl 非 Eq → `elab-tactic-failed`」与 `docs/protocol.md:404` 的 rfl 候选说明同轮改。
2. **备选 = 保持 Eq-only**：那就 (i) 错误文案必须**指出出路**（「目标是 `↔`：写 `exact Iff.refl`，
   或 `constructor` 后两个方向各证一次」）——今天的文案是死胡同；(ii) 把这条**偏离**写进设计文档 +
   `docs/TESTING.md`（「本语言的 `rfl` 只认 `Eq`；Lean 4 的 `rfl` 是 `exact Iff.rfl`」）；
   (iii) 课程/技能里教 `rfl` 的地方（`REQUIREMENTS.md:109` 第一课「等式与 rfl」）写明边界。
3. **可选 = 命名对齐**：加 `Iff.rfl`（Lean 4 是 `protected theorem Iff.rfl {a : Prop} : a ↔ a`）
   作为 `Iff.refl` 的别名，否则学生照 Mathlib 写 `Iff.rfl` 只会得到 `unknown identifier`。
   ⚠ 加 prelude 声明会动 `PRELUDE_NAMES`（今天 57，有计数断言 `prelude_names_match_installs`）、
   `prelude/*.sokonanoda` 三件套 + 镜子视图 + `prelude_shape`；按批次制，全课程 `--json` 逐字节对拍
   放到发版大节点。**做不做都要写明。**

**教学视角（用户原话「rfl 确实比较隐蔽，不一定适合教学」）**：把这条当**产品决定**做——
不管选哪条，交付里要有一句「屏幕上会多/少什么」：选对齐 ⇒ 学生写 `by rfl` 不再撞墙、
Mathlib 的 `Iff.rfl` 写法可用；选保持 ⇒ 学生必须学会 `exact Iff.refl` / `constructor`，**诊断要教会他**。

---

## 5. 交付物清单（DoD，逐条可勾）

1. `scripts/soko grade --json prelude/Prelude.sokonanoda` ⇒ **exit 0 / 0 诊断**；`prelude/L1.sokonanoda`
   同样 0 诊断；`cargo test -p sokonanoda-front --test prelude_mirror` 全绿（三段 / 视图 / `prelude_source()` 逐字节一致）。
2. 新守卫「prelude 生效源 0 诊断」进 `cargo test`；**反向验证已实测**（改回错体 ⇒ 判红）并记录读数。
3. `=` 上 F12 非 null、落点正确（含「不许自跳」断言）；`=` 的 hover 不再说假话。
4. 注释里的 `And`（以及 `Eq`）上 F12 非 null、落点 == 定义行；同族位置（`builtin-sugar` / `builtin-rust` /
   字符串里的符号）逐条有「做 / 不做 + 理由」。
5. `apply Set.ext` 的 `Set.ext` 上 hover = goal state + 分割线 + 类型行；`apply` 关键字上 hover
   与今天**逐字节相同**。
6. 反馈四：**决策已定并写进文档**（对齐 / 保持 + 理由），判据含「defeq 的 `Iff` 判绿」与
   「不 defeq 的 `Iff` 仍判红」两条（若选对齐）；`Iff.rfl` 命名项有明确表态；`docs/TESTING.md:52`、
   `docs/protocol.md:404`、`REQUIREMENTS.md:109` 相关口径同轮同步。
7. 文档同轮同步：`docs/protocol.md`（hover 契约）、`docs/design/notation-subset.md`（T-D20 段已过期，`:89`）、
   `docs/visible-changes.md`（用户可见改动）、`STATUS.md`（收尾）、`skills/` + `editor/vscode/`
   （若命令/反馈/语法可见面变了，按 AGENTS「收尾义务」同轮同步）；新缺口按契约进
   `docs/gaps/ledger.jsonl` + `docs/gaps/repro/*.sh`（退出码约定 0=缺口仍在 / ≠0=已修，
   `python3 scripts/gap.py check`）。
8. `scripts/dev-verify.sh` 绿 + `scripts/soko gate --fast` 绿；**只 push 一次**、推完盯 job 级
   （`gh run view <id> --json jobs`），红了当场修（别等整轮）。
9. 每条交付带**耗时账**与「修前 / 修后」读数，且读数**带构建身份**（同 commit + 同产物才可比，别跨构建并排比）。

## 6. 建议的动手顺序

1. 先修反馈二（改一行 + 加守卫，最独立、风险最低，且它让 F12 落进的 prelude 不再是红的）。
2. 再做反馈一的第 3 条（hover 文案）+ 第 2 条（注释目标名第二跳），最后做 `=` 的登记（第 1 条）
   ——它依赖前两条的落点语义定案。
3. 反馈四：先交 Lean 4 对照 + 决策（对齐 / 保持），再做实现与判据（若选对齐，它只动 `by` 引擎的
   `rfl` 分支 + 判据，独立一个 commit）。
4. 最后做反馈三（tactic hover），它动的是 hover 链最前面那条早退分支，单独一个 commit、单独一轮回归。
