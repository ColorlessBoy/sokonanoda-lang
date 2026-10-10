//! tactic 的**单一真相表**：关键字 · 一句话摘要 · AST 落点 · 编辑器归类 · 完整文档正文。
//!
//! 设计 `docs/design/tactic-docs.md`（§4.2 D3 的"四层防漏项机制"）。本表是 tactic
//! 的**第 ⑥ 份也是最后一份**表示，另五份（`Tactic` 枚举 · `Tactic::span()` 的穷尽臂 ·
//! parser 白名单 · `semantic::KEYWORDS` · `proof::render_tactic`）里的
//! **③ 白名单已改为查本表**（`parser::is_tactic_keyword` ⇒ [`is_tactic`]），
//! 于是"加了 tactic 却忘了补文档"**在构造上不可能**：
//!
//! * `parse_by_block` 只在 `tactic_keyword_ahead()` 为真时解析 tactic，而它走同一个
//!   [`is_tactic`] ⇒ **表里没有的关键字根本不被当成 tactic**（`by` 块直接收空、
//!   随后按普通表达式解析而报错）⇒ 功能用不了 ✗，不是"能跑但没文档" ✓；
//! * `markdown` 用 [`include_str!`] **编译期内嵌** ⇒ 表里有一行、文件不在 ⇒ **编译失败** ✓；
//! * 表里每条都随 `TACTIC_DOCS` 一起接受 `crates/front/tests/tactic_docs.rs` 的
//!   五条判据（手写清单双向 · `KEYWORDS` 具名例外 · "未知 tactic"文案含全 14 条 ·
//!   文件与 6 个小节齐备 · 索引逐字一致）✓；
//! * 另有一条**编译之外**的独立守卫 `scripts/tactic-docs-lint.py`（L4）：
//!   它直接读 `parser.rs` 里 `parse_tactic_inner` 的 `kw == "…"` 字面量与本表比对 ⇒
//!   "绕开 [`is_tactic`] 直接加 parser 臂"那种写法照样判红 ✓。
//!
//! **F12 落点**走 [`doc_path`] 的五档解析（设计 §4.9 D10：仓库真源 → 插件目录 →
//! 缓存物化 → 临时物化 → `None`）⇒ 开发树与发布产物都能落到一篇**真 `.md`** ✓。
//!
//! ⚠ **顺序即 [`whitelist_text`] 的顺序**（"未知 tactic"文案照它拼）—— 改动顺序会改
//! 用户可见的文案，判据 `unknown_tactic_message_lists_every_tactic` 盯着它 ✓。
//!
//! ⚠ **`sorry` 是具名例外**（[`TacticDoc::semantic_kind`]）：它在白名单里（是真 tactic），
//! 但**故意**被编辑器件归成 [`SemanticKind::Hole`] 而不是 `Keyword`
//! （`crates/front/src/semantic.rs:866-867`，判据在 `semantic.rs:999`/`:1419`）⇒
//! `KEYWORDS` 的判据**必须**按字段分流，**不许**写成"跳过所有不在 `KEYWORDS` 里的项" ✗
//! （那等于把判据阉掉）。

use crate::semantic::SemanticKind;

/// 一条 tactic 的全部真相。
///
/// **五个字段都有消费者，别省**（设计 §4.2）：
/// * [`name`](Self::name) —— 白名单派生 + "未知 tactic"文案 + 判据；
/// * [`summary`](Self::summary) —— hover 摘要 / 补全 `detail` / 索引 / 文档首句（§4.5 定稿）；
/// * [`variant`](Self::variant) —— 判据与"覆盖基线"对账（`match` 写 `"Exact"`）；
/// * [`semantic_kind`](Self::semantic_kind) —— `KEYWORDS` 判据的**具名例外**；
/// * [`markdown`](Self::markdown) —— 编译期内嵌（L2）+ F12 落点（P6）。
#[derive(Debug, Clone, Copy)]
pub struct TacticDoc {
    /// 关键字（= 设计 §1.1 基线表的"关键字"列）。
    pub name: &'static str,
    /// 一句话摘要（§4.5 定稿，**逐字**；索引与文档首句都取它）。
    pub summary: &'static str,
    /// AST 落点名（`match` **没有自己的变体**，写 `"Exact"` —— 它解析成
    /// `Tactic::Exact { expr: Expr::Match }`）。
    pub variant: &'static str,
    /// 编辑器把它归成什么：13 条是 `Keyword`，**`sorry` 是 `Hole`**（具名例外）。
    pub semantic_kind: SemanticKind,
    /// 完整文档正文（`reference/tactics/<name>.md`，编译期内嵌）。
    pub markdown: &'static str,
}

/// 本语言实现的**全部** tactic（14 条）。
///
/// ⚠ 新增一条 = 加一行 + 建 `reference/tactics/<name>.md`（否则**编译不过**），
/// 并让 `crates/front/tests/tactic_docs.rs` 的手写清单与 `scripts/tactic-docs-lint.py`
/// 一起跟上 ✓ —— 两条判据会分别从"表 ↔ 手写清单"和"表 ↔ parser 字面量"两个方向咬 ✓。
pub static TACTIC_DOCS: &[TacticDoc] = &[
    TacticDoc {
        name: "intro",
        summary: "引入假设：目标 `A → B`（或 `∀ x, …`）时，先假设 `A`，再证 `B`。",
        variant: "Intro",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/intro.md"),
    },
    TacticDoc {
        name: "exact",
        summary: "交出证明项 `e`；内核判 `e` 的类型与当前目标是否定义相等（defeq）。",
        variant: "Exact",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/exact.md"),
    },
    TacticDoc {
        name: "apply",
        summary: "用一条函数的结论对上目标，它剩下的前提各自变成新目标。",
        variant: "Apply",
        semantic_kind: SemanticKind::Keyword,
        // ⚠ 行为正在被修复线 `d9a701e4` 改动（设计 §1.3 的快照规则 / §7 边界 9）：
        // 这一句刻意写成**机制级**，不写"不做定义展开"这类能力边界 ⇒ 那条线落地后仍成立 ✓。
        markdown: include_str!("../../../reference/tactics/apply.md"),
    },
    TacticDoc {
        name: "assumption",
        summary: "在已有假设里找一条与目标定义相等的，直接结束当前目标。",
        variant: "Assumption",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/assumption.md"),
    },
    TacticDoc {
        name: "rfl",
        summary: "自反：目标是 `=` 或 `↔`，且两边定义相等时成立。",
        variant: "Rfl",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/rfl.md"),
    },
    TacticDoc {
        name: "match",
        summary: "对项做情形分析，臂体写的是项（等价于 `exact (match …)`）。",
        // `match` 是**唯一**"关键字 ≠ AST 变体"的一条：它解析成
        // `Tactic::Exact { expr: Expr::Match }`（`parser.rs` 的 match 臂），
        // 所以变体名与关键字不同名 —— 判据与 L4 都按**关键字**取值，不看变体名 ✓。
        variant: "Exact",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/match.md"),
    },
    TacticDoc {
        name: "constructor",
        summary: "按目标取第一个构造子，它的参数变成新目标。",
        variant: "Constructor",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/constructor.md"),
    },
    TacticDoc {
        name: "left",
        summary: "选目标归纳的第一个构造子（`A ∨ B` 上就是左边 `A`）。",
        variant: "Left",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/left.md"),
    },
    TacticDoc {
        name: "right",
        summary: "选目标归纳的第二个构造子（`A ∨ B` 上就是右边 `B`）。",
        variant: "Right",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/right.md"),
    },
    TacticDoc {
        name: "use",
        summary: "对 `∃ x, p x` 交出证人 `w`，接着去证 `p w`。",
        variant: "Use",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/use.md"),
    },
    TacticDoc {
        name: "exfalso",
        summary: "把当前目标换成 `False`；原来要证的东西留到后面用。",
        variant: "Exfalso",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/exfalso.md"),
    },
    TacticDoc {
        name: "cases",
        summary: "对假设做情形分析，每个构造子一个分支。",
        variant: "Cases",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/cases.md"),
    },
    TacticDoc {
        name: "have",
        summary: "在证明中间先证一条 `h : T`，当前目标不变。",
        variant: "Have",
        semantic_kind: SemanticKind::Keyword,
        markdown: include_str!("../../../reference/tactics/have.md"),
    },
    TacticDoc {
        name: "sorry",
        summary: "占位：目标保持开放。练习没做完的合法状态，不是错误。",
        variant: "Sorry",
        // ⚠ **具名例外**：`sorry` 在 `by` 白名单里，但编辑器把它归成 `Hole`
        // 而不是 `Keyword`（`semantic.rs:866-867`）⇒ `KEYWORDS` 的判据按本字段分流 ✓。
        semantic_kind: SemanticKind::Hole,
        markdown: include_str!("../../../reference/tactics/sorry.md"),
    },
];

/// 这个关键字是不是 tactic？（**白名单的唯一实现** —— `parser::is_tactic_keyword` 走它。）
pub fn is_tactic(name: &str) -> bool {
    TACTIC_DOCS.iter().any(|t| t.name == name)
}

/// 取一条 tactic 的真相（`F12` / hover 摘要 / 补全用）。
pub fn doc(name: &str) -> Option<&'static TacticDoc> {
    TACTIC_DOCS.iter().find(|t| t.name == name)
}

/// 一篇 tactic 文档的**真实文件路径**（`F12` 落点；设计 §4.9 D10 的**五档解析**）。
///
/// 顺序（**每一档都过 `is_file()`** —— 目录在而文件缺 ⇒ 继续往下走，不是死指针 ✗）：
///
/// | 档 | 来源 | 谁走这一档 |
/// |---|---|---|
/// | ① | 仓库真源 `<仓库>/reference/tactics/<name>.md` | 开发树 / CI |
/// | ② | `$SOKONANODA_DOCS_DIR/<name>.md`（扩展指到插件自带的 `docs/tactics`） | 安装形态 |
/// | ③ | 缓存物化 `<cache::root()>/reference/tactics/<name>.md` | CLI / opencode / DSH |
/// | ④ | 临时物化 `<temp_dir>/sokonanoda-reference/tactics/<name>.md` | 缓存不可写 |
/// | ⑤ | `None` | 五档都不成立 ⇒ 答"没有落点" |
///
/// ⚠ **① 是编译期路径**（`env!("CARGO_MANIFEST_DIR")` 烘的是**编译机**的路径）⇒
/// 只在开发树/CI 上存在；发布产物（VSIX / 缓存二进制）在用户机器上 `is_file()`
/// 为假 ⇒ 天然落到 ②（插件目录）或 ③（物化）✓ —— 这正是 `prelude_source_path`
/// 的既有形态（`compile/prelude.rs:499`，同一门纪律）。
///
/// ③④ 与 prelude 同款：**幂等**（读出来与 [`TacticDoc::markdown`] 一致就不重写，
/// 免得每次 F12 动一次 mtime 让编辑器反复重载）、写不进就试下一档、两档都写不进
/// ⇒ `None`（**绝不编造位置** ✗）。`sokonanoda clean` 只删 `<root>/compiled/*.json`
/// （`cache::clean_in`）⇒ **不动这些文档**（与平级的 `<root>/prelude/Prelude.sokonanoda`
/// 同待遇）✓。
///
/// 落点必须是**磁盘上的真 `.md`**：本仓只有 `file:` 一种 URI 形态
/// （零 `TextDocumentContentProvider`、零自定义 scheme）✓。
pub fn doc_path(name: &str) -> Option<std::path::PathBuf> {
    let doc = doc(name)?;
    let file_name = format!("{name}.md");
    // ① 仓库真源（开发树首选：改文档立即生效，不必重打包）。
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("reference")
        .join("tactics")
        .join(&file_name);
    if repo.is_file() {
        return Some(repo);
    }
    // ② 插件目录（安装形态：扩展把 `SOKONANODA_DOCS_DIR` 指到 `<extension>/docs/tactics`）。
    if let Some(dir) = std::env::var_os("SOKONANODA_DOCS_DIR") {
        let plugin = std::path::PathBuf::from(dir).join(&file_name);
        if plugin.is_file() {
            return Some(plugin);
        }
    }
    // ③ 缓存物化 → ④ 临时物化（照 `prelude_source_path` 的三段兜底，少一段仓库）。
    let candidates = crate::compile::cache::root()
        .map(|root| root.join("reference").join("tactics").join(&file_name))
        .into_iter()
        .chain(std::iter::once(
            std::env::temp_dir()
                .join("sokonanoda-reference")
                .join("tactics")
                .join(&file_name),
        ));
    for path in candidates {
        if matches!(std::fs::read_to_string(&path), Ok(old) if old == doc.markdown) {
            return Some(path);
        }
        let written = path
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .ok()
            .flatten()
            .and_then(|()| std::fs::write(&path, doc.markdown).ok());
        if written.is_some() {
            return Some(path);
        }
    }
    // ⑤ 五档都不成立 ⇒ 没有落点（编辑器答"没有定义"，比给死指针好 ✓）。
    None
}

/// 全部关键字（**表序**，即 [`whitelist_text`] 的顺序）。
pub fn names() -> impl Iterator<Item = &'static str> {
    TACTIC_DOCS.iter().map(|t| t.name)
}

/// "未知 tactic" 文案用的白名单文本：`intro / exact / … / sorry`。
///
/// **由表拼出**（不许在 `parser.rs` 里再手写一份 ✗）—— 那正是 2026-10-10 抓到的那条
/// 用户可见缺陷：手写的那份**漏了 `have`**、且全仓零判据覆盖 ⇒ 学习者打错一条 tactic
/// 时看到的"支持清单"是不完整的 ✗。现在它随表走，判据
/// `unknown_tactic_message_lists_every_tactic` 盯着 ✓。
pub fn whitelist_text() -> String {
    names().collect::<Vec<_>>().join(" / ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 表本身的两条最小不变式（**不替代** `crates/front/tests/tactic_docs.rs` 的五条判据，
    /// 只是让本模块的自测能独立咬住"手滑"）。
    #[test]
    fn the_table_has_no_duplicate_and_no_empty_entry() {
        assert_eq!(TACTIC_DOCS.len(), 14, "tactic 基线是 14 条（设计 §1.1）");
        for t in TACTIC_DOCS {
            assert!(!t.name.is_empty() && !t.summary.is_empty() && !t.variant.is_empty());
            assert!(!t.markdown.is_empty(), "`{}` 的正文是空的", t.name);
            assert_eq!(
                TACTIC_DOCS.iter().filter(|o| o.name == t.name).count(),
                1,
                "`{}` 在表里出现了不止一次",
                t.name
            );
        }
    }

    /// 白名单文本**恰好**含全部 14 条、且以 ` / ` 分隔（"未知 tactic"文案照它拼）。
    #[test]
    fn whitelist_text_lists_every_tactic() {
        let text = whitelist_text();
        let parts: Vec<&str> = text.split(" / ").collect();
        assert_eq!(parts, names().collect::<Vec<_>>());
        assert_eq!(parts.len(), 14);
    }

    /// `is_tactic` 与表**同集合**（两个方向都试）。
    #[test]
    fn is_tactic_agrees_with_the_table_both_ways() {
        for name in names() {
            assert!(is_tactic(name), "`{name}` 在表里却不被认作 tactic");
        }
        for other in [
            "def", "theorem", "fun", "=>", "with", "intros", "exact?", "",
        ] {
            assert!(!is_tactic(other), "`{other}` 不在表里却被认作 tactic");
        }
    }
}
