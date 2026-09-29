//! ST1（v0.77.0）的**分界守卫** —— 把决策记录 `docs/design/v077-st1-boundary.md`
//! 里逐项写下的「哪些用类型论自身表达、哪些确实必须外挂」钉到**真实内核行为**上。
//!
//! 为什么需要它（`docs/ONBOARDING.md`（原 PLAN-0.74-0.79 已删，2026-09-29） 的 ST1 环节 + 三条纪律）：
//!   * 决策记录是**一份判断**，判断会腐烂 —— 语言一改（例如哪天把 `Quot` 装进
//!     prelude、或让 `Acc` 立起来），记录里的结论就悄悄变成错的 ✗；
//!   * 所以每条结论都带**最小复现件**（`docs/gaps/repro/ST1-*.sokonanoda`），
//!     本文件逐个跑它们，并把「记录里写的诊断」与「内核真吐的诊断」**逐字**对账；
//!   * **反向验证**：改记录里那句话、或改复现件里那行源码 ⇒ 本文件判红
//!     （不是"再抄一遍期望值"，是**记录 ↔ 内核**两端对账）。
//!
//! 判据形状（每条都断言**用户可见的结果**：声明计数与诊断原文）：
//!   * `positive` 探针：`decl.checked` 条数 == 记录里写的数字，且 0 条诊断、exit 0；
//!   * `negative` 探针：每条诊断的 `code` + `message` **逐字**等于记录里的引文，
//!     且 `decl.checked` == 记录里写的数字、exit 1。
//!
//! 探针自足（无 `import`）⇒ 直接单文件判卷，不需要模块根。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 仓库根（`crates/cli` → 上两级）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root")
}

/// ST1 决策记录：**唯一真相**（结论、依据、影响面都在里面）。
fn decision_record() -> String {
    let path = repo_root().join("docs/design/v077-st1-boundary.md");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("ST1 决策记录读不出来（{}）：{e}", path.display()))
}

/// 判卷一个复现件，返回 `(exit_ok, decl_checked, diagnostics)`。
fn grade_probe(file: &str) -> (bool, usize, Vec<(String, String)>) {
    let path = repo_root().join("docs/gaps/repro").join(file);
    assert!(path.exists(), "复现件不存在：{}", path.display());
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("--json")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sokonanoda");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut checked = 0usize;
    let mut diagnostics = Vec::new();
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("{file}: 不是合法 JSON 行：{e}\n{line}"));
        match value.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "decl.checked" => checked += 1,
            "diagnostic" => diagnostics.push((
                value["code"].as_str().unwrap_or("<no code>").to_string(),
                value["message"]
                    .as_str()
                    .unwrap_or("<no message>")
                    .to_string(),
            )),
            _ => {}
        }
    }
    (out.status.success(), checked, diagnostics)
}

/// 记录里那张对账表的**一行**。
struct Row {
    file: String,
    verdict: String,
    checked: usize,
    quotes: Vec<String>,
}

/// 解析决策记录 §对账表：`| <文件> | <判据> | <decl.checked 数> | <逐字诊断> |`。
///
/// 形状**故意写死**（表头 + 4 列）—— 记录改形状 ⇒ 本函数直接判红，
/// 不允许"记录悄悄换了说法、守卫还在原地点头"。
fn reconciliation_table(record: &str) -> Vec<Row> {
    let marker = "## §对账表";
    let body = record
        .split_once(marker)
        .unwrap_or_else(|| panic!("决策记录里找不到 `{marker}`（对账表是守卫的锚点）"))
        .1;
    let mut rows = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            if !rows.is_empty() {
                break; // 表结束
            }
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        if cells.len() != 4 {
            panic!(
                "对账表每行必须 4 列（文件/判据/checked 数/逐字诊断），这行是 {} 列：{line}",
                cells.len()
            );
        }
        if cells[0] == "复现件" || cells[0].starts_with("---") {
            continue; // 表头 / 分隔行
        }
        // 文件与判据两列去掉行内代码的引号（`ST1-….sokonanoda` → ST1-….sokonanoda）
        let cell = |i: usize| cells[i].trim().trim_matches('`').trim().to_string();
        let checked: usize = cell(2)
            .parse()
            .unwrap_or_else(|_| panic!("对账表的 `decl.checked` 列必须是纯数字，这行是：{line}"));
        // 逐字诊断列：多条用 `;;` 分隔，**不加引号**（诊断原文里本来就有反引号，
        // 再加一层引号会让 Markdown 表格解析不出边界）
        let quotes = cells[3]
            .split(";;")
            .map(|q| q.trim().to_string())
            .filter(|q| !q.is_empty() && q != "—")
            .collect();
        rows.push(Row {
            file: cell(0),
            verdict: cell(1),
            checked,
            quotes,
        });
    }
    assert!(
        rows.len() >= 4,
        "对账表至少要覆盖 4 条探针（分离/无限并交 · 序数 · Quot · 良基递归），只有 {} 行",
        rows.len()
    );
    rows
}

/// 记录里写的逐字诊断，必须在**探针输出的诊断原文**里出现（双向对账的第一步）。
///
/// 口径：**把两边的 Markdown 行内代码反引号去掉再比**。理由：内核诊断原文里本来就
/// 用反引号括标识符（``unknown identifier `Quot` ``），而记录里那一格若也加反引号，
/// Markdown 表格就会解析不出边界（§对账表 的表头已经写明"不加引号"）。
/// 去反引号之后仍是**逐字**比对 —— 少一个字、错一个字都判红。
fn strip_ticks(s: &str) -> String {
    s.replace('`', "")
}

fn assert_quote_is_in_probe(row: &Row, diags: &[(String, String)]) {
    let haystack: String = diags
        .iter()
        .map(|(code, msg)| format!("{}\u{1}{}", strip_ticks(code), strip_ticks(msg)))
        .collect::<Vec<_>>()
        .join("\u{2}");
    for quote in &row.quotes {
        let needle = strip_ticks(quote);
        assert!(
            haystack.contains(needle.as_str()),
            "记录里对 `{}` 写的诊断，内核**没吐**：\n  记录：{quote}\n  内核：{diags:#?}",
            row.file
        );
    }
}

/// 反向：内核吐的每条诊断，也必须能在记录里找到 —— 否则记录**漏记**了新诊断。
///
/// 口径：诊断的 `code` 与 `message` 各自要被**记录里某一格**包含（可以是两格）。
/// 不要求整条 `code+message` 挤在同一格 —— 记录是给人读的，允许把码与正文分开列。
fn assert_probe_diagnostics_are_all_recorded(row: &Row, diags: &[(String, String)]) {
    for (code, msg) in diags {
        let code = strip_ticks(code);
        let msg = strip_ticks(msg);
        let code_recorded = row.quotes.iter().any(|q| q.contains(code.as_str()));
        let msg_recorded = row.quotes.iter().any(|q| msg.contains(&strip_ticks(q)));
        assert!(
            code_recorded && msg_recorded,
            "内核对 `{}` 吐了记录里**没有**的诊断（记录漏记 / 结论过期）：\n  [{code}] {msg}",
            row.file
        );
    }
}

#[test]
fn st1_record_claims_match_the_kernel() {
    let record = decision_record();
    let rows = reconciliation_table(&record);
    for row in &rows {
        let (ok, checked, diags) = grade_probe(&row.file);
        assert_eq!(
            checked, row.checked,
            "`{}` 的 `decl.checked` 数与记录不符（记录 {} / 内核 {}）",
            row.file, row.checked, checked
        );
        match row.verdict.as_str() {
            "positive" => {
                assert!(ok, "`{}` 记录里判 positive，但判卷 exit != 0", row.file);
                assert!(
                    diags.is_empty(),
                    "`{}` 记录里判 positive，但内核吐了诊断：{diags:#?}",
                    row.file
                );
            }
            "negative" => {
                assert!(
                    !ok,
                    "`{}` 记录里判 negative，但判卷 exit == 0（缺口没了 ⇒ 更新记录）",
                    row.file
                );
                assert!(
                    !diags.is_empty(),
                    "`{}` 记录里判 negative，但内核一条诊断都没有",
                    row.file
                );
                assert_quote_is_in_probe(row, &diags);
                assert_probe_diagnostics_are_all_recorded(row, &diags);
            }
            other => panic!("对账表的判据列只认 positive / negative，这行写的是 `{other}`"),
        }
    }
}

#[test]
fn st1_record_covers_every_probe_file() {
    // 反向守卫：`docs/gaps/repro/ST1-*.sokonanoda` **每一个**都必须在对账表里，
    // 否则新加的探针可以悄悄躺在目录里、记录与它无关。
    let dir = repo_root().join("docs/gaps/repro");
    let record = decision_record();
    let rows = reconciliation_table(&record);
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("read docs/gaps/repro")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("ST1-") && n.ends_with(".sokonanoda"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = rows.iter().map(|r| r.file.clone()).collect();
    listed.sort();
    assert_eq!(
        on_disk, listed,
        "`docs/gaps/repro/ST1-*.sokonanoda` 与对账表必须一一对应（磁盘 {on_disk:?} / 记录 {listed:?}）"
    );
}
