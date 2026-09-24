//! 记法**输入法**表：符号 ↔ 缩写（`\and` → `∧`）。
//!
//! 设计：`docs/design/notation-input.md`（D5，2026-09-19 用户要求「像 lean4 一样
//! `\xxx` 替换，同时 hover 内容提示用户如何输入对应符号」）。
//!
//! **这张表是唯一真相源**：LSP 的 hover 文案从它渲染；VS Code 扩展的缩写改写器
//! 用一份 JS 镜像，`crates/cli/tests/extension.rs` 的契约测试钉住两边**逐字相等**
//! （与 `semantic::KEYWORDS` ↔ TM 语法同一形制）。
//!
//! 三条纪律：
//! 1. **主缩写逐字照抄 Lean 4**（`leanprover/vscode-lean4` 的
//!    `lean4-unicode-input/src/abbreviations.json`）——硬规则 3：教学语法是真实
//!    Lean 4 的子集，肌肉记忆要能迁移。取证（带上游行号）在
//!    `docs/notes/course-lean-style/notation-input-plan.md` §2.1/§2.2。
//! 2. **单字母别名不抄**（`\i` `\v` `\r` `\l` `\a`）：它们是前缀陷阱，对学习者
//!    只有害处。多字母别名收不收见设计 §2.1（待拍板）。
//! 3. **`supported` 只描述「语言今天有没有这个符号」**，不描述「这个文件里在不在
//!    作用域」。`∈ ⊆ ∪ ∩ \ ∅ …` 由课程库声明，只有 `import` 了才可用
//!    （记法随 `import` 传播）——hover 说「输入 `\in`」不等于「这个文件里能写
//!    `∈`」，作用域判断走 [`declared_notation_at`]（本文件的记法表）。

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
}

/// 19 个符号的输入法表（设计 `docs/design/notation-input.md` §2）。
///
/// `''`（像）**不给缩写**——与 Lean 一致（直接打两个单引号），所以它不在表里；
/// hover 对它的说法写在 LSP 侧（"Lean 也没有缩写"）。
pub const TABLE: &[NotationInput] = &[
    NotationInput {
        symbol: "∧",
        abbreviation: "and",
        aliases: &["wedge"],
        supported: true,
    },
    NotationInput {
        symbol: "∨",
        abbreviation: "or",
        aliases: &["vee"],
        supported: true,
    },
    NotationInput {
        symbol: "↔",
        abbreviation: "iff",
        aliases: &["leftrightarrow"],
        supported: true,
    },
    NotationInput {
        symbol: "¬",
        abbreviation: "not",
        aliases: &["neg"],
        supported: true,
    },
    NotationInput {
        symbol: "→",
        abbreviation: "to",
        aliases: &["imp"],
        supported: true,
    },
    NotationInput {
        symbol: "∀",
        abbreviation: "forall",
        aliases: &[],
        supported: true,
    },
    NotationInput {
        symbol: "∃",
        abbreviation: "exists",
        aliases: &[],
        supported: true,
    },
    // `≠` 曾标 `supported: false`（语言当时没有 `Ne`）；**L2.3 已落地**
    // （`Ne` 进 L1 prelude 的 B9 族，`≠` 进 `BUILTIN_NOTATIONS`）⇒ 翻 true。
    NotationInput {
        symbol: "≠",
        abbreviation: "ne",
        aliases: &["neq"],
        supported: true,
    },
    NotationInput {
        symbol: "∈",
        abbreviation: "in",
        aliases: &["mem"],
        supported: true,
    },
    NotationInput {
        symbol: "⊆",
        abbreviation: "sub",
        aliases: &["subseteq"],
        supported: true,
    },
    NotationInput {
        symbol: "∪",
        abbreviation: "cup",
        aliases: &["union"],
        supported: true,
    },
    NotationInput {
        symbol: "∩",
        abbreviation: "cap",
        aliases: &["inter"],
        supported: true,
    },
    // `\` 与 leader **同字符**：改写器只在「`\` + 字母且整词命中」时替换
    // （`\setminus` 命中、单个 `\` 不命中）⇒ 集合差不会被误伤。
    NotationInput {
        symbol: "\\",
        abbreviation: "setminus",
        aliases: &[],
        supported: true,
    },
    NotationInput {
        symbol: "∅",
        abbreviation: "empty",
        aliases: &["emptyset"],
        supported: true,
    },
    // **不能用 `\P`**：Lean 里 `\P` 是 Π，不是幂集。
    NotationInput {
        symbol: "𝒫",
        abbreviation: "powerset",
        aliases: &[],
        supported: true,
    },
    NotationInput {
        symbol: "ᶜ",
        abbreviation: "compl",
        aliases: &["complement"],
        supported: true,
    },
    NotationInput {
        symbol: "⁻¹'",
        abbreviation: "preim",
        aliases: &["preimage"],
        supported: true,
    },
    NotationInput {
        symbol: "×ˢ",
        abbreviation: "xs",
        aliases: &[],
        supported: true,
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
pub fn notation_symbol_chars() -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for symbol in crate::parser::builtin_notation_symbols() {
        out.extend(symbol.chars());
    }
    for entry in TABLE {
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

/// 光标处的**记法符号**（`(符号, 展开目标)`）——**不要求本文件声明过**。
///
/// 符号集 = 本文件声明的（[`crate::token::scan_notation_decls`]）+ 内建记法
/// （`∧ ∨ ↔ ¬`）+ **输入法表里的**（`∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ ⁻¹' ×ˢ`，它们在课程里由
/// `lib/Set` 声明、随 `import` 传播）。所以 import 进来的符号也能被认出来，
/// 而"这个文件里到底能不能用"是另一回事（`declared_notation_at`）。
///
/// 展开目标只在**本文件声明**时给得出（词法扫描本文件）；import 来的符号目标
/// 在别的文件里，这里返回 `None`。
pub fn symbol_at(text: &str, offset: usize) -> Option<(String, Option<String>)> {
    let declared = crate::token::scan_notation_decls(text);
    let mut symbols: Vec<String> = declared.iter().map(|(symbol, _)| symbol.clone()).collect();
    // 用**喂给词法**的那一份（`lexer_builtin_symbols`，剔除 `=`）：`=` 一旦
    // 进了最长匹配表，`=>` 会被吃成 `=` + `>`，整份源码分词就错了（实测：
    // L1 prelude 的 `fun … => …` 当场解析失败）。`=` 本身仍认得出来——词法的
    // `'='` 分支**原生**产出 `Sym("=")`，不依赖符号表。
    for symbol in crate::parser::lexer_builtin_symbols() {
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    for entry in TABLE {
        let symbol = entry.symbol.to_string();
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
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
    let mut symbols: Vec<String> = declared.iter().map(|(symbol, _)| symbol.clone()).collect();
    for symbol in crate::parser::lexer_builtin_symbols() {
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    for entry in TABLE {
        let symbol = entry.symbol.to_string();
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
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
    let mut symbols: Vec<String> = declared.iter().map(|(symbol, _)| symbol.clone()).collect();
    for symbol in crate::parser::lexer_builtin_symbols() {
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    for entry in TABLE {
        let symbol = entry.symbol.to_string();
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
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
            18,
            "the design's table has 18 entries (`''` has none)"
        );
    }

    #[test]
    fn no_abbreviation_is_a_prefix_of_another() {
        // 前缀陷阱（设计 §3 方案 A）：`\in` 是 `\inter` 的前缀。改写器的规则是
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
        assert!(
            prefixed.contains(&("in", "inter")),
            "the documented prefix trap must still exist: {prefixed:?}"
        );
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
