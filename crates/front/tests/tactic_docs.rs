//! **tactic 文档的五条判据**（设计 `docs/design/tactic-docs.md` §5 P1 的 L3 层）。
//!
//! 这五条咬住的是**同一个病**：tactic 清单在本仓曾经有**六份**表示
//! （`Tactic` 枚举 · `Tactic::span()` 的穷尽臂 · parser 白名单 · `semantic::KEYWORDS` ·
//! `proof::render_tactic` · **"未知 tactic"错误文案**），靠注释与记忆同步 ⇒
//! `cases`/`have` 漂过一次、错误文案漏 `have` 漂了第二次（且**零判据**覆盖）✗。
//! 现在唯一真相是 [`sokonanoda_front::tactics::TACTIC_DOCS`]，本文件是它的**守卫**：
//!
//! 1. `hand_written_tactic_list_equals_the_table` —— **手写清单 ↔ 表**（双向）。
//!    Rust 没有反射（无 `strum`/`variant_count`），枚举不出变体名 ⇒ 手写清单是唯一可行
//!    形态；本仓已有同形先例：`crates/lsp/src/tests/hover.rs:1374-1389`。
//! 2. `semantic_keywords_cover_every_tactic_except_the_named_hole` —— `KEYWORDS` 的
//!    **具名例外**（`sorry` 是 `Hole`，不是 `Keyword`）。
//! 3. `unknown_tactic_message_lists_every_tactic` —— 那条**用户可见**的文案含全 14 条
//!    （2026-10-10 抓到的既有缺陷：它漏了 `have`）。
//! 4. `every_tactic_doc_file_exists_and_has_all_sections` —— 文件在 + H1 == 关键字 +
//!    首句 == 表里的摘要 + 五个小节齐备 + 预算。
//! 5. `index_lists_every_tactic_with_its_summary` —— 索引 14 行**逐字**等于表。
//! 6. `pages_marked_done_have_kernel_green_examples` —— **防真空判绿**：P1 骨架带阶段
//!    标记，标记一删，这一篇的例子就**必须**存在且**逐块喂真内核判绿** ✓。
//!
//! **反向验证**（改这里之前先跑，三条缺一不可）：
//! * 往 `parse_tactic_inner` 加一个**只有臂、没进表**的关键字 ⇒ 判据 1 与 L4 判红；
//! * 只往表里**加一行**、不建 `.md` ⇒ **编译失败**（`include_str!`）；
//! * 删掉某篇正文的一个 `##` 小节 ⇒ 判据 4 判红。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::parse;
use sokonanoda_front::semantic::{self, SemanticKind};
use sokonanoda_front::tactics::TACTIC_DOCS;
use std::path::{Path, PathBuf};

/// 仓库根（`crates/front` 往上两层）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn docs_dir() -> PathBuf {
    repo_root().join("reference").join("tactics")
}

/// **手写清单**（判据 1 的另一半）。故意写成字面量、且**与 `TACTIC_DOCS` 的顺序不同**，
/// 这样"顺序被误当成契约"这类写法也会被咬住 ✓。
const HAND_WRITTEN: &[&str] = &[
    "sorry",
    "have",
    "cases",
    "exfalso",
    "use",
    "right",
    "left",
    "constructor",
    "match",
    "rfl",
    "assumption",
    "apply",
    "exact",
    "intro",
];

/// 文档小节（**五个**：`# <关键字>` 是一级标题，另算）。
const SECTIONS: &[&str] = &[
    "## 怎么用",
    "## 什么时候用",
    "## 内核在背后判什么",
    "## 常见错误与出路",
    "## 相关",
];

/// P1 骨架标记：**有它 = 正文还没写**（判据 6 对该篇不生效）。
const STUB_MARKER: &str = "<!-- P1 骨架：";

// ── 判据 1：手写清单 ↔ 表 ────────────────────────────────────────────────────

#[test]
fn hand_written_tactic_list_equals_the_table() {
    let table: Vec<&str> = TACTIC_DOCS.iter().map(|t| t.name).collect();
    let mut from_table = table.clone();
    let mut hand = HAND_WRITTEN.to_vec();
    from_table.sort_unstable();
    hand.sort_unstable();
    assert_eq!(
        hand,
        from_table,
        "手写清单与 TACTIC_DOCS 必须**同集合**（某个方向多一条都算漂移 ✗）\n\
         表里独有：{:?}\n手写独有：{:?}",
        from_table
            .iter()
            .filter(|n| !hand.contains(n))
            .collect::<Vec<_>>(),
        hand.iter()
            .filter(|n| !from_table.contains(n))
            .collect::<Vec<_>>()
    );
    assert_eq!(table.len(), 14, "tactic 基线是 14 条（设计 §1.1）");
    assert_eq!(
        table,
        vec![
            "intro",
            "exact",
            "apply",
            "assumption",
            "rfl",
            "match",
            "constructor",
            "left",
            "right",
            "use",
            "exfalso",
            "cases",
            "have",
            "sorry"
        ],
        "**表序 = 白名单文案的顺序**（`whitelist_text`），改顺序会改用户可见文案 ⇒ 判据 3 也会红"
    );
}

// ── 判据 2：`KEYWORDS` 的具名例外 ────────────────────────────────────────────

#[test]
fn semantic_keywords_cover_every_tactic_except_the_named_hole() {
    let keywords = semantic::keywords();
    for t in TACTIC_DOCS {
        let listed = keywords.contains(&t.name);
        match t.semantic_kind {
            SemanticKind::Keyword => assert!(
                listed,
                "`{}` 归成 Keyword ⇒ 必须在 `semantic::KEYWORDS` 里（否则编辑器不着色、不补全 ✗）",
                t.name
            ),
            SemanticKind::Hole => {
                // **具名例外**：只允许 `sorry` 一条走这条路，且它必须**不在**词表里。
                assert_eq!(
                    t.name, "sorry",
                    "只有 `sorry` 可以是 Hole（它是有意归类，见 semantic.rs:866-867）✗"
                );
                assert!(
                    !listed,
                    "`{}` 归成 Hole ⇒ 刻意不在 `KEYWORDS` 里（判据不许写成\"跳过所有不在词表里的项\" ✗）",
                    t.name
                );
            }
            other => panic!(
                "`{}` 的 semantic_kind 是 {other:?} —— 只允许 Keyword 或 Hole",
                t.name
            ),
        }
    }
    // 反向：词表里的 tactic **都得**在表里（否则"表外还有 tactic 关键字"✗）。
    let tactic_set: Vec<&str> = TACTIC_DOCS
        .iter()
        .filter(|t| t.semantic_kind == SemanticKind::Keyword)
        .map(|t| t.name)
        .collect();
    for kw in keywords {
        // 词表里既有 tactic 也有别的东西（def/theorem/…）⇒ 只检查"是不是 tactic"这件事。
        if sokonanoda_front::tactics::is_tactic(kw) {
            assert!(
                tactic_set.contains(kw),
                "`{kw}` 被认作 tactic，但表里没有它（或它不是 Keyword）✗"
            );
        }
    }
}

// ── 判据 3：每条 tactic **真的**有 parser 臂（行为判据） ──────────────────────
//
// ⚠ **为什么不是"断言那条错误文案"**：2026-10-10 实测，`parse_tactic()` 的 3 个调用点
// 前面都有 `tactic_keyword_ahead()`（= 表成员）闸门，而表里 14 条在
// `parse_tactic_inner` 里**都有自己的臂** ⇒ 兜底臂**不可达** ✗：
// 打错一条 tactic（`hax`）走的是"空 `by` 块 + 未知命令"那条路，
// **根本到不了**"未知 tactic"文案。所以从**外面**断言那句话是断言不到的
// （初稿这么写过，实测判红 ⇒ 已改，见设计 §12 第 6 条更正）。
//
// **真正该判的是它的可达条件**：表里每一条**都有真的 parser 臂** ⇒ 兜底臂继续不可达 ✓。
// 反过来，若有人"表里加了一行、parser 忘了加臂"，那一条会**当场掉进兜底臂**，
// 本判据立刻判红（而不是等到用户报错才发现）✓。
// 文案本身则由 `parser.rs` 的单元测试
// `unknown_tactic_message_lists_every_tactic_and_claims_nothing_more` 直接判 ✓。

/// 每条 tactic 的最小片段（`parse` 只要求进到它的**臂**里；参数不全没关系——
/// 我们要判的是"臂存在"，不是"片段能判绿"）。
fn minimal_snippet(name: &str) -> String {
    format!("example : True := by\n  {name}\n")
}

#[test]
fn every_tactic_in_the_table_has_a_real_parser_arm() {
    for t in TACTIC_DOCS {
        let src = minimal_snippet(t.name);
        match parse(&src) {
            // 参数不全却解析成功也正常（有的 tactic 零参数）—— 只要没掉进兜底臂 ✓。
            Ok(_) => {}
            Err(e) => assert!(
                !e.message.contains("未知 tactic"),
                "`{}` 在表里，却掉进了「未知 tactic」兜底臂 ⇒ parser 忘了给它加臂 ✗\n\
                 兜底臂今天本该**不可达**（`parse_tactic` 的 3 个调用点都有白名单闸门）\n\
                 报错：{}",
                t.name,
                e.message
            ),
        }
    }
    // 反向：**不在表里**的关键字必须走不到 tactic 解析（否则白名单形同虚设 ✗）。
    for other in ["intros", "simp", "rw", "exact?", "hax"] {
        let src = minimal_snippet(other);
        let err = parse(&src).expect_err(&format!("`{other}` 不是 tactic ⇒ 必须报错"));
        assert!(
            !err.message.contains("未知 tactic"),
            "`{other}` 不在表里 ⇒ 它在 `by` 块里应该被当成空块 + 未知命令（不是\"未知 tactic\"）✗：{}",
            err.message
        );
    }
}

// ── 判据 4 + 5 + 6：文件夹的真相 ─────────────────────────────────────────────

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "读不到 {}：{e}\n（判据要能在 checkout 里跑；缺文件就是判红）",
            path.display()
        )
    })
}

#[test]
fn every_tactic_doc_file_exists_and_has_all_sections() {
    let dir = docs_dir();
    let mut total_lines = 0usize;
    for t in TACTIC_DOCS {
        let path = dir.join(format!("{}.md", t.name));
        assert!(path.is_file(), "缺正文文件 {}", path.display());
        let text = read(&path);

        // H1 == 关键字（F12 的 range 落在标题行时，屏幕上第一眼就是它的名字）。
        let first = text.lines().next().unwrap_or_default();
        assert_eq!(
            first,
            format!("# {}", t.name),
            "{} 的 H1 必须是 `# {}`",
            path.display(),
            t.name
        );

        // 首句 == 表里的摘要（**逐字**）⇒ hover / 补全 / 索引 / 正文**四处一份文本**。
        let second = text.lines().nth(2).unwrap_or_default();
        assert_eq!(
            second,
            t.summary,
            "{} 的首句必须逐字等于 TACTIC_DOCS 的 summary（§4.5 定稿）",
            path.display()
        );

        // 五个小节一个不缺（缺一个 ⇒ "完整介绍"会退化成一段话 ✗）。
        for sec in SECTIONS {
            assert!(
                text.lines().any(|l| l.trim() == *sec),
                "{} 缺小节 `{sec}`",
                path.display()
            );
        }

        // 预算（设计 §5 P3 判据 ④）：每篇 ≤120 行。
        let lines = text.lines().count();
        total_lines += lines;
        assert!(
            lines <= 120,
            "{} 有 {lines} 行 > 每篇上限 120",
            path.display()
        );
    }
    assert!(
        total_lines <= 1200,
        "本目录共 {total_lines} 行 > 总量上限 1200"
    );
}

#[test]
fn index_lists_every_tactic_with_its_summary() {
    let path = docs_dir().join("README.md");
    let text = read(&path);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() <= 80, "索引 {} 行 > 上限 80", lines.len());

    let mut seen = 0usize;
    for t in TACTIC_DOCS {
        let want_tail = format!(" | [{0}]({0}.md) |", t.name);
        let row = lines
            .iter()
            .find(|l| l.starts_with(&format!("| `{}` |", t.name)))
            .unwrap_or_else(|| panic!("索引里没有 `{}` 那一行 ⇒ 学生看不到这条 tactic", t.name));
        assert_eq!(
            *row,
            format!("| `{}` | {} | [{0}]({0}.md) |", t.name, t.summary),
            "索引那一行必须**逐字**等于「表里的 name + summary + 链接」（§4.7.3）"
        );
        assert!(row.ends_with(&want_tail), "索引行的链接形状不对：{row}");
        seen += 1;
    }
    assert_eq!(seen, 14);
    // 反向：索引里不许有表外的 tactic 行（否则"索引比真相多"✗）。
    let rows = lines.iter().filter(|l| l.starts_with("| `")).count();
    assert_eq!(rows, 14, "索引里有 {rows} 行 tactic，表里只有 14 条");
}

/// **防真空判绿**：P3–P5 每填完一篇就删掉骨架标记 ⇒ 从那一刻起，
/// "这篇必须有例子、且每个例子逐块喂真内核判绿"对它生效 ✓。
///
/// 为什么要有它：判据"每个 ```sokonanoda 块都判绿"在**零个**块时**真空通过** ✗
/// （AGENTS.md：「咬不住的守卫等于没有」）⇒ 骨架阶段用显式标记挡住，
/// 标记一删就必须真有例子。
#[test]
fn pages_marked_done_have_kernel_green_examples() {
    for t in TACTIC_DOCS {
        let path = docs_dir().join(format!("{}.md", t.name));
        let text = read(&path);
        if text.contains(STUB_MARKER) {
            continue; // P1 骨架：正文由 P3–P5 填（标记删掉的那一刻这条判据开始咬）
        }
        let blocks = fenced_sokonanoda_blocks(&text);
        assert!(
            !blocks.is_empty(),
            "{} 已删掉骨架标记 ⇒ 必须至少有一个 ```sokonanoda 例子（零例子真空判绿 ✗）",
            path.display()
        );
        for (i, block) in blocks.iter().enumerate() {
            let file = parse(block).unwrap_or_else(|e| {
                panic!(
                    "{} 的第 {} 个例子**解析都过不了**：{}\n--- 例子 ---\n{block}",
                    path.display(),
                    i + 1,
                    e.message
                )
            });
            let units = [SourceUnit::single("doc_example", &file)];
            let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
            assert!(
                out.errors.is_empty(),
                "{} 的第 {} 个例子**内核判红** ⇒ 文档在教错东西 ✗\n错误：{:?}\n--- 例子 ---\n{block}",
                path.display(),
                i + 1,
                out.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
            );
        }
    }
}

/// 抽出一篇文档里**每一个** ```sokonanoda 围栏块的正文（各自独立判定）。
fn fenced_sokonanoda_blocks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Option<Vec<&str>> = None;
    for line in text.lines() {
        match (&mut cur, line.trim_end()) {
            (None, "```sokonanoda") => cur = Some(Vec::new()),
            (Some(_), "```") => {
                let body = cur.take().expect("some");
                out.push(body.join("\n") + "\n");
            }
            (Some(body), _) => body.push(line),
            (None, _) => {}
        }
    }
    assert!(cur.is_none(), "有一个 ```sokonanoda 块没有闭合");
    out
}
