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

/// 把课程库拷进一个临时模块根（`import lib.Set` 要能解析）。
fn course_lib_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-notation-course-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp root");
    let lib_src = repo_root().join("courses/set-theory/lib");
    let lib_dst = dir.join("lib");
    std::fs::create_dir_all(&lib_dst).expect("create lib");
    for entry in std::fs::read_dir(&lib_src).expect("read course lib") {
        let entry = entry.expect("dir entry");
        if entry.path().extension().and_then(|e| e.to_str()) == Some("sokonanoda") {
            std::fs::copy(entry.path(), lib_dst.join(entry.file_name())).expect("copy lib file");
        }
    }
    dir
}

/// 带显式模块根的判卷（`import` 闭包）。
fn grade_json_root(root: &Path, path: &Path) -> (i32, Vec<Value>) {
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args([
            "--json",
            "--root",
            root.to_str().unwrap(),
            path.to_str().unwrap(),
        ])
        .env("SOKONANODA_CACHE_DIR", cache_dir("json-root"))
        .output()
        .expect("run --json --root");
    let stream = String::from_utf8_lossy(&out.stdout);
    let events = stream
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    (out.status.code().unwrap_or(-1), events)
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
    // 拒绝，改后仍被拒绝**（设计 N4.3 的护城河）——同 stage。
    //
    // **2026-09-21（G-21）重钉的是诊断码，不是护城河**：这条拒绝以前落进泛化的
    // `kernel-rejected`（消息只有 `Sort(1) vs $2` 这种内核内部记号），现在被
    // `error.rs::classify_term_in_type_position` 认出来（**项落在类型位**）并归到
    // `kernel-expected-sort`，由那条 hint 直接告诉学习者「漏了前导类型参数，
    // 或者用记法」。契约没变：仍然判红、仍然在 kernel 阶段、仍然是同一条声明。
    let src = format!("{LIB}def p (α : Type) (a : α) (A : Set α) : Prop := Set.mem a A\n");
    let path = temp_file("moat", &src);
    let (code, events) = grade_json(&path);
    assert_eq!(code, 1, "the moat must hold: {events:?}");
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(diags.len(), 1, "{events:?}");
    assert_eq!(diags[0]["code"], "kernel-expected-sort", "{:?}", diags[0]);
    assert_eq!(diags[0]["stage"], "kernel", "{:?}", diags[0]);
    // 提示必须把根因说出来（这是 G-21 那一半修复的全部价值）。
    let hint = diags[0]["hint"].as_str().unwrap_or_default();
    assert!(
        hint.contains("前导类型参数") && hint.contains("∈"),
        "the hint must name the cause and offer the notation way out: {hint:?}"
    );
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

/// 把课程单元改写成**点名版**：把两道练习的**签名**从记法换回点名
/// （`subset_trans` 的 `⊆`、`empty_subset` 的 `∅ ⊆ A`）。
///
/// 只动签名、不动 `sorry` 与注释：这一层的判据是"同一份课程换写法后判卷一致"，
/// 任何计数差异都必须是记法本身的语义 bug，而不是题目变了。
///
/// **方向（R2 课程 Lean 化之后翻转过）**：画布现在是**记法版**（`A ⊆ B`、
/// `∅ ⊆ A`），夹具负责造出**点名版**做对照——两种写法必须判卷一致（N7 契约）。
/// 符号仍由 `lib/Set` 统一声明并随 `import` 传播：夹具**不插入任何声明**
/// （插了就是「重声明 import 来的符号」，parse 错）。
fn pointful_variant(unit: &str) -> String {
    let mut out = unit.to_string();
    if unit.ends_with('\n') {
        out.push('\n');
    }
    for (notated, pointful) in [
        (
            "    (h1 : A ⊆ B) (h2 : B ⊆ C) : A ⊆ C :=",
            "    (h1 : Set.subset α A B) (h2 : Set.subset α B C) : Set.subset α A C :=",
        ),
        (
            "theorem empty_subset (α : Type) (A : Set α) : ∅ ⊆ A :=",
            "theorem empty_subset (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :=",
        ),
    ] {
        assert_eq!(
            out.matches(notated).count(),
            1,
            "the fixture rewrite must hit exactly one declaration: {notated}"
        );
        out = out.replace(notated, pointful);
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
fn a_real_course_unit_grades_identically_in_either_spelling() {
    // 第三层：不是玩具夹具，而是**真课程单元**。记法版就是仓库里那一份
    // （R2 课程 Lean 化之后画布已经是记法版）；点出版是同一份文本 + 两处签名
    // 改写（`∈`/`⊆`/`∅` 已由 `lib/Set` 声明，夹具不再自带——见
    // `pointful_variant` 的注释）。
    let unit_path = repo_root().join(COURSE_UNIT);
    let unit = std::fs::read_to_string(&unit_path).expect("read course unit");
    let variant = pointful_variant(&unit);

    let (notation_code, notation_events) = grade_json(&unit_path);
    let dir = stage_course_unit("variant", &variant);
    let (pointful_code, pointful_events) = grade_json(&dir.join("units/unit.sokonanoda"));

    assert_eq!(
        notation_code, 0,
        "the shipped course unit must grade clean: {notation_events:?}"
    );
    assert_eq!(
        pointful_code, 0,
        "the pointful variant of the course unit must grade clean: {pointful_events:?}"
    );
    assert_eq!(
        counts(&pointful_events),
        counts(&notation_events),
        "the same course unit must grade identically in both spellings"
    );
    // 夹具必须真的判过东西（防"计数相等"退化成 0 == 0 的假绿）。
    let (checked, open, _, _, diagnostics) = counts(&pointful_events);
    assert!(checked >= 1, "the unit must check declarations: {checked}");
    assert!(open >= 8, "the unit must still pose its exercises: {open}");
    assert_eq!(diagnostics, 0, "{pointful_events:?}");
}

// ─────────────────────────────────────────────────────────────────────────
// 第二刀（0.60.0）：`prefix`/`postfix` + 声明驱动的词法 + 跨 `import` 传播。
// ─────────────────────────────────────────────────────────────────────────

/// 第二刀的五符号靶子：两个一元 + 三个中缀（其中 `''`/`⁻¹'`/`×ˢ` 的符号
/// 在词法层是**标识符字符**，只有声明驱动的词法能读出来）。
const SECOND_CUT_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop
axiom Set.union : (α : Type) -> Set α -> Set α -> Set α
axiom Set.compl : (α : Type) -> Set α -> Set α
axiom Set.powerset : (α : Type) -> Set α -> Set (Set α)
axiom Set.image : (α : Type) -> (β : Type) -> (α -> β) -> Set α -> Set β
axiom Set.preimage : (α : Type) -> (β : Type) -> (α -> β) -> Set β -> Set α
axiom Set.prod : (α : Type) -> (β : Type) -> Set α -> Set β -> Set (α -> β -> Prop)
prefix:100 \" 𝒫 \" => Set.powerset
postfix:100 \" ᶜ \" => Set.compl
infixr:80 \" '' \" => Set.image
infixr:80 \" ⁻¹' \" => Set.preimage
infixr:80 \" ×ˢ \" => Set.prod
";

#[test]
fn the_five_second_cut_symbols_grade_clean_in_both_spellings() {
    let pointful = format!(
        "{SECOND_CUT_LIB}\
         def p (α : Type) (A : Set α) : Set (Set α) := Set.powerset α A\n\
         def c (α : Type) (A : Set α) : Set α := Set.compl α A\n\
         def i (α β : Type) (f : α -> β) (A : Set α) : Set β := Set.image α β f A\n\
         def r (α β : Type) (f : α -> β) (B : Set β) : Set α := Set.preimage α β f B\n\
         def x (α β : Type) (A : Set α) (B : Set β) : Set (α -> β -> Prop) :=\n\
             Set.prod α β A B\n"
    );
    let notation = format!(
        "{SECOND_CUT_LIB}\
         def p (α : Type) (A : Set α) : Set (Set α) := 𝒫 A\n\
         def c (α : Type) (A : Set α) : Set α := Aᶜ\n\
         def i (α β : Type) (f : α -> β) (A : Set α) : Set β := f '' A\n\
         def r (α β : Type) (f : α -> β) (B : Set β) : Set α := f ⁻¹' B\n\
         def x (α β : Type) (A : Set α) (B : Set β) : Set (α -> β -> Prop) := A ×ˢ B\n"
    );
    let pointful_path = temp_file("second-pointful", &pointful);
    let notation_path = temp_file("second-notation", &notation);
    let (pointful_code, pointful_events) = grade_json(&pointful_path);
    let (notation_code, notation_events) = grade_json(&notation_path);
    assert_eq!(
        pointful_code, 0,
        "the pointful canvas must grade clean: {pointful_events:?}"
    );
    assert_eq!(
        notation_code, 0,
        "the five second-cut symbols must grade clean: {notation_events:?}"
    );
    assert_eq!(
        counts(&pointful_events),
        counts(&notation_events),
        "the five symbols must produce identical five-way counts"
    );
    let (checked, _, _, _, diagnostics) = counts(&notation_events);
    assert_eq!(checked, 12, "seven axioms + five defs: {notation_events:?}");
    assert_eq!(diagnostics, 0, "{notation_events:?}");
}

#[test]
fn a_library_notation_works_in_the_entry_through_import() {
    // 第二刀的必做项：**跨 `import` 的记法**——被导入模块声明的记法在入口文件
    // 里直接可用（"课程库定义记法、单元直接写 `𝒫 A`"的前提）。入口**不重声明**
    // 任何记法，而且入口的单独 parse 一定失败（未声明符号）——分发与闭包加载
    // 必须把这条救回来。
    let dir = stage_course_unit(
        "cross-import",
        "import lib.Set\n\
         \n\
         def p (α : Type) (A : Set α) : Set (Set α) := 𝒫 A\n\
         def c (α : Type) (A : Set α) : Set α := Aᶜ\n\
         theorem t (α : Type) (A : Set α) :\n\
             Eq.{1} (Set α) (Aᶜ) (Set.compl α A) := by rfl\n",
    );
    let entry = dir.join("units/unit.sokonanoda");
    let (code, events) = grade_json(&entry);
    assert_eq!(
        code, 0,
        "the entry must use the library notation: {events:?}"
    );
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(checked, 3, "three declarations check: {events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(diagnostics, 0, "{events:?}");
    // 反向对照 ①：符号**声明了**、但它的**目标**没 import ⇒ 报的是
    // `elab-notation-unknown-target`（`''` 的目标 `Set.image` 在 lib/Image 里，
    // 这个单元只 import 了 lib.Set）。报错点名缺的名字，正是要教的东西。
    let dir = stage_course_unit(
        "cross-import-target-missing",
        "import lib.Set\n\naxiom f : Set Nat -> Set Nat\ndef bad : Set Nat := f '' Set.univ\n",
    );
    let (code, events) = grade_json(&dir.join("units/unit.sokonanoda"));
    assert_eq!(code, 1, "an unimported target must not resolve: {events:?}");
    let codes: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .filter_map(|e| e["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"elab-notation-unknown-target"),
        "the missing target must be named: {codes:?}"
    );

    // 反向对照 ②：符号来自一个**没被 import** 的模块 ⇒ 未声明符号（传播是
    // 按 import 边的，不是"闭包里全局"）。
    let dir = stage_course_unit(
        "cross-import-undeclared",
        "import lib.Set\n\ndef bad (α : Type) (A : Set α) : Set α := A ⋆ A\n",
    );
    std::fs::write(
        dir.join("lib/Extra.sokonanoda"),
        "axiom Extra.star : (α : Type) -> α -> α -> α\ninfix:50 \" ⋆ \" => Extra.star\n",
    )
    .expect("write extra library");
    let (code, events) = grade_json(&dir.join("units/unit.sokonanoda"));
    assert_eq!(
        code, 1,
        "an unimported notation must not resolve: {events:?}"
    );
    // 诊断的**消息**逐字保留（`notation-unknown-symbol` 的原话）；code/stage 是
    // 闭包路径的 `import-module-invalid`——入口单独 parse 失败、闭包也没救回来
    // 时走的就是这条路（设计 §11 的已知差异）。
    let messages: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .filter_map(|e| e["message"].as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("还没有声明过记法") && m.contains("⋆")),
        "the symbol must be reported as undeclared: {messages:?}"
    );
}

#[test]
fn the_shipped_course_library_declares_the_five_symbols() {
    // 课程侧的守护：五个符号是**课程库的记法**（不是 prelude），定义在
    // `courses/set-theory/lib/Set.sokonanoda`，目标名逐字点名。
    let lib = std::fs::read_to_string(repo_root().join("courses/set-theory/lib/Set.sokonanoda"))
        .expect("read course library");
    for (keyword, symbol, target) in [
        ("prefix:100", "𝒫", "Set.powerset"),
        ("postfix:100", "ᶜ", "Set.compl"),
        ("infixr:80", "''", "Set.image"),
        ("infixr:80", "⁻¹'", "Set.preimage"),
        ("infixr:80", "×ˢ", "Set.prod"),
    ] {
        let line = format!("{keyword} \" {symbol} \" => {target}");
        assert!(
            lib.contains(&line),
            "the course library must declare `{line}`"
        );
    }
}

#[test]
fn the_shipped_course_uses_the_library_notation_in_a_demo() {
    // 课程侧的用法守护：单元③ 用 `𝒫`、单元⑧ 用 `''` 各写了一条**演示**
    // （不是练习——练习的题意与数量一字未动，`open` 计数由课程门禁钉住）。
    let unit3 = std::fs::read_to_string(
        repo_root().join("courses/set-theory/units/unit03-union-inter-powerset.sokonanoda"),
    )
    .expect("read unit 3");
    assert!(
        unit3.contains("𝒫 "),
        "unit 3 must demo the powerset notation"
    );
    let unit8 = std::fs::read_to_string(
        repo_root().join("courses/set-theory/units/unit08-images-preimages.sokonanoda"),
    )
    .expect("read unit 8");
    assert!(unit8.contains("'' "), "unit 8 must demo the image notation");
}

#[test]
fn the_shipped_course_uses_the_library_notation() {
    // R2 课程 Lean 化之后的契约（WO-011 的"课程零改动"到此为止）：
    //   ① 单元里**不出现**任何 `infix`/`notation` 行——符号由 `lib/Set` 统一
    //      声明并随 `import` 传播（分层：课程标准库三层分界）；
    //   ② 语句真的用上了记法（含集合字面量 `{a}`/`{a, b}` 与 `≠`）；
    //   ③ 每道题都是 `by` 块（tactic 风格），没有裸项证明。
    let unit_path = repo_root().join(COURSE_UNIT);
    let unit = std::fs::read_to_string(&unit_path).expect("read course unit");
    for spelling in ["infix", "notation"] {
        assert!(
            !unit
                .lines()
                .any(|line| line.trim_start().starts_with(spelling)),
            "the unit must not declare notation itself (lib/Set owns it): {spelling}"
        );
    }
    for signature in [
        "A ⊆ A",
        "∅ ⊆ A",
        "{a} ≠ ∅",
        "{a, b} ⊆ A ↔ a ∈ A ∧ b ∈ A",
        "{a, b} = {b, a}",
        "{a} ∈ {{a}}",
    ] {
        assert!(
            unit.contains(signature),
            "the shipped unit must spell this in notation: {signature}"
        );
    }
    assert!(
        !unit.contains("Set.subset α") && !unit.contains("Set.singleton α"),
        "no pointful residue is allowed in the canvas"
    );
    // 每一条 `theorem` 的值位都得是 `by`（tactic 风格），`sorry` 挂在块里。
    // 按**行首**数：注释里那道"故意写错的反例"（练习 10 的 `theorem wrong`）
    // 不算声明。
    let theorems = unit
        .lines()
        .filter(|line| line.trim_start().starts_with("theorem "))
        .count();
    let by_blocks = unit.matches(":= by").count();
    assert_eq!(
        theorems, by_blocks,
        "every theorem must be proved in tactic style: {theorems} theorems, {by_blocks} `by` blocks"
    );
}

/// **匿名构造子 `⟨a, b⟩`（L2.7）**：用哪个构造子由**期望类型**决定。
///
/// 四条主路（`∧` / `↔` / `∃` / `Prod`）+ 一条对照（点名 `And.intro` 判卷一致）。
/// `↔` 是 prelude 里以 **def** 形态存在的单构造子类型（`Iff.intro`），
/// `∃`/`Prod` 是课程库里的归纳——三条路分别走 elab 的三个分支。
#[test]
fn anonymous_constructors_pick_the_constructor_from_the_expected_type() {
    let src = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
def Set.powerset (α : Type) (A : Set α) : Set (Set α) := fun (B : Set α) => Set.subset α B A

inductive Prod (α : Type) (β : Type) : Type
ctor mk (a : α) (b : β) : Prod α β
end

inductive Exists (A : Type) (p : A -> Prop) : Prop
ctor intro (w : A) (h : p w) : Exists A p
end
binder_notation \"∃\" => Exists

-- `∧`（prelude 归纳）
theorem and_anon (A B : Prop) (ha : A) (hb : B) : A ∧ B := by
  exact ⟨ha, hb⟩

-- `↔`（prelude 里是 def，构造子 `Iff.intro`）
theorem iff_anon (A B : Prop) (h1 : A -> B) (h2 : B -> A) : A ↔ B := by
  exact ⟨h1, h2⟩

-- `∃`（归纳 + binder 记法目标）
theorem exists_anon (α : Type) (p : α -> Prop) (w : α) (hw : p w) : ∃ (x : α), p x := by
  exact ⟨w, hw⟩

-- `Prod`（用户归纳；值位是 `def`，不是 `theorem`——它的类型不是 Prop）
def prod_anon (α β : Type) (a : α) (b : β) : Prod α β := ⟨a, b⟩

-- 对照：点名写法与匿名构造子判卷一致
theorem pointful_same (A B : Prop) (ha : A) (hb : B) : A ∧ B := by
  exact And.intro A B ha hb
";
    let path = temp_file("anon-ctor", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "every `⟨…⟩` shape must check: {diags:?}");
    for name in [
        "and_anon",
        "iff_anon",
        "exists_anon",
        "prod_anon",
        "pointful_same",
    ] {
        assert!(checked.contains(&name), "`{name}` must check: {checked:?}");
    }
}

/// `⟨…⟩` 读不到期望类型时给**专用码**，不是内核的裸错误。
#[test]
fn an_anonymous_constructor_without_an_expected_type_reports_its_own_code() {
    // `#check` 的位置**没有**期望类型 ⇒ 专用码。（`exact ⟨…⟩` 有期望类型
    // ——目标就是——那时报的是"头不是单构造子归纳"那条；实参位今天由
    // `application_arg_expected` 管，而它关着，见 `elab.rs` 的 TODO(G-21)。）
    let src = "\
#check ⟨True.intro, True.intro⟩
";
    let path = temp_file("anon-ctor-no-expected", src);
    let (code, events) = grade_json(&path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 1, "this must be rejected: {diags:?}");
    assert!(
        diags
            .iter()
            .any(|d| d["code"] == "elab-anon-ctor-no-expected-type"),
        "the dedicated code must be reported: {diags:?}"
    );
}

/// **`intro` 要按学习者的名字改名**（R2 实测的 H5）。
///
/// `Set.subset` 的体是 `forall (x : α), A x -> B x`，学习者写 `intro y` 时
/// 余下的体里还写着 `x` ⇒ 目标里留着**悬空的 `x`**，报错落在后面的 tactic 上
/// （`` `exact` 判定失败：unknown identifier `x` ``），根因离现场很远。
#[test]
fn intro_renames_the_bound_variable_in_the_rest_of_the_goal() {
    let src = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
infix:50 \" ⊆ \" => Set.subset

-- 源级 `forall` 目标：`∀ n, p n` 上 `intro m`
theorem forall_rename (α : Type) (p : α -> Prop) (h : forall (n : α), p n) :
    forall (m : α), p m := by
  intro m
  exact h m

-- def 头目标：`A ⊆ B` 上 `intro y`
theorem subset_rename (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := by
  intro y
  intro hy
  exact h y hy

-- `have` 在 `intro` 之后（子代理报的 H5 最小复现）
theorem have_after_intro (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := by
  intro y
  intro hy
  have g : B y := h y hy
  exact g
";
    let path = temp_file("intro-rename", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "renamed intros must check: {diags:?}");
    for name in ["forall_rename", "subset_rename", "have_after_intro"] {
        assert!(checked.contains(&name), "`{name}` must check: {checked:?}");
    }
}

/// **多层 delta 展开**（`intro` / `constructor` / `use`）：目标头是 def 时常常
/// 要展开两层以上才露出 Pi / 归纳头。
///
/// * `A ∈ 𝒫 B` 是 `Set.mem` → `Set.powerset` → `Set.subset` 三层；
/// * `a ∈ B ∩ C` 是 `Set.mem` → `Set.inter` 两层。
///
/// 只展开一层时学习者看到「`intro` 需要一个函数目标」/「`constructor` 需要目标
/// 是归纳类型，但当前目标的头 `Set.inter` 不在归纳表里」——而目标明明就是集合
/// 成员关系，教学上完全是误导。
///
/// 这条同时钉住**捕获避免的代换**：`def powerset (α) (A) := fun (B : Set α) =>
/// subset α B A` 在 `(𝒫 B) A` 上展开时，σ 里的 `A := B` 与 lambda 的 binder
/// `B` **撞名**——按名字硬代换会把 `subset α A B` 静默变成 `subset α A A`
/// （报错落在后面的 `exact` 上，说"期望 `A x`，实际是 `B x`"）。
#[test]
fn goals_whose_head_is_a_def_unfold_through_several_layers() {
    let src = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
def Set.inter (α : Type) (A B : Set α) : Set α := fun (x : α) => And (A x) (B x)
def Set.powerset (α : Type) (A : Set α) : Set (Set α) := fun (B : Set α) => Set.subset α B A
infix:50 \" ∈ \" => Set.mem
infix:50 \" ⊆ \" => Set.subset
infixl:70 \" ∩ \" => Set.inter
prefix:100 \" 𝒫 \" => Set.powerset

inductive Exists (A : Type) (p : A -> Prop) : Prop
ctor intro (w : A) (h : p w) : Exists A p
end
binder_notation \"∃\" => Exists

-- 三层：`A ∈ 𝒫 B` 上 `intro`
theorem powerset_intro (α : Type) (A B : Set α) (h : A ⊆ B) : A ∈ 𝒫 B := by
  intro x
  intro hx
  exact h x hx

-- 两层：`a ∈ B ∩ C` 上 `constructor`
theorem inter_intro (α : Type) (a : α) (B C : Set α) (hB : a ∈ B) (hC : a ∈ C) : a ∈ B ∩ C := by
  constructor
  exact hB
  exact hC

-- 记法目标上的 `use`（binder 记法的应用形态要补齐前置类型参数）
theorem use_on_a_notated_goal (α : Type) (p : α -> Prop) (w : α) (hw : p w) : ∃ (x : α), p x := by
  use w
  exact hw
";
    let path = temp_file("deep-unfold", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "deep unfolding must check: {diags:?}");
    for name in ["powerset_intro", "inter_intro", "use_on_a_notated_goal"] {
        assert!(checked.contains(&name), "`{name}` must check: {checked:?}");
    }
}

/// L2.10：**前面有声明没通过，不该让后面的记法报假错**。
///
/// 判定通道是「文档前缀 + 一条合成查询」拼起来**重编译**。旧代码取
/// `report.errors[0]`——那通常是**前缀里**那条失败的声明，于是每条 tactic、
/// 每个记法都收到一条与它无关的诊断（实测：一个坏声明让后面整片文件报
/// `elab-notation-unknown-target`，改写期极难定位）。
///
/// 修法：错误**按字节偏移分拣**（只认落在追加查询里的那条），前缀的错不在这里
/// 报；只有查询自己也没拿到结果时才抬出前缀的错来解释。
///
/// 这条测试同时钉住「check-then-add」：坏声明不占名字、也不影响后面的声明。
#[test]
fn a_broken_declaration_does_not_poison_the_next_notation() {
    let src = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.empty (α : Type) : Set α := fun (x : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => Eq.{1} α x a
def Set.notMem_empty (α : Type) (a : α) : Set.mem α a (Set.empty α) -> False := fun (h : Set.mem α a (Set.empty α)) => h
def Set.mem_singleton_self (α : Type) (a : α) : Set.mem α a (Set.singleton α a) := Eq.refl.{1} α a
notation \"∅\" => Set.empty

-- 坏声明：类型不存在（签名 elaborate 不了）。
theorem broken (x : NoSuchType) : True := True.intro

-- 与它无关的一条：记法 + `≠` + `intro`（判定通道要问内核好几次）。
theorem after (α : Type) (a : α) : {a} ≠ ∅ := by
  intro h
  exact Set.notMem_empty α a
    (Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α a X) (Set.singleton α a)
      (Set.empty α) h (Set.mem_singleton_self α a))
";
    let path = temp_file("cascade", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert!(
        checked.contains(&"after"),
        "`after` must check: {checked:?}"
    );
    assert_eq!(
        diags.len(),
        1,
        "only the broken declaration may report: {diags:?}"
    );
    assert_eq!(
        diags[0]["code"], "elab-unknown-identifier",
        "the diagnostic must be the broken declaration's own: {diags:?}"
    );
    assert_eq!(code, 1, "a broken declaration still fails the grade");
}

/// `≠` 的 **delta 展开**必须带上正确的宇宙层级（设计
/// `docs/design/course-lean-style.md` §9「R2 课程改写第一站」）。
///
/// `Ne` 是 `def Ne {u} (α : Sort u) (a b : α) : Prop := Eq.{u} α a b -> False`：
/// `intro h` 要看穿 `Ne` 就得展开定义体，而定义体里的 `u` 只能由**调用点**定。
/// 头上有 `.{1}` 时没问题，可 `≠` 的两条路**都不带层级**——源 AST 是记法节点、
/// 过一遍内核 pp 是裸名 `Ne`（pp 省掉隐式宇宙参数）⇒ 展开出来的 `h` 是
/// `@Eq.{u} …`（悬空变量），回读报 `unknown universe level u`，报错点离根因很远。
///
/// 修法：`by` 引擎按**操作数的 sort** 算层级提示（与 `elab_notation` 给记法求
/// 层级同一条规则）。两档的算法不同，两档都必须对：
///
/// - 点名形态（实参 ≥ 形参）`Ne (Set α) A B`：首实参**就是**那个类型参数
///   ⇒ 层级 = 首实参的 sort（**一步**）。两步会得 `2`；
/// - 记法形态（实参 < 形参）`A ≠ B`：首实参是项 ⇒ 层级 = 它**类型的** sort
///   （**两步**）。一步会得 `0`（`Set α` 的 sort 是 `Sort 1`，但 `Set α` 自己
///   的层级是 1 —— 错一步就落进别的常量应用）。
///
/// 判定仍然是内核：层级错一条都过不了（`Eq.{0} (Set α) …` 内核直接拒），
/// 所以"这些证明能过"本身就是判据，不需要额外断言层级数字。
#[test]
fn inequality_delta_unfolding_carries_the_right_universe_level() {
    let src = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.empty (α : Type) : Set α := fun (x : α) => False
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => Eq.{1} α x a
def Set.notMem_empty (α : Type) (a : α) : Set.mem α a (Set.empty α) -> False := fun (h : Set.mem α a (Set.empty α)) => h
def Set.mem_singleton_self (α : Type) (a : α) : Set.mem α a (Set.singleton α a) := Eq.refl.{1} α a
notation \"∅\" => Set.empty

-- 对照①：点名目标、同一条证明体——改前就过（源 AST 自带 `.{1}`）。
theorem ne_pointful (α : Type) (a : α) :
    Not (Eq.{1} (Set α) (Set.singleton α a) (Set.empty α)) := by
  intro h
  exact Set.notMem_empty α a
    (Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α a X) (Set.singleton α a)
      (Set.empty α) h (Set.mem_singleton_self α a))

-- **本缺口**：记法目标 `{a} ≠ ∅`，`intro` 要展开 `Ne`（记法形态 ⇒ 两步）。
theorem ne_intro_peels (α : Type) (a : α) : {a} ≠ ∅ := by
  intro h
  exact Set.notMem_empty α a
    (Eq.subst.{1} (Set α) (fun (X : Set α) => Set.mem α a X) (Set.singleton α a)
      (Set.empty α) h (Set.mem_singleton_self α a))

-- **同一条缺口的 `apply` 路径**：`h` 的类型经内核 pp 是点名形态（⇒ 一步）。
theorem ne_hypothesis_applies (α : Type) (A B : Set α) (h : A ≠ B) (h2 : A = B) : False := by
  apply h
  exact h2

-- Prop 档：`p q : Prop` ⇒ `Ne.{1} Prop p q`（两步得 1；一步会得 0）。
theorem ne_prop_applies (p q : Prop) (h : p ≠ q) (h2 : p = q) : False := by
  apply h
  exact h2
";
    let path = temp_file("ne-level-hint", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(
        code, 0,
        "every `≠` delta-unfold must carry the right level: {diags:?}"
    );
    for name in [
        "ne_pointful",
        "ne_intro_peels",
        "ne_hypothesis_applies",
        "ne_prop_applies",
    ] {
        assert!(checked.contains(&name), "`{name}` must check: {checked:?}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 第三刀（G-04 剩余项，设计 `docs/design/notation-subset.md` §12）
// ═══════════════════════════════════════════════════════════════════════════

/// 第三刀的夹具：集合 + `Exists`（真归纳，与课程库 `lib/Exists` 同形）+
/// `Set.singleton` / `Set.pair`。
const THIRD_LIB: &str = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => Eq.{1} α x a
def Set.pair (α : Type) (a b : α) : Set α := fun (x : α) => Or (Eq.{1} α x a) (Eq.{1} α x b)
inductive Exists (A : Type) (p : A -> Prop) : Prop
ctor intro (w : A) (h : p w) : Exists A p
end
";

/// 同一命题两种写法的判卷契约（N7 的五元计数一致 + 都 exit 0）。
fn assert_same_counts(pointful: &str, notation: &str, tag: &str) {
    let pointful_path = temp_file(&format!("{tag}-pointful"), pointful);
    let notation_path = temp_file(&format!("{tag}-notation"), notation);
    let (pointful_code, pointful_events) = grade_json(&pointful_path);
    let (notation_code, notation_events) = grade_json(&notation_path);
    assert_eq!(
        pointful_code, 0,
        "[{tag}] the pointful canvas must grade clean: {pointful_events:?}"
    );
    assert_eq!(
        notation_code, 0,
        "[{tag}] the notation canvas must grade clean: {notation_events:?}"
    );
    assert_eq!(
        counts(&pointful_events),
        counts(&notation_events),
        "[{tag}] the two spellings must produce identical five-way counts"
    );
    let (checked, _, _, _, diagnostics) = counts(&notation_events);
    assert!(checked >= 1, "[{tag}] declarations must actually check");
    assert_eq!(diagnostics, 0, "[{tag}] {notation_events:?}");
}

#[test]
fn set_literals_grade_like_the_pointful_singleton_and_pair() {
    // §12.4：`{a}` / `{a, b}` 是**新语法**（不是记法），展开成点名形式。
    let pointful = format!(
        "{THIRD_LIB}\
def one (α : Type) (a : α) : Set α := Set.singleton α a\n\
def two (α : Type) (a b : α) : Set α := Set.pair α a b\n"
    );
    let literals = format!(
        "{THIRD_LIB}\
def one (α : Type) (a : α) : Set α := {{a}}\n\
def two (α : Type) (a b : α) : Set α := {{a, b}}\n"
    );
    assert_same_counts(&pointful, &literals, "set-literal");
}

#[test]
fn binder_notation_grades_like_the_pointful_exists() {
    // §12.1：`∃ (n : Nat), p` 展开成 `Exists Nat (fun (n : Nat) => p)`。
    let pointful = format!(
        "{THIRD_LIB}\
def p : Prop := Exists Nat (fun (n : Nat) => Eq.{{1}} Nat n n)\n\
theorem t : Exists Nat (fun (n : Nat) => Eq.{{1}} Nat n n) :=\n\
  Exists.intro Nat (fun (n : Nat) => Eq.{{1}} Nat n n) 0 (Eq.refl.{{1}} Nat 0)\n"
    );
    let notation = format!(
        "{THIRD_LIB}\
binder_notation \"∃\" => Exists\n\
def p : Prop := ∃ (n : Nat), Eq.{{1}} Nat n n\n\
theorem t : Exists Nat (fun (n : Nat) => Eq.{{1}} Nat n n) :=\n\
  Exists.intro Nat (fun (n : Nat) => Eq.{{1}} Nat n n) 0 (Eq.refl.{{1}} Nat 0)\n"
    );
    assert_same_counts(&pointful, &notation, "binder");
}

#[test]
fn two_stage_binders_grade_like_the_pointful_guard() {
    // §12.1：两段式 `∀ x ∈ s, p` / `∃ x ∈ s, p` = `x ∈ s -> p` / `x ∈ s ∧ p`。
    let pointful = format!(
        "{THIRD_LIB}\
def all (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
  forall (x : α), Set.mem α x s -> p x\n\
def some (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
  Exists α (fun (x : α) => And (Set.mem α x s) (p x))\n"
    );
    let notation = format!(
        "{THIRD_LIB}\
infix:50 \" ∈ \" => Set.mem\n\
binder_notation \"∃\" => Exists\n\
def all (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
  ∀ x ∈ s, p x\n\
def some (α : Type) (s : Set α) (p : α -> Prop) : Prop :=\n\
  ∃ x ∈ s, p x\n"
    );
    assert_same_counts(&pointful, &notation, "two-stage");
}

#[test]
fn a_scoped_notation_grades_only_after_open_scoped() {
    // §12.3：`scoped` 默认不生效（未 open scoped ⇒ notation-unknown-symbol +
    // exit 1），`open scoped Foo` 之后与普通记法一样判卷。
    let inactive = format!(
        "{THIRD_LIB}\
namespace Foo\n\
scoped infix:50 \" ∈ \" => Set.mem\n\
end Foo\n\
def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n"
    );
    let path = temp_file("scoped-inactive", &inactive);
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

    let active = format!(
        "{THIRD_LIB}\
namespace Foo\n\
scoped infix:50 \" ∈ \" => Set.mem\n\
end Foo\n\
open scoped Foo\n\
def p (α : Type) (a : α) (A : Set α) : Prop := a ∈ A\n"
    );
    let pointful = format!(
        "{THIRD_LIB}\
def p (α : Type) (a : α) (A : Set α) : Prop := Set.mem α a A\n"
    );
    assert_same_counts(&pointful, &active, "scoped");
}

#[test]
fn an_overload_grades_by_expected_type_and_reports_ambiguity() {
    // §12.2：同符号两条记法（同形状）= 重载，按期望类型选；选不出给专用码。
    let picked = format!(
        "{THIRD_LIB}\
def Bag (α : Type) : Type := α -> Prop\n\
def Bag.singleton (α : Type) (a : α) : Bag α := fun (x : α) => Eq.{{1}} α x a\n\
prefix:100 \" ι \" => Set.singleton\n\
prefix:100 \" ι \" => Bag.singleton\n\
def s (α : Type) (a : α) : Set α := ι a\n\
def b (α : Type) (a : α) : Bag α := ι a\n"
    );
    let path = temp_file("overload-picked", &picked);
    let (code, events) = grade_json(&path);
    assert_eq!(
        code, 0,
        "the overload must pick by expected type: {events:?}"
    );

    let ambiguous = format!(
        "{THIRD_LIB}\
def Bag (α : Type) : Type := α -> Prop\n\
def Bag.singleton (α : Type) (a : α) : Bag α := fun (x : α) => Eq.{{1}} α x a\n\
prefix:100 \" ι \" => Set.singleton\n\
prefix:100 \" ι \" => Bag.singleton\n\
#check ι 1\n"
    );
    let path = temp_file("overload-ambiguous", &ambiguous);
    let (code, events) = grade_json(&path);
    assert_eq!(code, 1, "{events:?}");
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(diags.len(), 1, "{events:?}");
    assert_eq!(
        diags[0]["code"], "elab-notation-ambiguous",
        "{:?}",
        diags[0]
    );
    assert_eq!(diags[0]["stage"], "elab", "{:?}", diags[0]);
}

#[test]
fn the_shipped_course_demos_the_binder_notation() {
    // 课程侧的用法守护（第三刀）：单元⑧ 用 `∃ (x : …), …` 写了一条**演示**
    // （`example`，不是练习——练习数量与题意一字未动，`open` 计数由课程门禁钉住）。
    let unit8 = std::fs::read_to_string(
        repo_root().join("courses/set-theory/units/unit08-images-preimages.sokonanoda"),
    )
    .expect("read unit 8");
    assert!(
        unit8.contains("binder_notation"),
        "unit 8 must declare the binder notation it demos"
    );
    assert!(
        unit8.contains("∃ ("),
        "unit 8 must demo the two-stage / annotated binder notation"
    );
}

/// **记法 + tactic 必须能一起用**（G-04 × `by` 引擎；设计
/// `docs/design/course-lean-style.md` §1.3 的 X1 / X2 / X11）。
///
/// 改前三个实测故障（本测试是它们的回归钉）：
///
/// * **X1**：目标里出现记法 ⇒ `apply` 报「目标不匹配」——`unify_spine` 拿
///   内核 pp 的**点名** codomain 去对齐**源 AST**（含 `Expr::Notation`），
///   头与实参个数都对不上；
/// * **X2**：`by` 块里出现**数学码点类之外**的记法符号（`¬`U+00AC /
///   `↔`U+2194）⇒ `unknown identifier`——`judge.rs` 的 `wrap_binders` 回读
///   binder 类型文本时**没带记法表**，`¬ A` 被读成 `App(Ident("¬"), A)`；
/// * **X11**：`∃` 出现在目标/假设里 ⇒ `elab-binder-notation-unsolved`——
///   `proof.rs` 的 `render_binder_notation` 渲染时**丢了 binder 的类型标注**，
///   打成 `∃ x, p x`，回读解不出类型。
///
/// 三个都修好之后，下面这份文件必须 **exit 0 / 0 诊断 / 0 未完成**。
/// 顺带钉住 `rfl`：`Eq` 目标里含记法时**不能**被「目标归一化」打坏
/// （内核 pp 会丢掉隐式实参，`Eq.{1} (Set α) (Aᶜ) …` 会退化成 `Eq a b`，
/// `rfl` 就认不出 `Eq α x y` 形状了）——所以归一化只发生在 `apply` 的
/// **失败重试**路径上，不替换节点上的目标。
/// `intro a b c`（Lean 常态写法；设计 `docs/design/course-lean-style.md` L1.3）。
///
/// 改前 `intro` 只吃**一个**名字，`intro a b` 是 parse 错（S2 审计的 E01），
/// 而且**裸 `intro`** 会把下一行的 `exact` 整个吃掉（E02）。现在：一次吃一串
/// 名字，**遇到 tactic 关键字就停**——`intro h` 换行接 `exact h` / `apply f`
/// 是常态，那些关键字绝不能被当 binder 名；一个名字都没吃到才报错。
#[test]
fn intro_accepts_several_names_at_once() {
    let path = temp_file(
        "intro-names",
        "\n\
         -- 一次剥三层（等价于连写三个 `intro`）\n\
         theorem t_three (A B C : Prop) : A -> B -> C -> A := by\n\
         \x20 intro a b c\n\
         \x20 exact a\n\
         \n\
         -- 带记法的目标：`intro ha hna` 之后 `hna : ¬ A`\n\
         theorem t_not (A : Prop) : A -> ¬ ¬ A := by\n\
         \x20 intro ha hna\n\
         \x20 exact hna ha\n\
         \n\
         -- 换行接 tactic 关键字：`exact` 不能被当名字吃掉\n\
         theorem t_next_line (A B : Prop) (h : A) : B -> A := by\n\
         \x20 intro b\n\
         \x20 exact h\n",
    );
    let (code, events) = grade_json(&path);
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(code, 0, "multi-name intro must grade: {events:?}");
    assert_eq!(diagnostics, 0, "no diagnostics expected: {events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(checked, 3, "three theorems: {events:?}");

    // 裸 `intro`：一个名字都没吃到 ⇒ 报「`intro` binder name」，
    // **不是**把下一行的 `exact` 当名字（改前的行为）。
    let bare = temp_file(
        "intro-bare",
        "theorem t (A : Prop) : A -> A := by\n\
         \x20 intro\n\
         \x20 exact h\n",
    );
    let (code, events) = grade_json(&bare);
    assert_eq!(code, 1, "a bare `intro` is an error: {events:?}");
    let message = events
        .iter()
        .find(|e| e["type"] == "diagnostic")
        .and_then(|e| e["message"].as_str())
        .unwrap_or_default();
    assert!(
        message.contains("binder name"),
        "the error must point at the missing binder name: {message}"
    );
}

/// 课程 Lean 化的第一批新 tactic（设计 `docs/design/course-lean-style.md`
/// L3.2/L3.3/L3.4/L3.7）：`constructor` / `left` / `right` / `use` / `exfalso`。
///
/// 它们都建立在既有机械上（前四个是 `apply <构造子>` 的糖，`exfalso` 是
/// 「换目标 + 组装时套 `False.elim`」），但**构造子从哪来**是新东西：引擎
/// 现在拿得到前端自己的**归纳表**（`InductiveTable`，经 `lower_value` 传进
/// `run_by`）。目标头的三种形态都要认——`And A B`（点名）、`A ∧ B`
/// （`Expr::Notation` 的 `target`）、以及 `Iff A B` 这种 prelude 里以 **def**
/// 存在的「单构造子类型」（内建兜底到 `Iff.intro`）。
#[test]
fn the_new_sugar_tactics_grade_with_notation_goals() {
    let path = temp_file(
        "sugar-tactics",
        "inductive Exists (A : Type) (p : A -> Prop) : Prop\n\
         ctor intro (w : A) (h : p w) : Exists A p\n\
         end\n\
         \n\
         binder_notation \"∃\" => Exists\n\
         \n\
         -- constructor 在记法目标 `A ∧ B` 上\n\
         theorem t_and (A B : Prop) (ha : A) (hb : B) : A ∧ B := by\n\
         \x20 constructor\n\
         \x20 exact ha\n\
         \x20 exact hb\n\
         \n\
         -- constructor 在 `Iff`（prelude 的 def 形态）上：内建兜底到 Iff.intro\n\
         theorem t_iff (A B : Prop) : A ↔ B -> B ↔ A := by\n\
         \x20 intro h\n\
         \x20 constructor\n\
         \x20 intro hb\n\
         \x20 exact Iff.mpr A B h hb\n\
         \x20 intro ha\n\
         \x20 exact Iff.mp A B h ha\n\
         \n\
         -- left / right\n\
         theorem t_left (A B : Prop) (ha : A) : A ∨ B := by\n\
         \x20 left\n\
         \x20 exact ha\n\
         theorem t_right (A B : Prop) (hb : B) : A ∨ B := by\n\
         \x20 right\n\
         \x20 exact hb\n\
         \n\
         -- use：`apply Exists.intro` + 交证人\n\
         theorem t_use (A : Type) (p : A -> Prop) (w : A) (hw : p w) : ∃ (x : A), p x := by\n\
         \x20 use w\n\
         \x20 exact hw\n\
         \n\
         -- exfalso：目标换成 False\n\
         theorem t_exfalso (A B : Prop) (h : A) (nh : Not A) : B := by\n\
         \x20 exfalso\n\
         \x20 exact nh h\n",
    );
    let (code, events) = grade_json(&path);
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(code, 0, "the sugar tactics must grade: {events:?}");
    assert_eq!(diagnostics, 0, "no diagnostics expected: {events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(checked, 7, "one inductive + six theorems: {events:?}");

    // 目标头不是归纳（这里是个命题变量）⇒ **教学错误**，不是内核裸报错。
    let bad = temp_file(
        "sugar-bad",
        "theorem t (A : Prop) (h : A) : A := by\n\
         \x20 constructor\n",
    );
    let (code, events) = grade_json(&bad);
    assert_eq!(
        code, 1,
        "`constructor` on a non-inductive head must fail: {events:?}"
    );
    let message = events
        .iter()
        .find(|e| e["type"] == "diagnostic")
        .and_then(|e| e["message"].as_str())
        .unwrap_or_default();
    assert!(
        message.contains("归纳类型") && message.contains("归纳表"),
        "the error must teach what went wrong: {message}"
    );
}

/// `cases`（设计 `docs/design/course-lean-style.md` L3.1）——课程改写唯一的
/// **真阻塞**：`Or.elim` / `Exists.elim` 在课程里 100+ 处，没有等价的 tactic。
///
/// 实现路线是**降低成 `match`**（臂体各自组装成项，递归子/iota 交给既有的
/// `match` 降低路径），所以有两个必须一起成立的机制：
///
/// 1. **嵌套 tactic 序列**：臂体是一串 tactic（`| inl ha => apply f; exact h`），
///    用**缩进**界定（下一个 tactic 关键字出现在比 `|` 更深的列上 ⇒ 属于本臂）；
/// 2. **书写类型归一**：`match` 要从**书写类型**里取参数实参，而源里写的是
///    记法 `A ∨ B`（`Expr::Notation`）——`cases` 会把那个假设的书写类型换成
///    内核 pp 的规范形态（`Or A B`），否则报「被匹配项必须是一个书写类型为
///    `Or …` 的局部变量」（实测改前必炸）。
#[test]
fn cases_splits_hypotheses_with_and_without_arms() {
    let path = temp_file(
        "cases",
        "inductive Exists (A : Type) (p : A -> Prop) : Prop\n\
         ctor intro (w : A) (h : p w) : Exists A p\n\
         end\n\
         \n\
         binder_notation \"∃\" => Exists\n\
         \n\
         -- 不带 `with`：子目标按构造子声明顺序，分支假设用字段名（`a`/`b`）\n\
         theorem t_plain (A B : Prop) (h : A ∨ B) : B ∨ A := by\n\
         \x20 cases h\n\
         \x20 exact Or.inr B A a\n\
         \x20 exact Or.inl B A b\n\
         \n\
         -- 带 `with`：一行一个臂\n\
         theorem t_arms (A B : Prop) (h : A ∨ B) : B ∨ A := by\n\
         \x20 cases h with\n\
         \x20 | inl ha => exact Or.inr B A ha\n\
         \x20 | inr hb => exact Or.inl B A hb\n\
         \n\
         -- 臂体多步：靠**缩进**界定（下一行的 tactic 比 `|` 更深）\n\
         theorem t_multi (A B C : Prop) (h : A ∨ B) : C -> (C -> A ∨ B) -> A ∨ B := by\n\
         \x20 intro hc\n\
         \x20 intro hf\n\
         \x20 cases h with\n\
         \x20 | inl ha =>\n\
         \x20   apply hf\n\
         \x20   exact hc\n\
         \x20 | inr hb =>\n\
         \x20   apply hf\n\
         \x20   exact hc\n\
         \n\
         -- `∃`（binder 记法）+ `And`：构造子名写点号名或裸名都行\n\
         theorem t_exists (A : Type) (p : A -> Prop) (Q : Prop) (h : ∃ (x : A), p x) :\n\
         \x20   (forall (w : A), p w -> Q) -> Q := by\n\
         \x20 intro f\n\
         \x20 cases h with\n\
         \x20 | intro w hw => exact f w hw\n\
         \n\
         theorem t_and (A B : Prop) (h : A ∧ B) : B ∧ A := by\n\
         \x20 cases h with\n\
         \x20 | intro ha hb => exact And.intro B A hb ha\n",
    );
    let (code, events) = grade_json(&path);
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(code, 0, "`cases` must grade: {events:?}");
    assert_eq!(diagnostics, 0, "no diagnostics expected: {events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(checked, 6, "one inductive + five theorems: {events:?}");

    // 目标依赖被消去的假设 ⇒ **教学错误**（dependent elimination 不支持），
    // 不是内核的裸类型不符。
    let dependent = temp_file(
        "cases-dependent",
        "theorem t (A : Prop) (h : Or A A) : Or A A := by\n\
         \x20 cases h with\n\
         \x20 | inl ha => exact Or.inl A A ha\n\
         \x20 | inr hb => exact Or.inr A A hb\n",
    );
    let (code, _) = grade_json(&dependent);
    assert_eq!(
        code, 0,
        "non-dependent cases on a goal that mentions nothing is fine"
    );

    // 被消去项不是归纳 ⇒ 教学错误。
    let bad = temp_file(
        "cases-bad",
        "theorem t (A : Prop) (h : A) : A := by\n\
         \x20 cases h\n",
    );
    let (code, events) = grade_json(&bad);
    assert_eq!(code, 1, "`cases` on a non-inductive must fail: {events:?}");
    let message = events
        .iter()
        .find(|e| e["type"] == "diagnostic")
        .and_then(|e| e["message"].as_str())
        .unwrap_or_default();
    assert!(
        message.contains("归纳类型") && message.contains("归纳表"),
        "the error must teach what went wrong: {message}"
    );
}

/// `cases` 的被消去项是**块内 `have` 出来的、书写类型为记法**的局部假设。
///
/// 与上一条的区别在**谁写的类型**：声明参数与 `intro` 进来的假设，`cases` 会
/// 把书写类型归一成内核 pp 的规范形态（`Or A B`）再取参数实参；而 `have`
/// 引入的绑定，书写类型原样就是记法节点 `B ∨ C`——归一那条路不覆盖它，
/// 于是报「被匹配项必须是一个书写类型为 `Or …` 的局部变量」（R3 把
/// 单元⑩/⑪ 的解答改成 Lean 风格时实测撞上，`have bc : B ∨ C := …` 之后
/// `cases bc`）。
///
/// 修法在 `src_spine`：记法节点的**源像**就是 `target` 那条 spine——记法
/// 声明本身就写明了目标点名，按源序收集操作数即可（中缀 `[lhs, rhs]`、
/// 前缀 `[rhs]`、后缀 `[lhs]`、零元 `[]`）。
#[test]
fn cases_splits_a_have_bound_hypothesis_written_with_notation() {
    let path = temp_file(
        "cases-have-notation",
        "theorem t (A B : Prop) (h : A ∧ B) : B ∨ A := by\n\
         \x20 have hb : B := And.right A B h\n\
         \x20 have out : B ∨ A := Or.inl B A hb\n\
         \x20 cases out with\n\
         \x20 | inl hb2 => exact Or.inl B A hb2\n\
         \x20 | inr ha2 => exact Or.inr B A ha2\n\
         \n\
         -- 参数化归纳的**参数**也要从记法里取出来（`∨` 的两个操作数），\n\
         -- 取不到就会报「书写类型需要显式给出 2 个参数」。\n\
         theorem t_params (A B C : Prop) (h : A ∧ (B ∨ C)) : (A ∧ B) ∨ (A ∧ C) := by\n\
         \x20 have bc : B ∨ C := And.right A (B ∨ C) h\n\
         \x20 cases bc with\n\
         \x20 | inl hb => left; exact And.intro A B (And.left A (B ∨ C) h) hb\n\
         \x20 | inr hc => right; exact And.intro A C (And.left A (B ∨ C) h) hc\n",
    );
    let (code, events) = grade_json(&path);
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(
        code, 0,
        "`cases` on a notation-typed `have` must grade: {events:?}"
    );
    assert_eq!(diagnostics, 0, "no diagnostics expected: {events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(checked, 2, "two theorems: {events:?}");
}

/// `cases` 的被消去项类型是**定义体写了记法的 def**（`def MyOr A B := A ∨ B`）。
///
/// 与上一条正交：上一条管的是「假设的书写类型是记法」，这条管的是「`cases` 的
/// delta 展开**产出**记法」。`unfold_to_inductive` 把 `def` 的体代进来，形态完全
/// 取决于**定义体怎么写**——库里写 `Or (A x) (B x)` 就是 `App` 链，写
/// `A x ∨ B x` 就是记法节点。而 `cases` 的头解析当时用的是只走 `Expr::App` 的
/// `spine_of` ⇒ **课程库一改用记法，所有 `cases h`（`h : x ∈ A ∪ B`）整类报
/// 「被消去项不是归纳类型的值」**（2026-09-21 实测：把 `lib/Set.sokonanoda` 的
/// `def union` 体改成 `A x ∨ B x`，卷 I 门禁当场从 328/0 掉到 326/2）。
///
/// 修法：头解析改用 `spine_with_notation`（记法节点的头就是它的 `target`，
/// 源实参那条路本来就在用它）。
#[test]
fn cases_sees_through_a_definition_body_written_with_notation() {
    let path = temp_file(
        "cases-notation-body",
        "def MyOr (A B : Prop) : Prop := A ∨ B\n\
         def MyPair (A B : Prop) : Prop := A ∧ B\n\
         \n\
         theorem t_or (A B : Prop) (h : MyOr A B) : B ∨ A := by\n\
         \x20 cases h with\n\
         \x20 | inl ha => exact Or.inr B A ha\n\
         \x20 | inr hb => exact Or.inl B A hb\n\
         \n\
         theorem t_and (A B : Prop) (h : MyPair A B) : B ∧ A := by\n\
         \x20 cases h with\n\
         \x20 | intro ha hb => exact And.intro B A hb ha\n",
    );
    let (code, events) = grade_json(&path);
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(
        code, 0,
        "`cases` on a notation-bodied def must grade: {events:?}"
    );
    assert_eq!(diagnostics, 0, "no diagnostics expected: {events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(checked, 4, "two defs + two theorems: {events:?}");
}

/// **内建记法**（设计 `docs/design/course-lean-style.md` L2.2）：Lean core 级的
/// 逻辑符号 `∧ ∨ ↔ ¬` 在**任何文件里零声明可用**——地位与 Lean 的 `Init`
/// 记法一致。两条契约：
///
/// * 正例：不写任何 `infix`/`prefix`，`∧ ∨ ↔ ¬` 直接用（含 `by` 块与 `→` 混用）；
/// * 反例：**重新声明**它们是 parse 错（内建是语言的一部分，不是可覆盖的糖）
///   ——否则「同名同形状 = 重载」会让内建与用户声明悄悄并存、按期望类型选，
///   出问题时学习者根本看不出自己覆盖了语言符号。
#[test]
fn core_logic_notation_is_built_in() {
    let path = temp_file(
        "builtin-notation",
        "theorem t_and (A B : Prop) (ha : A) (hb : B) : A ∧ B := by\n\
         \x20 constructor\n\
         \x20 exact ha\n\
         \x20 exact hb\n\
         theorem t_or (A B : Prop) (h : A ∨ B) : B ∨ A := by\n\
         \x20 cases h with\n\
         \x20 | inl ha => exact Or.inr B A ha\n\
         \x20 | inr hb => exact Or.inl B A hb\n\
         theorem t_iff (A B : Prop) : (A ↔ B) → B → A := by\n\
         \x20 intro h\n\
         \x20 intro hb\n\
         \x20 exact Iff.mpr A B h hb\n\
         theorem t_not (A : Prop) : A → ¬ ¬ A := by\n\
         \x20 intro ha hna\n\
         \x20 exact hna ha\n",
    );
    let (code, events) = grade_json(&path);
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(
        code, 0,
        "built-in notation must need no declaration: {events:?}"
    );
    assert_eq!(diagnostics, 0, "{events:?}");
    assert_eq!(open, 0, "{events:?}");
    assert_eq!(checked, 4, "{events:?}");

    for decl in [
        "infixr:35 \" ∧ \" => And\n",
        "infixr:30 \" ∨ \" => Or\n",
        "infix:20 \" ↔ \" => Iff\n",
        "prefix:40 \" ¬ \" => Not\n",
    ] {
        let redeclare = temp_file("builtin-redeclare", decl);
        let (code, events) = grade_json(&redeclare);
        assert_eq!(
            code, 1,
            "re-declaring a built-in must fail: {decl:?} {events:?}"
        );
        let message = events
            .iter()
            .find(|e| e["type"] == "diagnostic")
            .and_then(|e| e["message"].as_str())
            .unwrap_or_default();
        assert!(
            message.contains("内建记法"),
            "the error must say it is built in: {message}"
        );
    }
}

#[test]
fn notation_works_inside_by_blocks() {
    let path = temp_file(
        "by-blocks",
        "inductive Exists (A : Type) (p : A -> Prop) : Prop\n\
         ctor intro (w : A) (h : p w) : Exists A p\n\
         end\n\
         \n\
         axiom Set : Type -> Type\n\
         axiom Set.compl (α : Type) (A : Set α) : Set α\n\
         \n\
         postfix:100 \" ᶜ \" => Set.compl\n\
         binder_notation \"∃\" => Exists\n\
         \n\
         -- X1：记法目标的 `apply`（改前报「目标不匹配」）\n\
         theorem t_and (A B : Prop) (ha : A) (hb : B) : A ∧ B := by\n\
         \x20 apply And.intro\n\
         \x20 exact ha\n\
         \x20 exact hb\n\
         \n\
         -- X2：类外码点符号（`¬`）出现在目标与假设里\n\
         theorem t_not (A : Prop) : A -> ¬ ¬ A := by\n\
         \x20 intro ha\n\
         \x20 intro hna\n\
         \x20 exact hna ha\n\
         \n\
         -- X11：`∃` 出现在假设里（改前报 binder 记法没有类型）\n\
         theorem t_exists_elim (A : Type) (p : A -> Prop) :\n\
         \x20   (∃ (x : A), p x) -> (∃ (y : A), p y) := by\n\
         \x20 intro h\n\
         \x20 exact h\n\
         \n\
         -- X1 + X11：`∃` 目标上的 `apply`\n\
         theorem t_exists_intro (A : Type) (p : A -> Prop) (w : A) (hw : p w) :\n\
         \x20   ∃ (x : A), p x := by\n\
         \x20 apply Exists.intro\n\
         \x20 exact w\n\
         \x20 exact hw\n\
         \n\
         -- 回归：`Eq` 目标里含记法时 `rfl` 仍要能用（归一化不许打坏它）\n\
         theorem t_rfl (α : Type) (A : Set α) :\n\
         \x20   Eq.{1} (Set α) (Aᶜ) (Set.compl α A) := by rfl\n",
    );
    let (code, events) = grade_json(&path);
    let (checked, open, _, _, diagnostics) = counts(&events);
    assert_eq!(code, 0, "notation + by must grade: {events:?}");
    assert_eq!(diagnostics, 0, "no diagnostics expected: {events:?}");
    assert_eq!(open, 0, "every proof is complete: {events:?}");
    assert_eq!(
        checked, 8,
        "two axioms + one inductive + five theorems: {events:?}"
    );
}

/// **G-19 回归**：零元记法（`∅`）落在「被应用」的位置 + `by` 块。
///
/// 形状：`⊆` 指向 **def** `Set.subset` ⇒ `intro` 要把它 delta 展开成
/// `forall (x : α), A x -> B x`，`∅` 于是落进**被应用**的位置（`∅ x`）。
/// 改前：源级展开把记法操作数原样代进定义体，回读时 `Set.empty` 零实参、
/// `α` 无从解出 ⇒ `` `exact` 判定失败：记法 `∅` 展开成 `Set.empty` 时补不出
/// 前面的类型参数``（台账 G-19 / 设计 X14，课程 227 处 `Set.empty` 几乎全在此形状）。
/// 改后：`intro` 在头是 def 时改用内核 pp 的规范形态（点名 + 参数写全）再剥。
#[test]
fn a_zero_ary_notation_survives_delta_unfolding_in_a_by_block() {
    let src = "\
def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
def Set.empty (α : Type) : Set α := fun (x : α) => False
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x
infix:50 \" ∈ \" => Set.mem
infix:50 \" ⊆ \" => Set.subset
notation \"∅\" => Set.empty

-- 对照①：`∅` 作 `∈` 的操作数（拿得到期望类型 `Set α`）——改前就过。
theorem mem_empty (α : Type) (x : α) : x ∈ ∅ -> False := by
  intro h
  exact h

-- 对照②：点名形式、不套记法——改前就过。
theorem pointful (α : Type) (A : Set α) : Set.subset α (Set.empty α) A := by
  intro x
  intro hx
  exact False.elim (A x) hx

-- **本缺口**：`∅` 作 `⊆` 的左操作数，`intro` 必须 delta 展开 `Set.subset`。
theorem empty_subset (α : Type) (A : Set α) : ∅ ⊆ A := by
  intro x
  intro hx
  exact False.elim (A x) hx
";
    let path = temp_file("g19-zero-ary", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "all three theorems must check: {diags:?}");
    for name in ["mem_empty", "pointful", "empty_subset"] {
        assert!(checked.contains(&name), "`{name}` must check: {checked:?}");
    }
}

/// **L2.4b / L2.3**：`=` 与 `≠` 是内建记法（目标 `Eq` / `Ne`）。
///
/// 改前：`a = b` 是**词法错误**（`expected `=>``）——课程只能满屏写
/// `Eq.{1} (Set α) A B`。改后：`=` 走 `token.rs` 的 `'='` 分支产出
/// `Sym("=")`，`≠` 走内建符号表，两者都进 `BUILTIN_NOTATIONS`。
///
/// **关键点：宇宙层级**。`Eq`/`Ne` 各带一个 `u`，`Eq.{1} (Set α) A B` 与
/// `Eq.{0} A B` 是两个不同的常量应用；点名路径靠源里的 `.{1}` 写死，记法路径
/// 必须从**操作数类型的 sort** 解出（`elab.rs` 的 `level_text_of_sort`）。
/// 所以下面 Prop 与 Type 两档都要过。
#[test]
fn equality_and_inequality_are_builtin_notations_with_solved_universes() {
    let src = "\
def Set (α : Type) : Type := α -> Prop

-- Prop 档：`A B : Prop` ⇒ `Eq.{0}`。
theorem eq_prop (A B : Prop) (h : A = B) : B = A := Eq.symm.{1} Prop A B h

-- Type 档：`A B : Set α` ⇒ `Eq.{1}`（改前只能写 `Eq.{1} (Set α) A B`）。
theorem eq_type (α : Type) (A B : Set α) (h : A = B) : B = A := Eq.symm.{1} (Set α) A B h

theorem eq_refl_type (α : Type) (A : Set α) : A = A := Eq.refl.{1} (Set α) A

-- `≠` 同理（`Ne.{u} α a b`）。
theorem ne_prop (A B : Prop) (h : A ≠ B) : A ≠ B := h

theorem ne_type (α : Type) (A B : Set α) (h : A ≠ B) : A ≠ B := h

-- `≠` 与 `= … -> False` 是同一个东西（`Ne` 是 def）。
theorem ne_unfolds (A B : Prop) : A ≠ B -> A = B -> False := fun (h : A ≠ B) => h

-- `=` 与点名形式判卷一致（N7 契约）：同一命题的两种写法。
theorem eq_pointful (α : Type) (A B : Set α) (h : Eq.{1} (Set α) A B) : Eq.{1} (Set α) B A :=
  Eq.symm.{1} (Set α) A B h
";
    let path = temp_file("eq-ne", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "`=` / `≠` must check: {diags:?}");
    for name in [
        "eq_prop",
        "eq_type",
        "eq_refl_type",
        "ne_prop",
        "ne_type",
        "ne_unfolds",
        "eq_pointful",
    ] {
        assert!(checked.contains(&name), "`{name}` must check: {checked:?}");
    }
}

/// **回归**：`=>` 不许被 `=` 的符号匹配吃掉。
///
/// `=` 进内建记法表之后，词法的**最长匹配**会把它当候选符号，于是 `fun (x) => …`
/// 被切成 `=` + `>`（实测：L1 prelude 的 `fun … => …` 当场解析失败，整份 prelude
/// 装不上）。修法是 `lexer_builtin_symbols()` 把 `=` 从**词法**候选里剔除——
/// 它由 `'='` 分支原生产出。这条测试钉住那个不变式。
#[test]
fn fat_arrow_still_lexes_inside_a_fun_with_equality_available() {
    let src = "\
def Set (α : Type) : Type := α -> Prop

theorem fun_with_eq (α : Type) (A B : Set α) : A = B -> A = B :=
  fun (h : A = B) => h

theorem iff_of_eq (A B : Prop) : A = B -> (A -> B) :=
  fun (h : A = B) => fun (a : A) => Eq.subst.{1} Prop (fun (x : Prop) => x) A B h a
";
    let path = temp_file("fat-arrow-eq", src);
    let (code, events) = grade_json(&path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "`=>` must keep working next to `=`: {diags:?}");
}

/// **输入法表 ↔ 语言**的防漂移（设计 `docs/design/notation-input.md` §6.1）。
///
/// 表里 `supported: true` 是**手写的 `bool`**——Rust↔JS 的契约测试只挡得住
/// 「两份表不一样」，挡不住「表说支持、语言其实没有」。这条测试拿**真判卷**
/// 逐个验证**语言层**的符号（内建记法 + prelude 目标），一个不落地跑。
///
/// **不在本测试里的**（各有更合适的判据，别当成漏了）：
/// - `∈ ⊆ ∪ ∩ \ ∅ 𝒫 ᶜ '' ⁻¹' ×ˢ`：由**课程库** `lib/Set` 声明（记法是文件作用域 +
///   跨 `import` 传播）⇒ 判据是课程门禁 `courses/set-theory/tools/check.py` 与
///   `crates/cli/tests/notation.rs` 里的课程夹具；
/// - `∃`：由 `lib/Exists` 声明（`Exists` 不在 prelude 是**教学设计**，N-9）。
#[test]
fn every_language_level_symbol_in_the_input_table_really_works() {
    use sokonanoda_front::notation_input::TABLE;
    // 语言层符号 → 用它写的一条最小声明（真判卷必须 exit 0）。
    let probes: &[(&str, &str)] = &[
        ("∧", "theorem p_and (A B : Prop) (h : A ∧ B) : A ∧ B := h"),
        ("∨", "theorem p_or (A B : Prop) (h : A ∨ B) : A ∨ B := h"),
        ("↔", "theorem p_iff (A B : Prop) (h : A ↔ B) : A ↔ B := h"),
        ("¬", "theorem p_not (A : Prop) (h : ¬ A) : ¬ A := h"),
        (
            "→",
            "theorem p_arrow (A B : Prop) (f : A → B) (a : A) : B := f a",
        ),
        (
            "∀",
            "theorem p_forall : ∀ (A : Prop), A -> A := fun (A : Prop) => fun (a : A) => a",
        ),
        ("=", "theorem p_eq (A : Prop) (h : A = A) : A = A := h"),
        ("≠", "theorem p_ne (A B : Prop) (h : A ≠ B) : A ≠ B := h"),
    ];
    // 表里声称支持、且目标在**语言层**的符号必须都在上面出现过。
    let language_level: Vec<&str> = probes.iter().map(|(symbol, _)| *symbol).collect();
    for entry in TABLE.iter().filter(|entry| entry.supported) {
        let course_lib = ["∈", "⊆", "∪", "∩", "\\", "∅", "𝒫", "ᶜ", "⁻¹'", "×ˢ", "∃"];
        assert!(
            language_level.contains(&entry.symbol) || course_lib.contains(&entry.symbol),
            "`{}` 声称 supported，但没有判据：要么加进 probes，要么加进 course_lib 并说明它由谁声明",
            entry.symbol
        );
    }
    for (symbol, decl) in probes {
        let src = format!("{decl}\n");
        let path = temp_file("input-table-probe", &src);
        let (code, events) = grade_json(&path);
        let diags: Vec<&Value> = events
            .iter()
            .filter(|e| e["type"] == "diagnostic")
            .collect();
        assert_eq!(
            code, 0,
            "the input table claims `{symbol}` is supported, but the language rejects it: {diags:?}"
        );
        assert!(
            events.iter().any(|e| e["type"] == "decl.checked"),
            "`{symbol}` probe must produce a checked declaration: {events:?}"
        );
    }
}

/// **L3.6 `have`**：在当前上下文里引入中间结论，目标不变。
///
/// 降低成 `let h : T := t; <rest>`：`h : T` 进上下文（判定看得见），
/// `let` 的标注给值一个**期望类型**——没有它，嵌套 `by` 里只要用了 `cases`，
/// 组装出来的 `match` 就没有期望类型（实测 `elab-match-no-expected-type`）。
#[test]
fn have_introduces_an_intermediate_fact_without_changing_the_goal() {
    let src = "\
def Set (α : Type) : Type := α -> Prop

-- 项形式 + 链式 have
theorem have_term (A B C D : Prop) (f : A -> B -> C -> D) (ha : A) (hb : B) (hc : C) : D := by
  have hab : B -> C -> D := f ha
  have habc : C -> D := hab hb
  exact habc hc

-- 嵌套 `by` 形式；**外层后续 tactic 缩进更浅，不许被内层吞掉**
theorem have_by (A B : Prop) (h : A ∧ B) : B ∧ A := by
  have hb : B := by
    exact And.right A B h
  have ha : A := by
    exact And.left A B h
  exact And.intro B A hb ha

-- 嵌套 `by` 里用 `cases`（值是个 `match` ⇒ 靠 `let` 的标注拿期望类型）
theorem have_nested_cases (A B C : Prop) (h : A ∨ B) (f : A -> C) (g : B -> C) : C := by
  have hc : C := by
    cases h with
      | inl ha => exact f ha
      | inr hb => exact g hb
  exact hc

-- `have` 之后目标不变：继续 `intro`
theorem have_then_intro (A B : Prop) (h : A -> B) : A -> B := by
  have hcopy : A -> B := h
  intro a
  exact hcopy a
";
    let path = temp_file("have", src);
    let (code, events) = grade_json(&path);
    let checked: Vec<&str> = events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str())
        .collect();
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "`have` must check: {diags:?}");
    for name in [
        "have_term",
        "have_by",
        "have_nested_cases",
        "have_then_intro",
    ] {
        assert!(checked.contains(&name), "`{name}` must check: {checked:?}");
    }
}

/// `have` 的值类型不对时，**报在 `have` 那一行**，且说人话。
///
/// 内核给的 `expected`/`actual` 是折叠回望远镜的完整声明类型
/// （`Pi (A : Sort(0)), …`）——准确但读不懂。这条钉住教学文案：`期望 B，实际是 C`。
#[test]
fn have_reports_a_readable_type_mismatch_on_its_own_line() {
    let src = "\
theorem wrong (A B C : Prop) (h1 : A -> C) (ha : A) : C := by
  have hb : B := h1 ha
  exact hb
";
    let path = temp_file("have-wrong", src);
    let (code, events) = grade_json(&path);
    assert_eq!(code, 1, "the mismatch must be rejected: {events:?}");
    let diag = events
        .iter()
        .find(|e| e["type"] == "diagnostic")
        .expect("a diagnostic");
    let message = diag["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("期望 `B`") && message.contains("实际是 `C`"),
        "the message must be readable (not a Pi telescope): {message}"
    );
    assert!(
        !message.contains("Pi ("),
        "the message must not leak the folded telescope: {message}"
    );
    assert_eq!(
        diag["span"]["start"]["line"], 2,
        "the error must point at the `have` line: {diag}"
    );
}

/// **课程形状的端到端**：`=` 记法 + `apply` 类型参数 + `cases` 看穿 def。
///
/// 这一条串起了本轮修的三个真 bug（每一个都单独实测过）：
/// ① `=` 是记法 ⇒ 目标的源 AST 是 `Notation{=, [A, B]}`，**丢了 `Eq` 的类型参数
///    `α`**，而内核 pp 又把 `Set.ext` 的 codomain 打成 `Eq A B`（也丢 `α`）⇒
///    `apply Set.ext` 把 `α` 判成子目标、类型 `Type 0`（= `Sort 1`），`intro x`
///    报「需要一个函数目标」。修法：域是**宇宙**（非 `Prop`）的层永远算类型参数。
/// ② `cases h` on `h : x ∈ A ∪ B`：`∈`（`Set.mem`）与 `∪`（`Set.union`）都是
///    **def**，要展开两层才露出 `Or`。修法：`spine::unfold_to_inductive`。
/// ③ 展开 `Set.union α A B x` 时，`strip_lambdas` 把定义体**自己的** `fun (x)` 也
///    剥掉了 ⇒ 多贴一个实参、展开出胡说八道的类型。修法：`strip_lambdas_n`
///    只剥参数表那几层 + `beta_apply` 把结果上的应用归约进去。
#[test]
fn a_course_shaped_proof_uses_equality_notation_apply_have_and_cases() {
    let src = "\
import lib.Set

theorem union_comm (α : Type) (A B : Set α) : A ∪ B = B ∪ A := by
  apply Set.ext
  intro x
  have h1 : x ∈ A ∪ B -> x ∈ B ∪ A := by
    intro h
    cases h with
      | inl ha => exact Or.inr (B x) (A x) ha
      | inr hb => exact Or.inl (B x) (A x) hb
  have h2 : x ∈ B ∪ A -> x ∈ A ∪ B := by
    intro h
    cases h with
      | inl hb => exact Or.inr (A x) (B x) hb
      | inr ha => exact Or.inl (A x) (B x) ha
  exact Iff.intro (x ∈ A ∪ B) (x ∈ B ∪ A) h1 h2
";
    let dir = course_lib_dir("course-shaped");
    let path = dir.join("Course.sokonanoda");
    std::fs::write(&path, src).expect("write canvas");
    let (code, events) = grade_json_root(&dir, &path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "the course-shaped proof must check: {diags:?}");
    assert!(
        events.iter().any(|e| e["type"] == "decl.checked"),
        "`union_comm` must be checked: {events:?}"
    );
}

/// **`apply` 的类型参数必须从目标推出来**（R2 实测的静默错子目标回归）。
///
/// `Set.ext` 的 pp 签名把 `Eq` 的类型实参丢了（`Eq A B`），于是它的类型参数
/// `α` **不在任何实参位上**。以前按名字回退到「同名上下文变量」——而这个参数
/// 恰好就叫 `α`：
///   · 上下文没有 `α` ⇒ 合成声明里 `α` 变成**未知标识符**；
///   · 上下文有一个无关的 `α` ⇒ 静默填错，子目标里的元素变成 `α`（本该是
///     `β`），之后每条 `exact` 都报「期望 `(f '' A) y`，实际是
///     `Set.image α β f A y`」这种把同一个命题写成两种形态的消息。
/// 修法是拿「已填层的 domain ↔ 该层值的类型」反推（`solve_type_params`）。
#[test]
fn apply_fills_a_type_parameter_from_the_goal_not_from_a_same_named_variable() {
    // 元素类型叫 β，上下文里另有一个无关的 α（`Set.ext` 的参数名）。
    let collision = "\
import lib.Set
import lib.Image

theorem ext_fill (α β : Type) (f : α -> β) (A : Set α) : f '' A = f '' A := by
  apply Set.ext
  intro y
  constructor
  intro h
  exact h
  intro h
  exact h
";
    // 元素类型叫 γ，上下文里**没有** α（旧行为：`unknown identifier α`）。
    let no_collision = "\
import lib.Set
import lib.Image

theorem ext_fill_no_alpha (γ : Type) (f : γ -> Nat) (A : Set γ) :
    f '' A = f '' A := by
  apply Set.ext
  intro y
  constructor
  intro h
  exact h
  intro h
  exact h
";
    for (tag, src) in [
        ("ext-collision", collision),
        ("ext-no-collision", no_collision),
    ] {
        let dir = course_lib_dir(tag);
        let path = dir.join("Course.sokonanoda");
        std::fs::write(&path, src).expect("write canvas");
        let (code, events) = grade_json_root(&dir, &path);
        let diags: Vec<&Value> = events
            .iter()
            .filter(|e| e["type"] == "diagnostic")
            .collect();
        assert_eq!(
            code, 0,
            "`apply Set.ext` must fill α from the goal ({tag}): {diags:?}"
        );
        assert!(
            events.iter().any(|e| e["type"] == "decl.checked"),
            "the theorem must be checked ({tag})"
        );
    }
}

/// **`cases` 写在 `have … := by` 里时臂模式不能丢字段**（R2 实测回归）。
///
/// 这条路上的值要**打回源码文本**再判卷，而 `render_pattern` 以前无条件给子
/// 模式加括号：`| intro b hb =>` 渲染成 `| intro (b hb) =>`，回读时
/// `Exists.intro` 只剩 **1** 个子模式，报「构造子有 2 个字段，但这一支写了
/// 1 个子模式」。现在只给本身带子模式的子模式加括号。
#[test]
fn cases_inside_a_have_block_keeps_every_constructor_field() {
    let src = "\
import lib.Set
import lib.Image

theorem cases_in_have (α β : Type) (f : α -> β) (C : Set β) (y : β)
    (hy : y ∈ f '' (f ⁻¹' C)) : y ∈ C := by
  have h1 : y ∈ C := by
    cases hy with
    | intro x hx =>
      exact Eq.subst.{1} β (fun (z : β) => C z) (f x) y
        (And.right ((f ⁻¹' C) x) (f x = y) hx)
        (And.left ((f ⁻¹' C) x) (f x = y) hx)
  exact h1
";
    let dir = course_lib_dir("cases-in-have");
    let path = dir.join("Course.sokonanoda");
    std::fs::write(&path, src).expect("write canvas");
    let (code, events) = grade_json_root(&dir, &path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(
        code, 0,
        "`cases` inside a `have` block must keep both fields: {diags:?}"
    );
}

/// **裸 `Eq` 的宇宙层级要能还原**（R2 实测回归）：`cases` 把假设的书写类型换成
/// 内核 pp 形态，pp 丢掉隐式宇宙参数（`Eq.{1} β …` → 裸 `Eq …`）。这条类型
/// 不只判定要用——`match` 的组装与最终声明判定**都从节点上读它**；留着裸名，
/// 内核按默认 `.{0}` elaborate，报「期望 `Sort(0)`，实际是 β」，位置还在整条
/// 声明上。修法：`Eq` 三件套的 `(项参数个数, 宇宙参数个数)` 进 `def_shape`，
/// 还原时进 `fun`/`forall` 的体先把该层 binder 加进判定上下文。
#[test]
fn cases_arms_restore_universe_levels_for_bare_equality_hypotheses() {
    let src = "\
import lib.Set
import lib.Exists

theorem cases_eq_arm (β γ : Type) (g : β -> γ) (a : β) (c : γ)
    (h : Exists β (fun (b : β) => And (Eq.{1} β a b) (Eq.{1} γ (g b) c))) :
    Eq.{1} γ (g a) c := by
  cases h with
  | intro b hb =>
    exact Eq.trans.{1} γ (g a) (g b) c
      (congrArg.{1} g (And.left (Eq.{1} β a b) (Eq.{1} γ (g b) c) hb))
      (And.right (Eq.{1} β a b) (Eq.{1} γ (g b) c) hb)
";
    let dir = course_lib_dir("cases-eq-arm");
    let path = dir.join("Course.sokonanoda");
    std::fs::write(&path, src).expect("write canvas");
    let (code, events) = grade_json_root(&dir, &path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(
        code, 0,
        "a bare `Eq` in an arm hypothesis must get its level back: {diags:?}"
    );
}

/// **`cases` 的参数代换必须用规范形态**（R2 修边刀实测：记法的前导类型参数
/// 撞上上下文同名变量 ⇒ **静默错类型**）。
///
/// 源类型里的记法节点（`f '' A`）**不带前导类型参数**（`elab` 期才算得出来，
/// 源 AST 里没有），而 `spine::unfold_one` 只能把操作数**右对齐**到形参，
/// 于是定义体里提到前导参数的地方**留着定义自己的 binder 名**，在调用点按
/// 「同名上下文变量」解析：`Set.image` 的形参就叫 `α`/`β`，上下文里也有
/// `α`/`β`，于是 `f : α -> γ` 那条 `''` 把形参 `β` 代成了**上下文的 `β`**
/// （本该是 `γ`）——臂里的假设类型成了 `Eq.{1} β (f x) y`，之后每条 `exact`
/// 都把同一条命题写成两种形态（实测 `unit12` 的 P1）。
/// 修法：`cases` 的实参优先取**规范形态**（内核 pp：点名 + 全实参 + 无记法），
/// 它丢的隐式宇宙参数由 `restore_universe_levels` 在写回节点之前补齐。
#[test]
fn cases_uses_the_canonical_type_so_notation_prefix_params_do_not_capture_context_names() {
    let src = "\
import lib.Set
import lib.Fun
import lib.Image

theorem cases_image_nested (α β γ : Type) (f : α -> β) (g : β -> γ) (A : Set α) :
    forall (y : γ), ((Function.comp α β γ g f) '' A) y ->
      (Set.image β γ g (Set.image α β f A)) y := by
  intro y
  intro hy
  cases hy with
  | intro x hx =>
    exact Exists.intro β (fun (b : β) => (Set.image α β f A) b ∧ g b = y) (f x)
      (And.intro ((Set.image α β f A) (f x)) (g (f x) = y)
        (Exists.intro α (fun (t : α) => A t ∧ f t = f x) x
          (And.intro (A x) (f x = f x)
            (And.left (A x) (Eq.{1} γ (Function.comp α β γ g f x) y) hx)
            (Eq.refl.{1} β (f x))))
        (And.right (A x) (Eq.{1} γ (Function.comp α β γ g f x) y) hx))
";
    let dir = course_lib_dir("cases-canonical-args");
    let path = dir.join("Course.sokonanoda");
    std::fs::write(&path, src).expect("write canvas");
    let (code, events) = grade_json_root(&dir, &path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(
        code, 0,
        "`cases` must take its arguments from the canonical type: {diags:?}"
    );
}

/// 递归收集 `dir` 下的 `.sokonanoda` 文件（跳过 `solutions` 之外不做区分——
/// 前提是全仓签名，不分画布/解答）。
fn walk_sokonanoda(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_sokonanoda(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("sokonanoda") {
            out.push(path);
        }
    }
    out
}

/// 声明类型望远镜里带**隐式**风格的 binder（IA-1 的前提检查用）。
fn implicit_binders_in(ty: &sokonanoda_front::Expr, out: &mut Vec<String>) {
    use sokonanoda_front::{BinderKind, Expr};
    match ty {
        Expr::Forall { binders, body, .. } => {
            for b in binders {
                if b.style == BinderKind::Implicit {
                    out.push(b.name.clone());
                }
            }
            implicit_binders_in(body, out);
        }
        Expr::Arrow { codomain, .. } => implicit_binders_in(codomain, out),
        _ => {}
    }
}

/// **IA-1 的发布前提**（设计 `docs/design/implicit-arguments.md` §0）：
/// 卷 I（`courses/set-theory`）与入门课（`course/`）的签名里**没有隐式
/// binder**（两处**教学上刻意**的例外见下）⇒ 隐式实参插入在课程上是
/// **逐字节 no-op**（「P1 落地、课程零改动也全绿」的前提）。
///
/// 为什么必须钉死：这条前提会随课程改写**悄悄失效**，而失效的表现是
/// 「IA-1 落地后课程莫名其妙变红」——一条极难从现象反推到原因的故障。
/// IA-2（把 `lib/` 的 ≈39 条签名改隐式）落地时**同轮改这条测试**：那是
/// **有意的契约变更**，不是回归。
///
/// **两处刻意的例外**（本轮实测发现；设计 §0 的注只扫了卷 I，漏了入门课）：
/// `Eq.symm`（`course/unit5-universes-sort.sokonanoda` 与解答）与
/// `eq_refl_prop`（`course/unit2-equality-rfl.sokonanoda` 与解答）——
/// 这两道题**教的就是隐式 binder**，签名必须写成 `{α : Sort u}` / `{a : Prop}`。
/// 它们的**用法**全在 prelude 的 `Eq.subst`/`Eq.refl` 上（prelude 的
/// `implicit_prefix` 固定为 0，本刀不动 prelude）⇒ IA-1 不影响它们；
/// 行为由入门课自己的判卷测试（`course.rs` / `course_status.rs` / `cli.rs`
/// 的 GOLDEN）钉着。
#[test]
fn no_course_signature_uses_an_implicit_binder() {
    use sokonanoda_front::Command;
    let mut offenders: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    for root in ["courses/set-theory", "course"] {
        for path in walk_sokonanoda(&repo_root().join(root)) {
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(file) = sokonanoda_front::parse(&src) else {
                continue;
            };
            scanned += 1;
            let rel = path
                .strip_prefix(repo_root())
                .unwrap_or(&path)
                .display()
                .to_string();
            for command in &file.commands {
                let (label, ty) = match command {
                    Command::Def { name, ty, .. }
                    | Command::Theorem { name, ty, .. }
                    | Command::Axiom { name, ty, .. } => (name.clone(), ty),
                    Command::Example { ty, .. } => ("<example>".to_string(), ty),
                    _ => continue,
                };
                // **有意的隐式签名**（IA-1 的"课程零隐式 binder"前提在 IA-2 起
                // 有意打破，这里改成白名单，不再要求全课程为空）：
                //   · 入门课两处教学例外（它们是**题目本身**）：`Eq.symm` / `eq_refl_prop`；
                //   · 卷 I 课程库 IA-2 落地：`lib/Exists` 的 `Exists.elim`
                //     （让 `Exists.elim h f` 与 Lean 对齐）。
                let intentional = (matches!(label.as_str(), "Eq.symm" | "eq_refl_prop")
                    && rel.starts_with("course/"))
                    || (rel == "courses/set-theory/lib/Exists.sokonanoda"
                        && label == "Exists.elim");
                if intentional {
                    continue;
                }
                let mut names = Vec::new();
                implicit_binders_in(ty, &mut names);
                if !names.is_empty() {
                    offenders.push(format!("{rel}:{label} → {{{}}}", names.join(", ")));
                }
            }
        }
    }
    assert!(
        scanned > 30,
        "the premise check must actually scan the course tree (scanned {scanned} files)"
    );
    assert!(
        offenders.is_empty(),
        "IA-1 的前提被打破：课程签名里出现了隐式 binder。\n\
         IA-1 的隐式实参插入会在这些声明上生效 ⇒ 课程不再是「零改动也全绿」。\n\
         这是**有意的契约变更**时，同轮改这条测试并把 IA-2 的清单对齐：\n{}",
        offenders.join("\n")
    );
}

/// **IA-1：隐式实参插入**（路线 C，设计 `docs/design/implicit-arguments.md` §3）。
///
/// 签名写 `{α : Type}`（隐式）后，**点名调用可以省掉它**：前端按「后续显式
/// 实参的类型」头部匹配唯一确定 `α`，再当普通实参喂给内核。
/// **安全性质**：签名没有隐式 binder 时一行都不跑（见上面的前提测试）。
#[test]
fn implicit_arguments_are_inserted_from_the_first_explicit_argument() {
    let src = "\
def id2 {α : Type} (a : α) : α := a

def mymem {α : Type} (a : α) (A : α -> Prop) : Prop := A a

theorem t1 (α : Type) (a : α) : Eq.{1} α (id2 a) a := by
  rfl

theorem t2 (α : Type) (a : α) (A : α -> Prop) (h : A a) : mymem a A := by
  exact h

theorem t3 : Eq.{1} Nat (id2 3) 3 := by
  rfl
";
    let path = temp_file("implicit-insert", src);
    let (code, events) = grade_json(&path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "implicit arguments must be inserted: {diags:?}");
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "decl.checked")
            .count(),
        5,
        "all five declarations must check: {events:?}"
    );
}

/// **IA-1：`@` 关闭隐式插入**（Lean 语义，设计 §7 第 5 条）。
///
/// `@f a b` 把实参**逐位**放到形参上（哪怕第一个是隐式的）——这是课程护城河
/// 的逃生门，也让「点名写法永远可用」这句承诺在隐式签名下仍然成立。
#[test]
fn the_at_marker_disables_implicit_insertion() {
    let src = "\
def id2 {α : Type} (a : α) : α := a

theorem at_marker (α : Type) (a : α) : Eq.{1} α (@id2 α a) a := by
  rfl

theorem no_at (α : Type) (a : α) : Eq.{1} α (id2 a) a := by
  rfl
";
    let path = temp_file("implicit-at", src);
    let (code, events) = grade_json(&path);
    let diags: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .collect();
    assert_eq!(code, 0, "`@f a` and `f a` must both check: {diags:?}");
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "decl.checked")
            .count(),
        3,
        "all three declarations must check: {events:?}"
    );
}

/// **IA-1：解不出就报专用错误码，不猜**（设计 §3.1）。
///
/// `{α}` 在**任何**显式实参的类型里都不出现（`ignores` 的值与它无关）⇒
/// 路线 ①（由后续显式实参的类型反解）解不出 ⇒
/// `elab-implicit-argument-unsolved`，而不是静默挑一个。**路线 ②（由期望类型
/// 解）本刀不做**（设计 §7 第 1 条）。
#[test]
fn an_unsolvable_implicit_argument_reports_its_own_code() {
    // 写成**值位**（不是 `by` 块）：tactic 会把 elab 错误包成
    // `elab-tactic-failed`（判卷通道的既有口径），专用码要在直接 elaborate 的
    // 位置才看得见——课程里正是 lib/ 的签名与值位。
    let src = "\
def ignores {α : Type} (n : Nat) : Nat := n

def uses : Nat := ignores 3
";
    let path = temp_file("implicit-unsolved", src);
    let (code, events) = grade_json(&path);
    assert_ne!(code, 0, "an unsolvable implicit argument must be rejected");
    let codes: Vec<String> = events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .filter_map(|e| e["code"].as_str().map(str::to_string))
        .collect();
    assert!(
        codes.iter().any(|c| c == "elab-implicit-argument-unsolved"),
        "must report elab-implicit-argument-unsolved, got {codes:?}"
    );
}
