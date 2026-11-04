//! **issue #7 的第三条**：用户写错 `Eq.subst` 的 motive 时，lsp 服务器不能死。
//!
//! 用户原文（2026-11-04）：「我在做题目的时候，写错了，写成
//! `apply Eq.subst (p := fun x => x ∈ ∅) hx` 或者
//! `apply Eq.subst (p := fun x => x ∈ ∅) (Eq.symm hx)` 等等, 直接造成 lsp server
//! 进程直接死掉了，我只能 Developer: Reload Window」。
//!
//! 判据（用户可见的结果 ✓）：**逐字符敲进去**（用户就是这么写的）⇒ 每个稳定点都要
//! 有回答（诊断 / 目标 / 悬停），且进程仍然活着。写错是常态 ⇒ 服务器必须继续服务。
//!
//! ⚠ 本文件是**复现件**（`AGENTS.md` 的贪心三件之一），修前必须判红。
//! 可用 `SOKO_TEST_LSP_BIN=<已发布二进制>` 对同一份夹具量已发布版本（探针纪律）。

mod common;

use common::Client;
use std::path::{Path, PathBuf};

/// 用户现场：课程单元 `unit02` 的末尾加了这道题，然后在这一行上写错。
const BAD_LINES: &[&str] = &[
    "theorem singleton_ne_empty (α : Type) (a : α) : {a} ≠ ∅ := by",
    "  intro hx",
    "  apply Set.notMem_empty a",
    "  apply Eq.subst (p := fun x => x ∈ ∅) hx",
];

/// 逐字符输入的目标行（`BAD_LINES[3]` 的前缀）。
fn typing_prefixes(line: &str) -> Vec<String> {
    line.char_indices()
        .map(|(idx, ch)| line[..idx + ch.len_utf8()].to_string())
        .collect()
}

fn course_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.join("courses/set-theory");
    if root.join("sokonanoda.toml").exists() {
        Some(root)
    } else {
        None
    }
}

fn scratch_entry(root: &Path, tag: &str) -> PathBuf {
    let dir = root
        .join(".sokonanoda")
        .join(format!("issue7-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.join("Entry.sokonanoda")
}

/// 课程单元正文（找不到就退回只含 import 的最小头部）。
fn unit_head(root: &Path) -> String {
    let path = root.join("units/I.1/unit02-subsets-empty.sokonanoda");
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => "import lib.Logic\nimport lib.Set\n".to_string(),
    }
}

fn battery(client: &mut Client, uri: &str, text: &str) {
    let doc = serde_json::json!({"textDocument": {"uri": uri}});
    let goals = client.request(1000, "soko/goals", doc.clone());
    assert!(goals.get("result").is_some(), "`soko/goals` 没答：{goals}");
    let line_of = |needle: &str| -> u64 {
        text[..text.find(needle).expect("needle")]
            .matches('\n')
            .count() as u64
    };
    let line = line_of("apply Eq.subst");
    let character = text
        .lines()
        .nth(line as usize)
        .map(|l| l.len() as u64)
        .unwrap_or(0);
    // 光标停在写错的那一行（用户正是在这里看目标 / 悬停）。
    let at = serde_json::json!({"textDocument": {"uri": uri},
                                "position": {"line": line, "character": character}});
    let state = client.request(1001, "soko/stateAt", at.clone());
    assert!(
        state.get("result").is_some(),
        "`soko/stateAt` 没答：{state}"
    );
    let goal_at = client.request(1002, "soko/goalAt", at.clone());
    assert!(
        goal_at.get("result").is_some(),
        "`soko/goalAt` 没答：{goal_at}"
    );
    let hints = client.request(1003, "soko/hints", at.clone());
    assert!(hints.get("result").is_some(), "`soko/hints` 没答：{hints}");
    let hover = client.request(1004, "textDocument/hover", at.clone());
    assert!(hover.get("result").is_some(), "hover 没答：{hover}");
    let end = serde_json::json!({"line": text.matches('\n').count() as u64 + 1, "character": 0});
    let inlay = client.request(
        1005,
        "textDocument/inlayHint",
        serde_json::json!({"textDocument": {"uri": uri},
                            "range": {"start": {"line": 0, "character": 0}, "end": end}}),
    );
    assert!(inlay.get("result").is_some(), "inlayHint 没答：{inlay}");
    let action = client.request(
        1006,
        "textDocument/codeAction",
        serde_json::json!({"textDocument": {"uri": uri},
                            "range": {"start": {"line": line, "character": 0},
                                      "end": {"line": line, "character": character}},
                            "context": {"diagnostics": []}}),
    );
    assert!(action.get("result").is_some(), "codeAction 没答：{action}");
    let symbols = client.request(1007, "textDocument/documentSymbol", doc.clone());
    assert!(
        symbols.get("result").is_some(),
        "documentSymbol 没答：{symbols}"
    );
    let tokens = client.request(1008, "textDocument/semanticTokens/full", doc.clone());
    assert!(
        tokens.get("result").is_some(),
        "semanticTokens 没答：{tokens}"
    );
    let folding = client.request(1009, "textDocument/foldingRange", doc.clone());
    assert!(
        folding.get("result").is_some(),
        "foldingRange 没答：{folding}"
    );
    let lens = client.request(1010, "textDocument/codeLens", doc.clone());
    assert!(lens.get("result").is_some(), "codeLens 没答：{lens}");
    let links = client.request(1012, "textDocument/documentLink", doc.clone());
    assert!(links.get("result").is_some(), "documentLink 没答：{links}");
    let completion = client.request(1013, "textDocument/completion", at.clone());
    assert!(
        completion.get("result").is_some(),
        "completion 没答：{completion}"
    );
    let selection = client.request(
        1014,
        "textDocument/selectionRange",
        serde_json::json!({"textDocument": {"uri": uri},
                            "positions": [{"line": line, "character": character}]}),
    );
    assert!(
        selection.get("result").is_some(),
        "selectionRange 没答：{selection}"
    );
    let project = client.request(1015, "soko/project", doc.clone());
    assert!(
        project.get("result").is_some(),
        "`soko/project` 没答：{project}"
    );
    let next_hole = client.request(1016, "soko/nextHole", at.clone());
    assert!(
        next_hole.get("result").is_some(),
        "`soko/nextHole` 没答：{next_hole}"
    );
    let again = client.request(1011, "soko/goals", doc);
    assert!(
        again.get("result").is_some(),
        "第二次 `soko/goals` 没答：{again}"
    );
}

#[test]
fn typing_an_ill_typed_subst_never_kills_the_server() {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml");
        return;
    };
    let head = unit_head(&root);
    let final_text = format!("{head}\n{}\n", BAD_LINES.join("\n"));
    let entry = scratch_entry(&root, "battery");
    std::fs::write(&entry, &final_text).expect("write entry");
    let cache = std::env::temp_dir().join(format!("soko-issue7-battery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start(&cache);
    let uri = Client::file_uri(&entry);
    let _ = client.open(&root, &uri, &final_text);
    battery(&mut client, &uri, &final_text);

    // 再把写错的那一行**逐字符**敲进去（用户真实动作）。
    let mut version = 2;
    let body = format!(
        "{head}\n{}\n{}\n{}\n",
        BAD_LINES[0], BAD_LINES[1], BAD_LINES[2]
    );
    let mut previous = body.clone();
    for prefix in typing_prefixes(BAD_LINES[3]) {
        let text = format!("{body}{prefix}\n");
        version += 1;
        let _ = client.did_change(&uri, version, &text);
        previous = text;
    }
    battery(&mut client, &uri, &previous);
    let _ = std::fs::remove_dir_all(entry.parent().unwrap());
}
