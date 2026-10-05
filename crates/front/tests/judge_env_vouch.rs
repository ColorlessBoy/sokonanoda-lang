//! **§3.C「前缀环境」这一档的判据**：主编译 pass 担保前缀 ⇒ judge 不再重查前缀。
//!
//! 设计来源 `docs/design/incremental-environment.md` §31（§31.4 的真数字 +
//! §31.1/§31.2 的刀口与夹取）。**一句话**：`by` 路径那 265 趟是 `judge_ms` 的 99%
//!（115.9s / 117.2s），而每趟都在 `compile_fol_with` 里**把整个前缀从源码重编一遍**；
//! 而那个前缀**刚在同一次 `run_pass` 里被逐条检查过** ⇒ 可以担保。
//!
//! ⚠ **一个档位一个文件**（同 `judge_inplace*` 的理由）：`SOKO_JUDGE_ENV_*` 走
//! `OnceLock` 只读一次环境，而**同一文件的多个 `#[test]` 共享一个进程** ⇒
//! 两条判据会抢同一个档位（实测过：`judge_inplace_by` 曾因此 6 连跑 4 红 2 绿 ✗）。
//! 守卫：`scripts/check-test-env-isolation.py`（已进 gate 与 CI）。
//!
//! 判据（缺一不算）：
//! 1. **判据不空转**：复用**必须真的发生**（`REUSED > 0`）—— 否则一个永远走
//!    "整份重查"的实现也能让"逐字节相同"变绿 ✗；
//! 2. **反向判据（本档真实踩过的坑）**：**不许多担保** —— 合成声明
//!    `_soko_judge_k` 必须**仍然被检查**。
//!    第一版把调用方的 `idx` 直接当 `before` 传，而它**恒比 `prefix_commands` 大**
//!    （探针在全课程上量到 `exact=86 · overshoot=179`）⇒ 多担保了若干条命令，
//!    那几条正是合成声明 ⇒ **判定声明根本没被检查** ✗ ⇒ 症状是 `--json` 里
//!    `compiled` 变 `failed`（**38 行不同**）。
//!    ⇒ 判据用一个**必然判错的 tactic**：担保正确时它必须**仍然报错** ✓。
//!
//! ⚠⚠ **夹具必须是「带 `import` 的多单元项目」，不能是单文件** —— 这是
//! **实测踩出来的**：单文件夹具的探针是 `exact=3 · overshoot=0`（**复现不出那个 bug**，
//! 注入"去掉夹紧"后判据照样绿 ⇒ **反向验证失效** ✗）；换成带 `import` 的
//! 两单元夹具后 `overshoot=3` ✓。
//! **为什么**：judge 的前缀文本走 `importless_source`（**去掉 `import` 行**），
//! 而调用方的 `idx` 是 **AST 命令序号**（**含** `import` 那条命令）⇒ 有 `import`
//! 才会错位。**单文件没有 `import` ⇒ 错位为 0 ⇒ 夹具测不到** ✗。
use sokonanoda_front::compile::{compile_all_units, CompileOptions, SourceUnit};
use sokonanoda_front::judge::REUSED;
use sokonanoda_front::parse;

/// 依赖单元（被 `import` 的那个）。
const DEP: &str = "\
def ident : Prop -> Prop := fun (x : Prop) => x
theorem dep_thm (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
";

/// 入口：**必须带 `import`**（否则错位为 0，判据空转 ✗），`by` 块里**必然判错**
/// —— 目标 `P ∧ Q` 的第二个子目标是 `Q`，却给 `hp : P`。
///
/// 形状是刻意的：错误在**第二个** tactic 上，而第一个 `apply And.intro` 的判定
/// **已经**产生了合成声明 ⇒ 若"担保"多盖了合成声明，第二个 tactic 的判定就被跳过
/// ⇒ 错误消失 ✗。
const ENTRY_BAD: &str = "\
import Dep
theorem wrong (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hp
";

/// 同形但**正确**的版本（证明"不报错"不是因为整条判定链被关掉了）。
const ENTRY_GOOD: &str = "\
import Dep
theorem right (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
";

fn run(entry: &str) -> String {
    let dep = parse(DEP).expect("dep parses");
    let entry = parse(entry).expect("entry parses");
    let units = [
        SourceUnit::single("Dep", &dep),
        SourceUnit::single("Entry", &entry),
    ];
    let (out, _reports) = compile_all_units(&units, &CompileOptions::default());
    format!("errors={:?}\nevents={:?}", out.errors, out.events)
}

/// **判据 1 + 2**：担保真的发生 · 错判**不许**被担保掉 · 正确的不许误报。
///
/// ⚠ 断言查的是 **`ElabTacticFailed`**（`errors=[…]` 是 `Debug` 形态，打的是
/// Rust 枚举名），**不是** wire 上的 `elab-tactic-failed` —— 第一版写成后者 ⇒
/// **假红**（实测：错误其实在，只是字符串对不上）✗。
#[test]
fn the_main_pass_vouches_for_the_prefix_without_swallowing_errors() {
    std::env::set_var("SOKO_JUDGE_ENV_REUSE", "1");
    std::env::set_var("SOKO_JUDGE_ENV_VOUCH", "1");

    // ① 判据不空转：担保必须真的发生。
    let before = REUSED.load(std::sync::atomic::Ordering::Relaxed);
    let bad = run(ENTRY_BAD);
    let mid = REUSED.load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        mid > before,
        "前缀复用一次都没发生（REUSED {before} → {mid}）⇒ **判据空转**：\
         夹具没让主编译 pass 压栈（`SOKO_JUDGE_ENV_VOUCH=1` 那条路）"
    );

    // ② **反向判据（本档真实踩过的坑）**：错判**必须仍然报错**。
    //   第一版多担保了合成声明 ⇒ 这里会变绿 ✗。
    assert!(
        bad.contains("ElabTacticFailed"),
        "**错判被担保掉了** ✗ —— `by` 块里 `exact hp` 在目标 `Q` 上必然失败，\
         而结论里看不到 `ElabTacticFailed` ⇒ 合成声明 `_soko_judge_k` **没被检查**\
         （`before` 多盖了：调用方的 AST 序号 `idx` 含 `import` 那条命令，\
         而 judge 的前缀文本走 `importless_source` ⇒ 两者错位）。\n实际：{bad}"
    );

    // ③ 同形的**正确**版本必须仍然通过（证明 ② 的"会报错"有区分力）。
    let good = run(ENTRY_GOOD);
    assert!(
        !good.contains("ElabTacticFailed"),
        "正确的 `by` 块**不该**报错（否则判据 ② 没有区分力）✗\n实际：{good}"
    );
}
