# 设计：记法**输入表**（希腊字母 + 课程符号）

> 日期：2026-10-01。触发（用户原话）：「`α` 没有快捷输入（如 `\a` 或 `\alpha`）。
> Lean4 里应该有大量同类输入法」。
>
> 本文取代 2026-09-19 的同名旧文（2026-09-30 随 64 篇设计一起删除，原文 ⇒
> `git log --all -- docs/design/notation-input.md`）。旧文讲的是「做不做 Tab 改写器」
> ——**已落地**（`editor/vscode/src/abbreviation-rewriter.js`）；本文讲**表本身**。

## 0. 一句话

表从 **18 条**（只有逻辑/集合符号）扩到 **75 条**：**希腊字母 48**（大小写各 24）
+ **课程库记法 7**（`≈ ∘ ⁻¹ • ⊕ ⋃₀ ⋂₀`）+ **匿名构造子括号 2**（`⟨ ⟩`）。
表仍是**唯一真相源**（Rust `front::notation_input::TABLE` ↔ JS 逐条镜像），
hover 文案与 VS Code 改写器都从它来。

## 1. 调研：Lean 4 / vscode-lean4 的输入法体系（2026-10-01 取证）

上游 `leanprover/vscode-lean4@master`，逐字取证（`raw.githubusercontent.com` 需代理；
jsdelivr 镜像与 raw **逐字节相同**，`sha256 fa5e317d…b5b0`，37039 B）。

**表格式**：`lean4-unicode-input/src/abbreviations.json` 是**扁平** `{缩写: 替换文本}`，
**1865 个键**，没有 `abbreviation`/`aliases` 之分——**「别名」是运行时导出的**
（`AbbreviationProvider.collectAllAbbreviations(sym)` = 所有值等于该符号的键）。
替换文本可含 `$CURSOR`（26 条，用于 `\<>` → `⟨⟩` 把光标放中间）。

**算法**（`AbbreviationRewriter.ts` / `TrackedAbbreviation.ts` / `AbbreviationProvider.ts`）：

| 面 | 规则（逐字，行号见底稿） |
|---|---|
| leader | `\`（设置 `lean4.input.leader` 可改）；跟踪**从敲下 leader 那一刻**开始 |
| 何时落定 | ① 敲的字符让「已敲文本」不再是**任何键的前缀** ⇒ 落定；② 或按 `Tab`（命令 `lean4.input.convert`，**强制**落定）；③ 或 `eagerReplacementEnabled` 且「唯一且完整」（只有一个键以它开头**且**它自己就是键） |
| 选哪个符号 | `findSymbolsByAbbreviationPrefix(已敲)[0]`：**长度升序稳定排序** ⇒ **最短的键赢**；同长按 JSON 声明序。**没有回溯、没有歧义态** |
| 大小写 | **完全敏感**（`\And`→⋀ 而 `\and`→∧；`\Ga`→α 而 `\GA`→Α） |
| leader 转义 | 连敲两个 `\` ⇒ 一个 `\`（`\\`→`\`、`\\\`→`\\`） |
| 注释/字符串 | **不特判**（改写在原文偏移上做） |
| 上游测试 | `lean4-unicode-input` **没有测试**（全仓 392 条路径扫描，无 `*.test.ts`）⇒ 判据只能自己立 |

**前缀冲突的答案**（直接回答「`\alpha` 是否允许 `\a`」）：

* **允许**：`\a` 是 α 的键之一（α 的键 = `a` `alpha` `Ga`），`\b`→β、`\c`→**χ**（不是 γ）、
  `\e`→ε、`\g`→γ、`\m`→μ；大写 `\D`→Δ、`\G`→Γ、`\L`→Λ、`\S`→Σ、`\p`/`\P`→**Π**（不是 π）。
* **等待而非歧义**：`\a` 单独敲**不替换**（`a` 是 `alpha`… 的前缀）⇒ `\a` + 空格 → `α `；
  `\a`+`n`+空格 → ∧（`an` 是 ∧ 的键）；`\in` + 空格 → ∈，`\in`+`t`+空格 → ℤ（`int`）。
* **δ ζ η θ ι κ λ ν ξ ο π ρ σ τ υ φ ψ ω 没有单字母键**（`\d` 是 ↓、`\i` 是 ∩、`\o` 是 ∘、
  `\l` 是 ←、`\r` 是 →、`\t` 是 ▸、`\u` 是 ↑）——Lean 用 `G` 族（`\Gd`→δ、`\GD`→Δ）绕开。

## 2. 差距：课程**代码位**实际用到的符号（43 个课程文件，逐字统计）

`α`1872 · `β`748 · `γ`319 · `δ`5 · `∈`568 · `→`469 · `∧`371 · `∃`148 · `⊆`146 · `¬`143 ·
`⁻¹'`/`⁻¹`141 · `∀`139 · `∅`123 · `↔`96 · `∪`95 · `∩`81 · `≈`50 · `∨`45 · `∘`44 · `𝒫`40 ·
`ᶜ`19 · `≠`19 · `×ˢ`9 · `•`2 · `⋃₀`1 · `⋂₀`1 · `⊕`1 · `⟨⟩`1

现状 18 条里**没有**的：**全部希腊字母** + `≈ ∘ ⁻¹ • ⊕ ⋃₀ ⋂₀ ⟨ ⟩`。
后 7 个不是「没声明的符号」——它们在课程库里都有声明（`lib/{Equiv,Fun,Rel,Sum,SUnion}`），
缺的只是**输入法**与 hover 的那一行「怎么打」。

**补全后逐字复核**（2026-10-01，用上表逐字符对照）：课程**代码位**的每一个非 ASCII
字符都被表覆盖，**唯一例外是 `₁`/`₂`**——它们只出现在**库内部**的变量名 `α₁`/`α₂`
（`lib/Cardinal.sokonanoda`），不是记法符号，**学生画布上一个都不用打**。

## 3. 补全决策

**D1 · 收键口径（机械可判，不逐条拍脑袋）**：新条目的主缩写 = Lean 的**拼写名**
（`alpha`…`omega` / `Alpha`…`Omega` / `approx` `comp` `inv` `smul` `oplus` `sUnion`
`sInter` `langle` `rangle`）；别名 = 上游**映射到同一符号**的其余键，且同时满足
① 纯 ASCII 字母 ② **不是 `G`+字母族**（`Ga`/`Gb`/`GTH`…——那是 Lean 给自己单字母冲突
打的补丁，我们不收那些冲突键，故不需要）③ **单字母只收希腊字母名**
（`a b c e g m D G L S p P`）。**这条规则对全表一视同仁**（2026-10-01 修订）：原先
「已有 18 条只增不改」的写法被推翻——一个规则好过两套，且 Lean 的常用键（`\subset`
`\un` `\ex` `\all` `\member` `\an` `\lr`…）正是肌肉记忆要迁移的那批。**只增键、不改键**：
没有任何一个键改变含义，也没有把一个「完整」的键变成别人的前缀（`\an` ⊂ `\and` 是
Lean 本来就有的前缀关系，Tab/封口口径照旧）。

**D2 · `\a` 是安全的**：改写器已有「完整表词」两态口径（`requireComplete`）——
还在敲字母时**前缀不落定**，敲了空格/标点或按 Tab 才封口。所以 `\a` 与 `\and`/`\approx`
共存**不引入歧义**，与 Lean 同规则（判据见 §4）。

**D3 · 单字母只收希腊字母**：Lean 里 `\v`→∨、`\i`→∩、`\o`→∘、`\r`→→ 这些**不收**
（`\i` 给 ∩ 而 `\in` 给 ∈ 是反直觉的，拼写名 `\or` `\cap` `\comp` `\to` 已经够短）；
`\p`/`\P`→Π 收（它是希腊字母名，Lean 的怪癖逐字保留）。

**D4 · `notation_symbol` 新字段（Rust）/ `notationSymbol`（JS）**：希腊字母与 `⟨⟩` 是
**标识符/语法括号**，不是记法符号，必须与 `∈`/`𝒫` 分开——三处消费口径不同：
① `merge_known` **只喂记法符号**给词法（把 `α` 喂进去它就成了 `Sym("α")` ⇒ 变量 hover、
`F12`、`rename` 守卫 `references::cursor_is_on_notation` 当场全坏）；
② `notation_symbol_chars()` 只着色记法符号（TM 的 `variables` 规则管标识符，
`α` 不该长成算子的颜色）；③ hover：记法符号说「记法符号 + 展开成什么」，标识符只说「怎么输入」。

**D5 · hover 对标识符也给输入行**：`α` 走的是普通表达式 hover（`α : Type`），
在它后面**追加**一行「输入：`\alpha`（别名 `\a`）」——词法判据（`Ident` / `Langle`/`Rangle`
token），且**只在**该 token 不是记法符号时追加（否则与 `notation_symbol_hover` 重复）。

**D6 · `⟨ ⟩` 用 Lean 的 `\<` / `\>`**：这是全表**唯一**非字母缩写（`<`/`>`），
需要把「缩写只能是字母」的不变式放宽到**恰好这两个**；`\` 仍是 leader、孤立 `\`
（集合差）永不替换这条不变。

**D7 · 明写的偏离**（Lean 有、我们不做，各有理由）：`\\`→`\` 的 leader 转义
（我们的 `\` 只在「`\`+表词」时命中，孤立的 `\` 永远字面）；`$CURSOR` 条目；
`lean4.input.customTranslations`；非希腊的单字母键（D3）；`G` 族（D1）。

## 4. 判据（每条都可执行）

* **双镜像**：`cargo test -p sokonanoda-cli --test extension` —— `abbreviation_table_mirrors_the_single_source`
  逐条比 symbol/abbreviation/aliases/supported/**notationSymbol**/顺序；
  `tm_grammar_math_symbols_follow_the_single_source` 比 TM 的 `mathsymbols` 类。
* **前缀不歧义**（D2）：`front` 单测 `no_abbreviation_is_a_prefix_of_another` 钉住前缀对
  （`a`⊂`alpha`、`in`⊂`inv`、`in`⊂`inter`…）；`editor/vscode/test-extension-host.js`
  的 `\a` + Tab / `\a` + 空格 / `\a`+`n` 三态。
* **表说支持 ⇒ 语言真有**：`crates/cli/tests/notation.rs` 按 `notation_symbol` 分类——
  记法符号进 `probes`（真判卷）或 `course_lib`（课程门禁）；
  标识符型（希腊 48）用**一条**把它们全当 binder 的声明真判卷；`⟨⟩` 用匿名构造子那条。
* **hover**：`crates/lsp/src/tests/hover.rs` 断言 `α` 的 hover 含「输入：`\alpha`」，
  且 `∈` 不重复出现两行输入提示。
* **真宿主（用户看得见的那层）**：`editor/vscode/src/test/extension.test.js` 的
  `notation input: greek letters and the anon-ctor brackets, in a real host` ——
  真 VS Code 1.138.0 + 真 LSP：hover 绑定变量 `α` 含 `\alpha` 且**不**说"记法符号"、
  `\a`/`\alpha` + 命令 → `α`、`\<` + 命令 → `⟨`。台账 `docs/e2e/ledger.jsonl`（39/39）。

## 5. 已知限制（记着，别当成漏了）

* **TM 不着色希腊字母**：`sokonanoda.tmLanguage.json` 的 `variables`/`constants` 类是
  ASCII 的（`\b[a-z_][A-Za-z0-9_']*\b`），`α` 今天**没有任何 scope**（课程里 1872 次）。
  与 R-6（数学符号不着色）同族，但是**另一条线**：改它要动 TM 的规则顺序，留作后续。
* `''`（像）仍**没有缩写**——与 Lean 一致（上游也没有），直接打两个单引号。
* 上游表会漂移：本次取证日期 **2026-10-01**，`abbreviations.json` 最后改动
  `2026-09-30`（#797）。本仓**不 vendor** 上游文件（硬规则 2），表是手抄 + 本仓判据。
