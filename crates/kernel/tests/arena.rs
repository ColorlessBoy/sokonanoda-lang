use sokonanoda::pretty_printer::PpOptions;
use sokonanoda::util::Config;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use stumpalo::Arena;

const MAX_EXPORT_BYTES: u64 = 64 * 1024 * 1024;

fn arena_root() -> Option<PathBuf> { std::env::var_os("LEAN_KERNEL_ARENA").map(PathBuf::from) }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ExpectedOutcome {
    Accept,
    Reject,
    Either,
}

fn expected_outcome(root: &Path, stem: &str) -> Option<ExpectedOutcome> {
    // **按 stem 递归找**（T-K02，2026-09-24）：原来只认平铺的
    // `tests/<stem>.yaml` ⇒ 语料里 `tests/perf/*`、`tests/corner-cases/*` 那些
    // yaml **完全不可见**，于是它们对应的导出即便被收集到也过不了这一关。
    let spec_text = find_yaml(&root.join("tests"), stem)?;
    spec_text.lines().find_map(|l| l.strip_prefix("outcome:")).map(|v| match v.trim() {
        "accept" => ExpectedOutcome::Accept,
        "reject" => ExpectedOutcome::Reject,
        "either" => ExpectedOutcome::Either,
        other => panic!("bad outcome {other:?} for {stem}"),
    })
}

/// 在 `dir` 下**递归**找 `<stem>.yaml`，返回内容。
fn find_yaml(dir: &Path, stem: &str) -> Option<String> {
    let entries = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
            continue;
        }
        if path.file_stem().map(|s| s == stem).unwrap_or(false)
            && path.extension().map(|e| e == "yaml").unwrap_or(false)
        {
            return fs::read_to_string(&path).ok();
        }
    }
    subdirs.iter().find_map(|sub| find_yaml(sub, stem))
}

/// 体积上限：默认 64MB，可用 `LEAN_KERNEL_ARENA_MAX_BYTES` 放宽
/// （语料里的 `init`(309MB)/`std`(526MB) 就是这样被挡住的）。
fn max_export_bytes() -> u64 {
    std::env::var("LEAN_KERNEL_ARENA_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(MAX_EXPORT_BYTES)
}

/// 收集用例，**递归**扫 `_build/tests`（T-K02）。
///
/// 返回 `(收集到的用例, 因体积跳过数, 因缺 yaml 跳过数)` —— 三个数都要报出来：
/// 原来"只收集到 1 条"是**静默**的（不递归 + 体积 + yaml 平铺三件事叠在一起），
/// 于是"语料对拍"看起来在跑、其实只有一个用例，是**安慰剂**。
pub(crate) fn collect_cases(root: &Path, out: &mut Vec<(PathBuf, ExpectedOutcome)>) -> (usize, usize) {
    let cap = max_export_bytes();
    let mut skipped_big = 0usize;
    let mut skipped_spec = 0usize;
    let mut exports = Vec::new();
    walk_exports(&root.join("_build/tests"), &mut exports);
    for path in exports {
        if fs::metadata(&path).map(|m| m.len() > cap).unwrap_or(true) {
            skipped_big += 1;
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        match expected_outcome(root, &stem) {
            Some(expected) => out.push((path, expected)),
            None => skipped_spec += 1,
        }
    }
    (skipped_big, skipped_spec)
}

/// 递归收集 `*.ndjson`（任意深度）。
fn walk_exports(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_exports(&path, out);
        } else if path.extension().map(|e| e == "ndjson").unwrap_or(false) {
            out.push(path);
        }
    }
}

enum Outcome {
    Accepted,
    ParseError(String),
    KernelRejected(String),
    UnexpectedPanic(String),
}

fn run_case(export: PathBuf) -> Outcome {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || run_case_inner(export))
        .expect("spawn case thread")
        .join()
        .unwrap_or_else(|_| Outcome::UnexpectedPanic("thread join failed".to_string()))
}

fn run_case_inner(export: PathBuf) -> Outcome {
    let cfg = Config {
        export_file_path: Some(export),
        use_stdin: false,
        permitted_axioms: None,

        permit_standard_axioms: false,
        unpermitted_axiom_hard_error: false,
        parse_only: false,
        num_threads: 1,
        nat_extension: true,
        string_extension: true,
        pp_declars: None,
        unknown_pp_declar_hard_error: false,
        pp_options: PpOptions::default(),
        pp_output_path: None,
        pp_to_stdout: false,
        print_success_message: false,
        print_axioms: false,
        unsafe_permit_all_axioms: true,
    };
    let global_arena = Arena::new();
    let ef = match cfg.to_export_file(global_arena.as_arena_ref()) {
        Ok((ef, _)) => ef,
        Err(e) => return Outcome::ParseError(format!("{e}")),
    };
    let result = panic::catch_unwind(AssertUnwindSafe(|| ef.check_all_declars()));
    match result {
        Ok(()) => Outcome::Accepted,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "panic".to_string()
            };
            if msg == "def_eq failed" || msg.starts_with("def_eq") {
                Outcome::KernelRejected(msg)
            } else {
                Outcome::UnexpectedPanic(msg)
            }
        }
    }
}

#[test]
fn arena_fast_tier() {
    let Some(root) = arena_root() else {
        eprintln!("arena_fast_tier: LEAN_KERNEL_ARENA is not set; skipping");
        return;
    };
    let mut cases = Vec::new();
    let (skipped_big, skipped_spec) = collect_cases(&root, &mut cases);
    cases.sort();
    // **把"没收集到什么"也报出来**（T-K02）：原来"只收集到 1 条"是完全静默的，
    // 于是这条"语料对拍"看起来在跑、其实只有一个用例。
    eprintln!(
        "arena_fast_tier: {} 条用例（按体积跳过 {skipped_big}，缺 outcome yaml 跳过 {skipped_spec}；\
         体积上限 {} 字节，可用 LEAN_KERNEL_ARENA_MAX_BYTES 放宽）",
        cases.len(),
        max_export_bytes()
    );
    assert!(!cases.is_empty(), "no arena cases found under {}", root.display());

    if std::env::var("LEAN_KERNEL_ARENA_VERBOSE").is_err() {
        panic::set_hook(Box::new(|_| {}));
    }

    let mut failures = Vec::new();
    for (export, expected) in &cases {
        let outcome = run_case(export.clone());
        let (got_accept, detail) = match &outcome {
            Outcome::Accepted => (true, "(accepted)".to_string()),
            Outcome::ParseError(e) => (false, format!("parse: {e}")),
            Outcome::KernelRejected(e) => (false, format!("kernel: {e}")),
            Outcome::UnexpectedPanic(e) => (false, format!("panic: {e}")),
        };
        let mismatch = is_mismatch(
            *expected,
            got_accept,
            matches!(outcome, Outcome::UnexpectedPanic(_)),
            matches!(outcome, Outcome::KernelRejected(_)),
        );
        if mismatch {
            let name = export.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let expected = match expected {
                ExpectedOutcome::Accept => "accept",
                ExpectedOutcome::Reject => "reject",
                ExpectedOutcome::Either => "either",
            };
            failures.push(format!("{name}: expected {expected}, got accept={got_accept} ({detail})"));
        }
    }

    if !failures.is_empty() {
        panic!("{}/{} arena cases mismatched:\n{}", failures.len(), cases.len(), failures.join("\n"));
    }
}

/// **判负语义**（T-K02 修，2026-09-24）——抽成纯函数，好在**不依赖外部语料**的
/// 情况下把它钉死：
///   * `reject` 要求的是**内核拒绝**（`def_eq failed`）。原来写 `got_accept`
///     ⇒ **解析失败也算通过** ✗：一条根本读不进来的导出，会被当成"成功地拒绝了
///     它想拒绝的东西"——最危险的一种假绿；
///   * `either` 是"接受或拒绝都行"，但 **panic 永远不行**。原来写死 `false`
///     ⇒ 连 panic 都不算失败 ✗。
fn is_mismatch(expected: ExpectedOutcome, accepted: bool, panicked: bool, kernel_rejected: bool) -> bool {
    match expected {
        ExpectedOutcome::Accept => !accepted || panicked,
        ExpectedOutcome::Reject => !kernel_rejected || panicked,
        ExpectedOutcome::Either => panicked,
    }
}

/// **T-K02 的判据（收集逻辑）**：不依赖外部语料，用临时目录搭一个**小语料**，
/// 断言三件事——① 子目录里的导出**收得到**；② 子目录里的 outcome yaml
/// **找得到**（按 stem 递归）；③ 三个计数如实报出来。
///
/// 为什么必须有这条：修之前"只收集到 1 条"是**静默**的（不递归 + 体积 + yaml
/// 平铺三件事叠在一起），只有真语料在手才看得出来；这条测试把三件事各自钉住。
#[test]
fn collect_cases_walks_subdirectories_and_finds_nested_specs() {
    let dir = std::env::temp_dir().join(format!("soko-arena-collect-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("_build/tests/perf")).expect("mkdir build");
    fs::create_dir_all(dir.join("tests/perf")).expect("mkdir specs");
    fs::write(dir.join("_build/tests/flat.ndjson"), "{}\n").expect("flat export");
    fs::write(dir.join("_build/tests/perf/nested.ndjson"), "{}\n").expect("nested export");
    fs::write(dir.join("tests/flat.yaml"), "outcome: accept\n").expect("flat spec");
    fs::write(dir.join("tests/perf/nested.yaml"), "outcome: reject\n").expect("nested spec");
    // 一个没有 outcome yaml 的导出：必须被**计数**，不是静默丢掉。
    fs::write(dir.join("_build/tests/orphan.ndjson"), "{}\n").expect("orphan export");

    let mut cases = Vec::new();
    let (skipped_big, skipped_spec) = collect_cases(&dir, &mut cases);
    cases.sort();
    let names: Vec<String> = cases
        .iter()
        .map(|(p, _)| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["flat".to_string(), "nested".to_string()], "子目录里的导出要收得到");
    assert_eq!(skipped_big, 0, "没有超大文件");
    assert_eq!(skipped_spec, 1, "没有 outcome yaml 的导出要被计数");
    assert!(
        cases.iter().any(|(_, e)| *e == ExpectedOutcome::Reject),
        "子目录里的 outcome yaml 要按 stem 递归找到（nested: reject）"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// **T-K02 的判据（判负语义）**：三条语义各自钉住——尤其是"reject 不许拿
/// parse error 当通过"（这条以前是**假绿**：读不进来的导出算"成功拒绝"）。
#[test]
fn reject_requires_a_kernel_rejection_not_a_parse_error() {
    // 解析失败：`accepted=false`、`kernel_rejected=false` ⇒ reject **必须**判负。
    assert!(
        is_mismatch(ExpectedOutcome::Reject, false, false, false),
        "reject 用例遇到 parse error 必须判负（以前算通过 —— 假绿）"
    );
    // 真·内核拒绝 ⇒ 通过。
    assert!(!is_mismatch(ExpectedOutcome::Reject, false, false, true), "内核拒绝即满足 reject");
    // panic 对任何预期都是失败。
    assert!(is_mismatch(ExpectedOutcome::Reject, false, true, false), "panic 永远判负");
    assert!(is_mismatch(ExpectedOutcome::Accept, true, true, false), "panic 永远判负");
    assert!(
        is_mismatch(ExpectedOutcome::Either, false, true, false),
        "either 也要挡 panic（原来写死 false ⇒ 连 panic 都不算失败）"
    );
    // 正常的 accept / either 不该被误伤。
    assert!(!is_mismatch(ExpectedOutcome::Accept, true, false, false));
    assert!(!is_mismatch(ExpectedOutcome::Either, true, false, false));
    assert!(!is_mismatch(ExpectedOutcome::Either, false, false, true));
    // accept 用例被解析失败挡住 ⇒ 判负（这本来就是原来的行为，钉住别退化）。
    assert!(is_mismatch(ExpectedOutcome::Accept, false, false, false));
}
