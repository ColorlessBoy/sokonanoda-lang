//! **S2（`docs/design/dialect-config.md` §3.3）：方言（严格隐式参数）进缓存键**的守卫。
//!
//! 这里只有**两把键**，各一条判据：
//! * 编译缓存键（[`cache::key`] / [`cache::key_with_build`]）—— [`the_compile_cache_key_moves_with_the_dialect`]；
//! * 判定缓存键（`judge::options_key`，私有的 ⇒ 从**行为**上判）——
//!   [`a_loose_judgement_is_never_replayed_for_the_strict_dialect`]。
//!
//! ## 为什么判据**不能**用环境变量做自变量（否则是假绿 ✗）
//!
//! 今天方言还来自 `SOKO_STRICT_AT`，而它自己就以 `SOKO_` 开头 ⇒ `flags_state()` 早就把
//! 它顺手折进键了。用「`SOKO_STRICT_AT=0` 跑一次、`=1` 再跑一次」当判据 ⇒ **撤掉本段的
//! 修复也照样绿**（那是环境变量那半的功劳，不是方言进键的功劳 ✗）。S3+ 的来源是 CLI 旗标 /
//! `sokonanoda.toml` / 编辑器设置，**不经过环境** ⇒ 判据必须让方言**不经环境**地变化
//! （[`CompileOptions::with_dialect`]，同一进程 ⇒ 同一组 `SOKO_*` ✓）。
//!
//! **反向验证**（实测，见 commit message）：把 `flags_state(options.dialect)` 改回
//! `flags_state()` 的旧形、把 `options_key` 改回 `format!("{:?}", options.prelude)`
//! ⇒ 本文件**两条判据当场全红** ✓（第二条红 = 严格档回放了宽松档的 `Match` ✗，
//! 正是"两档互相命中"那个用户可见后果）。
//!
//! 为什么落在 `tests/`（集成测试）而不是 `src/` 的单测：`crates/front/src/judge.rs` 是
//! 判据 ⑨ 的**行数红线**文件（上限 4670、S2 之前 4666），本文件一个字都不占它的预算 ✓。

use sokonanoda_front::compile::cache::{key, key_with_build};
use sokonanoda_front::compile::{CompileOptions, Dialect, PreludeMode};
use sokonanoda_front::judge::{judge_terms, Judgement, OpenGoalSpec};

/// **编译缓存键**：同一份源 + 同一环境（同一进程）+ **只差 `options.dialect`** ⇒ 键必须不同。
///
/// 为什么这一条能咬住 S3+ 的坑：键只认 `options.dialect`（[`Dialect`] 的实参），
/// **与"这个值从哪来"无关** ⇒ CLI 旗标 / toml / 编辑器设置改出来的方言一样换键 ✓。
#[test]
fn the_compile_cache_key_moves_with_the_dialect() {
    let src = "def x : Nat := 1\n";
    let loose = CompileOptions::with_dialect(PreludeMode::Full, Dialect::Loose);
    let strict = CompileOptions::with_dialect(PreludeMode::Full, Dialect::Strict);
    // ① 纯函数：同输入同键（防"随便折了个时间戳/随机数"那种假区分）。
    assert_eq!(
        key_with_build(src, &loose, 7),
        key_with_build(src, &loose, 7),
        "同一档两次必须同键"
    );
    // ② **方言进键**：只差方言 ⇒ 键必须不同（撤掉那格 ⇒ 这里当场红）。
    assert_ne!(
        key_with_build(src, &loose, 7),
        key_with_build(src, &strict, 7),
        "方言必须进检查点键：两档共用同一个 `<模块根>/.sokonanoda/` ⇒ 会互相命中 ✗"
    );
    // ③ 生产入口（`cache::key`，带 build stamp）同样区分 —— ② 不是只对测试形成立。
    assert_ne!(
        key(src, &loose),
        key(src, &strict),
        "`cache::key`（`load`/`store` 与 `project::cache` 实际用的那把）必须区分方言"
    );
}

/// 判定夹具的**前缀**：用户点名的 `b2` 形状（`crates/cli/tests/implicit_at_alignment.rs`
/// 的 `REPRO` 逐字，只留定义与前缀 —— 判定的术语单独给）。
const JUDGE_PREFIX: &str = "def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a\n";

/// `b2` 的项：正常形式（不带 `@`）喂了两个位置隐式实参 ⇒ 严格档必须拒 ✓。
const JUDGE_TERM: &str = "h Nat Nat (fun (n : Nat) => n) 5";

/// **判定缓存键**（`judge::options_key`，私有）的**行为**判据：先宽松档（进缓存）、再严格档
/// ⇒ 严格档必须**重新判**并判红，不许回放宽松档的 `Match`。
///
/// 顺序是判据的一半：两档反过来跑量不到"宽松档污染严格档"这条（那才是用户会踩的方向 ✓）。
#[test]
fn a_loose_judgement_is_never_replayed_for_the_strict_dialect() {
    let loose = CompileOptions::with_dialect(PreludeMode::Full, Dialect::Loose);
    let strict = CompileOptions::with_dialect(PreludeMode::Full, Dialect::Strict);
    let open = OpenGoalSpec {
        universe: Vec::new(),
        ty: "Nat".to_string(),
        binders: Vec::new(),
        metas: Vec::new(),
    };
    // ① 宽松档接受这条 `b2` 形状 ⇒ 它写进本进程的判定缓存 ✓（夹具前提，必须为真）。
    let first = judge_terms(JUDGE_PREFIX, &loose, &open, &[JUDGE_TERM]);
    assert_eq!(
        first[0],
        Judgement::Match,
        "夹具前提：宽松档必须接受 `{JUDGE_TERM}`（否则第二条量不到串味）"
    );
    // ② 严格档：同一前缀/目标/术语，**只差方言** ⇒ 必须判红。
    //    判定缓存键漏了方言 ⇒ 这里会直接命中①的条目、原样返回 `Match` ✗。
    let second = judge_terms(JUDGE_PREFIX, &strict, &open, &[JUDGE_TERM]);
    match &second[0] {
        Judgement::Error { code, message, .. } => assert_eq!(
            code, "elab-implicit-argument-unsolved",
            "严格档的诊断码必须是它（实测 message: {message}）"
        ),
        other => panic!(
            "严格档必须拒绝 `{JUDGE_TERM}`；拿到 {other:?} ⇒ 判定缓存键串味了 \
             （宽松档的 `Match` 被严格档回放 ✗）"
        ),
    }
}
