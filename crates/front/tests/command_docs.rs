//! **命令文档（`reference/commands/`）的守卫** —— 与 `tactic_docs.rs` 同形，咬的是
//! 同一类病：命令在本仓曾经有**三份**表示（`parser.rs::is_reserved_command` 的白名单 ·
//! `crates/cli/src/help.rs` 的自文档化文案 · `reference/commands/` 的正文），靠注释与
//! 记忆同步 ⇒ 加一条命令忘了补文档，学生就看不到它 ✗（2026-10-11 用户实测：
//! 「`set_option pp.explicit` 的文档在哪」—— 功能有、用户向文档没有 ✗）。
//!
//! **唯一真相是 [`sokonanoda_front::commands::COMMAND_DOCS`]**（2026-10-11 起；在那
//! 之前本文件里另有一份测试局部的 `COMMANDS` 表 = 第二份表示 ✗ —— 现在 parser 的
//! 白名单也由那张表派生）。
//!
//! 判据：
//! 1. `every_command_in_the_table_has_a_doc_page` —— **表 ↔ 文件**（双向：每条命令都
//!    要有一篇；目录里也不许有表外的孤儿文档）。
//! 2. `parser_reserved_commands_are_derived_from_the_table` —— **反向**：直接读
//!    `parser.rs::is_reserved_command` 里的 `#` 字面量 ⇒ 必须 ⊆ 表（"绕开表、往
//!    `matches!` 里直接加一条 `#eval`"照样判红 ✓）。
//! 3. `every_documented_command_really_parses` —— **行为判据**：每条命令的最小片段
//!    真的能过 parser；表外的 `#` 命令必须解析失败。
//! 4. `semantic_keywords_cover_every_command` —— 命令字必须在 `semantic::KEYWORDS`
//!    里（否则编辑器不着色、LSP 的 `command_doc_at` 也认不出它 —— F12 那条链的第一跳）。
//! 5. `embedded_markdown_matches_the_file_on_disk` —— `include_str!` 内嵌的那份必须与
//!    磁盘上的 `reference/commands/<stem>.md` **逐字节相同**（接缝：编译期内嵌 ↔ 打包源）。
//! 6. `every_command_doc_file_has_all_sections_and_a_budget` —— H1 == 命令名 + 首句 ==
//!    表里的摘要（**逐字**）+ 五个小节齐备 + 每篇 ≤120 行 / 本目录 ≤600 行。
//! 7. `index_lists_every_command_with_its_summary` —— 索引行**逐字**等于「命令 + 摘要 +
//!    链接」，且不许有表外的行。
//! 8. `pages_have_kernel_green_examples` —— **防真空判绿**：每篇至少一个
//!    ```sokonanoda 例子，且每个例子**逐块喂真内核**、零诊断（文档在教错东西就判红）。
//!
//! **反向验证**（改这里之前先跑，三条缺一不可）：
//! * 往 `is_reserved_command` 的 `matches!` 里加一条 `#eval` 字面量 ⇒ 判据 2 判红；
//! * 删掉某篇正文的一个 `##` 小节 ⇒ 判据 6 判红；
//! * 把某篇的一个例子改错（`#check nope`）⇒ 判据 8 判红。

use sokonanoda_front::commands::{self, COMMAND_DOCS};
use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::parse;
use sokonanoda_front::semantic;
use std::path::{Path, PathBuf};

/// 文档小节（**五个**：`# <命令>` 是一级标题，另算）。
///
/// ⚠ 与 tactic 的五个小节**只差第三条**：命令是"内核在背后**做**什么"（tactic 是
/// "判什么"）—— 命令里 `#reduce` 只求值不判定，照抄 tactic 的说法会撒谎 ✗。
const SECTIONS: &[&str] = &[
    "## 怎么用",
    "## 什么时候用",
    "## 内核在背后做什么",
    "## 常见错误与出路",
    "## 相关",
];

const MAX_PAGE_LINES: usize = 120;
const MAX_DIR_LINES: usize = 600;
const MAX_INDEX_LINES: usize = 80;

/// 每条命令的**最小可解析片段**（判据 3 的夹具）。
///
/// ⚠ 这**不是**第二份真相表：它只证明"这条命令 parser 真的认"，与文档内容无关。
/// 新增一条命令 ⇒ 这里没写 ⇒ 判据 3 **当场 panic**（提醒补夹具，不是静默跳过 ✗）。
fn snippet_for(name: &str) -> &'static str {
    match name {
        "#check" => "#check Nat\n",
        "#reduce" => "#reduce 1\n",
        "#print" => "#print Nat\n",
        "set_option" => "set_option pp.explicit true\n",
        other => panic!("表里新加了命令 `{other}` ⇒ 请在本函数里补它的最小可解析片段"),
    }
}

/// 仓库根（`crates/front` 往上两层）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn docs_dir() -> PathBuf {
    repo_root().join("reference").join("commands")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "读不到 {}：{e}\n（判据要能在 checkout 里跑；缺文件就是判红）",
            path.display()
        )
    })
}

/// 从 `parser.rs` 的 `is_reserved_command` 里抽 `#` 开头的字面量（判据 2 的输入）。
///
/// 为什么读**源码文本**：命令那半现在由表派生（`crate::commands::is_command`）⇒
/// 正常路径下这里**一个 `#` 字面量都不该有**；但"绕开表、直接往 `matches!` 里加一条"
/// 那种写法**只有文本侧看得见** ⇒ 这一条是那类写法的唯一守卫 ✓。
fn parser_hash_literals() -> Vec<String> {
    let path = repo_root()
        .join("crates")
        .join("front")
        .join("src")
        .join("parser.rs");
    let src = read(&path);
    let start = src
        .find("fn is_reserved_command")
        .unwrap_or_else(|| panic!("{} 里找不到 `fn is_reserved_command`", path.display()));
    let rest = &src[start..];
    let end = rest
        .find("\n}")
        .unwrap_or_else(|| panic!("`is_reserved_command` 的收尾 `}}` 找不到"));
    let body = &rest[..end];
    let mut out: Vec<String> = body
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|s| s.starts_with('#'))
        .map(str::to_string)
        .collect();
    out.sort();
    out.dedup();
    out
}

// ── 判据 1：表 ↔ 文件（双向） ────────────────────────────────────────────────

#[test]
fn every_command_in_the_table_has_a_doc_page() {
    assert!(!COMMAND_DOCS.is_empty(), "真相表是空的（没扫到 ≠ 绿 ✗）");
    for doc in COMMAND_DOCS {
        let path = docs_dir().join(format!("{}.md", doc.stem));
        assert!(
            path.is_file(),
            "表里有 `{}`，但缺正文文件 {}",
            doc.name,
            path.display()
        );
    }
    // 反向：目录里不许有表外的孤儿文档（`README.md` 是索引，不算）。
    let mut files: Vec<String> = std::fs::read_dir(docs_dir())
        .unwrap_or_else(|e| panic!("读不到 {}：{e}", docs_dir().display()))
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md") && n != "README.md")
        .map(|n| n.trim_end_matches(".md").to_string())
        .collect();
    files.sort();
    let mut want: Vec<String> = COMMAND_DOCS.iter().map(|d| d.stem.to_string()).collect();
    want.sort();
    assert_eq!(files, want, "`reference/commands/` 里有表外的孤儿文档 ✗");
}

// ── 判据 2：parser 的白名单由表派生（反向：字面量必须 ⊆ 表） ──────────────────

#[test]
fn parser_reserved_commands_are_derived_from_the_table() {
    let documented: Vec<&str> = COMMAND_DOCS.iter().map(|d| d.name).collect();
    for literal in parser_hash_literals() {
        assert!(
            documented.contains(&literal.as_str()),
            "`parser.rs::is_reserved_command` 里有一条手写 `#` 字面量 `{literal}`，\
             但真相表里没有它 ⇒ 要么把它删掉（命令由 `crate::commands::is_command` 派生 ✓），\
             要么加进 `COMMAND_DOCS` + 补 `reference/commands/<stem>.md` ✗"
        );
    }
    // 正向：表里每条都真的被认作命令（行为判据的一半，另一半是判据 3）。
    for doc in COMMAND_DOCS {
        assert!(
            commands::is_command(doc.name),
            "`{}` 在表里，但 `commands::is_command` 说不认识它",
            doc.name
        );
    }
    // 反向：表外的 `#` 命令不许被认作命令。
    for bogus in ["#eval", "#env", "#help"] {
        assert!(
            !commands::is_command(bogus),
            "`{bogus}` 不在表里 ⇒ `is_command` 必须答 false"
        );
    }
}

// ── 判据 3：每条命令**真的**存在（行为判据） ─────────────────────────────────

#[test]
fn every_documented_command_really_parses() {
    for doc in COMMAND_DOCS {
        let snippet = snippet_for(doc.name);
        parse(snippet).unwrap_or_else(|e| {
            panic!(
                "`{}` 在真相表里，但它的最小片段解析不过 ⇒ 命令根本不存在（或片段写错了）✗\n\
                 片段：{snippet:?}\n报错：{}",
                doc.name, e.message
            )
        });
    }
    // 反向：表外的 `#` 命令必须解析失败（表不是"什么都能写"的装饰）。
    for bogus in ["#eval 1\n", "#env\n"] {
        assert!(
            parse(bogus).is_err(),
            "`{bogus}` 不在表里 ⇒ 必须解析失败（否则它是一条没文档的命令 ✗）"
        );
    }
}

// ── 判据 4：命令字必须在 `KEYWORDS` 里（F12 那条链的第一跳） ─────────────────

#[test]
fn semantic_keywords_cover_every_command() {
    let keywords = semantic::keywords();
    for doc in COMMAND_DOCS {
        assert!(
            keywords.contains(&doc.name),
            "`{}` 不在 `semantic::KEYWORDS` 里 ⇒ 编辑器不着色、LSP 的 `command_doc_at` \
             （它按 `SemanticKind::Keyword` 的 run 认命令字）也认不出它 ⇒ F12 那条链断在第一跳 ✗",
            doc.name
        );
    }
}

// ── 判据 5：内嵌那份 == 磁盘那份（编译期 ↔ 打包源的接缝） ─────────────────────

#[test]
fn embedded_markdown_matches_the_file_on_disk() {
    for doc in COMMAND_DOCS {
        let path = docs_dir().join(format!("{}.md", doc.stem));
        let on_disk = read(&path);
        assert_eq!(
            doc.markdown,
            on_disk,
            "{} 与 `COMMAND_DOCS` 里 `include_str!` 内嵌的那份不一致 ⇒ \
             重新编译（内嵌是编译期的）或检查文件被谁改过 ✗",
            path.display()
        );
    }
}

// ── 判据 6 + 7 + 8：文件夹的真相 ─────────────────────────────────────────────

#[test]
fn every_command_doc_file_has_all_sections_and_a_budget() {
    let dir = docs_dir();
    let mut total_lines = 0usize;
    for doc in COMMAND_DOCS {
        let path = dir.join(format!("{}.md", doc.stem));
        let text = read(&path);

        // H1 == 命令名（屏幕上第一眼就是它）。
        let first = text.lines().next().unwrap_or_default();
        assert_eq!(
            first,
            format!("# {}", doc.name),
            "{} 的 H1 必须是 `# {}`",
            path.display(),
            doc.name
        );

        // 首句 == 表里的摘要（**逐字**）⇒ 索引 / 正文 / 表**一处文本**。
        let third = text.lines().nth(2).unwrap_or_default();
        assert_eq!(
            third,
            doc.summary,
            "{} 的首句必须逐字等于 `COMMAND_DOCS` 的摘要",
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

        // 预算（与 `reference/tactics/` 同口径）：每篇 ≤120 行。
        let lines = text.lines().count();
        total_lines += lines;
        assert!(
            lines <= MAX_PAGE_LINES,
            "{} 有 {lines} 行 > 每篇上限 {MAX_PAGE_LINES}",
            path.display()
        );
    }
    assert!(
        total_lines <= MAX_DIR_LINES,
        "本目录共 {total_lines} 行 > 总量上限 {MAX_DIR_LINES}"
    );
}

#[test]
fn index_lists_every_command_with_its_summary() {
    let path = docs_dir().join("README.md");
    let text = read(&path);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.len() <= MAX_INDEX_LINES,
        "索引 {} 行 > 上限 {MAX_INDEX_LINES}",
        lines.len()
    );

    for doc in COMMAND_DOCS {
        let want = format!(
            "| `{}` | {} | [{}]({}.md) |",
            doc.name, doc.summary, doc.stem, doc.stem
        );
        assert!(
            lines.iter().any(|l| *l == want),
            "索引里没有逐字的那一行（学生看不到这条命令）：\n{want}"
        );
    }
    // 反向：索引里不许有表外的命令行（否则"索引比真相多"✗）。
    let rows = lines.iter().filter(|l| l.starts_with("| `")).count();
    assert_eq!(
        rows,
        COMMAND_DOCS.len(),
        "索引里有 {rows} 行命令，表里只有 {}",
        COMMAND_DOCS.len()
    );
}

/// **防真空判绿**：每篇必须有例子，且每个例子**逐块喂真内核**判绿。
///
/// ⚠ 喂的是 `doc.markdown`（**编译期内嵌**那份，判据 5 已钉住它 == 磁盘那份）——
/// 这样"文档在教错东西"这件事在**编译产物**上就被咬住，不依赖测试时读盘 ✓。
#[test]
fn pages_have_kernel_green_examples() {
    for doc in COMMAND_DOCS {
        let blocks = fenced_sokonanoda_blocks(doc.markdown);
        assert!(
            !blocks.is_empty(),
            "`reference/commands/{}.md`（`{}`）一个 ```sokonanoda 例子都没有\
             （零例子真空判绿 ✗）",
            doc.stem,
            doc.name
        );
        for (i, block) in blocks.iter().enumerate() {
            let file = parse(block).unwrap_or_else(|e| {
                panic!(
                    "`{}` 的第 {} 个例子**解析都过不了**：{}\n--- 例子 ---\n{block}",
                    doc.name,
                    i + 1,
                    e.message
                )
            });
            let units = [SourceUnit::single("doc_example", &file)];
            let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
            assert!(
                out.errors.is_empty(),
                "`{}` 的第 {} 个例子**内核判红** ⇒ 文档在教错东西 ✗\n错误：{:?}\n--- 例子 ---\n{block}",
                doc.name,
                i + 1,
                out.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
            );
        }
    }
}

/// 抽出一篇文档里**每一个** ```sokonanoda 围栏块的正文（各自独立判定）。
///
/// ⚠ 与 `tactic_docs.rs` 的同名函数是**故意重复**的两份：两个 tests 二进制之间没有
/// 共享模块（Rust 集成测试各编各的），而这一小段是"文档例子必须真判绿"这条纪律的
/// 唯一实现 —— 抽成 `tests/common/` 反而会让两份判据的**严格度**有机会分叉 ✗。
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

/// 让 `CommandDoc` 的字段在编译期都被用到（防"加了字段没人读"）—— 判据本身在别处。
#[test]
fn command_doc_fields_are_all_consumed() {
    for doc in COMMAND_DOCS {
        let _: (&str, &str, &str, &str) = (doc.name, doc.stem, doc.summary, doc.markdown);
    }
}
