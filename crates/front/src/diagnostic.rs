//! 解析期诊断：`DiagnosticKind`/`Diagnostic` 及其机器码与教学提示。

use crate::span::Span;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    UnexpectedEof,
    UnexpectedToken {
        found: String,
        expected: String,
    },
    /// `import` 写法不合法：缺名字、一行写了多个、或名字后面跟了别的东西。
    ImportMalformed {
        detail: String,
    },
    /// 模块名不满足标识符规则（`-`、数字开头、空分量…）。`hint` 由
    /// `project::ModuleNameError::hint` 提供，逐原因定制。
    ImportNotAModuleName {
        module: String,
        message: String,
        hint: &'static str,
    },
    /// `import` 出现在声明之后（官方 Lean 同款规则）。
    ImportMustPrecedeDeclarations,
    /// 字符串字面量没有闭合（记法命令的符号）。span 指向**开引号**。
    UnterminatedString,
    /// 记法命令形状不合法：缺优先级、缺 `=>`、符号是标识符词/空串、重复声明
    /// 同一符号。`detail` 是逐原因的正文（`docs/design/notation-subset.md` N1）。
    NotationShape {
        detail: String,
    },
    /// 用了**没有声明过**的记法符号（`a ∈ A` 但本文件里没有 `infix … " ∈ "`）。
    /// hint 给「先声明」与「点名写法」两条出路。
    NotationUnknownSymbol {
        symbol: String,
    },
    /// `end` 的名字与最近的未闭合 `namespace` 不同名（或根本没有可闭合的
    /// `namespace`）。G-05，设计 `docs/design/namespace-open.md` N1/N2。
    NamespaceMismatch {
        /// 写出来的 `end` 名字。
        found: String,
        /// 期望的名字（最近的未闭合 `namespace`）；没有时 `None`。
        expected: Option<String>,
    },
    /// 文件结束仍有未闭合的 `namespace`（span 指回那条 `namespace`）。
    NamespaceUnclosed {
        name: String,
    },
    /// `namespace` / `end` / `open` 的形状不合法（缺名字、名字不是标识符）。
    NamespaceShape {
        detail: String,
    },
    /// **集合字面量**（第三刀 §12.4）形状不合法：空 `{}`、三个及以上元素、
    /// 缺 `}`。`detail` 是逐原因的正文。
    SetLiteralShape {
        detail: String,
    },
    /// **集合建构式**（G-60，`docs/design/notation-subset.md` §19）形状不合法：
    /// ① `{x | P x}` —— **故意拒绝**（没有元变量 ⇒ `x` 的类型没有来源）；
    /// ② `{x ∈ A | …}` 里的关系符号不是 `∈`；③ 缺 `|` / 缺 `}`。
    /// `detail` 是逐原因的正文。
    ///
    /// **为什么不复用 `SetLiteralShape`**：`{x | P x}` 不是字面量，那句
    /// 「集合字面量要写成 {a} 或 {a, b}」**指错路** ✗；而且它的修法是「补一个
    /// 类型来源」，与字面量的修法（1–2 个元素）不是同一件事。
    SetBuilderShape {
        detail: String,
    },
    /// **`set_option`**（`docs/design/set-option-pp.md`）形状不合法：缺选项名、
    /// 选项名不在白名单（只 `pp.explicit` / `pp.all`）、值不是 `true`/`false`、
    /// 或写了本轮不做的 `… in …`。`detail` 是逐原因的正文。
    SetOptionShape {
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(kind: DiagnosticKind, span: Span, message: String) -> Self {
        Self {
            kind,
            span,
            message,
        }
    }

    /// Stable machine code for the diagnostic (used by `--json`).
    pub fn code(&self) -> &'static str {
        match self.kind {
            DiagnosticKind::UnexpectedEof => "unexpected-eof",
            DiagnosticKind::UnexpectedToken { .. } => "unexpected-token",
            DiagnosticKind::ImportMalformed { .. } => "import-malformed",
            DiagnosticKind::ImportNotAModuleName { .. } => "import-not-a-valid-module-name",
            DiagnosticKind::ImportMustPrecedeDeclarations => "import-must-precede-declarations",
            DiagnosticKind::UnterminatedString => "unterminated-string",
            DiagnosticKind::NotationShape { .. } => "notation-shape",
            DiagnosticKind::NotationUnknownSymbol { .. } => "notation-unknown-symbol",
            DiagnosticKind::NamespaceMismatch { .. } => "parse-namespace-mismatch",
            DiagnosticKind::NamespaceUnclosed { .. } => "parse-namespace-unclosed",
            DiagnosticKind::NamespaceShape { .. } => "parse-namespace-shape",
            DiagnosticKind::SetLiteralShape { .. } => "set-literal-shape",
            DiagnosticKind::SetBuilderShape { .. } => "set-builder-shape",
            DiagnosticKind::SetOptionShape { .. } => "set-option-shape",
        }
    }

    /// Pipeline stage of this diagnostic.
    pub fn stage_code(&self) -> &'static str {
        "parse"
    }

    /// First teaching hint for parse errors.
    pub fn hint(&self) -> &'static str {
        match &self.kind {
            DiagnosticKind::UnexpectedEof => {
                "输入到这里就结束了。检查是不是漏写了 `:=` 的值、右括号或 `end`。"
            }
            DiagnosticKind::UnexpectedToken { .. } => {
                "这里的写法不符合当前课程语法。检查命令拼写、括号配对，以及是否多写了还没学过的符号。"
            }
            DiagnosticKind::ImportMalformed { .. } => {
                "`import` 一行只写一个点分模块名（例如 `import Lesson.Logic`），并且必须写在文件最上方。"
            }
            DiagnosticKind::ImportNotAModuleName { hint, .. } => hint,
            DiagnosticKind::ImportMustPrecedeDeclarations => {
                "官方 Lean 的规则：`import` 必须写在**任何声明之前**。把这行移到文件开头。"
            }
            DiagnosticKind::UnterminatedString => {
                "字符串没有闭合：记法命令里的符号要写在一对引号之间，例如 infix:50 \" ∈ \" => Set.mem。"
            }
            DiagnosticKind::NotationShape { .. } => {
                "记法命令的形状是：infix:50 \" ∈ \" => Set.mem（infixl/infixr 同形，N 取 1–1000）；零元记法写 notation \"∅\" => Set.empty（不写优先级）。"
            }
            DiagnosticKind::NotationUnknownSymbol { .. } => {
                "这个符号还没有在本文件里声明过。先在它前面写一行记法命令（例如 infix:50 \" ∈ \" => Set.mem），或者改用点名写法（Set.mem α a A）。"
            }
            DiagnosticKind::NamespaceMismatch { .. } => {
                "`end` 的名字要与最近的 `namespace` 一模一样（`namespace Foo` 用 `end Foo` 收）。检查是不是写错了名字，或者中间少了/多了 `end`。"
            }
            DiagnosticKind::NamespaceUnclosed { .. } => {
                "这个 `namespace` 一直没有闭合。在文件末尾（或块结束处）补一行 `end <名字>`，名字与 `namespace` 那行相同。"
            }
            DiagnosticKind::NamespaceShape { .. } => {
                "作用域命令的形状是：namespace Foo（开块）、end Foo（收块，名字必须写出）、open Foo（短名可用，可加点分名字 A.B）。`open` 还能挑名字：open Foo (a b)（只要这两个）、open Foo hiding a b（挡掉这两个）、open Foo renaming a => b（改名），以及只影响一条命令的 open Foo in <命令>；export Foo 同形，但导入本文件的文件也看得到。"
            }
            DiagnosticKind::SetLiteralShape { .. } => {
                "集合字面量的形状是 {a}（单元素）或 {a, b}（两元素）：元素之间用 `,` 隔开，最后用 `}` 收尾；空集写 Set.empty α，三个及以上用 Set.pair 点名嵌套。"
            }
            DiagnosticKind::SetBuilderShape { .. } => {
                "集合建构式的形状是 `{x : α | P x}`（类型自己给）或 `{x ∈ A | P x}`（类型从 A 来）—— 变量与谓词之间用一个 `|` 隔开；`{x | P x}` 写不出来：本语言没有元变量，`x` 的类型没有来源。"
            }
            DiagnosticKind::SetOptionShape { .. } => {
                "显示选项的形状是：set_option pp.explicit true（单独一行，对本行之后的命令生效）。本语言只支持 pp.explicit 与 pp.all 两条名字，值只认 true / false；`set_option … in …` 这一轮不做。"
            }
        }
    }
}

// ── 源级渲染（R-07）：诊断里**绝不许**出现 Rust 的 `Debug` 串 ────────────────
//
// 病根：`parse_command` 的兜底臂把 `Token` 整串 `Debug` 拼进消息 ⇒ 学习者看到
// `expected a .sokonanoda command, found Token { kind: Ident("·"), span: Span {
// start: Pos { offset: 148, … } } }`（内部结构 + 字节偏移，没有一个字是他能用的）。
// 同族还有 `Ident("hax")` / `Eof` 这类 `TokenKind` 的 `Debug`（`{x:?}` 二十来处）。
//
// 做法照 Lean 4（`lean4/src/Lean/Parser/Module.lean::mkErrorMessage`）：把意外记号
// 渲染成**源级文本**（Lean 打 `unexpected token '·'` / `unexpected identifier`），
// 位置只进诊断的 `span` 字段，**不进消息** ✗。

/// **反向验证逃生门**（R-07）：`SOKO_NO_TOKEN_RENDER=1` ⇒ 退回旧的 `Debug`
/// 渲染（`Token { … }` / `Ident("…")` 回来）。只给判别性实验用，默认关闭。
fn legacy_debug_render() -> bool {
    std::env::var_os("SOKO_NO_TOKEN_RENDER").is_some()
}

/// [`TokenKind`] 的**源级文本**：学习者写下的那个记号（`` `·` `` / `` `:=` `` /
/// `` `∀` `` …）——不是 Rust 枚举形状。
pub(crate) fn token_kind_text(kind: &TokenKind) -> String {
    if legacy_debug_render() {
        return format!("{kind:?}");
    }
    match kind {
        TokenKind::Ident(name) => format!("`{name}`"),
        TokenKind::Num(value) => format!("`{value}`"),
        TokenKind::Str(text) => format!("`\"{text}\"`"),
        TokenKind::Sym(symbol) => format!("`{symbol}`"),
        TokenKind::Hole => "`_`".to_string(),
        TokenKind::Colon => "`:`".to_string(),
        TokenKind::ColonEq => "`:=`".to_string(),
        TokenKind::Arrow => "`->`".to_string(),
        TokenKind::Plus => "`+`".to_string(),
        TokenKind::Pipe => "`|`".to_string(),
        TokenKind::FatArrow => "`=>`".to_string(),
        TokenKind::Forall => "`∀`".to_string(),
        TokenKind::At => "`@`".to_string(),
        TokenKind::LParen => "`(`".to_string(),
        TokenKind::RParen => "`)`".to_string(),
        TokenKind::LBrace => "`{`".to_string(),
        TokenKind::RBrace => "`}`".to_string(),
        TokenKind::Langle => "`⟨`".to_string(),
        TokenKind::Rangle => "`⟩`".to_string(),
        TokenKind::Comma => "`,`".to_string(),
        TokenKind::Semicolon => "`;`".to_string(),
        TokenKind::Eof => "输入末尾".to_string(),
    }
}

/// [`Token`] 的源级文本（位置信息留在 `Diagnostic::span`，不进消息）。
///
/// ⚠ 逃生门开着时退回的是**整个 `Token`** 的 `Debug`（`Token { kind: …, span:
/// Span { … } }`）—— 那正是 R-07 之前的原样（老代码是 `{tok:?}` ✗）；只退回
/// `TokenKind` 的 `Debug` 会让「关掉开关 ⇒ 探针必须重新 exit 0」**测不出来**
/// （`Ident("·")` 里既没有 `Token {` 也没有 `Span {`）✓。
pub(crate) fn token_text(tok: &Token) -> String {
    if legacy_debug_render() {
        return format!("{tok:?}");
    }
    token_kind_text(&tok.kind)
}

/// **命令位置**上的 Lean 常见写法 → 一句「错在哪 + 改成什么」。
///
/// 为什么单独有这一层：`by` 块**只在 `crate::tactics::TACTIC_DOCS` 白名单里认
/// tactic**（`parser::is_tactic_keyword`），所以学习者照 Lean 写 `·` 子弹、`case`
/// 分支标签或 `simp` 这类本子集没有的 tactic 时，`by` 块**当场结束**、报错点落到
/// **下一条命令**上 —— 消息必须把「你还在写 Lean、这里已经出了 `by` 块」说清楚，
/// 否则学习者会去改一条**没错的**声明 ✗。
pub(crate) fn command_position_note(kind: &TokenKind) -> Option<String> {
    if legacy_debug_render() {
        return None;
    }
    let word = match kind {
        TokenKind::Ident(word) => word.as_str(),
        // `⋅`（U+22C5）不是 Lean 的子弹（子弹是 `·` U+00B7），但两者常被混用
        // ⇒ 与 `·` 共用同一句提示。
        TokenKind::Sym(symbol) if symbol == "⋅" => "·",
        _ => return None,
    };
    match word {
        "·" => Some(
            "`·` 是 Lean 的子弹（`constructor` 之后给每个子目标分组的记号），本子集没有它：\
             把 `·` 那一行删掉，在 `by` 块里直接依次写各子目标的 tactic（例如 `constructor` \
             之后写 `exact hp`、再写 `exact hq`）。"
                .to_string(),
        ),
        "case" | "next" => Some(format!(
            "`{word}` 是 Lean 的分支标签（给某个子目标命名），本子集没有它：\
             要么在 `cases … with` 的分支里直接写 `| 构造子 名字… => tactic`，\
             要么把该子目标的 tactic 直接写进 `by` 块。"
        )),
        _ if !crate::tactics::is_tactic(word) && is_lean_common_tactic(word) => Some(format!(
            "`{word}` 不在本子集的 tactic 白名单里（只有 {}）—— `by` 块遇到不属于它的记号就结束，\
             所以这条错误报在**下一条命令**的位置上（不是上面那条声明写错了）。",
            crate::tactics::whitelist_text()
        )),
        _ => None,
    }
}

/// Lean 4 里常见、**本子集故意没做**的 tactic 关键字（只用于提示，不参与判定）。
///
/// ⚠ 它与 [`crate::tactics::TACTIC_DOCS`] 的关系是**单向**的：这里只是"学习者可能
/// 从 Lean 带过来"的名字，命中后还要过 `!is_tactic(word)` 闸门 ⇒ 哪天某条真进了
/// 白名单，提示自动闭嘴 ✓（不会说"本语言没有 `simp`" ✗）。
fn is_lean_common_tactic(word: &str) -> bool {
    matches!(
        word,
        "intros"
            | "simp"
            | "simpa"
            | "simp_all"
            | "rw"
            | "rewrite"
            | "induction"
            | "omega"
            | "linarith"
            | "nlinarith"
            | "positivity"
            | "ring"
            | "ring_nf"
            | "field_simp"
            | "norm_num"
            | "subst"
            | "contradiction"
            | "by_contra"
            | "by_cases"
            | "contrapose"
            | "push_neg"
            | "specialize"
            | "decide"
            | "tauto"
            | "aesop"
            | "trivial"
            | "exact_mod_cast"
            | "interval_cases"
            | "funext"
            | "ext"
            | "refine"
            | "convert"
            | "show"
            | "change"
            | "conv"
            | "obtain"
            | "rcases"
            | "gcongr"
            | "norm_cast"
    )
}

pub type Result<T> = std::result::Result<T, Diagnostic>;

// ── r17（2026-10-12）：常见 Lean 语法被拒时**点名 + 给替代** ──────────────────
//
// 病：从任何 Lean 教程过来的学习者先写 `intros` / `simp` / `rw [X]`，而工具侧
// 只说「不在白名单里」（`command_position_note`）或干脆是词法兜底那句
// `expected a valid .sokonanoda token, found [`（parser 的注解**没机会说话** ✗）
// ⇒ 学习者分不清"语法没实现"和"我写错了"。
//
// 做法照 Lean 4（`lean4/src/Lean/Parser/Module.lean::mkErrorMessage`）：**把意外
// 记号名字化**（`unexpected token '·'`）并把它变成一句可行动的提示。我们多给一层
// ——「本子集不支持 X，改用 Y」；替代项只写**白名单里真有**的那个（`is_tactic`
// 现查）⇒ 哪天 `simp` 真做出来了，这里的"本子集没有"自动闭嘴 ✓（不会说谎 ✗）。

/// **反向验证逃生门**（r17）：`SOKO_NO_LEAN_LESSON=1` ⇒ 这一轮的两句教学提示
/// （tactic 名字化 + `[ … ]` 名字化）全部闭嘴 ⇒ 回到改动前的诊断
/// （`scripts/ux-probes/lang/r17-common-lean-syntax.sh` 回到 exit 0）。
/// 只给判别性实验用，默认关闭。
fn legacy_no_lean_lesson() -> bool {
    std::env::var_os("SOKO_NO_LEAN_LESSON").is_some()
}

/// 常见 Lean 4 tactic → 本子集的出路（`None` = 本子集**没有**对应物，那就说清楚
/// 「这一步得自己给证明项」）。**唯一真相**：`is_lean_common_tactic` 的名单只决定
/// "值不值得解释"，这里决定"怎么解释"。
fn lean_tactic_replacement(word: &str) -> Option<&'static str> {
    Some(match word {
        "intros" => "intro",
        // Lean 里 `rw`/`rewrite` 的活在本子集是"把那条引理显式用出来"：
        // `unfold Foo at h`（定义展开）+ `exact` / `apply`（把引理交出去）。
        "rw" | "rewrite" => "unfold",
        _ => return None,
    })
}

/// 一句「本子集**不支持** X（不是写错了）+ 你能写什么」。
///
/// 只对**本子集确实没有**的 Lean 常见写法开口（`!is_tactic` 闸门与
/// [`command_position_note`] 同款）⇒ 表里哪天收了一条，这里自动闭嘴 ✓。
pub(crate) fn unsupported_lean_tactic_lesson(word: &str) -> Option<String> {
    if legacy_no_lean_lesson() || crate::tactics::is_tactic(word) || !is_lean_common_tactic(word) {
        return None;
    }
    let alternative = lean_tactic_replacement(word);
    // 替代项必须在白名单里（否则就是"指了一条也不通的路"✗）。查不到就退回
    // "本子集没有它"这一半——那半永远成立 ✓。
    let advice = match alternative.filter(|name| crate::tactics::is_tactic(name)) {
        Some(name) => format!("请改用 `{name}`"),
        None => "本子集没有对应物，这一步要自己给出证明项（例如手里有 `h : P` 而目标就是 `P`，写 `exact h`）".to_string(),
    };
    Some(format!(
        "`{word}` 是 Lean 的 tactic，**本子集不支持**（不是你把名字写错了）：{advice}。\
         本子集的 tactic 只有 {}。",
        crate::tactics::whitelist_text()
    ))
}

/// **记法样式的 `[ … ]`**（r17 第二层）：`rw [Set.mem_singleton_iff]` 里的 `[`
/// 在**词法**层就没有 token（它不是数学符号码点）⇒ parser 的注解没机会说话
/// （实测报 `expected a valid .sokonanoda token, found [`）✗。
///
/// ⇒ 在词法的兜底臂上把它名字化并给一句出路（这正是 `mkErrorMessage` 的名字化
/// 那一步，只是我们的兜底臂在 lexer 里）。
pub(crate) fn unsupported_bracket_symbol(c: char) -> Option<String> {
    if legacy_no_lean_lesson() {
        return None;
    }
    let what = match c {
        '[' => "列表式语法 `[ … ]`",
        ']' => "列表式语法 `… ]`",
        _ => return None,
    };
    Some(format!(
        "{what} 是 Lean 的写法（`rw [lemma]` / `simp [lemma]`），**本子集不支持**：\
         引理要显式用出来——`unfold Foo at h` 展开定义、`exact` / `apply` 交出去。"
    ))
}

#[cfg(test)]
mod unsupported_lean_tactic_lesson_tests {
    use super::*;

    /// 判据（r17）：三种写法都要有「不支持」+ 出路，且**不许**说白名单里有的
    /// tactic 不被支持（那句必须在构造上不可能 ✓）。
    #[test]
    fn unsupported_tactics_all_get_a_lesson() {
        for word in ["intros", "simp", "rw", "rewrite", "omega", "linarith"] {
            let lesson = unsupported_lean_tactic_lesson(word)
                .unwrap_or_else(|| panic!("`{word}` 是本子集没有的常见 tactic，必须给一句提示"));
            assert!(lesson.contains("不支持"), "{word}: {lesson}");
            assert!(
                lesson.contains("请改用") || lesson.contains("本子集没有对应物"),
                "{word}: {lesson}"
            );
        }
        // 替代项指错了就是"指了一条不通的路"✗：`intros` 必须指向 `intro`。
        let intros = unsupported_lean_tactic_lesson("intros").unwrap();
        assert!(intros.contains("请改用 `intro`"), "{intros}");
        // **白名单里真有的**一律不许多嘴（`intro` 是支持的）。
        assert!(unsupported_lean_tactic_lesson("intro").is_none());
        assert!(unsupported_lean_tactic_lesson("exact").is_none());
        // 不认识的名字也不许多嘴（那时正确答案是"未知 tactic"，见 parser）。
        assert!(unsupported_lean_tactic_lesson("hax").is_none());
    }

    /// 第二层：`[` 要名字化 + 给出路；其它字符不许被这条抢走。
    #[test]
    fn bracket_symbol_gets_a_lesson_but_others_do_not() {
        let lesson = unsupported_bracket_symbol('[').expect("`[` 要有提示");
        assert!(lesson.contains("本子集不支持"), "{lesson}");
        assert!(lesson.contains("`rw [lemma]`"), "{lesson}");
        assert!(unsupported_bracket_symbol('*').is_none());
    }
}
