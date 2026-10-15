//! **真相层**判据：生成式夹具的**解析结构**（`AGENT-DOES-NOT-PRODUCE-USER-INPUTS`）。
//!
//! 输入来自 `sokonanoda_front::fixtures`（**生成器**，不是手写实例 ✗）—— 四个家族
//! 逐条对应用户报告的输入特征（过程态 / 差分 / 阴性观察 / 边界）。
//!
//! ## 这一层断言什么
//!
//! * **结构**：每份夹具解析出的**顶层命令条数**必须等于生成器说的那个数
//!   （`Expect::declarations`）—— 一条都不许丢 ✓；
//! * **后续命令完好**：未关的 `by` 块**不许吞掉**后面那条命令的关键字
//!   （`SUB-parse-error-kills-whole-file` 的根因①：`intro` / `unfold … at`
//!   的名字串跨行吃名字）；
//! * **差分关系**（用户③）：变体对之间**只差该差的那一点** ——
//!   `sorry` 有/无 ⇒ 命令结构**逐条相同**；删掉一行 ⇒ 声明条数不变、
//!   只有"目标关不关"变了。
//!
//! ## 改前 / 改后（实测读数）
//!
//! 改前：这一层**只有两条手写实例**（`crates/front/src/tests/parser_boundary.rs`
//! 的 `intro` 与 `unfold … at`）⇒ `unfold`（不带 `at`）与 `cases` 分支 binder
//! **没人管** ✗ —— 修复 commit 自己写着"四处参数表都用 `at_or_left_of`"，
//! 而用例只覆盖两处 ⇒ **修实例不修类** ✗。
//! 改后：本文件对 `Terminator` 的**四处全组合**跑同一组断言 ✓。
//!
//! ## 反向验证（咬得住历史 bug ✓）
//!
//! 撤掉 `crates/front/src/parser.rs` 里四处参数表的 `at_or_left_of(column)` 判断
//! ⇒ 本文件的 `process_state_*` 用例**判红**（实测：`commands.len()` 从 2 掉到 1、
//! `theorem` 被吃成 binder 名）。读数见 `.cache/ux/fix-ui1/VERDICT.md`。

use sokonanoda_front::fixtures::{self, Family};
use sokonanoda_front::{parse, Command, Expr};

/// 顶层命令的**声明名**（`example` 无名 ⇒ 返回 `None`）。
fn decl_name(command: &Command) -> Option<&str> {
    match command {
        Command::Theorem { name, .. } | Command::Def { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

/// 顶层命令里**具名声明**的名字表（按源序）—— 差分断言用它比对"只差该差的那点"。
fn names(src: &str) -> Vec<String> {
    parse(src)
        .unwrap_or_else(|err| panic!("夹具必须解析成功：{}", err.message))
        .commands
        .iter()
        .filter_map(|c| decl_name(c).map(str::to_string))
        .collect()
}

#[test]
fn every_generated_input_parses_with_the_expected_command_count() {
    let all = fixtures::all();
    assert!(
        all.len() >= 24 + 6 + 3 + 3,
        "夹具生成器至少要产出四个家族的常规输入，实测 {} 份",
        all.len()
    );
    for input in &all {
        // ⚠ **阴性观察那一族是故意坏的**（用户④要的就是"坏命令夹在好命令中间"）
        // ⇒ 它走**恢复式**解析；其余三族必须**严格**解析成功 ✓。
        if input.family == Family::Negative {
            let recovered = sokonanoda_front::parse_recovering(&input.source)
                .unwrap_or_else(|err| panic!("[{}] 词法都不该失败：{}", input.id, err.message));
            let got: Vec<String> = recovered
                .file
                .commands
                .iter()
                .filter_map(|c| decl_name(c).map(str::to_string))
                .collect();
            // **该有的在**：坏命令**前后**两条干净定理都必须被救回来 ✓。
            for want in ["a", "c"] {
                assert!(
                    got.iter().any(|n| n == want),
                    "[{}] 恢复式解析丢了 `{want}` ⇒ **声明列表不回来** ✗（实测 {got:?}）",
                    input.id
                );
            }
            assert!(
                !recovered.diagnostics.is_empty(),
                "[{}] 坏命令必须留下诊断（**阳性**那一半也不许丢 ✗）",
                input.id
            );
            continue;
        }
        let file = parse(&input.source).unwrap_or_else(|err| {
            panic!(
                "[{}]（{}）必须解析成功，实测 {} @ line {}：{}",
                input.id,
                input.family.name(),
                err.code(),
                err.span.start.line,
                err.message
            )
        });
        assert_eq!(
            file.commands.len(),
            input.expect.declarations,
            "[{}]（{}）顶层命令条数不对（**该有的丢了** ✗）：{:?}",
            input.id,
            input.family.name(),
            file.commands
                .iter()
                .filter_map(decl_name)
                .collect::<Vec<_>>()
        );
    }
}

/// 过程态家族的核心不变量：**未关的 `by` 块不许吞掉后面那条命令**。
///
/// 这条正是 `SUB-parse-error-kills-whole-file` 根因①的形状；它必须在
/// **四处参数表**上都成立（不只是修复时补了用例的那两处 ✓）。
#[test]
fn an_unfinished_by_block_never_swallows_the_following_command() {
    let cases = fixtures::process_state_inputs();
    assert_eq!(cases.len(), 24, "四处参数表 × 三种后续命令 × 有无空行");
    for input in &cases {
        let file = parse(&input.source).expect("过程态夹具必须解析成功");
        let tail = file.commands.last().expect("至少一条命令");
        // 后续命令**必须是它自己**：名字/关键字完好，没被吃成 tactic 的实参。
        let intact = match input.id.as_str() {
            id if id.contains("/example/") => matches!(tail, Command::Example { .. }),
            id if id.contains("/def/") => matches!(tail, Command::Def { name, .. } if name == "b"),
            _ => matches!(tail, Command::Theorem { name, .. } if name == "b"),
        };
        assert!(intact, "[{}] 后续命令被吃坏了：{:?}", input.id, tail);
    }
}

/// 用户②的**另一半**：未关块自己**仍然是未关的**（没有被"补"成别的东西）。
///
/// 只有"后面那条命令完好"还不够 —— 若解析器把 `intro h` 吃成了别的东西，
/// 结构计数可能照样对 ✓ 而语义已经错了 ✗。这里断言**第一条命令的值位是 `by` 块**、
/// 且它的 tactic 数**恰好**是生成器写下的那几步。
#[test]
fn the_unfinished_block_keeps_exactly_its_own_tactics() {
    for input in &fixtures::process_state_inputs() {
        let file = parse(&input.source).expect("过程态夹具必须解析成功");
        // 未关的那条声明叫 `a`（`unfold` 家族前面垫了一条辅助 `def myid` ⇒ 不能取 `[0]`）。
        let first = file
            .commands
            .iter()
            .find(|c| decl_name(c) == Some("a"))
            .unwrap_or_else(|| panic!("[{}] 找不到未关的声明 `a`", input.id));
        let val = match first {
            Command::Theorem { val, .. } | Command::Def { val, .. } => val,
            other => panic!("[{}] 期望声明，实测 {other:?}", input.id),
        };
        // 声明 binder 会被裹成 `Lambda`，`by` 块在它体内。
        let body = match val {
            Expr::Lambda { body, .. } => body.as_ref(),
            other => other,
        };
        let Expr::By { tactics, .. } = body else {
            panic!("[{}] 期望 `by` 块，实测 {body:?}", input.id);
        };
        // 未关块自己的步数：`intro` 1 步、`unfold` 1 步、`unfold … at` 1 步、
        // `cases` 是 `intro` + `cases` 两步。
        let expected = if input.id.starts_with("process-state/cases/") {
            2
        } else {
            1
        };
        assert_eq!(
            tactics.len(),
            expected,
            "[{}] 未关块自己的 tactic 步数不对（**多吞了一步** ⇒ 后面那条命令要遭殃 ✗）：{tactics:?}",
            input.id
        );
    }
}

/// 用户③差分：**`sorry` 有/无只差在"目标关不关"**，命令结构必须逐条相同。
#[test]
fn the_sorry_differential_changes_only_the_goal_state() {
    let pairs = fixtures::differential_pairs();
    assert_eq!(pairs.len(), 3, "三对变体");
    for (a, b) in &pairs {
        assert_eq!(
            names(&a.source),
            names(&b.source),
            "[{}] vs [{}]：变体之间的**声明结构**必须逐条相同（差分测试的前提 ✗）",
            a.id,
            b.id
        );
        assert_eq!(
            a.expect.declarations, b.expect.declarations,
            "[{}] vs [{}]：声明条数不许因变体而变",
            a.id, b.id
        );
    }
}

/// 用户③的**删掉一行**：删的是收尾 tactic ⇒ 声明**一条都不少**，
/// 变的只有"目标关不关"（`open` 从 0 变 1）。
#[test]
fn deleting_the_closing_tactic_keeps_every_declaration() {
    let (full, deleted) = fixtures::differential_pairs()
        .into_iter()
        .find(|(a, _)| a.id == "differential/delete-line/full")
        .expect("删掉一行那一对");
    assert_eq!(names(&full.source), names(&deleted.source));
    assert_eq!(full.expect.open, 0, "删之前目标应当是关的");
    assert_eq!(deleted.expect.open, 1, "删之后目标应当是开的");
}

/// 边界家族：文件尾的未关块（**对照**：改前也解析正常）与文件中的未关块
/// 必须**同样**解析成功 —— 用户⑩的最小现场就是靠这一对定位的 ✓。
#[test]
fn the_boundary_variants_parse_like_the_reported_minimal_repro() {
    let boundaries = fixtures::boundary_inputs();
    assert_eq!(boundaries.len(), 3);
    for input in &boundaries {
        let file = parse(&input.source).expect("边界夹具必须解析成功");
        assert_eq!(
            file.commands.len(),
            input.expect.declarations,
            "[{}]",
            input.id
        );
    }
    // 对照那一份：只有一条命令（后面没有别的声明）——用户实测"解析正常" ✓。
    let eof = boundaries
        .iter()
        .find(|i| i.id == "boundary/unclosed-at-eof")
        .expect("对照夹具");
    assert_eq!(parse(&eof.source).expect("解析").commands.len(), 1);
}

/// 夹具生成器自己的守卫：**四个家族都在、id 唯一**（否则判据会静默少跑一类 ✗）。
#[test]
fn the_generator_covers_every_family_with_unique_ids() {
    let all = fixtures::all();
    for family in [
        Family::ProcessState,
        Family::Differential,
        Family::Negative,
        Family::Boundary,
    ] {
        assert!(
            all.iter().any(|i| i.family == family),
            "家族 `{}` 一份夹具都没有 ⇒ 判据静默少跑一类 ✗",
            family.name()
        );
    }
    let mut ids: Vec<&str> = all.iter().map(|i| i.id.as_str()).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "夹具 id 必须唯一：{ids:?}");
}

/// `unfold` 家族的**可归属**守卫：`def` 的名字与 `unfold` 的目标**必须是同一个**
/// （`fixtures::HELPER`）。
///
/// 为什么值得一条判据：名字一旦漂移，`unfold` 会**语义**失败 ⇒ 入口层读数就不再
/// 是"解析结构"的读数（**读数不可归属** ✗，正是值守纪律点名的第二种反向结论之因）。
#[test]
fn the_unfold_family_unfolds_the_helper_it_declares() {
    for input in &fixtures::process_state_inputs() {
        if !input.id.starts_with("process-state/unfold") {
            continue;
        }
        assert!(
            input.source.contains(&format!("def {} ", fixtures::HELPER)),
            "[{}] 必须先声明 `def {}`：{}",
            input.id,
            fixtures::HELPER,
            input.source
        );
        assert!(
            input
                .source
                .contains(&format!("unfold {}", fixtures::HELPER)),
            "[{}] `unfold` 的目标必须是同一个 `{}`：{}",
            input.id,
            fixtures::HELPER,
            input.source
        );
    }
}
