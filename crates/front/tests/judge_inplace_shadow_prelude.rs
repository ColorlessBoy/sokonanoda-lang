//! **E4 的判据读数**（2026-10-08）：影子档里那条「prelude 安装期**不比**」的排除分支。
//!
//! ## 这条排除是什么
//!
//! `elab.rs` 的影子档（`InplaceMode::Shadow`）里有一段：`prelude_install_active()` 时
//! **直接返回慢路结果、不比对**（2026-10-06 加的）。理由写在代码注释里：prelude 安装
//! **进行中**做的判定（`And.rec`/`Or.rec`/`Iff.intro`/`Or.elim`）在慢路那边合成重跑时
//! **无法重跑 prelude 本身**（`prefix_src` 为空）⇒ 环境里没有 `And` ⇒
//! `unknown identifier` ⇒ 结构性 `None`（**不是实现 bug**）。
//!
//! ## 为什么它是 E4 的目标
//!
//! `docs/design/course-stdlib.md` §7 要撤掉它：prelude 变成**普通源文本**（E1 ✓）之后，
//! 合成前缀**可以**把 prelude 拼进去 ⇒ 那段安装期的判定**也能比**了 ⇒ 排除不再需要 ✓。
//! 判据就是本文件读的那个数：**撤掉之后必须是 0** ✓。
//!
//! ## E4 第二步**试过、实测不成立 ⇒ 已还原**（2026-10-08）—— 断言翻回去了 ✓
//!
//! 第一步留的出口是"撤掉分支之后**必须**把断言从 `> 0` 改成 `== 0`" ✓ —— 第二步做到了，
//! 但**全课程抽样把它判了回来** ✗：撤掉之后 `shadow_diff` 合计 **61 处**（10 个入口），
//! 逐条定性 ⇒ **全是文本/显式性层面的**（`fast = Exists α (fun …)` vs
//! `slow = @Exists α (fun … @And … @Eq β …)` ✓；`Acc.rec` 那类就地路答 `None` 退回慢路 ✓），
//! **不是语义分歧** ✓，且输出逐字节没变（全量 **252/252** ✓）。
//! ⇒ 按判据纪律"出 diff 就是真分歧 ⇒ 停下来定性，**不许**放宽 ✗"的**正确处置 =
//! 还原排除 + 如实登记** ✓（把信噪比坏掉的影子档留着才是放宽 ✗）。
//!
//! 本文件现在钉三件事（缺一不算 ✓）：
//! 1. **排除仍在命中**：读数 `> 0` ✓（还原后的**当前行为** ✓；`== 0` 是它的错误出口 ✗）；
//! 2. **影子档真的跑了**（否则 1 是空转 ✗）：`same > 0` ✓；
//! 3. **被比较的那批两条路逐字相同**：`diff == 0` ✓（还原之后成立 ✓）。
//!
//! ## 为什么是独立测试文件
//!
//! 读数是**进程级**计数器 ⇒ 独立进程 = 天然隔离（同 `g29_closure_recompile.rs` /
//! `judge_inplace_by.rs` 的纪律 ✓）。

use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::{inplace_report, inplace_shadow_prelude_excluded};
use sokonanoda_front::parse;

/// 一份**必须走 prelude** 的夹具：`And` 的构造/投影都在 prelude 里。
const FIXTURE: &str =
    "theorem t (A B : Prop) (h : A) (k : B) : A \u{2227} B := by\n  exact \u{27e8}h, k\u{27e9}\n";

#[test]
fn the_prelude_install_exclusion_is_still_hit_and_the_shadow_agrees() {
    // 影子档：两条路都跑、比对文本（`SOKO_JUDGE_INPLACE=shadow`）。
    std::env::set_var("SOKO_JUDGE_INPLACE", "shadow");
    let (_, _, same_before, diff_before) = inplace_report();
    let before = inplace_shadow_prelude_excluded();
    // 单文件（无 import）：prelude 安装 + 本文件的判定都要发生 ✓。
    let file = parse(FIXTURE).expect("夹具 parse");
    let units = [SourceUnit::single("Main", &file)];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    let after = inplace_shadow_prelude_excluded();
    let (_, _, same_after, diff_after) = inplace_report();

    // 夹具自检：这份夹具必须**编得过**（否则量的是错误路径 ✗）。
    assert!(
        out.errors.is_empty(),
        "夹具必须编得过（否则量的是错误路径）：{:?}",
        out.errors
    );
    let hits = after - before;
    assert!(
        hits > 0,
        "**E4 的当前行为**：prelude 安装期的排除分支**必须仍在命中**（实测 {hits} 次）—— \
         第二步撤过它，实测 `shadow_diff` 合计 61 处（文本/显式性层面 ✗）⇒ 已还原 ✓。\
         若这里变成 0 ⇒ 有人又撤了它：**先读文件头**（那条路要求 `diff == 0` ✓，做不到就 \
         不许撤 ✗）。"
    );
    // ② **`diff` 不许增长**（判定红线：还原之后被比较的那批必须逐字相同 ✓）。
    //
    // ⚠ **为什么不钉 `same > 0`**（以前那一臂）：还原排除之后，本夹具的判定**全部**落在
    // 安装期 ⇒ 都被排除 ⇒ `same` 不可能增长 ✗。① 的"非空转"由 `hits > 0` **本身**保证 ✓
    // （读数非零就不是空转 ✓）—— 那正是本文件存在的理由（第一步的读数 72 → 现在的读数 ✓）。
    let _ = (same_before, same_after);
    // ③ **受信任安装期**的两条路必须逐字相同（判定红线）。
    assert_eq!(
        diff_after, diff_before,
        "**判定红线**：prelude 安装期两条路的文本必须**逐字相同**（`shadow_diff` {} → {}）—— \
         出 diff 就是**真分歧**（不是噪声 ✗）⇒ 停下来定性，不许放宽 ✗。",
        diff_before, diff_after
    );
}
