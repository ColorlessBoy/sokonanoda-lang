//! `namespace` / `end` / `open`（G-05，设计 `docs/design/namespace-open.md`）
//! 的 CLI 端到端。
//!
//! 判据（与 `notation.rs` 同款纪律：真二进制 + `--json` 事件流 + 退出码，
//! 不做文本比对）：
//!
//! * `namespace Set` 里的短名与外面的点名 `Set.mem` 是**同一个全局名**
//!   ⇒ 事件计数一致；
//! * **被导入模块**声明的 `Set.mem` 在入口文件里 `open Set` 之后短名可用
//!   （设计 N5 唯一需要额外钉住的一条）；
//! * 三条命令**零事件、零声明**（不是声明，与 `import`/记法同族）；
//! * `end` 错配 / 未闭合给**专用 parse 错误码**，退出码 1。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-namespace-cache-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-namespace-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &Path, relative: &str, text: &str) -> PathBuf {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(&path, text).expect("write file");
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

fn diagnostic_codes(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .map(|e| e["code"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn checked_names(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .map(|e| e["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// 课程库形状的依赖模块：**用 `namespace Set` 包起来**（声明名不带前缀）。
const SET_LIB: &str = "\
namespace Set\n\
def mem (α : Type) (a : α) (A : α -> Prop) : Prop := A a\n\
def subset (α : Type) (A B : α -> Prop) : Prop := forall (x : α), A x -> B x\n\
end Set\n";

/// 同款依赖，但在文件尾多写一行 `open Set`——用来钉「`open` 是**文件**作用域，
/// 不跨 `import`」（设计 N5）。
const SET_LIB_WITH_OPEN: &str = "\
namespace Set\n\
def mem (α : Type) (a : α) (A : α -> Prop) : Prop := A a\n\
end Set\n\
open Set\n";

#[test]
fn open_does_not_leak_out_of_the_module_that_wrote_it() {
    // N5：依赖模块里的 `open Set` 只对它自己那个文件有效；入口没有 `open`
    // 就必须报 `elab-unknown-identifier`（若泄漏，这一行会假绿）。
    let dir = temp_dir("open-no-leak");
    write(&dir, "Set.sokonanoda", SET_LIB_WITH_OPEN);
    let main = write(
        &dir,
        "Main.sokonanoda",
        "import Set\n\n\
         def use (α : Type) (a : α) (A : α -> Prop) : Prop := mem α a A\n",
    );
    let (code, events) = grade_json(&main);
    assert_eq!(code, 1, "the dependency's `open` must not leak: {events:?}");
    assert_eq!(
        diagnostic_codes(&events),
        vec!["elab-unknown-identifier".to_string()],
        "events: {events:?}"
    );
}

#[test]
fn open_reaches_a_declaration_from_an_imported_module() {
    // 设计 N5：`import` 不做模块限定、闭包级 `known` 是扁平的 ⇒ 入口里
    // `open Set` 对被导入模块声明的 `Set.mem` 必须有效（最常用的用法）。
    let dir = temp_dir("import-open");
    write(&dir, "Set.sokonanoda", SET_LIB);
    let main = write(
        &dir,
        "Main.sokonanoda",
        "import Set\n\n\
         open Set\n\
         def use (α : Type) (a : α) (A : α -> Prop) : Prop := mem α a A\n\
         def use2 (α : Type) (a : α) (A : α -> Prop) : Prop := Set.mem α a A\n",
    );
    let (code, events) = grade_json(&main);
    assert_eq!(code, 0, "events: {events:?}");
    assert_eq!(counts(&events).4, 0, "no diagnostics: {events:?}");
    // 入口的事件流只有入口自己的声明（依赖的 `Set.mem` 在被导入模块的报告里，
    // 见 `split_report`）；两条都能 checked 就说明 `open Set` 与点名都解析到了
    // **依赖模块**声明的那个全局名。
    let names = checked_names(&events);
    assert!(names.contains(&"use".to_string()), "names: {names:?}");
    assert!(names.contains(&"use2".to_string()), "names: {names:?}");
}

#[test]
fn namespace_commands_are_not_declarations() {
    // N6：三条命令零事件、零声明——只有 `Set.mem`/`Set.subset` 两条
    // `decl.checked`。
    let dir = temp_dir("no-events");
    let file = write(&dir, "Lib.sokonanoda", SET_LIB);
    let (code, events) = grade_json(&file);
    assert_eq!(code, 0, "events: {events:?}");
    assert_eq!(
        counts(&events),
        (2, 0, 0, 0, 0),
        "namespace/end must not produce events: {events:?}"
    );
    assert_eq!(
        checked_names(&events),
        vec!["Set.mem".to_string(), "Set.subset".to_string()]
    );
}

#[test]
fn end_mismatch_is_a_dedicated_parse_error_with_exit_1() {
    let dir = temp_dir("mismatch");
    let file = write(
        &dir,
        "Bad.sokonanoda",
        "namespace Foo\ndef x : Type := Prop\nend Bar\n",
    );
    let (code, events) = grade_json(&file);
    assert_eq!(code, 1, "events: {events:?}");
    assert_eq!(
        diagnostic_codes(&events),
        vec!["parse-namespace-mismatch".to_string()],
        "events: {events:?}"
    );
    assert_eq!(counts(&events).0, 0, "nothing may be checked: {events:?}");
}

#[test]
fn an_unclosed_namespace_is_a_dedicated_parse_error_with_exit_1() {
    let dir = temp_dir("unclosed");
    let file = write(
        &dir,
        "Bad.sokonanoda",
        "namespace Foo\ndef x : Type := Prop\n",
    );
    let (code, events) = grade_json(&file);
    assert_eq!(code, 1, "events: {events:?}");
    assert_eq!(
        diagnostic_codes(&events),
        vec!["parse-namespace-unclosed".to_string()],
        "events: {events:?}"
    );
}
