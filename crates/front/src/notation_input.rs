//! 记法**输入法**表：符号 ↔ 缩写（`\and` → `∧`、`\alpha` → `α`）。
//!
//! 设计：`docs/design/notation-input.md`（2026-10-01 重写版：调研 + 差距 + 补全决策
//! D1–D7）。D5（2026-09-19 用户要求）是它的前身：「像 lean4 一样 `\xxx` 替换，
//! 同时 hover 内容提示用户如何输入对应符号」。
//!
//! **这张表是唯一真相源**：LSP 的 hover 文案从它渲染；VS Code 扩展的缩写改写器
//! 用一份 JS 镜像，`crates/cli/tests/extension.rs` 的契约测试钉住两边**逐条相等**
//! （与 `semantic::KEYWORDS` ↔ TM 语法同一形制）。
//!
//! 四条纪律：
//! 1. **键逐字照抄 Lean 4**（`leanprover/vscode-lean4` 的
//!    `lean4-unicode-input/src/abbreviations.json`，取证 2026-10-01）——硬规则 3：
//!    教学语法是真实 Lean 4 的子集，肌肉记忆要能迁移。收键口径见设计 D1。
//! 2. **单字母别名只收「希腊字母名」**（`\a`→α `\b`→β `\c`→χ `\e`→ε `\g`→γ `\m`→μ
//!    `\D`→Δ `\G`→Γ `\L`→Λ `\S`→Σ `\p`/`\P`→Π）：Lean 里指向逻辑/集合符号的单字母键
//!    （`\v`→∨ `\i`→∩ `\o`→∘ `\r`→→）**故意不收**——`\i` 给 ∩ 而 `\in` 给 ∈ 是反直觉的
//!    （设计 D3）。`\a` 与 `\and`/`\approx` 共存**不引入歧义**：改写器按「完整表词」
//!    两态口径走（还在敲字母时前缀不落定，见 `abbreviation-rewriter.js`）。
//! 3. **`supported` 只描述「语言今天有没有这个符号」**，不描述「这个文件里在不在
//!    作用域」。`∈ ⊆ ∪ ∩ \ ∅ …` 由课程库声明，只有 `import` 了才可用
//!    （记法随 `import` 传播）——hover 说「输入 `\in`」不等于「这个文件里能写
//!    `∈`」，作用域判断走 [`declared_notation_at`]（本文件的记法表）。
//! 4. **`notation_symbol` 把「记法符号」与「标识符」分开**（设计 D4）——见字段文档。

/// 一个符号的输入法条目。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotationInput {
    /// 符号本身（`∧`）。
    pub symbol: &'static str,
    /// 主缩写（不带 leader）：`and` 表示打 `\and`。
    pub abbreviation: &'static str,
    /// 其他别名（不含主缩写、不含 leader）。
    pub aliases: &'static [&'static str],
    /// 语言今天**有**这个符号吗。`false` ⇒ hover 不承诺可输入（改说"语言还没有
    /// 这个符号"），改写器也不该收它的缩写。
    pub supported: bool,
    /// 这个符号在语言里是**记法符号**（`Sym` token）还是**标识符 / 语法括号**。
    ///
    /// **希腊字母是后者**（设计 `docs/design/notation-input.md` D4）。三处消费口径
    /// 因此不同，混在一起会真坏：
    /// * [`merge_known`] **只喂记法符号**给词法——把 `α` 喂进去，
    ///   `tokenize_with_symbols` 会把它切成 `Sym("α")` ⇒ 变量 hover / `F12` /
    ///   rename 守卫（`references::cursor_is_on_notation`）当场全坏
    ///   （`α` 在课程里出现 1872 次，全是类型变量名）；
    /// * [`notation_symbol_chars`] 只着色记法符号（TM 的 `variables` 规则管标识符，
    ///   `α` 不该长成算子的颜色）；
    /// * LSP hover：记法符号说「记法符号 + 展开成什么」，标识符只说「怎么输入」。
    pub notation_symbol: bool,
}

/// 73 个符号的输入法表（设计 `docs/design/notation-input.md` §2/§3）：
/// 逻辑/集合 **18** + 希腊字母 **48**（大小写各 24）+ 课程库记法 **7**。
///
/// `''`（像）**不给缩写**——与 Lean 一致（直接打两个单引号），所以它不在表里；
/// hover 对它的说法写在 LSP 侧（"Lean 也没有缩写"）。
pub const TABLE: &[NotationInput] = &[
    NotationInput {
        symbol: "∧",
        abbreviation: "and",
        aliases: &["wedge"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∨",
        abbreviation: "or",
        aliases: &["vee"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "↔",
        abbreviation: "iff",
        aliases: &["leftrightarrow"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "¬",
        abbreviation: "not",
        aliases: &["neg"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "→",
        abbreviation: "to",
        aliases: &["imp"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∀",
        abbreviation: "forall",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∃",
        abbreviation: "exists",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
    // `≠` 曾标 `supported: false`（语言当时没有 `Ne`）；**L2.3 已落地**
    // （`Ne` 进 L1 prelude 的 B9 族，`≠` 进 `BUILTIN_NOTATIONS`）⇒ 翻 true。
    NotationInput {
        symbol: "≠",
        abbreviation: "ne",
        aliases: &["neq"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∈",
        abbreviation: "in",
        aliases: &["mem"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "⊆",
        abbreviation: "sub",
        aliases: &["subseteq"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∪",
        abbreviation: "cup",
        aliases: &["union"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∩",
        abbreviation: "cap",
        aliases: &["inter"],
        supported: true,
        notation_symbol: true,
    },
    // `\` 与 leader **同字符**：改写器只在「`\` + 字母且整词命中」时替换
    // （`\setminus` 命中、单个 `\` 不命中）⇒ 集合差不会被误伤。
    NotationInput {
        symbol: "\\",
        abbreviation: "setminus",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∅",
        abbreviation: "empty",
        aliases: &["emptyset"],
        supported: true,
        notation_symbol: true,
    },
    // **不能用 `\P`**：Lean 里 `\P` 是 Π，不是幂集。
    NotationInput {
        symbol: "𝒫",
        abbreviation: "powerset",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "ᶜ",
        abbreviation: "compl",
        aliases: &["complement"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "⁻¹'",
        abbreviation: "preim",
        aliases: &["preimage"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "×ˢ",
        abbreviation: "xs",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "α",
        abbreviation: "alpha",
        aliases: &["a"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "β",
        abbreviation: "beta",
        aliases: &["b", "be"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "γ",
        abbreviation: "gamma",
        aliases: &["g", "ga"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "δ",
        abbreviation: "delta",
        aliases: &["de"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ε",
        abbreviation: "epsilon",
        aliases: &["e", "ep", "eps"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ζ",
        abbreviation: "zeta",
        aliases: &["ze"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "η",
        abbreviation: "eta",
        aliases: &["et"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "θ",
        abbreviation: "theta",
        aliases: &["th"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ι",
        abbreviation: "iota",
        aliases: &["io"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "κ",
        abbreviation: "kappa",
        aliases: &["ka"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "λ",
        abbreviation: "lambda",
        aliases: &["la", "lamda", "lam", "fun"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "μ",
        abbreviation: "mu",
        aliases: &["m"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ν",
        abbreviation: "nu",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ξ",
        abbreviation: "xi",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ο",
        abbreviation: "omicron",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "π",
        abbreviation: "pi",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ρ",
        abbreviation: "rho",
        aliases: &["rh"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "σ",
        abbreviation: "sigma",
        aliases: &["si"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "τ",
        abbreviation: "tau",
        aliases: &["ta"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "υ",
        abbreviation: "upsilon",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "φ",
        abbreviation: "phi",
        aliases: &["ph", "straightphi"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "χ",
        abbreviation: "chi",
        aliases: &["c", "ch"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ψ",
        abbreviation: "psi",
        aliases: &["ps"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "ω",
        abbreviation: "omega",
        aliases: &["om"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Α",
        abbreviation: "Alpha",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Β",
        abbreviation: "Beta",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Γ",
        abbreviation: "Gamma",
        aliases: &["G"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Δ",
        abbreviation: "Delta",
        aliases: &["D"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ε",
        abbreviation: "Epsilon",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ζ",
        abbreviation: "Zeta",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Η",
        abbreviation: "Eta",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Θ",
        abbreviation: "Theta",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ι",
        abbreviation: "Iota",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Κ",
        abbreviation: "Kappa",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Λ",
        abbreviation: "Lambda",
        aliases: &["L", "Lamda"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Μ",
        abbreviation: "Mu",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ν",
        abbreviation: "Nu",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ξ",
        abbreviation: "Xi",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ο",
        abbreviation: "Omicron",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Π",
        abbreviation: "Pi",
        aliases: &["p", "P"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ρ",
        abbreviation: "Rho",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Σ",
        abbreviation: "Sigma",
        aliases: &["S"],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Τ",
        abbreviation: "Tau",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Υ",
        abbreviation: "Upsilon",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Φ",
        abbreviation: "Phi",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Χ",
        abbreviation: "Chi",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ψ",
        abbreviation: "Psi",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "Ω",
        abbreviation: "Omega",
        aliases: &[],
        supported: true,
        notation_symbol: false,
    },
    NotationInput {
        symbol: "≈",
        abbreviation: "approx",
        aliases: &["thickapprox"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "∘",
        abbreviation: "comp",
        aliases: &["circ"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "⁻¹",
        abbreviation: "inv",
        aliases: &["sy"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "•",
        abbreviation: "smul",
        aliases: &["bub", "bu"],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "⊕",
        abbreviation: "oplus",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "⋃₀",
        abbreviation: "sUnion",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
    NotationInput {
        symbol: "⋂₀",
        abbreviation: "sInter",
        aliases: &[],
        supported: true,
        notation_symbol: true,
    },
];

/// 语言**开箱可用**的记法符号字符（非 ASCII），供编辑器 TextMate 语法着色。
///
/// **这是单一真相源**：`editor/vscode/syntaxes/sokonanoda.tmLanguage.json` 的
/// `mathsymbols` 类必须与它**逐字相等**（守护测试
/// `crates/cli/tests/extension.rs::tm_grammar_math_symbols_follow_the_single_source`）。
///
/// 为什么需要它：TM 的符号类从前是**手写的码点范围**（`U+2200–22FF` +
/// `U+2A00–2AFF`），于是 `↔`(U+2194)、`¬`(U+00AC)、`𝒫`(U+1D4AB)、`ᶜ`(U+1D9C)、
/// `⁻¹'`(U+207B/00B9/0027)、`×ˢ`(U+00D7/02E2) **一律不着色**（设计
/// `docs/design/notation-input.md` 的 R-6）——课程里最常用的几个反而看不见。
///
/// `=` **不在**这里：它是 ASCII，归 TM 的 `operators` 规则（与 `->`/`=>` 同族）。
/// **希腊字母也不在**（D4）：它们是标识符，TM 的 `variables`/`constants` 规则管它们
/// ——把 `α` 染成算子的颜色是错的（已知限制：那两个类今天是 ASCII 的，见设计 §5）。
pub fn notation_symbol_chars() -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for symbol in crate::parser::builtin_notation_symbols() {
        out.extend(symbol.chars());
    }
    for entry in TABLE.iter().filter(|entry| entry.notation_symbol) {
        out.extend(entry.symbol.chars());
    }
    // `∀`/`∃` 是词法关键字与 binder 记法（`∃` 由课程库声明），但都是学习者
    // 天天要看的符号 ⇒ 一并着色。
    out.push('∀');
    out.push('∃');
    // `→` 是词法别名（不是记法），同样要着色。
    out.push('→');
    // `''`（像）在输入法表里没有条目（Lean 也没有缩写），但它是课程符号。
    out.push('\'');
    out.retain(|c| !c.is_ascii());
    out.sort_unstable();
    out.dedup();
    out
}

/// 查一个符号的输入法条目。
pub fn input_for(symbol: &str) -> Option<&'static NotationInput> {
    TABLE.iter().find(|entry| entry.symbol == symbol)
}

/// 查一个**缩写**（不带 leader）对应的符号。VS Code 改写器用（镜像侧同一份语义）。
pub fn symbol_for_abbreviation(abbreviation: &str) -> Option<&'static NotationInput> {
    TABLE
        .iter()
        .find(|entry| entry.abbreviation == abbreviation || entry.aliases.contains(&abbreviation))
}

/// hover 用的「怎么输入」一行（`None` = 表里没有这个符号，或语言还没有它）。
///
/// 文案契约（设计 §4）：表里 `supported: false` 的符号**不承诺**可输入。
pub fn input_hint(symbol: &str) -> Option<String> {
    let entry = input_for(symbol)?;
    if !entry.supported {
        return None;
    }
    let mut hint = format!("输入：`\\{}`", entry.abbreviation);
    if !entry.aliases.is_empty() {
        let aliases: Vec<String> = entry
            .aliases
            .iter()
            .map(|alias| format!("`\\{alias}`"))
            .collect();
        hint.push_str(&format!("（别名 {}）", aliases.join(" ")));
    }
    Some(hint)
}

/// 光标处那个**标识符型**表项（希腊字母）的「怎么输入」一行（设计 D5）。
///
/// 为什么单独要它：`α` 是**标识符**（D4），LSP 的 `notation_symbol_hover` 只认
/// `Sym` token ⇒ 它根本不会触发，用户反馈的「`α` 没有快捷输入」就死在这里。
///
/// 判据是**词法**的：用**不喂符号**的普通词法取覆盖 `offset` 的 token，只认
/// [`crate::TokenKind::Ident`]，且该标识符必须是表里 `notation_symbol: false`
/// 的条目。所以 `α` 查得到；`∈`（`Sym`）、`𝒫`（表里 `notation_symbol: true`，
/// 由 LSP 的 `notation_symbol_hover` 负责）查不到——两条路**不会重复**。
pub fn input_hint_at(text: &str, offset: usize) -> Option<String> {
    let tokens = crate::token::tokenize(text).ok()?;
    let token = tokens.iter().find(|token| {
        let start = token.span.start.offset;
        let end = token.span.end.offset;
        start <= offset && offset < end
    })?;
    let crate::TokenKind::Ident(name) = &token.kind else {
        return None;
    };
    let entry = input_for(name)?;
    if entry.notation_symbol {
        return None; // 记法符号走 LSP 的 `notation_symbol_hover`（那里有展开目标）
    }
    input_hint(name)
}

/// 光标处的**记法符号**（`(符号, 展开目标)`）——**不要求本文件声明过**。
///
/// 符号集 = 本文件声明的（[`crate::token::scan_notation_decls`]）+ 内建记法
/// （`∧ ∨ ↔ ¬`）+ **输入法表里 `notation_symbol: true` 的**（`∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ ⁻¹' ×ˢ`
/// 与课程库的 `≈ ∘ ⁻¹ • ⊕ ⋃₀ ⋂₀`，它们在课程里由 `lib/*` 声明、随 `import` 传播）。
/// 所以 import 进来的符号也能被认出来，而"这个文件里到底能不能用"是另一回事
/// （`declared_notation_at`）。**表里 `notation_symbol: false` 的（希腊字母）不进来**
/// ——它们是标识符，喂给词法会把 `α` 切成 `Sym`（D4）。
///
/// 展开目标只在**本文件声明**时给得出（词法扫描本文件）；import 来的符号目标
/// 在别的文件里，这里返回 `None`。
/// **符号表的唯一装配**（审计 #7(b)，2026-09-25 ✓）：把"词法内建 + `TABLE`"
/// 并进**已扫到的声明符号**里（去重、保持顺序 ✓）。
///
/// 这三步原来在本文件里**抄了四遍** ✗（`symbol_at` / `symbol_span_at` /
/// `known_symbols` / `symbol_occurrences` ✓），四份**逐字相同** ✓ ⇒ 收成一处 ✓。
/// **纯重构、零行为变化** ✓：判据 = 全语料 `--json` 逐字节对拍 + front 单测全绿 ✓。
///
/// ⚠ **只收本文件的这四份** ✗：`parser.rs` 那两处与 `semantic.rs:313-321` 面对的是
/// 不同输入、且规则**确实不同**（`semantic` 更严 ✓）⇒ 合并它们要先定设计 ✓
/// （台账 #7(a) ✓）。
fn merge_known(mut symbols: Vec<String>) -> Vec<String> {
    // 用**喂给词法**的那一份（`lexer_builtin_symbols`，剔除 `=`）：`=` 一旦进了
    // 最长匹配表，`=>` 会被吃成 `=` + `>`，整份源码分词就错了（R-2 的真因 ✓）。
    for symbol in crate::parser::lexer_builtin_symbols() {
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    for entry in TABLE.iter().filter(|entry| entry.notation_symbol) {
        let symbol = entry.symbol.to_string();
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    symbols
}

pub fn symbol_at(text: &str, offset: usize) -> Option<(String, Option<String>)> {
    let declared = crate::token::scan_notation_decls(text);
    let symbols: Vec<String> = declared.iter().map(|(symbol, _)| symbol.clone()).collect();
    // 用**喂给词法**的那一份（`lexer_builtin_symbols`，剔除 `=`）：`=` 一旦
    // 进了最长匹配表，`=>` 会被吃成 `=` + `>`，整份源码分词就错了（实测：
    // L1 prelude 的 `fun … => …` 当场解析失败）。`=` 本身仍认得出来——词法的
    // `'='` 分支**原生**产出 `Sym("=")`，不依赖符号表。
    let symbols = merge_known(symbols);
    let token = symbol_token_at(text, offset, &symbols)?;
    let crate::TokenKind::Sym(symbol) = &token.kind else {
        return None;
    };
    // 展开目标：本文件声明的从**声明行**拿（词法扫描）；内建记法（`=`/`∧`/…）
    // 在本文件里没有声明行 ⇒ 从内建表拿。import 来的（`∈`）两边都够不着 ⇒ `None`
    // （目标在别的文件里；hover 仍会说"这是个记法符号 + 怎么输入"）。
    let target = declared
        .into_iter()
        .find(|(name, _)| name == symbol)
        .and_then(|(_, target)| target)
        .or_else(|| {
            crate::parser::builtin_notation_target(symbol).map(|target| target.to_string())
        });
    Some((symbol.clone(), target))
}

/// 光标处那个**记法符号 token** 的 span（T-D17）。
///
/// 为什么单独要它：hover 的 `range` 以前是 `None`（客户端按"光标词"高亮，
/// 对 `∈` 这种单字符还行，对 `⁻¹'`/`×ˢ` 这种多字符符号就不准）。有了它，
/// hover 能给出**精确**的符号范围。判据走的是与 [`symbol_at`] **同一条**词法
/// 查找（共享 [`symbol_token_at`]），不会两边漂移。
pub fn symbol_span_at(text: &str, offset: usize) -> Option<crate::Span> {
    let declared = crate::token::scan_notation_decls(text);
    let symbols: Vec<String> = declared.iter().map(|(symbol, _)| symbol.clone()).collect();
    let symbols = merge_known(symbols);
    symbol_token_at(text, offset, &symbols).map(|token| token.span)
}

/// 光标落在哪个 token 上（`symbol_at` 与 `symbol_span_at` 共用的那一步）。
fn symbol_token_at(text: &str, offset: usize, symbols: &[String]) -> Option<crate::Token> {
    let tokens = crate::token::tokenize_with_symbols(text, symbols).ok()?;
    tokens
        .iter()
        .find(|token| {
            let start = token.span.start.offset;
            let end = token.span.end.offset;
            start <= offset && offset < end
        })
        .cloned()
}

/// 认得出一个文件里**所有已知记法符号**（本文件声明的 + 内建静态表）——
/// 给"把一个符号喂给词法"的调用方用（T-D24 的 `documentHighlight` 要按符号收
/// token，不喂的话 `∈`/`↔` 这种不在数学符号码点类里的会被切成 `Ident`）。
///
/// 为什么放在前端：**内建表归前端所有**（`parser::lexer_builtin_symbols` 与静态
/// `TABLE` 都是 crate-private），LSP 侧不该复制一份。
pub fn known_symbols(doc: &str) -> Vec<String> {
    let symbols: Vec<String> = crate::token::scan_notation_decls(doc)
        .into_iter()
        .map(|(symbol, _)| symbol)
        .collect();
    merge_known(symbols)
}

/// 符号在文本里的**每一处**（T-D24 的 `documentHighlight`/`references` 用它）。
///
/// **词法**答案，不掺语义：用 [`known_symbols`] 喂词法（不喂的话 `∈`/`↔` 这种不在
/// 数学符号码点类里的会被切成 `Ident`），再收所有 `Sym(symbol)` token 的 span。
/// **故意不用子串匹配**：那会把注释与字符串里的同形字符也算进来。
pub fn symbol_occurrences(doc: &str, symbol: &str) -> Vec<crate::Span> {
    let mut symbols = known_symbols(doc);
    if !symbols.iter().any(|s| s == symbol) {
        symbols.push(symbol.to_string());
    }
    let Ok(tokens) = crate::token::tokenize_with_symbols(doc, &symbols) else {
        return Vec::new();
    };
    tokens
        .iter()
        .filter(|tok| matches!(&tok.kind, crate::TokenKind::Sym(s) if s == symbol))
        .map(|tok| tok.span)
        .collect()
}

/// 光标是不是落在**记法声明的目标名**上（T-D50 / 缺口 G-37）。
///
/// 现场：`infixr:80 " '' " => Set.image` 里 `=>` 后面的 `Set.image` 是**普通
/// 引用**，但它在 AST 里**不是使用点**（没有 hover 行）⇒ `definition_at` 答不上
/// 来、ctrl+点击没反应（真 LSP 实测：五条声明的目标名 × {definition, hover,
/// documentHighlight} = 15 个请求全为 null）。
///
/// 判据是**词法**的：找到记法命令关键字（`infix*`/`prefix`/`postfix`/
/// `notation`/`binder_notation`），跳过紧随其后的**符号字符串**，再取**第一个
/// 标识符**——命令的形状就是这样（`keyword [:N] "sym" => Target`），所以不需要
/// 认 `=>` 这个 token（词法里它是 `=` + `>` 两个 token）。
pub fn notation_target_at(text: &str, offset: usize) -> Option<(String, crate::Span)> {
    const KEYWORDS: &[&str] = &[
        "infix",
        "infixl",
        "infixr",
        "prefix",
        "postfix",
        "notation",
        "binder_notation",
    ];
    // 喂符号：`''`/`⁻¹'` 这些不喂就切不出来（同 `symbol_at` 的理由）。
    let declared = crate::token::scan_notation_decls(text);
    let symbols: Vec<String> = declared.iter().map(|(symbol, _)| symbol.clone()).collect();
    let symbols = merge_known(symbols);
    let toks = crate::token::tokenize_with_symbols(text, &symbols).ok()?;
    let mut index = 0usize;
    while index < toks.len() {
        let is_keyword = matches!(
            &toks[index].kind,
            crate::TokenKind::Ident(name) if KEYWORDS.contains(&name.as_str())
        );
        if is_keyword {
            // 跳过关键字之后的**符号字符串**，再取第一个标识符 = 目标名。
            let mut cursor = index + 1;
            while cursor < toks.len() && !matches!(toks[cursor].kind, crate::TokenKind::Str(_)) {
                cursor += 1;
            }
            cursor += 1;
            while cursor < toks.len() {
                if let crate::TokenKind::Ident(name) = &toks[cursor].kind {
                    let span = toks[cursor].span;
                    if span.start.offset <= offset && offset < span.end.offset {
                        return Some((name.clone(), span));
                    }
                    break; // 这条命令的目标不是光标处，继续下一条
                }
                cursor += 1;
            }
            index = cursor;
            continue;
        }
        index += 1;
    }
    None
}

/// 同 [`symbol_at`]，但展开目标还能从**闭包里**找（T-D02）。
///
/// `symbol_at` 只看**本文件**的声明 + 内建 ⇒ **`import` 来的记法（`∈`/`⊆`）
/// 目标永远是 `None`**（声明在别的文件里，词法扫描够不着）。而 hover 要给
/// 学习者的"原始类型"（`Set.mem : …`）恰恰需要那个目标名。
///
/// `closure_srcs` = 拓扑序里**本文件之前**那些模块的源码（LSP 侧就是
/// `judge_prefix` 拼出来的那段）。目标按 `closure_srcs` 的顺序找**第一个**声明
/// ——与编译期"后声明的覆盖先声明的"不同，但 hover 只要一个名字来解释符号，
/// 而同一个符号在闭包里重复声明本来就会被 parser 报冲突。
pub fn symbol_at_with_sources(
    text: &str,
    offset: usize,
    closure_srcs: &[&str],
) -> Option<(String, Option<String>)> {
    let (symbol, target) = symbol_at(text, offset)?;
    if target.is_some() {
        return Some((symbol, target));
    }
    for src in closure_srcs {
        if let Some((_, Some(found))) = crate::token::scan_notation_decls(src)
            .into_iter()
            .find(|(name, target)| name == &symbol && target.is_some())
        {
            return Some((symbol, Some(found)));
        }
    }
    Some((symbol, None))
}

/// 光标处的**本文件声明的记法符号**：`(符号, 展开目标)`。
///
/// 为什么需要它：`semantic` 把**已声明**的记法符号归进 `SemanticKind::Keyword`
/// （与 `∀` 同族，见 `semantic.rs` 的 `TokenKind::Sym` 分支），于是 LSP 的
/// 「关键字不吐 hover」闸门会把它们一起吞掉——**本文件声明的符号 hover 完全
/// 静默**（实测：`⊗` 无反应，内建 `∧` 有反应，光标右移一格又有了）。
///
/// 判据是**词法**的（不依赖 parse 成功）：用本文件自己的符号表重新分词，找
/// 覆盖 `offset` 的 `TokenKind::Sym`。展开目标由 [`crate::token::scan_notation_decls`]
/// 的词法扫描给出（使用库记法的文件单文件 parse 会失败，不能靠 parse 拿）。
pub fn declared_notation_at(text: &str, offset: usize) -> Option<(String, Option<String>)> {
    if crate::token::scan_notation_decls(text).is_empty() {
        return None;
    }
    let (symbol, target) = symbol_at(text, offset)?;
    let declared = crate::token::scan_notation_decls(text);
    declared
        .iter()
        .any(|(name, _)| *name == symbol)
        .then_some((symbol, target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_self_consistent() {
        let mut symbols: Vec<&str> = Vec::new();
        let mut abbreviations: Vec<&str> = Vec::new();
        for entry in TABLE {
            assert!(
                !entry.abbreviation.is_empty(),
                "{} must have a primary abbreviation",
                entry.symbol
            );
            assert!(
                !symbols.contains(&entry.symbol),
                "duplicate symbol {}",
                entry.symbol
            );
            symbols.push(entry.symbol);
            for abbreviation in std::iter::once(&entry.abbreviation).chain(entry.aliases) {
                assert!(
                    !abbreviation.is_empty(),
                    "{} has an empty abbreviation",
                    entry.symbol
                );
                assert!(
                    abbreviation.chars().all(|c| c.is_ascii_alphabetic()),
                    "abbreviation `{abbreviation}` must be letters only (the rewriter keys on `\\` + letters)"
                );
                assert!(
                    !abbreviations.contains(abbreviation),
                    "abbreviation `{abbreviation}` is claimed twice"
                );
                abbreviations.push(abbreviation);
            }
        }
        assert_eq!(
            TABLE.len(),
            73,
            "the design's table has 73 entries (`''` has none; `⟨`/`⟩` 见下一片)"
        );
    }

    /// **`notation_symbol` 的口径**（设计 D4）：它决定三件事，所以必须与符号本身
    /// 对得上，不能随手填。
    #[test]
    fn only_notation_symbols_are_fed_to_the_lexer() {
        // 希腊字母是**标识符**：喂给词法会让 `α` 变成 `Sym("α")`。
        for (symbol, _) in [("α", ()), ("β", ()), ("Ω", ()), ("ω", ())] {
            assert!(
                !input_for(symbol).expect("in table").notation_symbol,
                "`{symbol}` is an identifier, not a notation symbol"
            );
        }
        // 记法符号（内建 + 课程库）必须喂。
        for symbol in ["∧", "∈", "𝒫", "≈", "∘", "•", "⋃₀"] {
            assert!(
                input_for(symbol).expect("in table").notation_symbol,
                "`{symbol}` is a notation symbol"
            );
        }
        // 词法装配：`α` 认得出来是**标识符**，`∈` 是 `Sym`。
        let src = "theorem t (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h";
        let offset = src.find('α').expect("α 在文本里");
        assert!(
            symbol_at(src, offset).is_none(),
            "`α` 是标识符 ⇒ 不该被认成记法符号（认了，变量 hover/F12/rename 全坏）"
        );
        let offset = src.find('∈').expect("∈ 在文本里");
        assert_eq!(symbol_at(src, offset).map(|(s, _)| s).as_deref(), Some("∈"));
    }

    /// 希腊字母**着色表里没有**（D4）：TM 的 `variables` 规则管标识符，`α` 不该
    /// 长成算子的颜色。
    #[test]
    fn greek_letters_are_not_coloured_as_math_symbols() {
        let chars = notation_symbol_chars();
        assert!(!chars.contains(&'α'), "`α` 是标识符，不进 mathsymbols 类");
        assert!(!chars.contains(&'Ω'), "`Ω` 是标识符，不进 mathsymbols 类");
        assert!(chars.contains(&'∈'));
        assert!(chars.contains(&'≈'), "课程库记法要着色");
        assert!(chars.contains(&'•'));
    }

    /// **标识符型表项的输入提示**（D5，用户反馈的直接判据）。
    #[test]
    fn an_identifier_in_the_table_teaches_how_to_type_it() {
        let src = "theorem t (α : Prop) (h : α) : α := h\n";
        let offset = src.find('α').expect("α 在文本里");
        let hint = input_hint_at(src, offset).expect("`α` 有缩写 ⇒ 要给提示");
        assert!(hint.contains("\\alpha"), "{hint}");
        assert!(hint.contains("\\a"), "单字母别名也要列：{hint}");

        // 记法符号走另一条路（`notation_symbol_hover`），这里**不许**重复给。
        let sym = "theorem t (a b : Prop) (h : a ∧ b) : a ∧ b := h\n";
        let offset = sym.find('∧').expect("∧ 在文本里");
        assert!(input_hint_at(sym, offset).is_none(), "记法符号不该走这条路");
        // `𝒫` 也是记法符号（课程库声明），哪怕它在词法里是标识符字符。
        let powerset = "theorem t (A : Set α) : A ∈ 𝒫 A := sorry\n";
        let offset = powerset.find('𝒫').expect("𝒫 在文本里");
        assert!(input_hint_at(powerset, offset).is_none(), "𝒫 是记法符号");
        // 表外标识符 / 标点：沉默（别编）。
        let plain = "theorem t (x : Prop) : x := x\n";
        assert!(input_hint_at(plain, plain.find('x').expect("x")).is_none());
        assert!(input_hint_at(plain, plain.find('(').expect("(")).is_none());
    }

    #[test]
    fn no_abbreviation_is_a_prefix_of_another() {
        // 前缀陷阱（设计 D2）：`\in` 是 `\inter` 的前缀。改写器的规则是
        // 「缩写**完整**且**不是更长缩写的前缀**」才即时替换——这条测试钉住
        // 「哪些对是前缀关系」，改表时不会不知不觉破坏那条规则的前提。
        let mut all: Vec<&str> = Vec::new();
        for entry in TABLE {
            all.push(entry.abbreviation);
            all.extend(entry.aliases.iter().copied());
        }
        let mut prefixed: Vec<(&str, &str)> = Vec::new();
        for short in &all {
            for long in &all {
                if short != long && long.starts_with(short) {
                    prefixed.push((short, long));
                }
            }
        }
        // 用户反馈的那条（`\a`）：它**必须**是前缀（`alpha`/`approx`/`and`…），
        // 否则「还在敲字母时不落定」这条规则就失去前提 ⇒ 改成断言它**在**。
        for pair in [
            ("in", "inter"),
            ("a", "alpha"),
            ("a", "approx"),
            ("a", "and"),
            ("b", "beta"),
            ("e", "epsilon"),
            ("g", "gamma"),
            ("m", "mu"),
            ("in", "inv"),
            ("c", "chi"),
            ("p", "pi"),
            ("P", "Pi"),
        ] {
            assert!(
                prefixed.contains(&pair),
                "the documented prefix trap {pair:?} must still exist: {prefixed:?}"
            );
        }
    }

    #[test]
    fn unsupported_symbols_do_not_promise_input() {
        // `≠` 曾经是 `supported: false`（语言没有 `Ne`）；L2.3 落地后翻 true。
        assert!(input_hint("≠").unwrap().contains("\\ne"));
        assert!(input_hint("∧").unwrap().contains("\\and"));
        assert!(input_hint("∈").unwrap().contains("\\in"));
        assert!(input_hint("∈").unwrap().contains("\\mem"));
        assert!(input_hint("𝒫").unwrap().contains("\\powerset"));
        assert!(input_hint("(").is_none(), "punctuation is not a notation");
    }

    #[test]
    fn abbreviations_round_trip_to_their_symbol() {
        for entry in TABLE {
            assert_eq!(
                input_for(entry.symbol).map(|e| e.symbol),
                Some(entry.symbol)
            );
            assert_eq!(
                symbol_for_abbreviation(entry.abbreviation).map(|e| e.symbol),
                Some(entry.symbol)
            );
            for alias in entry.aliases {
                assert_eq!(
                    symbol_for_abbreviation(alias).map(|e| e.symbol),
                    Some(entry.symbol),
                    "alias `{alias}` must resolve to {}",
                    entry.symbol
                );
            }
        }
    }

    #[test]
    fn a_locally_declared_symbol_is_found_at_its_own_offset() {
        let src = "def myop (a b : Prop) : Prop := a\ninfix:50 \" ⊗ \" => myop\ntheorem t (a b : Prop) (h : a ⊗ b) : a ⊗ b := h\n";
        let offset = src.rfind('⊗').expect("second use exists");
        let (symbol, target) = declared_notation_at(src, offset).expect("symbol at cursor");
        assert_eq!(symbol, "⊗");
        assert_eq!(target.as_deref(), Some("myop"));
    }

    #[test]
    fn a_keyword_or_plain_identifier_is_not_a_notation_symbol() {
        let src = "def myop (a b : Prop) : Prop := a\ninfix:50 \" ⊗ \" => myop\n";
        let offset = src.find("def").expect("`def` exists");
        assert!(declared_notation_at(src, offset).is_none());
        let offset = src.find("myop").expect("`myop` exists");
        assert!(declared_notation_at(src, offset).is_none());
    }

    #[test]
    fn an_imported_symbol_is_not_declared_in_this_file() {
        // 记法随 `import` 传播：使用处**本文件没有声明** ⇒ 这里返回 `None`
        // （作用域判断归这条），而 hover 的「怎么输入」仍由表回答。
        let src =
            "import lib.Set\n\ntheorem t (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n";
        let offset = src.rfind('∈').expect("use exists");
        assert!(declared_notation_at(src, offset).is_none());
        assert!(input_hint("∈").is_some());
    }
}

#[cfg(test)]
mod target_resolution_tests {
    use super::*;

    /// **T-D03 的契约**：解析不出 target 就返回 `None`（调用方据此**不编**那一行）。
    ///
    /// 这条为什么钉在**函数**层而不是 hover 层：在**能编译**的文件里，凡是认得出来
    /// 的记法符号都必有 target（本文件声明的 ✓ / 内建的 ✓ / `import` 来的 ✓），
    /// 所以"解析不出"在 LSP 那条路上**不可达**——它是**防御性**的（半成品文件、
    /// 表里有但没人声明的符号）。契约钉在这里，hover 那侧靠"那一行写在
    /// `if let Some(target)` 里"结构性保证。
    #[test]
    fn a_symbol_nobody_declares_has_no_target() {
        // `∈` 在**输入法表**里（有 `\in` 缩写）但这份文本既没声明它、也没 import
        // 声明它的库 ⇒ 符号认得出来、目标解析不出。
        let src = "theorem t (a b : Prop) : a ∈ b := sorry\n";
        let offset = src.find('∈').expect("符号在文本里");
        let (symbol, target) =
            symbol_at_with_sources(src, offset, &[]).expect("输入法表里的符号认得出来");
        assert_eq!(symbol, "∈");
        assert_eq!(target, None, "没人声明它 ⇒ 没有目标可给");

        // 对照：同一个符号，闭包里有人声明 ⇒ 目标就有了（T-D10 那条路）。
        let lib = "def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
                   infix:50 \" ∈ \" => Set.mem\n";
        let (symbol, target) = symbol_at_with_sources(src, offset, &[lib]).expect("认得出来");
        assert_eq!(symbol, "∈");
        assert_eq!(target.as_deref(), Some("Set.mem"), "闭包里有声明就能解析");
    }
}
