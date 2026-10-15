//! **wire 契约层**判据：`soko/goals` 在**过程态 / 坏命令**输入下"**该有的在不在**"
//! （`AGENT-DOES-NOT-PRODUCE-USER-INPUTS` 的三层各司其职之一）。
//!
//! ## 为什么这一层单独要一条（三层各司其职，`AGENTS.md` 验证设计纪律 2 ✓）
//!
//! * front 层（`crates/front/tests/process_state_fixtures.rs`）验的是**真相**（解析结构 ✓）；
//! * CLI 层（`crates/cli/tests/process_state_entry.rs`）验的是 `query check` 的 **counts** ✓；
//! * **这一层**验的是 **wire 字段**：`soko/goals` 的响应里 `result/decls`
//!   **到底有没有那几条声明** ✓。
//!
//! ⚠ 三层缺一层就是洞 —— 两次真实事故（T-D52 的 `value`/`value_runs` 漏映射、
//! R-2 的目标不高亮）都是"**数据对了 ≠ 用户看见了**"✗：真相层全绿，而 Infoview 上
//! 什么都没有 ✓。用户 2026-10-15 亲报的形态正是这一层：「声明列表**不回来**」✗
//! ⇒ 判据必须钉在**这条 wire 上**，不能只钉 counts ✗。
//!
//! ## 输入
//!
//! 来自 `sokonanoda_front::fixtures`（**同一个生成器** ✓，与另两层共用一份输入面）。
//! 这里取两个**最有代表性**的：过程态（未关的 `by` 块 + 后续命令）与阴性观察
//! （坏命令夹在好命令中间）—— 后者正是用户报的那一条 ✓。

mod common;

use common::Client;
use sokonanoda_front::fixtures::{self, Family, UserInput};

/// 响应里 `result/decls` 的**声明名**集合（wire 上的原样，不猜不造 ✓）。
fn decl_names(answer: &serde_json::Value) -> Vec<String> {
    answer
        .pointer("/result/decls")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    // 名字字段在不同版本里叫 `name`；取不到就退回把整条序列化后找 ——
                    // 这一层要的是"**在不在**"，不是"字段叫什么" ✓。
                    d.get("name")
                        .and_then(|n| n.as_str())
                        .map(str::to_string)
                        .or_else(|| Some(d.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 在真 LSP 上打开一份夹具、问 `soko/goals` ⇒ `(wire 上的声明名, 打开时的诊断文本)`。
fn goals_over_wire(input: &UserInput, id: i64) -> (Vec<String>, String) {
    let dir = std::env::temp_dir().join(format!(
        "soko-goals-wire-{}-{}",
        input.id.replace('/', "_"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("Fixture.sokonanoda");
    std::fs::write(&path, &input.source).expect("write fixture");

    let cache = dir.join(".cache");
    let mut client = Client::start(&cache);
    let uri = Client::file_uri(&path);
    // `root` = 夹具目录：没有 `import` ⇒ 模块根就是入口目录 ✓
    let diagnostics = client.open(&dir, &uri, &input.source);
    let answer = client.request(
        id,
        "soko/goals",
        serde_json::json!({"textDocument": {"uri": uri}, "position": null}),
    );
    let names = decl_names(&answer);
    let _ = std::fs::remove_dir_all(&dir);
    (names, diagnostics)
}

/// ②过程态：未关的 `by` 块**后面**那条命令必须**出现在 wire 上** ✓。
///
/// 只有这一条还不够（解析层已经验过结构 ✓）—— 这里钉的是**消费者看得见**：
/// Infoview 的声明列表就是读 `result/decls` 渲染的 ✓。
#[test]
fn the_goals_wire_carries_the_command_after_an_unfinished_block() {
    let input = fixtures::process_state_inputs()
        .into_iter()
        .find(|i| i.id == "process-state/intro/theorem/blank")
        .expect("夹具 process-state/intro/theorem/blank");
    let (names, _diagnostics) = goals_over_wire(&input, 7101);
    let blob = names.join("\u{1}");
    assert!(
        names.len() >= 2,
        "[{}] wire 上只有 {} 条声明 ⇒ **该有的没有了** ✗（用户形态：声明列表不回来）：{blob}",
        input.id,
        names.len()
    );
    for want in ["a", "b"] {
        assert!(
            blob.contains(want),
            "[{}] wire 上找不到声明 `{want}` ⇒ **该有的没有了** ✗：{blob}",
            input.id
        );
    }
}

/// ④阴性观察：文件**真的坏了**时，wire 上**不许静默地空** ✗ ——
/// 空 `decls` **必须**伴随一条 parse 诊断（"该有的没有了"要**说得出为什么** ✓）。
///
/// ## 这条判据的口径是**照契约**定的，不是照我的猜想（如实写明）
///
/// `crates/lsp/src/lib.rs:1204-1206` 白纸黑字：
///
/// > 解析失败（`NotParsable`）与"还没有报告"**同答空**：LSP 的 parse 诊断走
/// > `publishDiagnostics`（`Doc::diagnostics` 已经是 parse 优先），
/// > `soko/goals` 的 **wire 形状不改**（G-17 只动 CLI/MCP 的 ok 信封）。
///
/// ⇒ 对**真的解析不了**的文件，"`decls` 为空"是**契约** ✓，不是缺口 ✓
/// （对照：`query goals` 对同一份输入给 `ok:false` + `not-parsable` ✓ —— 两条通道
/// 各自用自己的信封说同一件事 ✓）。
///
/// ⚠ 我第一版把这条写成"`a`/`c` 必须出现在 wire 上" ⇒ **判红** ✗ ——
/// 那是**我的期望错了**，不是产品错了 ✓（三处对照：`query goals` 给 `ok:false`、
/// LSP 的注释、`Doc::diagnostics` 的 parse 优先）。**如实记下这次纠错**：
/// 判据写错方向比不写更坏（它会逼人改产品去迁就一条假判据 ✗）。
///
/// ⇒ 于是这条判据钉的是**契约里真正要紧的那一半**：
/// **空列表 + 一条 parse 诊断**（= "说得出为什么" ✓）；**空列表 + 零诊断** ⇒ 判红 ✗
/// （那才是 G-17 那类"假绿"：读起来就是"这份文件没有声明" ✗）。
#[test]
fn a_genuinely_broken_file_is_never_silently_empty_on_the_wire() {
    let input = fixtures::negative_observation_inputs()
        .into_iter()
        .find(|i| i.id == "negative/missing-type")
        .expect("夹具 negative/missing-type");
    assert_eq!(input.family, Family::Negative);
    let (names, diagnostics) = goals_over_wire(&input, 7102);
    let blob = names.join("\u{1}");
    if names.is_empty() {
        assert!(
            diagnostics.contains("unexpected-token") || diagnostics.contains("parse"),
            "[{}] wire 上 `decls` 是**空的**，而 `publishDiagnostics` 里**也没有** parse 诊断 \
             ⇒ 这就是 G-17 那类**假绿**（读起来是「这份文件没有声明」✗）：diagnostics={diagnostics}",
            input.id
        );
    } else {
        // 若哪天 LSP 也走恢复式解析（把救回的声明发上 wire），那 `a`/`c` 就该在 ✓
        // —— 这条分支不是"放行"：它要求**救回来的必须成对出现**（不许只回来一半 ✗）。
        for want in ["a", "c"] {
            assert!(
                blob.contains(want),
                "[{}] wire 上已经有声明了（{} 条），但缺 `{want}` ⇒ 恢复式解析只回来一半 ✗：{blob}",
                input.id,
                names.len()
            );
        }
    }
}
