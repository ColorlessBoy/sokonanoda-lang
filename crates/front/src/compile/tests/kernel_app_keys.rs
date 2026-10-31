//! **P2-8 判据**：内核应用闸门（`SOKO_STRICT_KERNEL_APP`）的**生效档**必须进每一把缓存键。
//!
//! 夹具 = unit12 c2 的自包含形（`kernel_app_tests::C2_SRC`）：strict + 闸门关 ⇒ 拒；strict + 闸门开 ⇒ 收。
//! 判定缓存是**进程级**的 ⇒ 同一进程里先跑一档、再换另一档，若键里没有闸门，第二档就会拿到第一档的
//! 结论。每个测试给源码加一个**独有的盐**（一行 `axiom`），免得并行测试之间互相预热。
//!
//! 反向验证（实测，见提交说明）：把 `judge::options_key` 里的 `key_tag` 去掉 ⇒ 前两条判红。

use super::kernel_app::{effective, key_bit, key_tag, TEST_OVERRIDE};
use super::kernel_app_tests::C2_SRC;
use super::tests::on_compile_stack;
use super::{compile_fol_with, CompileOptions, Dialect, PreludeMode};
use crate::parse;

const STRICT: CompileOptions = CompileOptions {
    prelude: PreludeMode::Full,
    dialect: Dialect::Strict,
};

/// 在**当前线程**（调用方已在编译栈上）按闸门 `on` 编 `src`，返回诊断。
fn errors_with(src: &str, on: bool) -> Vec<String> {
    TEST_OVERRIDE.with(|c| c.set(Some(on)));
    let file = parse(src).unwrap_or_else(|e| panic!("parse: {e:?}\n{src}"));
    let out = compile_fol_with(&file, &STRICT);
    TEST_OVERRIDE.with(|c| c.set(None));
    out.errors.iter().map(|e| format!("{e:?}")).collect()
}

/// 同一进程、同一份源：按 `order` 依次换挡，每一档的判定必须与该档的**新鲜**结论一致。
fn assert_order(tag: &'static str, order: [bool; 4]) {
    let src = format!("axiom p28_salt_{tag} : Prop\n{C2_SRC}");
    on_compile_stack(move || {
        for (i, on) in order.into_iter().enumerate() {
            let errors = errors_with(&src, on);
            if on {
                assert_eq!(
                    errors,
                    Vec::<String>::new(),
                    "第 {i} 趟（闸门开）必须收 c2；拒绝来自另一档的缓存 ⇒ 闸门不在键里"
                );
            } else {
                assert!(
                    !errors.is_empty(),
                    "第 {i} 趟（闸门关）必须拒 c2；接受来自另一档的缓存 ⇒ 闸门不在键里"
                );
            }
        }
    });
}

#[test]
fn gate_off_then_on_in_one_process_is_not_stale() {
    assert_order("off_first", [false, true, false, true]);
}

#[test]
fn gate_on_then_off_in_one_process_is_not_stale() {
    assert_order("on_first", [true, false, true, false]);
}

/// 键的形状：**关 ⇒ 空串 / 0 位**（闸门关的旧键逐字节不变）；宽松档恒关（闸门只对 strict 生效）。
#[test]
fn key_parts_are_empty_when_the_gate_is_not_effective() {
    on_compile_stack(|| {
        for on in [false, true] {
            TEST_OVERRIDE.with(|c| c.set(Some(on)));
            assert!(!effective(Dialect::Loose));
            assert_eq!((key_tag(Dialect::Loose), key_bit(Dialect::Loose)), ("", 0));
            assert_eq!(effective(Dialect::Strict), on);
            let want = if on { ("/kapp", 0b1_0000) } else { ("", 0) };
            assert_eq!((key_tag(Dialect::Strict), key_bit(Dialect::Strict)), want);
        }
        TEST_OVERRIDE.with(|c| c.set(None));
    });
}

/// 磁盘键：strict 下闸门换挡 ⇒ `cache::key` 变；loose 下闸门不生效 ⇒ 键不变。
#[test]
fn disk_key_changes_with_the_gate_only_when_effective() {
    on_compile_stack(|| {
        let loose = CompileOptions {
            dialect: Dialect::Loose,
            ..STRICT
        };
        let keys = |on: bool| {
            TEST_OVERRIDE.with(|c| c.set(Some(on)));
            let k = (
                super::cache::key("p28", &STRICT),
                super::cache::key("p28", &loose),
            );
            TEST_OVERRIDE.with(|c| c.set(None));
            k
        };
        let (off, on) = (keys(false), keys(true));
        assert_ne!(off.0, on.0, "strict：闸门换挡必须换磁盘键");
        assert_eq!(off.1, on.1, "loose：闸门不生效 ⇒ 键不变");
    });
}

/// **判定缓存层**（`judge::judge_infer`，进程级）：c2 的子项 `{∅} ∩ {(@Set.univ Nat)}` 两档答案不同
/// （实测 off=`(Nat -> Prop) -> Prop`、on=`Set Nat -> Prop`）。同一前缀、同一项，按 `order` 换挡问；
/// 每一问都必须等于该档用**独有前缀**（缓存必 miss）问出的新鲜答案。
fn assert_infer_order(tag: &'static str, order: [bool; 4]) {
    let base = C2_SRC
        .rsplit_once("theorem c2 : ")
        .expect("c2 shape")
        .0
        .to_string();
    on_compile_stack(move || {
        let term = "{∅} ∩ {(@Set.univ Nat)}";
        let ask = |salt: &str, on: bool| {
            TEST_OVERRIDE.with(|c| c.set(Some(on)));
            let prefix = format!("axiom p28_{tag}_{salt} : Prop\n{base}");
            let r = crate::judge::judge_infer(&prefix, &STRICT, &[], term);
            TEST_OVERRIDE.with(|c| c.set(None));
            r
        };
        let fresh = [ask("fresh_off", false), ask("fresh_on", true)];
        assert_ne!(
            fresh[0], fresh[1],
            "夹具失效：两档新鲜答案相同 ⇒ 本测试咬不住任何东西"
        );
        for (i, on) in order.into_iter().enumerate() {
            assert_eq!(
                ask("shared", on),
                fresh[usize::from(on)],
                "第 {i} 趟（闸门 {on}）≠ 新鲜答案 ⇒ 判定缓存回放了另一档 ⇒ 闸门不在键里"
            );
        }
    });
}

#[test]
fn judge_infer_off_then_on_is_not_stale() {
    assert_infer_order("infer_off_first", [false, true, false, true]);
}

#[test]
fn judge_infer_on_then_off_is_not_stale() {
    assert_infer_order("infer_on_first", [true, false, true, false]);
}
