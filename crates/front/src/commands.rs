//! 命令的**单一真相表**：关键字 · 文档文件名 · 一句话摘要 · 完整文档正文
//! （`reference/commands/*.md`）。
//!
//! 设计 `docs/design/command-docs.md`（与 `docs/design/tactic-docs.md` 同形）。
//! 与 [`crate::tactics`] 的分工：tactic 是 `by` 块里的关键字（进 AST、有语义种类、
//! 有"未知 tactic"文案），命令是**顶层命令字**（`#check` / `#reduce` / `#print` /
//! `set_option`）—— 它们是**叶子命令**、不进环境。
//!
//! **防漏项三层**（照 tactic 侧的四层机制，去掉不适用的那一层）：
//! * `markdown` 用 [`include_str!`] **编译期内嵌** ⇒ 表里有一行、文件不在 ⇒
//!   **编译失败** ✓（"加了命令忘了补文档"在构造上不可能）；
//! * `parser::is_reserved_command` **查本表**（[`is_command`]）⇒ 表里没有的 `#`
//!   命令 parser 根本不认 ⇒ 功能用不了 ✗，不是"能跑但没文档" ✓；
//! * 另有一条**编译之外**的文本守卫（`crates/front/tests/command_docs.rs` 的反向
//!   一条）：直接读 `parser.rs` 里 `is_reserved_command` 的 `#` 字面量与本表比对 ⇒
//!   "绕开 [`is_command`] 直接往那个 `matches!` 里加一条 `#eval`"照样判红 ✓。
//!
//! ⚠ **索引与正文共用一份摘要**：`summary` 同时是 `reference/commands/README.md`
//! 的索引行与每篇第 3 行（判据逐字比对）⇒ 漂移当场判红 ✓。

/// ② 档读的插件目录键（扩展侧写的**新增那一个键**，见 `editor/vscode/extension.js`）。
///
/// ⚠ 与 tactic 的 `SOKONANODA_DOCS_DIR` **分开**：那个键今天指 `<ext>/docs/tactics`，
/// 改成父目录会让既有 wire 判据（`navigation.rs` 的插件目录那一条）与 e2e P7 的
/// 路径断言同时失效 ✗ ⇒ 新能力**只加一个新键**（零破坏 ✓）。键名 ↔ 目录名的对账
/// 判据在 `crates/cli/tests/extension.rs`。
pub const PLUGIN_DOCS_DIR_ENV: &str = "SOKONANODA_COMMAND_DOCS_DIR";

/// 一条命令的全部真相。
///
/// **四个字段都有消费者，别省**：
/// * [`name`](Self::name) —— `is_reserved_command` 派生 + 文档落点 + 判据；
/// * [`stem`](Self::stem) —— 文件名（`#check` ⇒ `check.md`：`#` 不能进文件名）；
/// * [`summary`](Self::summary) —— 索引行 / 正文第 3 行（**逐字**，判据盯着）；
/// * [`markdown`](Self::markdown) —— 编译期内嵌（防漏项 + 物化用）。
#[derive(Debug, Clone, Copy)]
pub struct CommandDoc {
    /// 关键字（`#check` / `#reduce` / `#print` / `set_option`）。
    pub name: &'static str,
    /// 文档文件名（不含 `.md`）。
    pub stem: &'static str,
    /// 一句话摘要（与 `reference/commands/README.md` 的索引行**逐字**相同）。
    pub summary: &'static str,
    /// 完整文档正文（`reference/commands/<stem>.md`，编译期内嵌）。
    pub markdown: &'static str,
}

/// 本语言实现的**全部**命令（4 条）。
///
/// ⚠ 新增一条 = 加一行 + 建 `reference/commands/<stem>.md`（否则**编译不过**），
/// 并让 `crates/front/tests/command_docs.rs` 的判据与 `reference/commands/README.md`
/// 的索引行跟上（判据逐字比对 ⇒ 漏一处当场判红）✓。
///
/// **顺序即索引顺序**（`reference/commands/README.md` 那张表）。
pub static COMMAND_DOCS: &[CommandDoc] = &[
    CommandDoc {
        name: "#check",
        stem: "check",
        summary: "把表达式交给内核，印出它推出来的类型。",
        markdown: include_str!("../../../reference/commands/check.md"),
    },
    CommandDoc {
        name: "#reduce",
        stem: "reduce",
        summary: "把表达式交给内核算到底，印出归约后的值。",
        markdown: include_str!("../../../reference/commands/reduce.md"),
    },
    CommandDoc {
        name: "#print",
        stem: "print",
        summary: "印出环境里某条声明的类型与定义体（证明项也在内）。",
        markdown: include_str!("../../../reference/commands/print.md"),
    },
    CommandDoc {
        name: "set_option",
        stem: "set_option",
        summary: "调 `#print` / `#check` / `#reduce` 的显示档 —— 让隐式实参看得见。",
        markdown: include_str!("../../../reference/commands/set_option.md"),
    },
];

/// 按关键字取一条命令的真相。
pub fn doc(name: &str) -> Option<&'static CommandDoc> {
    COMMAND_DOCS.iter().find(|d| d.name == name)
}

/// 这个关键字是不是一条**命令**（`parser::is_reserved_command` 由它派生）。
pub fn is_command(name: &str) -> bool {
    doc(name).is_some()
}

/// 全部命令关键字（**表序** = 索引顺序）。
pub fn names() -> impl Iterator<Item = &'static str> {
    COMMAND_DOCS.iter().map(|d| d.name)
}

/// 命令文档的**落点**（五档解析，与 tactic 共用同一份机械 ——
/// [`crate::reference_docs::doc_path`]）。
///
/// 与 [`crate::tactics::doc_path`] 唯一的差别：子目录是 `commands`、插件目录键是
/// [`PLUGIN_DOCS_DIR_ENV`]。五档的语义、幂等性、"绝不编造位置"三条纪律见那份机械。
pub fn doc_path(name: &str) -> Option<std::path::PathBuf> {
    let doc = doc(name)?;
    crate::reference_docs::doc_path("commands", doc.stem, doc.markdown, PLUGIN_DOCS_DIR_ENV)
}
