//! **P2-12 S4c 判据**：两件事，各占一段。
//!
//! ## ① 记法前导位的**切分**必须与老路同一条结构前提（`ok_notation_argmismatch` 2 次）
//!
//! 病根（修前实测，`docs/gaps/repro/G45-comp-notation.sokonanoda:54`）：
//! `Function.comp (α β γ : Type) (g : β → γ) (f : α → β) : α → γ` 的望远镜被 `peel_pi` 数出
//! **6** 层（结果类型里的箭头也占一层、名字是空串）。2 个操作数时旧规则只看前缀层 ⇒ 取 `m = 4`
//! ⇒ 操作数被摆到第 4 层 `f` 与第 5 层那个无名箭头上 ⇒ 内核把 `g` 对着 `f : α → β` elaborate
//! ⇒ `ArgMismatch{arg:0}` ⇒ 回落老路（`fallback_old_ok_notation_argmismatch = 2`）。
//! 老路 `notation_prefix_args` 是"从大往小试到**解得出**"才落定，同一份签名它落到 `m = 3` ⇒ 本刀
//! 把那条结构前提（操作数位有名字）写进候选判据 ⇒ **在调内核之前**就取对切分。
//!
//! ## ② `prefix_meta` = **写明的例外**（用户 2026-11-01 15:2x 拍板，S6 安全性的前提）
//!
//! 老路自己也只给得出占位符那一族（`\0soko_m*` ⇒ 内核 `Expr::Meta`）**不许**在开关开时变成诊断：
//! Lean 到声明末尾会报 "don't know how to synthesize implicit argument"，而我们的老路**接受**
//! 带占位符的记法前导位 ⇒ strict 不许比 loose 更严（否则 `unit19-solution` 这类现在能过的课程文件会被拒）。
//! ⇒ 出口记 `nofallback_kept_prefix_meta` 并照旧回落，**census 键一个字节都不改**。

use super::kernel_app::{TEST_FALLBACK, TEST_OVERRIDE, TEST_SOLVED};
use super::kernel_app_census::TEST_SETTLED_ROUTED;
use super::kernel_app_fallback::{TEST_DEAD, TEST_KEPT, TEST_NO_FALLBACK, TEST_NO_PREFIX_META_KEPT};
use super::kernel_app_notation::{TEST_NAIVE_MISSING, TEST_NO_RETRY, TEST_RETRY};
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

/// `G45-comp-notation.sokonanoda` 的最小形状（`Function.comp` + `infixr:90 " ∘ "`，零 import）。
/// 修前：`fallback_old_ok_notation_argmismatch = 2`（老路接住 ⇒ 判卷本来就能过 ⇒ 这条判据咬的是**内核**有没有解出）。
const COMP: &str = "\
def Function.comp (α β γ : Type) (g : β → γ) (f : α → β) : α → γ :=
  fun (x : α) => g (f x)
infixr:90 \" ∘ \" => Function.comp
theorem fun_comp_notation (α β γ : Type) (g : β → γ) (f : α → β) (x : α) :
    (g ∘ f) x = g (f x) :=
  @Eq.refl.{1} γ (g (f x))
";

/// `Set.Le` 那一族（结果不是箭头）**不许**被本刀收窄：`missing` 仍然是 2 ⇒ `A ≼ ∅` 仍走 S4b 的重试。
/// 这是"收窄过头就会把 S4b 救回来的站点又推出去"的反向护栏。
const LE: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.Le (α β : Type) (A : Set α) (B : Set β) : Prop := True
infix:50 \" ≼ \" => Set.Le
def Set.empty (α : Type) : Set α := fun (x : α) => False
notation \"∅\" => Set.empty
theorem t3 (α : Type) (A : Set α) (h : A ≼ ∅) : True := True.intro
";

/// `∅ ≼ A` 在**声明类型**里：空集那一侧的 `β` 没有期望类型可借 ⇒ 老路只能给占位符（= `prefix_meta`）。
/// 与 `unit19-solution.sokonanoda:43` / `unit19-cardinal-le.sokonanoda:85` 同形。
const PREFIX_META: &str = "\
def Set (α : Type) : Type := α → Prop
def Set.Le (α β : Type) (A : Set α) (B : Set β) : Prop := True
infix:50 \" ≼ \" => Set.Le
def Set.empty {α : Type} : Set α := fun (x : α) => False
notation \"∅\" => Set.empty
theorem t (α : Type) (A : Set α) : ∅ ≼ A := True.intro
";

struct Out {
    errors: Vec<String>,
    solved: u64,
    fallback: u64,
    /// `(ok|err, <route><why>)` —— 与 census 的 `fallback_old_<tag>_<route><why>` **同源**。
    settled: Vec<(&'static str, String)>,
    retries: (u64, u64),
    /// `nofallback_kept_*` 的原因序列（开关开着仍回落的那一条）。
    kept: Vec<&'static str>,
    /// 开关开着时**报错**的次数（= `nofallback_dead`）。
    dead: u64,
}

/// `run` 的三个开关。
#[derive(Clone, Copy, Default)]
struct Knobs {
    /// 切分退回 S4c 之前。
    naive: bool,
    /// S3 的第二把开关 `SOKO_STRICT_NO_FALLBACK`。
    no_fallback: bool,
    /// 反向验证：取消 `prefix_meta` 这条写明的例外。
    drop_prefix_meta_kept: bool,
}

fn old_ok(v: &[(&'static str, String)], why: &str) -> usize {
    v.iter()
        .filter(|(tag, w)| *tag == "ok" && w.as_str() == why)
        .count()
}

/// 在新编译栈线程里、闸门强制开、strict 方言下编 `src`。默认 `Knobs` = 今天的生产档。
fn run(src: &str, knobs: Knobs) -> Out {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_NAIVE_MISSING.with(|c| c.set(knobs.naive));
        TEST_NO_RETRY.with(|c| c.set(false));
        TEST_NO_FALLBACK.with(|c| c.set(Some(knobs.no_fallback)));
        TEST_NO_PREFIX_META_KEPT.with(|c| c.set(knobs.drop_prefix_meta_kept));
        TEST_SOLVED.with(|c| c.set(0));
        TEST_FALLBACK.with(|c| c.set(0));
        TEST_RETRY.with(|c| c.set((0, 0)));
        TEST_DEAD.with(|c| c.set(0));
        TEST_KEPT.with(|v| v.borrow_mut().clear());
        TEST_SETTLED_ROUTED.with(|v| v.borrow_mut().clear());
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let options = CompileOptions {
            prelude: PreludeMode::Full,
            dialect: Dialect::Strict,
        };
        let out = compile_fol_with(&file, &options);
        let done = Out {
            errors: out.errors.iter().map(|e| format!("{e:?}")).collect(),
            solved: TEST_SOLVED.with(|c| c.get()),
            fallback: TEST_FALLBACK.with(|c| c.get()),
            settled: TEST_SETTLED_ROUTED.with(|v| v.borrow().clone()),
            retries: TEST_RETRY.with(|c| c.get()),
            kept: TEST_KEPT.with(|v| v.borrow().clone()),
            dead: TEST_DEAD.with(|c| c.get()),
        };
        TEST_OVERRIDE.with(|c| c.set(None));
        TEST_NAIVE_MISSING.with(|c| c.set(false));
        TEST_NO_FALLBACK.with(|c| c.set(None));
        TEST_NO_PREFIX_META_KEPT.with(|c| c.set(false));
        done
    })
}

/// **正**：`∘` 由内核解出（不是老路接住）——`fallback_old_ok_notation_argmismatch == 0`，判卷不变。
#[test]
fn the_notation_split_counts_operand_layers_and_the_kernel_solves_the_composition() {
    let on = run(COMP, Knobs::default());
    assert_eq!(on.errors, Vec::<String>::new(), "判卷必须仍然通过");
    assert_eq!(
        old_ok(&on.settled, "notation_argmismatch"),
        0,
        "`fallback_old_ok_notation_argmismatch` 必须归零（修前 = 2）：{:?}",
        on.settled
    );
    assert!(
        on.solved > 0,
        "`∘` 必须由**内核**解出（solved={} fallback={} settled={:?}）",
        on.solved,
        on.fallback,
        on.settled
    );
}

/// **反**：把切分规则退回 S4c 之前 ⇒ 回落回来（`argmismatch ≥ 2`、内核解出的次数严格更少）⇒ 上面那条判红。
#[test]
fn the_old_split_restores_the_fallback_so_the_judgment_bites() {
    let off = run(
        COMP,
        Knobs {
            naive: true,
            ..Default::default()
        },
    );
    assert_eq!(off.errors, Vec::<String>::new(), "老路接住 ⇒ 判卷不变");
    // 判据是"**回落重新出现**"，不是某个固定条数：同一个夹具、同一份二进制，
    // **单测通道读到 1**、**CLI 通道读到 2**（`scripts/strict/fallback-one.sh` 实测）——
    // 差别在"这条声明在两条通道里被 elaborate 几次"，与切分规则无关。
    // ⚠ 所以这里只咬"≥ 1"；"内核解出的次数严格更少"由下面那条断言咬（那才是本刀的牙）。
    assert!(
        old_ok(&off.settled, "notation_argmismatch") >= 1,
        "退回旧切分 ⇒ `fallback_old_ok_notation_argmismatch` 必须重新出现（单测通道实测 1 次）：{:?}",
        off.settled
    );
    let on = run(COMP, Knobs::default());
    assert!(
        off.solved < on.solved,
        "退回旧切分 ⇒ 内核解出的次数必须**严格更少**（旧 {} vs 新 {}）",
        off.solved,
        on.solved
    );
}

/// **不收窄过头**：结果不是 Arrow 的签名（`Set.Le`）照旧 `missing = 2` ⇒ S4b 的重试仍然救得回 `A ≼ ∅`。
#[test]
fn the_narrowing_does_not_shrink_signatures_whose_result_is_not_an_arrow() {
    let on = run(LE, Knobs::default());
    assert_eq!(on.errors, Vec::<String>::new(), "判卷必须仍然通过");
    assert_eq!(
        old_ok(&on.settled, "notation_unsolved"),
        0,
        "`Set.Le` 这一族仍由 S4b 的重试救回（`fallback_old_ok_notation_unsolved == 0`）：{:?}",
        on.settled
    );
    assert!(
        on.retries.0 > 0,
        "重试必须真的发生过（切分没被收窄过头）：{:?}",
        on.retries
    );
}

/// **正（`prefix_meta` 例外）**：开关开 ⇒ 不报错、`kept` 里只有 `prefix_meta`，而 **census 的键一个字节都没变**
/// （仍是 `notation_unsolved` —— 不许把 `Unsolved` 改名成"不计"）。
#[test]
fn the_placeholder_family_stays_a_fallback_and_keeps_the_census_keys() {
    let off = run(PREFIX_META, Knobs::default());
    assert_eq!(off.errors, Vec::<String>::new(), "开关关 ⇒ 判卷通过");
    assert!(
        old_ok(&off.settled, "notation_unsolved") >= 2,
        "夹具有效性：`∅ ≼ A` 在声明类型里，前导值只能是占位符 ⇒ `notation_unsolved` 至少 2：{:?}",
        off.settled
    );
    assert_eq!(
        off.kept,
        Vec::<&'static str>::new(),
        "开关关着 ⇒ 一个 kept 桶都不许记"
    );

    let on = run(
        PREFIX_META,
        Knobs {
            no_fallback: true,
            ..Default::default()
        },
    );
    assert_eq!(
        on.errors,
        Vec::<String>::new(),
        "开关开 ⇒ 这一族**仍走老路**（strict 不许比 loose 更严）：{:?}",
        on.errors
    );
    assert_eq!(on.dead, 0, "一次诊断都不许出");
    assert_eq!(
        on.kept,
        vec!["prefix_meta", "prefix_meta"],
        "每一次都要记进 `nofallback_kept_prefix_meta`：{:?}",
        on.kept
    );
    assert!(
        old_ok(&on.settled, "notation_unsolved") >= 2,
        "**指标口径不变**：census 键仍是 `notation_unsolved`（不是 `prefix_meta`）：{:?}",
        on.settled
    );
}

/// **反（`prefix_meta` 例外）**：取消这条例外 ⇒ 同两个站点在开关开时走到 `Report::Error` ⇒ `Fell::Dead`
/// （`nofallback_dead > 0`，`SOKO_KERNEL_APP_LOG` 上有 `dead unsolved Set.Le <line>:<col>` 行）⇒ 上面那条判红。
/// 这就是"S6 翻默认之前必须有这条例外"的读数：没有它，26 个真实站点会从"回落"变成"报错路径"。
///
/// ⚠ 实测口径（`reports/2026-11-01-p212-s4c-readings.md` §6.3）：本夹具的站点在**声明类型**里，
/// `Fell::Dead` 之后外层那条声明仍可能判过（rc=0、0 条 `diagnostic`）⇒ **用户可见的红要看站点在
/// 类型位还是值位**；本判据咬的是内核层的 `dead`（结构计数），不拿 rc 冒充。
#[test]
fn dropping_the_placeholder_exception_turns_the_same_sites_into_errors() {
    let dropped = run(
        PREFIX_META,
        Knobs {
            no_fallback: true,
            drop_prefix_meta_kept: true,
            ..Default::default()
        },
    );
    assert_eq!(
        dropped.kept,
        Vec::<&'static str>::new(),
        "例外被取消 ⇒ 一条 kept 都不该记"
    );
    assert!(
        dropped.dead > 0,
        "例外被取消 ⇒ 这两个站点当场变成诊断（`nofallback_dead` > 0）"
    );
    let on = run(
        PREFIX_META,
        Knobs {
            no_fallback: true,
            ..Default::default()
        },
    );
    assert_eq!(on.dead, 0, "有例外 ⇒ 一次 dead 都没有");
    assert!(
        dropped.dead > on.dead,
        "例外被取消 ⇒ `nofallback_dead` 必须严格更多（{} vs {}）",
        dropped.dead,
        on.dead
    );
}
