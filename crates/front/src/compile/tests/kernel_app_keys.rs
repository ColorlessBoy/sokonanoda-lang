//! **P2-8 / P2-12 S6 判据**：内核应用路径的**生效档**必须进每一把缓存键。
//!
//! P2-12 S6 **翻默认**之后，"生效档"不再是环境变量，而是**方言**：
//!
//! * strict ⇒ 内核路径 + 不回落**恒开** ⇒ 键里 `"/kapp/nofb"` / 位 `0b11_0000`；
//! * loose  ⇒ 恒关 ⇒ `""` / `0`（⇒ 宽松档的键与 P2-8 之前**逐字节相同**）。
//!
//! 三件事，一条不许少（同 P2-8 的原意：判定缓存是**进程级**的，键里少了这一维，先跑的那一档会被
//! 另一档**原样回放**）：
//!
//! 1. 形状 = **方言常量**（`TEST_OVERRIDE` / `TEST_NO_FALLBACK` 这两个**测试钩子**另占三档，见
//!    `test_hooks_change_the_key_like_the_removed_switches`），且 `effective` / `no_fallback` 同源；
//! 2. 同一进程换**方言**，编译结论不许串味（`compile_fol_with`，夹具 c1：strict 拒 / loose 收）；
//! 3. 同一进程换方言，**判定缓存**（`judge::options_key`）与**磁盘键**（`cache::key`）都不许回放。
//!
//! 每条判据都用**独有盐**（一行 `axiom`）给源码/前缀打标记，免得并行测试之间互相预热。
//!
//! 反向验证（实测，见 `reports/2026-11-01-p212-s6-readings.md` §2.2）：把 `judge::options_key` 里的
//! `key_tag`（或 `cache::key` 里的 `key_bit`）去掉 ⇒ 本文件相应的用例判红。

use super::kernel_app::{effective, key_bit, key_tag, TEST_OVERRIDE};
use super::kernel_app_fallback::{no_fallback, TEST_NO_FALLBACK};
use super::kernel_app_tests::C2_SRC;
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

const STRICT: CompileOptions = CompileOptions {
    prelude: PreludeMode::Full,
    dialect: Dialect::Strict,
};

const LOOSE: CompileOptions = CompileOptions {
    prelude: PreludeMode::Full,
    dialect: Dialect::Loose,
};

/// 设计稿 §Q1 的 c1（逐字）：strict 档少了回落 ⇒ 内核报「隐式 `A` 解不出」；宽松档照旧接受。
const C1: &str = "axiom g : {A : Type} -> Nat -> Prop\ntheorem t1 : g 0 := sorry\n";

/// 本文件的测试钩子都清掉（`None` = 生产形态）。
fn clear_hooks() {
    TEST_OVERRIDE.with(|c| c.set(None));
    TEST_NO_FALLBACK.with(|c| c.set(None));
}

/// **翻默认的牙**：不设任何钩子（= 生产）时 strict 档两把都开、宽松档两把都关。
///
/// 它咬的是"有人又把闸门装回去且默认关"这一类回归：真那样，`effective(Strict)` 会是 `false`。
#[test]
fn strict_is_the_default_mode_without_any_knob() {
    on_compile_stack(|| {
        clear_hooks();
        assert!(
            effective(Dialect::Strict) && no_fallback(Dialect::Strict),
            "strict 档必须恒走内核路径且不回落（P2-12 S6 翻默认）"
        );
        assert!(
            !effective(Dialect::Loose) && !no_fallback(Dialect::Loose),
            "宽松档必须恒不走内核路径（两档的判定面才分得开）"
        );
    });
}

/// 键的形状：**方言常量** —— strict ⇒ (`"/kapp/nofb"`, `0b11_0000`)、loose ⇒ (`""`, `0`)。
///
/// `0b11_0000` 的两片各自有主（`0b1_0000` = 内核路径、`0b10_0000` = 不回落），低四位归元变量档
/// 与 `SOKO_META_*`（`cache/keys.rs`）⇒ 这条也守住"不与它们撞位"。
#[test]
fn key_parts_are_dialect_constants() {
    on_compile_stack(|| {
        clear_hooks();
        assert_eq!(
            (key_tag(Dialect::Strict), key_bit(Dialect::Strict)),
            ("/kapp/nofb", 0b11_0000)
        );
        assert_eq!(
            (key_tag(Dialect::Loose), key_bit(Dialect::Loose)),
            ("", 0),
            "宽松档的键必须与 P2-8 之前逐字节相同（老缓存继续可用）"
        );
        assert_eq!(
            key_bit(Dialect::Strict) & 0b1111,
            0,
            "低四位归元变量档 / SOKO_META_*，内核路径不许借位"
        );
    });
}

/// **测试钩子的牙**（`#[cfg(test)]`）：同一进程里换档必须换键 —— 否则一臂的判定会被另一臂
/// **原样回放**（实测：`universe_app_heads_are_solved_by_kernel` 的基线档 5→4）。生产没有这一支。
#[test]
fn test_hooks_change_the_key_like_the_removed_switches() {
    on_compile_stack(|| {
        let parts = || (key_tag(Dialect::Strict), key_bit(Dialect::Strict));
        clear_hooks();
        let production = parts();
        TEST_OVERRIDE.with(|c| c.set(Some(true)));
        TEST_NO_FALLBACK.with(|c| c.set(Some(true)));
        assert_eq!(parts(), production, "钩子设成生产档 ⇒ 键必须与生产逐字相同");
        TEST_NO_FALLBACK.with(|c| c.set(Some(false)));
        assert_eq!(
            parts(),
            ("/kapp", 0b1_0000),
            "只开内核路径 = S3 之前的键（回落仍走老路）"
        );
        TEST_OVERRIDE.with(|c| c.set(Some(false)));
        assert_eq!(parts(), ("", 0), "内核路径关 = P2-8 之前的旧键");
        clear_hooks();
    });
}

/// 在**当前线程**（调用方已在编译栈上）用 `options` 编 `src`，返回诊断。
fn errors_with(src: &str, options: &CompileOptions) -> Vec<String> {
    let file = parse(src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
    let out = compile_fol_with(&file, options);
    out.errors.iter().map(|e| format!("{e:?}")).collect()
}

/// 同一进程、同一份源：按 `order` 依次换**方言**，每一档的判定必须与该档的**新鲜**结论一致。
fn assert_order(tag: &'static str, order: [Dialect; 4]) {
    let src = format!("axiom p212s6_salt_{tag} : Prop\n{C1}");
    on_compile_stack(move || {
        clear_hooks();
        let fresh = [errors_with(&src, &LOOSE), errors_with(&src, &STRICT)];
        assert_ne!(
            fresh[0], fresh[1],
            "夹具失效：两档判同一份源必须给出**不同**的诊断（否则本测试咬不住任何东西）：\
             loose={:?} strict={:?}",
            fresh[0], fresh[1]
        );
        for (i, dialect) in order.into_iter().enumerate() {
            let options = if dialect == Dialect::Strict {
                &STRICT
            } else {
                &LOOSE
            };
            let want = &fresh[usize::from(dialect == Dialect::Strict)];
            assert_eq!(
                &errors_with(&src, options),
                want,
                "第 {i} 趟（{dialect:?}）≠ 新鲜答案 ⇒ 编译缓存回放了另一档 ⇒ 方言不在键里"
            );
        }
    });
}

#[test]
fn compile_loose_then_strict_is_not_stale() {
    assert_order(
        "loose_first",
        [
            Dialect::Loose,
            Dialect::Strict,
            Dialect::Loose,
            Dialect::Strict,
        ],
    );
}

#[test]
fn compile_strict_then_loose_is_not_stale() {
    assert_order(
        "strict_first",
        [
            Dialect::Strict,
            Dialect::Loose,
            Dialect::Strict,
            Dialect::Loose,
        ],
    );
}

/// **判定缓存层**（`judge::judge_infer`，进程级）：c2 的子项 `{∅} ∩ {(@Set.univ Nat)}` 两档答案不同
/// （内核路径给 `Set Nat -> Prop`；老路给 `(Nat -> Prop) -> Prop`）。同一前缀、同一项，按 `order`
/// 换方言问；每一问都必须等于该档用**独有前缀**（缓存必 miss）问出的新鲜答案。
fn assert_infer_order(tag: &'static str, order: [Dialect; 4]) {
    let base = C2_SRC
        .rsplit_once("theorem c2 : ")
        .expect("c2 shape")
        .0
        .to_string();
    on_compile_stack(move || {
        clear_hooks();
        let term = "{∅} ∩ {(@Set.univ Nat)}";
        let ask = |salt: &str, options: &CompileOptions| {
            let prefix = format!("axiom p212s6_{tag}_{salt} : Prop\n{base}");
            crate::judge::judge_infer(&prefix, options, &[], term)
        };
        let fresh = [ask("fresh_loose", &LOOSE), ask("fresh_strict", &STRICT)];
        assert_ne!(
            fresh[0], fresh[1],
            "夹具失效：两档新鲜答案相同 ⇒ 本测试咬不住任何东西（loose={:?} strict={:?}）",
            fresh[0], fresh[1]
        );
        for (i, dialect) in order.into_iter().enumerate() {
            let options = if dialect == Dialect::Strict {
                &STRICT
            } else {
                &LOOSE
            };
            assert_eq!(
                ask("shared", options),
                fresh[usize::from(dialect == Dialect::Strict)],
                "第 {i} 趟（{dialect:?}）≠ 新鲜答案 ⇒ 判定缓存回放了另一档 ⇒ 方言不在键里"
            );
        }
    });
}

#[test]
fn judge_infer_loose_then_strict_is_not_stale() {
    assert_infer_order(
        "infer_loose_first",
        [
            Dialect::Loose,
            Dialect::Strict,
            Dialect::Loose,
            Dialect::Strict,
        ],
    );
}

#[test]
fn judge_infer_strict_then_loose_is_not_stale() {
    assert_infer_order(
        "infer_strict_first",
        [
            Dialect::Strict,
            Dialect::Loose,
            Dialect::Strict,
            Dialect::Loose,
        ],
    );
}

/// **磁盘键**（`cache::key`）：两档的键必须不同 —— 键里少了方言/生效档，跨会话就会把另一档的
/// 编译产物原样回放（P2-8 之前那个 bug）。
#[test]
fn disk_key_changes_with_the_dialect() {
    on_compile_stack(|| {
        clear_hooks();
        let strict = super::cache::key("p212s6", &STRICT);
        let loose = super::cache::key("p212s6", &LOOSE);
        assert_ne!(strict, loose, "方言必须换磁盘键");
    });
}
