//! **入口层**判据：生成式夹具在**真 CLI** 上的读数（`AGENT-DOES-NOT-PRODUCE-USER-INPUTS`）。
//!
//! 输入来自 `sokonanoda_front::fixtures`（**同一个生成器** —— 真相层与入口层
//! 共用一份输入面 ✓，不是各写各的 ✗）。
//!
//! ## 这一层断言的是**阴性断言**（"该有的在不在"）
//!
//! 病根（用户 2026-10-15 的结构性诊断 ③）：判据历来只断言"**没有诊断**" ✗，
//! 而用户注意到的是「前两行的 **goal 都没了**」「声明列表**不回来**」——
//! 是"**该有的没有了**" ✗。所以这里逐条断言 counts 的**下界**：
//!
//! * `decl_checked` —— 干净声明**该有的条数**；
//! * `exercise_open` —— 未关目标**该有的条数**；
//! * `example_checked` —— `example` **该有的条数**。
//!
//! ⚠ 每一条都同时**印出实测读数**（不是只有 assert 消息）—— 判据红了要能一眼
//! 看出"少了哪一个数"，而不是"某条断言失败了" ✗。
//!
//! ## 这一层**不**断言什么（如实写明边界）
//!
//! * 不断言"渲染出来长什么样" —— 那是 e2e 那一层的事（三层各司其职 ✓）；
//! * 不断言**声明栏 / Infoview 的渲染** —— LSP 那一层的事（`soko/stateAt` 实测：
//!   坏命令**前后**两条好定理都有 goal，见
//!   `docs/gaps/repro/G117-grade-json-vs-query-check-divergence.sh` 的说明 ✓）。
//!
//! ## G-117（2026-10-15，**已修**）：`grade --json` 与 `query check` 的两个读数
//!
//! 原来这一层**不**断言 `grade --json` 的事件流：实测它与 `query check` 在"入口有
//! parse 错"时**不一致**（`query check` 报 `decl_checked:2`、`grade --json` 一条
//! `decl.checked` 都不发 ✗）。修法 = `crates/cli/src/check_recovery.rs`（单文件回退
//! 改走 `Session::update`，与 `query check` **同一个编译者** ⇒ 一致性是构造出来的 ✓）。
//! 现在两个方向都钉住，判据落本文件末尾的
//! [`grade_json_and_query_check_agree_on_a_parse_failed_entry`] ✓。

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use sokonanoda_front::fixtures::{self, Family, UserInput};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 每份夹具一个**独立**目录（夹具之间不许互相看见：模块根 = 入口目录 ✓）。
fn scratch(input: &UserInput) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-process-state-{}-{}-{}",
        input.id.replace('/', "_"),
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// `sokonanoda query check --file <path>` ⇒ `(counts, 诊断码表, exit)`。
///
/// 走 `query check` 而不是 `grade --json`：前者是**agent 面向的单 JSON 视图**
/// （`docs/protocol.md`），也是本判据要钉住的那个消费者 ✓。
fn check(input: &UserInput) -> (serde_json::Value, Vec<String>, i32) {
    let dir = scratch(input);
    let path = dir.join("Fixture.sokonanoda");
    std::fs::write(&path, &input.source).expect("write fixture");
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["query", "check", "--file"])
        .arg(&path)
        .env("SOKONANODA_NO_CACHE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sokonanoda");
    let code = out.status.code().unwrap_or(-1);
    let text = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "[{}] `query check` 必须印一个 JSON 对象（{e}）：{text}",
            input.id
        )
    });
    let failed: Vec<String> = value["data"]["failed"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|f| f["code"].as_str().unwrap_or("?").to_string())
                .collect()
        })
        .unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (value["data"]["counts"].clone(), failed, code)
}

fn count(counts: &serde_json::Value, key: &str) -> u64 {
    counts[key].as_u64().unwrap_or(0)
}

/// **阴性断言**：把一份夹具该有的三个数逐条对上，并印出实测读数。
fn assert_has_the_expected_content(input: &UserInput) {
    let (counts, failed, exit) = check(input);
    let got = (
        count(&counts, "decl_checked"),
        count(&counts, "exercise_open"),
        count(&counts, "example_checked"),
    );
    // 干净声明 = 该被判定总数 − 开放练习 − example。
    let want_checked = input
        .expect
        .judged
        .saturating_sub(input.expect.open)
        .saturating_sub(input.expect.examples);
    let reading = format!(
        "[{}]（{}）实测 decl_checked={} exercise_open={} example_checked={} exit={exit} 诊断={failed:?}",
        input.id,
        input.family.name(),
        got.0,
        got.1,
        got.2
    );
    assert!(
        got.0 as usize >= want_checked,
        "{reading}\n⇒ **该有的声明没有了** ✗（至少要 {want_checked} 条干净声明被判定）"
    );
    assert!(
        got.1 as usize >= input.expect.open,
        "{reading}\n⇒ **该有的开放练习没有了** ✗（至少要 {} 条）",
        input.expect.open
    );
    assert!(
        got.2 as usize >= input.expect.examples,
        "{reading}\n⇒ **该有的 example 没有了** ✗（至少要 {} 条）",
        input.expect.examples
    );
}

/// ②过程态 / ⑩边界：**每一份**夹具的"该有的"都要在（24 + 3 份）。
#[test]
fn process_state_and_boundary_inputs_keep_everything_they_should_have() {
    let mut inputs = fixtures::process_state_inputs();
    inputs.extend(fixtures::boundary_inputs());
    assert_eq!(
        inputs.len(),
        27,
        "四处参数表 × 三种后续命令 × 有无空行 + 三个边界"
    );
    for input in &inputs {
        assert_has_the_expected_content(input);
    }
}

/// ③差分：**同一输入的变体，该有的都不许少** —— 变体只许改变"目标关不关"，
/// **不许**让任何一条声明消失（用户③"删掉那行再看"的核心不变量 ✓）。
#[test]
fn differential_variants_never_lose_a_declaration() {
    for (a, b) in &fixtures::differential_pairs() {
        assert_has_the_expected_content(a);
        assert_has_the_expected_content(b);
    }
}

/// ④阴性观察（**这条是本判据的头条**）：一条坏命令夹在好命令中间时，
/// **坏命令前面和后面的**干净声明都必须照常被判定 ✓。
///
/// 用户的原话形态是「**声明列表不回来**」✗ —— 所以判据钉的是"**`a` 与 `c` 都还在**"
/// （`decl_checked >= 2`），**不是**"报了一条诊断" ✗（后者是阳性断言，抓不到"少了什么"）。
///
/// ⚠ 实测（2026-10-15）：这条判据在**修 `prefix_recovery` 之前是红的** ——
/// 出错那条命令**之后**的 `c` 报 `elab-tactic-failed … 前缀源码无法解析`、
/// `decl_checked` 只有 1 ✗。修法与反向验证见 `.cache/ux/fix-ui1/VERDICT.md`。
#[test]
fn a_broken_command_does_not_take_the_declarations_around_it() {
    let negatives = fixtures::negative_observation_inputs();
    assert_eq!(negatives.len(), 3, "坏命令的三种形状");
    for input in &negatives {
        let (counts, failed, exit) = check(input);
        let reading = format!(
            "[{}] 实测 decl_checked={} exercise_open={} exit={exit} 诊断={failed:?}",
            input.id,
            count(&counts, "decl_checked"),
            count(&counts, "exercise_open"),
        );
        // **阳性**那一半也不许丢：坏命令必须留下诊断 ✓
        assert!(
            !failed.is_empty(),
            "{reading}\n⇒ 坏命令**没有留下诊断** ✗（判据自己坏了）"
        );
        // **阴性**那一半才是这条判据的要点：坏命令**前后**的干净声明都还在 ✓
        assert_has_the_expected_content(input);
    }
}

/// 夹具生成器自己的守卫：入口层与真相层**吃的是同一份输入面** ✓。
///
/// 若生成器少了某个家族，上面几条 `assert_eq!` 会先红；这里再钉一条"总数不为零、
/// 且四个家族都能在 `all()` 里看到" —— 防的是"判据静默少跑一类" ✗。
#[test]
fn the_entry_layer_consumes_every_generated_family() {
    let all = fixtures::all();
    for family in [
        Family::ProcessState,
        Family::Differential,
        Family::Negative,
        Family::Boundary,
    ] {
        assert!(
            all.iter().any(|i| i.family == family),
            "家族 `{}` 在生成器里不存在 ⇒ 入口层判据静默少跑一类 ✗",
            family.name()
        );
    }
    assert!(all.len() >= 36, "夹具总数实测 {}", all.len());
}

/// **G-117 的守卫（已翻面）**：`grade --json` 与 `query check` 在"入口有 parse 错"时
/// **必须给出同一个读数** ✓。
///
/// 这条判据原来是 `known_gap_grade_json_emits_nothing_for_a_parse_failed_entry`
/// （断言**当前错行为**、并写明"谁修好了它这条会立刻判红，逼他回来翻过来"）——
/// 2026-10-15 修好（`crates/cli/src/check_recovery.rs`：单文件回退改走
/// `Session::update`，与 `query check` **同一个编译者**）⇒ 按它自己的指示翻面 ✓。
///
/// 实测（2026-10-15，同一份夹具 `negative/missing-type`，改前 → 改后）：
///
/// ```text
/// query check  ⇒ counts = {decl_checked: 2, …} · failed = [unexpected-token]   （不变 ✓）
/// grade --json ⇒ 改前：只有 1 条 diagnostic、**零** decl.checked 事件 ✗
///                改后：2 条 decl.checked + 1 条 diagnostic ✓ · exit 1 ✓
/// ```
///
/// 仓库自己的文档写着这两个视图是**同一份真相**（`crates/cli/tests/query.rs` 的头注释：
/// "`query check` and `--json` must agree, because they are two views of the same
/// compile — a divergence would mean a second source of truth"）⇒ 这里断言的就是它 ✓。
///
/// ⚠ **两个方向都钉住**：读数**一致**（`decl_events == counts.decl_checked`）**且**
/// 坏命令的诊断**还在**、退出码**仍是 1** —— 只钉前者会允许"把错吞掉去凑一致" ✗。
#[test]
fn grade_json_and_query_check_agree_on_a_parse_failed_entry() {
    let input = fixtures::negative_observation_inputs()
        .into_iter()
        .find(|i| i.id == "negative/missing-type")
        .expect("夹具 negative/missing-type");
    let dir = scratch(&input);
    let path = dir.join("Fixture.sokonanoda");
    std::fs::write(&path, &input.source).expect("write fixture");

    // ① `query check` 这一半：**该有的在** ✓（修 `prefix_recovery` 之后）。
    let (counts, failed, query_exit) = check(&input);
    assert_eq!(
        count(&counts, "decl_checked"),
        2,
        "`query check` 这一半必须已经好了（坏命令**前后**两条定理都在）"
    );
    assert_eq!(
        failed,
        vec!["unexpected-token"],
        "坏命令的诊断不许被吞掉（放开判定去凑一致是反方向的修法 ✗）"
    );
    assert_eq!(query_exit, 1, "解析失败是拒绝 ⇒ exit 1");

    // ② `grade --json` 这一半：与 ① **同一个读数** ✓ —— G-117 的本体。
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("grade")
        .arg(&path)
        .arg("--json")
        .env("SOKONANODA_NO_CACHE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sokonanoda");
    let stream = String::from_utf8_lossy(&out.stdout);
    let events: Vec<serde_json::Value> = stream
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("每行一个 JSON 事件"))
        .collect();
    let decl_events = events
        .iter()
        .filter(|e| {
            matches!(
                e["type"].as_str(),
                Some("decl.checked") | Some("exercise.open")
            )
        })
        .count();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        out.status.code(),
        Some(1),
        "真语法错仍必须是拒绝 ⇒ exit 1（不许为了让两视图一致就把错吞掉 ✗）"
    );
    assert!(
        events.iter().any(|e| e["type"] == "diagnostic"),
        "坏命令必须留下诊断（这一半两个通道都该有）"
    );
    let diagnostics = events.iter().filter(|e| e["type"] == "diagnostic").count();
    assert_eq!(
        diagnostics,
        failed.len(),
        "两个视图的诊断条数也必须一致（query failed={failed:?}）"
    );
    assert_eq!(
        decl_events,
        count(&counts, "decl_checked") as usize,
        "⭐ G-117：`grade --json` 与 `query check` 必须给出**同一个** decl.checked 读数 \
         （同一次编译的两个视图 —— `crates/cli/tests/query.rs` 头注释）。实测事件：{events:?}"
    );
    assert_eq!(
        decl_events, 2,
        "恢复式解析救回的两条定理**该在**（坏命令前后的 `a` / `c`）"
    );
}
