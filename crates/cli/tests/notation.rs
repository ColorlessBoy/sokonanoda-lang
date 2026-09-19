//! 记法（G-04 / WO-011，设计 `docs/design/notation-subset.md`）的 CLI 端到端。
//!
//! 判据是 **N7 教学契约**：同一命题的两种写法——点名 `Set.mem α a A` 与记法
//! `a ∈ A`——**判卷结果一致**（`grade` 退出码一致 + 五元事件计数一致），且
//! **点名形式继续可用**（设计 N4.3 的护城河：省 `α` 的点名写法仍被内核拒绝）。
//!
//! 一切判定走真二进制 + `--json` 事件流（课程线纪律：只看 `grade` 的退出码与
//! 事件流，`courses/set-theory/AGENTS.md`）。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

/// 仓库根（这个 crate 在 `<root>/crates/cli`）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-notation-cache-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn temp_file(tag: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sokonanoda-notation-{tag}-{}-{}.sokonanoda",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, text).expect("write canvas");
    path
}

/// `sokonanoda --json <file>`：返回 `(退出码, 事件流)`。
fn grade_json(path: &Path) -> (i32, Vec<Value>) {
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["--json", path.to_str().unwrap()])
        .env("SOKONANODA_CACHE_DIR", cache_dir("json"))
        .output()
        .expect("run --json");
    let stream = String::from_utf8_lossy(&out.stdout);
    let events = stream
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    (out.status.code().unwrap_or(-1), events)
}

/// 五元事件计数（`docs/protocol.md` 的 `counts` 口径）。
fn counts(events: &[Value]) -> (usize, usize, usize, usize, usize) {
    let n = |kind: &str| events.iter().filter(|e| e["type"] == kind).count();
    (
        n("decl.checked"),
        n("exercise.open"),
        n("expr.reduced"),
        n("expr.typed"),
        n("diagnostic"),
    )
}

/// 课程库形状的夹具（与 `courses/set-theory/lib/Set.sokonanoda` 同签名：
/// `α` 是**显式**前导参数 ⇒ 记法展开必须自己补它）。
const LIB: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
def Set.empty (α : Type) : Set α := fun (x : α) => False
";

/// 同一命题的两种写法：只有声明行不同，注释/顺序/名字逐字一致。
fn pointful_canvas() -> String {
    format!(
        "{LIB}\
def p (α : Type) (a : α) (A : Set α) : Prop := Set.mem α a A\n\
def s (α : Type) (A B : Set α) (h : Set.subset α A B) : Set.subset α A B := h\n\
def e (α : Type) : Set α := Set.empty α\n\
#check Set.empty\n"
    )
}

fn notation_canvas() -> String {
    format!(
        "{LIB}\
infix:50 \" ∈ \" => Set.mem\n\
infix:50 \" ⊆ \" => Set.subset\n\
notation \"∅\" => Set.empty\n\
def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n\
def s (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := h\n\
def e (α : Type) : Set α := ∅\n\
#check Set.empty\n"
    )
}

#[test]
fn notation_and_pointful_canvases_grade_identically() {
    let pointful = temp_file("pointful", &pointful_canvas());
    let notation = temp_file("notation", &notation_canvas());

    let (pointful_code, pointful_events) = grade_json(&pointful);
    let (notation_code, notation_events) = grade_json(&notation);

    assert_eq!(
        pointful_code, 0,
        "the pointful canvas must grade clean: {pointful_events:?}"
    );
    assert_eq!(
        notation_code, 0,
        "the notation canvas must grade clean: {notation_events:?}"
    );
    assert_eq!(
        counts(&pointful_events),
        counts(&notation_events),
        "the two spellings must produce identical five-way counts"
    );
    // 夹具必须真的判过东西（否则"计数相等"是 0 == 0 的假绿）。
    let (checked, _, _, typed, diagnostics) = counts(&notation_events);
    assert!(checked >= 3, "declarations must actually check: {checked}");
    assert!(typed >= 1, "the #check must actually type: {typed}");
    assert_eq!(diagnostics, 0, "{notation_events:?}");
}

#[test]
fn notation_emits_no_new_event_kinds() {
    // N6：记法命令不产生事件 ⇒ 事件种类集合**没有新增**。
    let notation = temp_file("kinds", &notation_canvas());
    let (code, events) = grade_json(&notation);
    assert_eq!(code, 0, "{events:?}");
    let mut kinds: Vec<&str> = events.iter().filter_map(|e| e["type"].as_str()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let allowed = [
        "decl.checked",
        "example.checked",
        "expr.typed",
        "expr.reduced",
        "decl.printed",
        "exercise.open",
        "diagnostic",
        "warning",
    ];
    for kind in &kinds {
        assert!(
            allowed.contains(kind),
            "the notation canvas must not invent an event kind: {kind:?} in {kinds:?}"
        );
    }
}

#[test]
fn the_pointful_spelling_keeps_working_and_the_moat_holds() {
    // 兼容策略 1：记法只是糖，点名形式一字不改。省 `α` 的点名写法**今天被内核
    // 拒绝，改后仍被拒绝**（设计 N4.3 的护城河）——同码同 stage。
    let src = format!("{LIB}def p (α : Type) (a : α) (A : Set α) : Prop := Set.mem a A\n");
    let path = temp_file("moat", &src);
    let (code, events) = grade_json(&path);
    assert_eq!(code, 1, "the moat must hold: {events:?}");
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(diags.len(), 1, "{events:?}");
    assert_eq!(diags[0]["code"], "kernel-rejected", "{:?}", diags[0]);
    assert_eq!(diags[0]["stage"], "kernel", "{:?}", diags[0]);
}

#[test]
fn undeclared_symbol_is_a_parse_diagnostic_with_a_teaching_hint() {
    // 未声明符号 ⇒ 专用 parse 诊断（**不是** unknown identifier），
    // hint 给「先声明」与「点名写法」两条出路。
    let src = format!("{LIB}def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n");
    let path = temp_file("undeclared", &src);
    let (code, events) = grade_json(&path);
    assert_eq!(code, 1, "{events:?}");
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(diags.len(), 1, "{events:?}");
    assert_eq!(
        diags[0]["code"], "notation-unknown-symbol",
        "{:?}",
        diags[0]
    );
    assert_eq!(diags[0]["stage"], "parse", "{:?}", diags[0]);
    let hint = diags[0]["hint"].as_str().unwrap_or_default();
    assert!(hint.contains("点名写法"), "hint: {hint}");
}

#[test]
fn notation_command_alone_is_a_clean_grade_with_checked_declarations() {
    // 复现件的判据（`scripts/gap.py:64-78`）：干净判卷 + 有 `decl.checked`。
    // 这里就是 `docs/gaps/repro/G04-notation.sokonanoda` 的形状。
    let src = "\
def mem (α : Type) (a : α) (s : α -> Prop) : Prop := s a

infix:50 \" ∈ \" => mem

def use (α : Type) (a : α) (A : α -> Prop) : Prop := a ∈ A
";
    let path = temp_file("repro", src);
    let (code, events) = grade_json(&path);
    assert_eq!(code, 0, "the repro shape must grade clean: {events:?}");
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(checked, 2, "both declarations check: {events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(diagnostics, 0, "{events:?}");
}

#[test]
fn notation_nullary_without_an_expected_type_reports_the_unsolved_code() {
    // `#check ∅`（无期望类型）⇒ `elab-notation-argument-unsolved`（设计 N4.2）。
    let src = format!("{LIB}notation \"∅\" => Set.empty\n#check ∅\n");
    let path = temp_file("unsolved", &src);
    let (code, events) = grade_json(&path);
    assert_eq!(code, 1, "{events:?}");
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(diags.len(), 1, "{events:?}");
    assert_eq!(
        diags[0]["code"], "elab-notation-argument-unsolved",
        "{:?}",
        diags[0]
    );
    assert_eq!(diags[0]["stage"], "elab", "{:?}", diags[0]);
}

#[test]
fn stdin_and_a_file_agree_on_a_notation_canvas() {
    // 记法走的是与文件路径同一条 parse/elab 流水线：stdin 与文件必须逐字节一致
    // （与既有 `stdin_matches_a_file` 同一纪律）。
    let src = notation_canvas();
    let path = temp_file("stdin", &src);
    let (file_code, file_events) = grade_json(&path);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["--json"])
        .env("SOKONANODA_CACHE_DIR", cache_dir("stdin"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(src.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    let stream = String::from_utf8_lossy(&out.stdout);
    let stdin_events: Vec<Value> = stream
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();

    assert_eq!(file_code, out.status.code().unwrap_or(-1));
    assert_eq!(file_events, stdin_events, "stdin and file must agree");
}

// ─────────────────────────────────────────────────────────────────────────
// TDD 第三层（课程用例）：真课程单元 + 记法变体必须判卷一致。
// ─────────────────────────────────────────────────────────────────────────

/// 课程单元的相对路径（卷 I 单元②：子集、空集与包含三律）。
const COURSE_UNIT: &str = "courses/set-theory/units/unit02-subsets-empty.sokonanoda";

/// 把课程单元改写成记法版：在**最后一个 import 之后**插入记法声明，并把两道
/// 练习的**签名**换成记法（`subset_trans` 的 `⊆`、`empty_subset` 的 `∅ ⊆ A`）。
///
/// 只动签名、不动 `sorry` 与注释：这一层的判据是"同一份课程换写法后判卷一致"，
/// 任何计数差异都必须是记法本身的语义 bug，而不是题目变了。
fn notation_variant(unit: &str) -> String {
    let mut lines: Vec<String> = unit.lines().map(str::to_string).collect();
    let last_import = lines
        .iter()
        .rposition(|line| line.starts_with("import "))
        .expect("the unit imports the course library");
    lines.splice(
        last_import + 1..last_import + 1,
        [
            String::new(),
            "infix:50 \" ∈ \" => Set.mem".to_string(),
            "infix:50 \" ⊆ \" => Set.subset".to_string(),
            "notation \"∅\" => Set.empty".to_string(),
        ],
    );
    let mut out = lines.join("\n");
    if unit.ends_with('\n') {
        out.push('\n');
    }
    for (pointful, notated) in [
        (
            "    (h1 : Set.subset α A B) (h2 : Set.subset α B C) : Set.subset α A C :=",
            "    (h1 : A ⊆ B) (h2 : B ⊆ C) : A ⊆ C :=",
        ),
        (
            "theorem empty_subset (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :=",
            "theorem empty_subset (α : Type) (A : Set α) : ∅ ⊆ A :=",
        ),
    ] {
        assert_eq!(
            out.matches(pointful).count(),
            1,
            "the fixture rewrite must hit exactly one declaration: {pointful}"
        );
        out = out.replace(pointful, notated);
    }
    out
}

/// 把课程单元（+ 课程库 + 清单）复制到临时目录：**课程树零改动**——记法变体
/// 只活在 `/tmp`，`courses/` 一个字节都不动。
fn stage_course_unit(tag: &str, text: &str) -> PathBuf {
    let src = repo_root().join("courses/set-theory");
    let dst = std::env::temp_dir().join(format!(
        "sokonanoda-notation-course-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(dst.join("units")).expect("mkdir units");
    copy_tree(&src.join("lib"), &dst.join("lib"));
    std::fs::copy(src.join("sokonanoda.toml"), dst.join("sokonanoda.toml")).expect("copy manifest");
    std::fs::write(dst.join("units/unit.sokonanoda"), text).expect("write unit");
    dst
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file_type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("copy");
        }
    }
}

#[test]
fn a_real_course_unit_grades_identically_in_notation() {
    // 第三层：不是玩具夹具，而是**真课程单元**。课程树本轮零改动，所以点出版
    // 就是仓库里那一份；记法版是同一份文本 + 记法声明 + 两处签名改写。
    let unit_path = repo_root().join(COURSE_UNIT);
    let unit = std::fs::read_to_string(&unit_path).expect("read course unit");
    let variant = notation_variant(&unit);

    let (pointful_code, pointful_events) = grade_json(&unit_path);
    let dir = stage_course_unit("variant", &variant);
    let (notation_code, notation_events) = grade_json(&dir.join("units/unit.sokonanoda"));

    assert_eq!(
        pointful_code, 0,
        "the shipped course unit must grade clean: {pointful_events:?}"
    );
    assert_eq!(
        notation_code, 0,
        "the notation variant of the course unit must grade clean: {notation_events:?}"
    );
    assert_eq!(
        counts(&pointful_events),
        counts(&notation_events),
        "the same course unit must grade identically in both spellings"
    );
    // 夹具必须真的判过东西（防"计数相等"退化成 0 == 0 的假绿）。
    let (checked, open, _, _, diagnostics) = counts(&notation_events);
    assert!(checked >= 1, "the unit must check declarations: {checked}");
    assert!(open >= 8, "the unit must still pose its exercises: {open}");
    assert_eq!(diagnostics, 0, "{notation_events:?}");
}

#[test]
fn the_shipped_course_still_uses_the_pointful_spelling() {
    // WO-011 的**课程零改动**契约：记法只是糖，本轮课程画布不重写（"记法版课程"
    // 是后续单独一轮的对照实验）。这一条把该契约钉住——真要改课程时，
    // 请连同本测试一起更新，并同时改 `docs/design/notation-subset.md`。
    let unit_path = repo_root().join(COURSE_UNIT);
    let unit = std::fs::read_to_string(&unit_path).expect("read course unit");
    for spelling in ["infix", "notation"] {
        assert!(
            !unit
                .lines()
                .any(|line| line.trim_start().starts_with(spelling)),
            "the shipped course must stay pointful this round: {spelling}"
        );
    }
    assert!(
        unit.contains("Set.subset α A C"),
        "the shipped unit keeps the pointful signature"
    );
}
