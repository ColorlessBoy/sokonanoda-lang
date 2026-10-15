//! **tactic 参数表的边界**（`SUB-parse-error-kills-whole-file`，用户 2026-10-15 亲报）。
//!
//! 从 `parser.rs` 拆出来（`AGENTS.md` 硬规则 5 + `docs-lint` 判据 ⑨：`parser.rs`
//! 已在存量基线上，**只许减不许增**）—— 用例本身只有 ~90 行，放在这里不动那条棘轮。
//!
//! 两处出口同一条根因：`intro` / `unfold … at` 的**名字串跨行吃到了下一条命令的
//! 关键字**（`theorem` 不是 tactic 关键字 ⇒ `is_tactic_keyword` 拦不住）⇒ 紧接着的
//! `(` 变成命令级 parse 错 ⇒ **一处还没收尾的 `by` 块让整份文件废掉** ✗。
//!
//! **Lean 4 对照**（`colGt`，`~/Documents/lean/lean4/src/Lean/Parser/Basic.lean:1512`：
//! 「下一个 token 的列必须**严格更大**」）：
//! * `intro` —— `Init/Tactics.lean:50` `"intro" notFollowedBy("|") (ppSpace colGt term:max)*`
//! * `unfold` —— `Init/Tactics.lean:802` `"unfold" (ppSpace colGt ident)+ (location)?`
//! * `unfold … at` —— `Lean/Elab/Tactic/Location.lean:28`
//!   `locationHyp := (ppSpace colGt (term:max <|> locationType))+`（`at` 自己用
//!   `withPosition` 存列）
//!
//! 撤掉 `parser.rs` 的 `at_or_left_of` 调用 ⇒ 两条都判红 ✓（反向验证）。

use crate::ast::UnfoldAt;
use crate::{parse, Command, Expr, Tactic};

/// 声明 binder（`(p : Prop)`）被裹成 `Lambda`，`by` 块在它体内。
fn by_tactics_of(command: &Command) -> &Vec<Tactic> {
    let Command::Theorem { val, .. } = command else {
        panic!("expected a theorem, got {command:?}");
    };
    let Expr::Lambda { body, .. } = val else {
        panic!("expected the declaration binders as a lambda, got {val:?}");
    };
    let Expr::By { tactics, .. } = body.as_ref() else {
        panic!("expected a by block, got {body:?}");
    };
    tactics
}

#[test]
fn an_unfinished_by_block_does_not_swallow_the_next_command() {
    let file = parse(
        "theorem a (p : Prop) : p → p := by\n  intro h\n\n\
         theorem b (q : Prop) : q → q := by\n  intro h\n  exact h\n",
    )
    .expect("两处都要解析成功");
    assert_eq!(file.commands.len(), 2, "commands: {:?}", file.commands);
    let Command::Theorem { name, .. } = &file.commands[0] else {
        panic!("expected theorem a, got {:?}", file.commands[0]);
    };
    assert_eq!(name, "a");
    let tactics = by_tactics_of(&file.commands[0]);
    assert_eq!(tactics.len(), 1, "只该有 `intro` 这一步：{tactics:?}");
    let Tactic::Intro { names, .. } = &tactics[0] else {
        panic!("expected intro, got {:?}", tactics[0]);
    };
    assert_eq!(
        names,
        &vec!["h".to_string()],
        "`theorem` / `b` 绝不能被吃成 binder 名：{names:?}"
    );
    assert!(
        matches!(&file.commands[1], Command::Theorem { name, .. } if name == "b"),
        "第二条命令必须是 `theorem b`：{:?}",
        file.commands[1]
    );
}

#[test]
fn an_unfinished_unfold_at_does_not_swallow_the_next_command() {
    let file = parse(
        "theorem a (p : Prop) : p → p := by\n  intro h\n  unfold p at h\n\n\
         theorem b (q : Prop) : q → q := by\n  intro h\n  exact h\n",
    )
    .expect("两处都要解析成功");
    assert_eq!(file.commands.len(), 2, "commands: {:?}", file.commands);
    let tactics = by_tactics_of(&file.commands[0]);
    let Tactic::Unfold { at, .. } = &tactics[1] else {
        panic!("expected unfold, got {:?}", tactics[1]);
    };
    let UnfoldAt::Hyps(hyps) = at else {
        panic!("expected `at` hypotheses, got {at:?}");
    };
    assert_eq!(
        hyps.iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec!["h"],
        "`theorem` / `b` 绝不能被吃成假设名：{hyps:?}"
    );
    assert!(
        matches!(&file.commands[1], Command::Theorem { name, .. } if name == "b"),
        "第二条命令必须是 `theorem b`：{:?}",
        file.commands[1]
    );
}

// ---------------------------------------------------------------------------
// `by` 块的边界用例（同一主题，从 `parser.rs` 的内联测试模块搬来 ——
// 那边的文件在 `docs-lint` 判据 ⑨ 的存量基线上，**只许减不许增** ✗）。

/// 取出首个 `def` 值位 `by` 块里的 tactics（无声明 binder 时直接是 By）。
fn by_tactics(src: &str) -> Vec<Tactic> {
    let file = parse(src).unwrap();
    let Command::Def { val, .. } = &file.commands[0] else {
        panic!("expected def");
    };
    let val = match val {
        Expr::Lambda { body, .. } => body.as_ref(),
        other => other,
    };
    let Expr::By { tactics, .. } = val else {
        panic!("expected a by block, got {val:?}");
    };
    tactics.clone()
}

#[test]
fn by_block_newlines_separate_tactics() {
    let tactics = by_tactics("def t : Prop := by\n  intro a\n  exact a\n  assumption\n");
    assert_eq!(tactics.len(), 3, "tactics: {tactics:?}");
    assert!(matches!(tactics[0], Tactic::Intro { .. }));
    assert!(matches!(tactics[1], Tactic::Exact { .. }));
    assert!(matches!(tactics[2], Tactic::Assumption { .. }));
}

#[test]
fn by_block_newline_boundary_beats_application() {
    // `exact f` 换行 `apply g`：`apply` 是 tactic 关键字，不能吃成 f 的实参。
    let tactics = by_tactics("def t : Prop := by\n  exact f\n  apply g\n");
    assert_eq!(tactics.len(), 2, "tactics: {tactics:?}");
    let Tactic::Exact { expr, .. } = &tactics[0] else {
        panic!("expected Exact: {:?}", tactics[0]);
    };
    assert!(
        matches!(expr, Expr::Ident { name, .. } if name == "f"),
        "`exact f` must not absorb the next tactic: {expr:?}"
    );
    assert!(matches!(tactics[1], Tactic::Apply { .. }));
}

#[test]
fn by_block_multiline_application_is_one_tactic() {
    // 续行以非关键字开头时仍是同一个应用（一个 tactic）。
    let tactics = by_tactics("def t : Prop := by\n  exact f\n    a\n    b\n  assumption\n");
    assert_eq!(tactics.len(), 2, "tactics: {tactics:?}");
    let Tactic::Exact { expr, .. } = &tactics[0] else {
        panic!("expected Exact: {:?}", tactics[0]);
    };
    assert!(
        matches!(expr, Expr::App { .. }),
        "multi-line application must stay one Exact: {expr:?}"
    );
    assert!(matches!(tactics[1], Tactic::Assumption { .. }));
}

#[test]
fn by_block_semicolons_still_work_and_mix_with_newlines() {
    let tactics = by_tactics("def t : Prop := by intro a; exact a\n  assumption\n");
    assert_eq!(tactics.len(), 3, "tactics: {tactics:?}");
    assert!(matches!(tactics[0], Tactic::Intro { .. }));
    assert!(matches!(tactics[1], Tactic::Exact { .. }));
    assert!(matches!(tactics[2], Tactic::Assumption { .. }));
}

#[test]
fn by_block_does_not_consume_the_next_command() {
    // by 块收尾后必须停下，不能把 `#check` 也吞进去/当成实参。
    let file = parse("def t : Prop := by\n  exact a\n#check t\n").unwrap();
    assert_eq!(file.commands.len(), 2, "commands: {:?}", file.commands);
    let Command::Def { val, .. } = &file.commands[0] else {
        panic!("expected def");
    };
    let Expr::By { tactics, .. } = val else {
        panic!("expected by block, got {val:?}");
    };
    assert_eq!(tactics.len(), 1);
    assert!(matches!(&file.commands[1], Command::Check { .. }));
}
