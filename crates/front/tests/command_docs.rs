//! **命令文档（`reference/commands/`）的守卫** —— 与 `tactic_docs.rs` 同形，咬的是
//! 同一类病：命令在本仓有**三份**表示（`parser.rs::is_reserved_command` 的白名单 ·
//! `crates/cli/src/help.rs` 的自文档化文案 · `reference/commands/` 的正文），靠注释与
//! 记忆同步 ⇒ 加一条命令忘了补文档，学生就看不到它 ✗（2026-10-11 用户实测：
//! 「`set_option pp.explicit` 的文档在哪」—— 功能有、用户向文档没有 ✗）。
//!
//! **唯一真相是 [`COMMANDS`]**，本文件是它的守卫：
//!
//! 1. `every_parser_command_has_a_doc_page` —— **parser ↔ 文档**（双向：`is_reserved_command`
//!    里每条 `#` 命令 + `set_option` 都要有页面；目录里也不许有表外的孤儿文档）。
//! 2. `every_documented_command_really_parses` —— **行为判据**：每条命令的最小片段真的能过
//!    parser（不是"名字写在文档里"就算数）；反向：表外的 `#` 命令必须解析失败。
//! 3. `every_command_doc_file_has_all_sections_and_a_budget` —— H1 == 命令名 + 首句 ==
//!    表里的摘要（**逐字**）+ 五个小节齐备 + 每篇 ≤120 行 / 本目录 ≤600 行。
//! 4. `index_lists_every_command_with_its_summary` —— 索引行**逐字**等于「命令 + 摘要 +
//!    链接」，且不许有表外的行。
//! 5. `pages_have_kernel_green_examples` —— **防真空判绿**：每篇至少一个 ```sokonanoda
//!    例子，且每个例子**逐块喂真内核**、零诊断（文档在教错东西就判红）。
//!
//! **反向验证**（改这里之前先跑，三条缺一不可）：
//! * 往 `is_reserved_command` 加一条 `#eval`、不建页面 ⇒ 判据 1 判红；
//! * 删掉某篇正文的一个 `##` 小节 ⇒ 判据 3 判红；
//! * 把某篇的一个例子改错（`#check nope`）⇒ 判据 5 判红。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::parse;
use std::path::{Path, PathBuf};

/// **唯一真相**：命令 →（文档文件名, 一句话摘要, 最小可解析片段）。
///
/// 摘要同时出现在三处（索引行 · 正文第 3 行 · 本条），判据 3/4 逐字比对 ⇒
/// 「索引与正文漂移」这类事当场判红 ✓。
const COMMANDS: &[(&str, &str, &str, &str)] = &[
    (
        "#check",
        "check",
        "把表达式交给内核，印出它推出来的类型。",
        "#check Nat\n",
    ),
    (
        "#reduce",
        "reduce",
        "把表达式交给内核算到底，印出归约后的值。",
        "#reduce 1\n",
    ),
    (
        "#print",
        "print",
        "印出环境里某条声明的类型与定义体（证明项也在内）。",
        "#print Nat\n",
    ),
    (
        "set_option",
        "set_option",
        "调 `#print` / `#check` / `#reduce` 的显示档 —— 让隐式实参看得见。",
        "set_option pp.explicit true\n",
    ),
];

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

/// 从 `parser.rs` 的 `is_reserved_command` 里抽**命令字面量**：`#` 开头的 + `set_option`。
///
/// **为什么只这两类**：声明关键字（`def` / `theorem` / …）不是命令（它们进环境、产
/// `decl.checked`），语法文档在 `README.md`；`set_option` 是唯一非 `#` 的命令 ⇒ 具名收进来。
fn parser_commands() -> Vec<String> {
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
        .filter(|s| s.starts_with('#') || *s == "set_option")
        .map(str::to_string)
        .collect();
    out.sort();
    out.dedup();
    out
}

// ── 判据 1：parser ↔ 文档（双向） ────────────────────────────────────────────

#[test]
fn every_parser_command_has_a_doc_page() {
    let from_parser = parser_commands();
    assert!(
        !from_parser.is_empty(),
        "抽取规则失效：`is_reserved_command` 里一条命令都没扫到（没扫到 ≠ 绿 ✗）"
    );
    let mut documented: Vec<String> = COMMANDS.iter().map(|(cmd, ..)| cmd.to_string()).collect();
    documented.sort();
    assert_eq!(
        documented,
        from_parser,
        "parser 的命令 ↔ 文档表必须**同集合**（某个方向多一条都算漂移 ✗）\n\
         parser 独有（加了命令没补文档）：{:?}\n文档独有（写了文档但命令没了）：{:?}",
        from_parser
            .iter()
            .filter(|c| !documented.contains(c))
            .collect::<Vec<_>>(),
        documented
            .iter()
            .filter(|c| !from_parser.contains(c))
            .collect::<Vec<_>>()
    );

    // 反向：目录里不许有表外的孤儿文档（`README.md` 是索引，不算）。
    let mut files: Vec<String> = std::fs::read_dir(docs_dir())
        .unwrap_or_else(|e| panic!("读不到 {}：{e}", docs_dir().display()))
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md") && n != "README.md")
        .map(|n| n.trim_end_matches(".md").to_string())
        .collect();
    files.sort();
    let mut want: Vec<String> = COMMANDS
        .iter()
        .map(|(_, stem, ..)| stem.to_string())
        .collect();
    want.sort();
    assert_eq!(files, want, "`reference/commands/` 里有表外的孤儿文档 ✗");
}

// ── 判据 2：每条命令**真的**存在（行为判据） ─────────────────────────────────

#[test]
fn every_documented_command_really_parses() {
    for (cmd, _, _, snippet) in COMMANDS {
        parse(snippet).unwrap_or_else(|e| {
            panic!(
                "`{cmd}` 在文档表里，但它的最小片段解析不过 ⇒ 命令根本不存在（或片段写错了）✗\n\
                 片段：{snippet:?}\n报错：{}",
                e.message
            )
        });
    }
    // 反向：表外的 `#` 命令必须解析失败（文档表不是"什么都能写"的装饰）。
    for bogus in ["#eval 1\n", "#env\n"] {
        assert!(
            parse(bogus).is_err(),
            "`{bogus}` 不在表里 ⇒ 必须解析失败（否则它是一条没文档的命令 ✗）"
        );
    }
}

// ── 判据 3 + 4 + 5：文件夹的真相 ─────────────────────────────────────────────

#[test]
fn every_command_doc_file_has_all_sections_and_a_budget() {
    let dir = docs_dir();
    let mut total_lines = 0usize;
    for (cmd, stem, summary, _) in COMMANDS {
        let path = dir.join(format!("{stem}.md"));
        assert!(path.is_file(), "缺正文文件 {}", path.display());
        let text = read(&path);

        // H1 == 命令名（屏幕上第一眼就是它）。
        let first = text.lines().next().unwrap_or_default();
        assert_eq!(
            first,
            format!("# {cmd}"),
            "{} 的 H1 必须是 `# {cmd}`",
            path.display()
        );

        // 首句 == 表里的摘要（**逐字**）⇒ 索引 / 正文**一处文本**。
        let third = text.lines().nth(2).unwrap_or_default();
        assert_eq!(
            third,
            *summary,
            "{} 的首句必须逐字等于 COMMANDS 的摘要",
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

    for (cmd, stem, summary, _) in COMMANDS {
        let want = format!("| `{cmd}` | {summary} | [{stem}]({stem}.md) |");
        assert!(
            lines.iter().any(|l| *l == want),
            "索引里没有逐字的那一行（学生看不到这条命令）：\n{want}"
        );
    }
    // 反向：索引里不许有表外的命令行（否则"索引比真相多"✗）。
    let rows = lines.iter().filter(|l| l.starts_with("| `")).count();
    assert_eq!(
        rows,
        COMMANDS.len(),
        "索引里有 {rows} 行命令，表里只有 {}",
        COMMANDS.len()
    );
}

/// **防真空判绿**：每篇必须有例子，且每个例子**逐块喂真内核**判绿。
#[test]
fn pages_have_kernel_green_examples() {
    for (cmd, stem, _, _) in COMMANDS {
        let path = docs_dir().join(format!("{stem}.md"));
        let text = read(&path);
        let blocks = fenced_sokonanoda_blocks(&text);
        assert!(
            !blocks.is_empty(),
            "{}（`{cmd}`）一个 ```sokonanoda 例子都没有（零例子真空判绿 ✗）",
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
