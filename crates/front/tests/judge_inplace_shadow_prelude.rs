//! **W-5 第二半的判据**（2026-10-13）：prelude 安装期**不再**排除出影子档比对。
//!
//! ## 被撤掉的是什么
//!
//! `elab.rs` 的影子档（`InplaceMode::Shadow`）里曾有一段：`prelude_install_active()`
//! 时**直接返回慢路结果、两条路不比**（2026-10-06 加；2026-10-08 E4 第二步撤过一次、
//! 实测出 61 处文本分叉后按判据纪律还原）。原文理由 = 安装期的判定在慢路那边合成重跑
//! 时"无法重跑 prelude 本身"（`prefix_src` 为空）⇒ 环境里没有 `And` ⇒
//! `unknown identifier` ⇒ 结构性 `None`。
//!
//! ## 为什么现在能撤（W-5 第二半 ✓）
//!
//! * **E1 剥离** ✓：prelude 是**普通源文本**（`prelude/*.sokonanoda` + `include_str!`）；
//! * **`prelude.rs::prelude_prefix_before`** ✓：安装时把"**本条之前**那段 prelude 源"
//!   交给 `ctx.prefix_src` ⇒ 合成重跑重建出**同一个环境**（含 `And`/`Or`/`Iff`）
//!   ⇒ 安装期与用户期走**同一条**比对 ✓；
//! * E4 第二步那 61 处分叉的**增量**已消失 ⇒ 撤掉之后全语料 `shadow_diff` 增量
//!   **逐条 0** ✓（读数见 commit message / `VERDICT.md`）。
//!
//! 影子档返回的**永远**是 `slow()`（源码重跑那份）⇒ 撤掉排除改的只是"要不要顺带跑一趟
//! 就地、并逐字比文本"，**不改答案** ✓ —— 另有独立判据守 `--json` 逐字节（默认档与
//! 影子档**两侧**全语料 356/356 ✓）。
//!
//! ## 本文件钉两件事（缺一不算 ✓）
//!
//! 1. **安装期的比对真的发生了**：`inplace_shadow_prelude_compared()` 增量 **> 0** ✓
//!    —— 这是撤掉排除之后**唯一的正读数**，也是判据的**牙齿**：
//!    排除一旦回来，本夹具的判定**全部**落回安装期 ⇒ 这个数**归零** ⇒ 判据当场判红 ✗
//!    （**不依赖**谁记得再加一个反计数器 ✓）。
//!    ⚠ 反向验证实测：临时把排除加回去 ⇒ 读数从 `compared=12` 翻成 `compared=0`
//!    （同一次读数里 `excluded=189` —— 那个反计数器随排除一起删掉了：**没有写点的
//!    计数器是空转** ✗，见 `judge.rs` 的说明），断言报 `left: 0`、测试 **FAILED** ✓。
//! 2. **判定红线**：受信任安装期两条路**逐字相同** ⇒ `shadow_diff` 增量 **== 0** ✓。
//!
//! ## 为什么是独立测试文件
//!
//! 读数是**进程级**计数器 ⇒ 独立进程 = 天然隔离（同 `g29_closure_recompile.rs` /
//! `judge_inplace_by.rs` 的纪律 ✓）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::{inplace_report, inplace_shadow_prelude_compared};
use sokonanoda_front::parse;

/// 一份**必须走 prelude** 的夹具：`And` 的构造/投影都在 prelude 里。
const FIXTURE: &str =
    "theorem t (A B : Prop) (h : A) (k : B) : A \u{2227} B := by\n  exact \u{27e8}h, k\u{27e9}\n";

#[test]
fn the_prelude_install_period_is_compared_now_and_the_two_paths_agree() {
    // 影子档：两条路都跑、比对文本（`SOKO_JUDGE_INPLACE=shadow`）。
    std::env::set_var("SOKO_JUDGE_INPLACE", "shadow");
    let (_, _, _same_before, diff_before) = inplace_report();
    let compared_before = inplace_shadow_prelude_compared();
    // 单文件（无 import）：prelude 安装 + 本文件的判定都要发生 ✓。
    let file = parse(FIXTURE).expect("夹具 parse");
    let units = [SourceUnit::single("Main", &file)];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    let compared_after = inplace_shadow_prelude_compared();
    let (_, _, _same_after, diff_after) = inplace_report();

    // 夹具自检：这份夹具必须**编得过**（否则量的是错误路径 ✗）。
    assert!(
        out.errors.is_empty(),
        "夹具必须编得过（否则量的是错误路径）：{:?}",
        out.errors
    );

    // ① **W-5 第二半的判据**：安装期的比对**真的发生过** ✓。
    // 排除一旦回来 ⇒ 本夹具的判定全部落回安装期 ⇒ 这个数归零 ⇒ 这里判红 ✓。
    let compared = compared_after - compared_before;
    assert!(
        compared > 0,
        "**W-5 第二半的判据**：prelude 安装期的比对**必须真的发生过**（实测 {compared} 次）—— \
         这里是 0 只有两种可能，都是判据要咬的东西：① 有人把「安装期不比」的排除加回来了 ✗；\
         ② 影子档在安装期整段没跑到（判据空转 ✗，那就先修夹具/接线）。"
    );

    // ② **判定红线**：受信任安装期两条路必须逐字相同（`shadow_diff` 不许增长 ✓）。
    assert_eq!(
        diff_after, diff_before,
        "**判定红线**：prelude 安装期两条路的文本必须**逐字相同**（`shadow_diff` {} → {}）—— \
         出 diff 就是**真分歧**（不是噪声 ✗）⇒ 停下来定性，不许放宽 ✗。",
        diff_before, diff_after
    );
}
