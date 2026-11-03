//! **P2-12 S3/S6 判据**：strict 档的**不回落**出口（S6 翻默认：这是 strict 的默认，不再有环境变量开关）。
//!
//! 覆盖四件事：
//! * **档位**：只看方言（strict ⇒ `true`、宽松档 ⇒ `false`）；`TEST_NO_FALLBACK` 只给单测关掉它；
//! * **`finish` 的出口**：不回落 ⇒ `Fell::Dead`（种子 c1：`axiom g : {A : Type} -> Nat -> Prop` +
//!   `theorem t1 : g 0 := sorry`，设计稿 Q1 的漏名用例）；关掉（测试钩子 / 宽松档）⇒ `Fell::Fallback`，
//!   诊断与**闸门关**逐字相同；
//! * **`ReportInput` 谁来填**：真数据（注册表签名 + `render_msg` + 望远镜）⇒ 文字点名**源级** `A`、span 落在
//!   `g 0`（2:14）上、正文里一个内核名都不剩；数据取不到（`None`）⇒ **回落**；宽松档 ⇒ 填充器**一次都不跑**
//!   （用一个会 panic 的 `facts` 钉住 ⇒ 热路径零成本不是口号）；
//! * **写明的例外**：`ArgMismatch`（c2）开关开着也**逐字**回落老路（`Report::Fallback("argmismatch_text")`）。
//!
//! 读数用线程局部（`TEST_DEAD` / `TEST_KEPT`）：全局原子计数会被并行测试串扰。

use super::kernel_app::TEST_OVERRIDE;
use super::kernel_app_fallback::{fell, no_fallback, Fell, TEST_DEAD, TEST_KEPT, TEST_NO_FALLBACK};
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileError, CompileOptions, Dialect, PreludeMode};
use crate::parse;
use sokonanoda::elab_app::AppOutcome;

const STRICT: CompileOptions = CompileOptions {
    prelude: PreludeMode::Full,
    dialect: Dialect::Strict,
};

/// 设计稿 §Q1 的 c1：`A` 只出现在隐式位、结果里也没有 ⇒ 内核 `Unsolved{Implicit{0}}`。
const C1: &str = "axiom g : {A : Type} -> Nat -> Prop\ntheorem t1 : g 0 := sorry\n";

/// 设计稿 §Q1 的 c2：第二个显式实参类型不对 ⇒ 内核 `ArgMismatch`（写明的例外，S3 不报）。
const C2: &str = "axiom h : {A : Type} -> A -> A -> Prop\ntheorem t2 : h 0 Nat := sorry\n";

/// 一次编译的读数。
struct Run {
    errors: Vec<CompileError>,
    /// 报错出口（`Fell::Dead`）次数。
    dead: u64,
    /// strict 档仍回落的原因（写明的例外）。
    kept: Vec<&'static str>,
}

impl Run {
    fn texts(&self) -> Vec<String> {
        self.errors.iter().map(|e| format!("{e:?}")).collect()
    }
}

/// 在新编译栈线程里、内核路径 / 不回落用**测试钩子**强制为给定值、strict 方言下编 `src`。
fn run(src: &str, gate: bool, nofb: bool) -> Run {
    let src = src.to_string();
    on_compile_stack(move || {
        TEST_OVERRIDE.with(|c| c.set(Some(gate)));
        TEST_NO_FALLBACK.with(|c| c.set(Some(nofb)));
        TEST_DEAD.with(|c| c.set(0));
        TEST_KEPT.with(|v| v.borrow_mut().clear());
        let file = parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
        let out = compile_fol_with(&file, &STRICT);
        let run = Run {
            errors: out.errors,
            dead: TEST_DEAD.with(|c| c.get()),
            kept: TEST_KEPT.with(|v| v.borrow().clone()),
        };
        TEST_OVERRIDE.with(|c| c.set(None));
        TEST_NO_FALLBACK.with(|c| c.set(None));
        run
    })
}

/// **P2-12 S6 翻默认**："不回落"只由**方言**决定 —— strict 档**默认**就是 `true`，不再看环境变量；
/// 宽松档恒 `false`。`TEST_NO_FALLBACK` 是单测专用（对比两条路），生产没有这道闸门。
#[test]
fn no_fallback_is_the_strict_default() {
    on_compile_stack(|| {
        TEST_OVERRIDE.with(|c| c.set(None));
        TEST_NO_FALLBACK.with(|c| c.set(None));
        assert!(
            no_fallback(Dialect::Strict),
            "strict 档必须**默认**不回落（P2-12 S6 翻默认的判据）"
        );
        assert!(!no_fallback(Dialect::Loose), "宽松档恒不回落");
        TEST_NO_FALLBACK.with(|c| c.set(Some(false)));
        assert!(
            !no_fallback(Dialect::Strict),
            "测试钩子仍要能关掉它（S3 那两条对比臂靠它）"
        );
        TEST_NO_FALLBACK.with(|c| c.set(None));
    });
}

/// **开关关 ⇒ 与闸门关逐字节相同**（P1 的不变量：不接线就不许有可观察差别）。
#[test]
fn switch_off_keeps_todays_diagnostics_verbatim() {
    let gate_off = run(C1, false, false);
    let switch_off = run(C1, true, false);
    assert_eq!(
        switch_off.texts(),
        gate_off.texts(),
        "开关关必须与闸门关逐字相同"
    );
    assert_eq!(
        (switch_off.dead, switch_off.kept.len()),
        (0, 0),
        "开关关 ⇒ 报错出口一次都不走"
    );
    assert!(!gate_off.errors.is_empty(), "夹具有效性：c1 在两档下都判红");
}

/// **开关开 ⇒ `finish` 出 `Fell::Dead`**：人能读的错、点名源级 `A`、span 落在 `g 0`（2:14）、无内核名。
#[test]
fn switch_on_reports_a_readable_error_instead_of_falling_back() {
    let old = run(C1, true, false);
    let new = run(C1, true, true);
    // **G3（2026-11-01）把前提的方向翻过来了**（**不是**放宽断言 ✗）：修前老路的出口是
    // **漏内部名 + 假 span**（`` unknown identifier `\u{0}soko_m*` ``，设计稿 Q1 量过）——
    // 那正是 G3 修的 bug（`pp_shape::elab_engine_value` 在严格档把 `metas` 摘成 `None`
    // ⇒ `decl_meta_carrier` 的第一道闸当场失败 ⇒ 载体漏成 unknown-identifier）。
    // 修后载体照样走 `decl_meta_carrier` ⇒ **老路给的也是能读的诊断** ⇒ 前提从"还漏名"
    // 翻成"**也不许漏名**"：方向反了、**只强不弱**（原来只查 `soko_m` 一根针，现在四根）。
    for bad in ["soko_m", "soko_p", "\u{0}", "?1073741824"] {
        assert!(
            !old.texts().iter().any(|e| e.contains(bad)),
            "G3 后**老路**也不许出现内核名 {bad}（夹具前提已翻方向）：{:?}",
            old.texts()
        );
    }
    assert_eq!(new.dead, 1, "开关开 ⇒ 恰恰一次报错出口：{:?}", new.texts());
    assert_eq!(
        new.kept,
        Vec::<&'static str>::new(),
        "c1 不是例外 ⇒ 不许回落到老路"
    );
    assert_ne!(new.texts(), old.texts(), "开关开必须换掉今天的诊断");
    assert_eq!(new.errors.len(), 1, "恰好一条诊断：{:?}", new.texts());
    let e = &new.errors[0];
    assert_eq!(e.kind, super::ErrorKind::ElabImplicitArgumentUnsolved);
    assert!(
        e.message.contains("`A`"),
        "必须点名**源级**形参 A：{}",
        e.message
    );
    assert!(
        e.message.contains("{A : Type} -> Nat -> Prop"),
        "必须带源级签名：{}",
        e.message
    );
    for bad in ["soko_m", "soko_p", "\u{0}", "?1073741824"] {
        assert!(
            !e.message.contains(bad),
            "诊断里不许出现内核名 {bad}：{}",
            e.message
        );
    }
    // span 必须是**真的**：`theorem t1 : g 0 := sorry` 的 `g 0` 从第 2 行第 14 列起（不是 0:0）。
    assert_eq!(
        (e.span.start.line, e.span.start.column),
        (2, 14),
        "span 必须落在 `g 0` 上：{:?}",
        e.span
    );
}

/// **A′（2026-11-03）**：`ArgMismatch` **直接报**，不再逐字回落老路。
///
/// ⚠ 这条测试的前身叫 `argmismatch_is_still_a_written_down_exception`，断言的正好相反
/// （"必须逐字回落、`dead == 0`、记成 `argmismatch_text` 例外"）。A′ 把那条决策翻过来了：
/// 旧路要回落是因为它要"问判卷器再打印"，而那正是 G-30 那 4,640 字节的来源。
/// 现在正文由前端按"第几位 + 签名"生成（不印内核类型 ⇒ 不会漏内部名）。
#[test]
fn argmismatch_reports_directly_without_falling_back() {
    let old = run(C2, true, false);
    let new = run(C2, true, true);
    assert_eq!(new.dead, 1, "A′：ArgMismatch 直接走报错出口");
    assert!(new.kept.is_empty(), "不再记成写明的例外：{:?}", new.kept);
    let texts = new.texts();
    assert_eq!(texts.len(), 1, "只报一条：{texts:?}");
    assert!(
        texts[0].contains("类型不匹配") && texts[0].contains('h'),
        "正文要说清「哪一位、哪个头」：{texts:?}"
    );
    assert!(
        !texts[0].contains("$0") && !texts[0].contains("def_eq"),
        "不许漏内核内部形态：{texts:?}"
    );
    assert!(!old.errors.is_empty(), "夹具有效性：c2 判红");
}

/// **开关关 ⇒ 填充器一次都不跑**（`facts` 用一个会 panic 的闭包钉住）；**数据取不到 ⇒ 回落**（不报半截错）。
#[test]
fn facts_are_lazy_and_incomplete_data_falls_back() {
    on_compile_stack(|| {
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        let out = AppOutcome::Unsupported("probe");
        TEST_NO_FALLBACK.with(|c| c.set(Some(false)));
        let off = fell(Dialect::Strict, "shape", Some(&out), || {
            panic!("开关关 ⇒ 报错数据一个字节都不许算")
        });
        assert!(matches!(off, Fell::Fallback("shape")), "开关关 ⇒ 回落");
        TEST_NO_FALLBACK.with(|c| c.set(Some(true)));
        let missing = fell(Dialect::Strict, "unsolved", Some(&out), || None);
        assert!(
            matches!(missing, Fell::Fallback("unsolved")),
            "数据不全 ⇒ 回落"
        );
        TEST_NO_FALLBACK.with(|c| c.set(None));
        TEST_OVERRIDE.with(|c| c.set(None));
    });
}
