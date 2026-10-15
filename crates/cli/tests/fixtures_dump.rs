//! **第三层的桥**：`sokonanoda fixtures --json` 必须与生成器**逐字节同构**。
//!
//! ## 这条守卫守什么（`AGENT-DOES-NOT-PRODUCE-USER-INPUTS`，2026-10-15，high）
//!
//! 三层判据共用**一份输入面**（`sokonanoda_front::fixtures`）：
//!
//! * 真相层 `crates/front/tests/process_state_fixtures.rs`（Rust 内，直接调生成器）；
//! * 入口层 `crates/cli/tests/process_state_entry.rs`（Rust 内，直接调生成器）；
//! * **第三层** `editor/vscode/src/test/extension.test.js`（**Node 内**，真 VS Code）
//!   —— 它只能经由 `sokonanoda fixtures --json` 这个口拿到输入。
//!
//! ⇒ **这个口就是一条接缝**（A 层产出 Rust / B 层消费 JS）。接缝的纪律是
//! `AGENTS.md` 验证设计纪律 3：**要一条 A∖B 对账**，且**守卫必须咬得住** ✓。
//!
//! 咬得住的形状：**多一份 / 少一份 / 改一个字**都必须判红 ——
//! 少一份 ⇒ e2e 少验一类用户输入（"修实例不修类"复发 ✗）；
//! 改一个字 ⇒ e2e 验的不是生成器那份输入（第二份真相 ✗）。
//!
//! ⚠ 本文件**不**重复断言夹具的语义（那是 `process_state_entry.rs` 的事 ✗）——
//! 它只断言**桥没走样** ✓。

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use sokonanoda_front::fixtures::{self, Family};

const SCHEMA: &str = "soko.fixtures/1";

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sokonanoda-fixtures-dump-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// 跑真二进制：隔离缓存 ⇒ `(exit, stdout, stderr)`。
fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(args)
        .env("SOKONANODA_CACHE_DIR", scratch())
        .output()
        .expect("run sokonanoda");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn dump() -> Value {
    let (code, stdout, stderr) = run(&["fixtures", "--json"]);
    assert_eq!(
        code, 0,
        "`fixtures --json` 必须 exit 0（它是只打印的子命令）：stderr={stderr}"
    );
    serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!(
            "`fixtures --json` 必须交出一个可解析的 JSON 对象：{err}\nstdout 前 200 字：{}",
            &stdout.chars().take(200).collect::<String>()
        )
    })
}

/// ⭐ **主守卫**：dump 与生成器**逐字节同构**（同序、同数、同 source/why/expect）。
#[test]
fn the_dump_is_a_byte_faithful_mirror_of_the_generator() {
    let value = dump();
    assert_eq!(
        value["schema"], SCHEMA,
        "schema 是 e2e 侧的版本闸（它按这个键判形状）"
    );

    let all = fixtures::all();
    let inputs = value["inputs"]
        .as_array()
        .unwrap_or_else(|| panic!("`inputs` 必须是数组：{value}"));
    assert_eq!(
        value["count"].as_u64(),
        Some(all.len() as u64),
        "`count` 必须等于生成器的份数"
    );
    assert_eq!(
        inputs.len(),
        all.len(),
        "**少一份 / 多一份都是洞**：dump 有 {} 份，生成器有 {} 份 —— \
         e2e 会照 dump 跑，少掉的那一类用户输入就再也没人验 ✗",
        inputs.len(),
        all.len()
    );
    assert!(!all.is_empty(), "生成器不许是空的（否则三层全部空转 ✗）");

    for (index, input) in all.iter().enumerate() {
        let got = &inputs[index];
        assert_eq!(
            got["id"].as_str(),
            Some(input.id.as_str()),
            "第 {index} 份的 id 或**顺序**漂了（e2e 按 id 定位是哪一类输入红了）"
        );
        assert_eq!(
            got["source"].as_str(),
            Some(input.source.as_str()),
            "`{}` 的 source **必须逐字节相同** —— 差一个字，e2e 验的就不是生成器那份输入 ✗",
            input.id
        );
        assert_eq!(
            got["why"].as_str(),
            Some(input.why),
            "`{}` 的 why（「用户为什么会输入这个」）漂了",
            input.id
        );
        let expect = &got["expect"];
        assert_eq!(
            (
                expect["declarations"].as_u64(),
                expect["judged"].as_u64(),
                expect["open"].as_u64(),
                expect["examples"].as_u64(),
            ),
            (
                Some(input.expect.declarations as u64),
                Some(input.expect.judged as u64),
                Some(input.expect.open as u64),
                Some(input.expect.examples as u64),
            ),
            "`{}` 的 expect（阴性断言的**下界**）漂了",
            input.id
        );
    }

    // 家族表：四个家族一个不许少，计数与生成器一致（e2e 按家族挑用例）。
    let families = value["families"]
        .as_array()
        .unwrap_or_else(|| panic!("`families` 必须是数组：{value}"));
    assert_eq!(families.len(), 4, "四个家族都要在：{families:?}");
    for family in [
        Family::ProcessState,
        Family::Differential,
        Family::Negative,
        Family::Boundary,
    ] {
        let expected = all.iter().filter(|input| input.family == family).count();
        let row = families
            .iter()
            .find(|row| row["label"].as_str() == Some(family.name()))
            .unwrap_or_else(|| panic!("家族 `{}` 不在 dump 里：{families:?}", family.name()));
        assert_eq!(
            row["count"].as_u64(),
            Some(expected as u64),
            "家族 `{}` 的计数与生成器不一致",
            family.name()
        );
        assert!(
            expected > 0,
            "家族 `{}` 一份都没有 —— 它对应的那类用户输入就没人验了 ✗",
            family.name()
        );
        assert!(
            row["slug"].as_str().is_some_and(|slug| !slug.is_empty()),
            "家族 `{}` 缺 ASCII slug（JSON 消费方要稳定的机器键）",
            family.name()
        );
    }
}

/// 第三层**必须**有一族"坏命令夹在好命令中间"的输入 —— 用户报的就是这一族
/// （「声明列表**不回来**」）。它是 e2e 判据的**前置**：空了 ⇒ e2e 空转 ✗。
#[test]
fn the_negative_family_the_e2e_suite_depends_on_is_present() {
    let value = dump();
    let inputs = value["inputs"].as_array().expect("inputs");
    let negative: Vec<&Value> = inputs
        .iter()
        .filter(|input| input["family"].as_str() == Some("negative"))
        .collect();
    assert!(
        !negative.is_empty(),
        "`negative` 家族空了 ⇒ 第三层没有输入可喂（e2e 会空转）"
    );
    for input in negative {
        let judged = input["expect"]["judged"].as_u64().unwrap_or(0);
        let declarations = input["expect"]["declarations"].as_u64().unwrap_or(0);
        assert!(
            judged >= 2 && declarations > judged,
            "`{}` 的期望形状不对：坏命令夹在好命令中间 ⇒ \
             顶层命令 > 该判定的干净声明数 ≥ 2（实际 declarations={declarations} judged={judged}）",
            input["id"]
        );
        let source = input["source"].as_str().unwrap_or_default();
        assert!(
            source.contains("broken")
                && source.contains("theorem a")
                && source.contains("theorem c"),
            "`{}` 的 source 必须**真的是**「好-坏-好」三段（e2e 断言 a 与 c 都还在）",
            input["id"]
        );
    }
}

/// 用法错误是 **exit 2**（`docs/protocol.md` 的 0/1/2 契约），不是静默忽略 ✗。
#[test]
fn extra_arguments_are_a_usage_error() {
    let (code, stdout, stderr) = run(&["fixtures", "please-ignore-me"]);
    assert_eq!(
        code, 2,
        "多给参数必须是用法错误（exit 2）—— 实际 stdout={stdout} stderr={stderr}"
    );
    assert!(stderr.contains("不接受参数"), "要给一句人话：{stderr}");
}

/// 人读视图也要能用（不给 `--json` 时不炸、且列出每一份的 id）。
#[test]
fn the_human_view_lists_every_fixture() {
    let (code, stdout, stderr) = run(&["fixtures"]);
    assert_eq!(code, 0, "人读视图必须 exit 0：stderr={stderr}");
    for input in fixtures::all() {
        assert!(
            stdout.contains(&input.id),
            "人读视图必须列出 `{}`（红了要能一眼看出是哪一类输入）",
            input.id
        );
    }
}
