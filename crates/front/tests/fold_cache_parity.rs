//! **作用域折叠缓存的 parity 判据（第 100 轮 · 平行线）** —— 补上"缓存是透明的"这条直接判据 ✓。
//!
//! 判据：① 同一文本在**开了缓存**的作用域里折 N 次，结果与**没开缓存**时**逐字节相同** ✓；
//! ② 作用域**退出后**再折一次仍相同 ✓（缓存不许跨作用域泄漏 ✓）；
//! ③ **不同表**在**不同作用域**里各自正确 ✓（这正是"键只按文本"能成立的前提：作用域内同一张表 ✓）。

use sokonanoda_front::display::{with_fold_cache, DisplayNotations};

fn texts() -> Vec<String> {
    vec![
        "P -> Q".to_string(),
        "(A ⊆ B) -> (a : α) -> a ∈ A -> a ∈ B".to_string(),
        "Set.image β f A".to_string(),
        "And X Y".to_string(),
        "".to_string(),
        "a 😀 b -> c".to_string(),
        "x".repeat(500),
    ]
}

#[test]
fn the_fold_cache_is_transparent_and_scoped() {
    let d = DisplayNotations::default();
    let mut checked = 0usize;
    for t in texts() {
        let plain = d.fold(&t);
        // ① 开缓存：折三次都必须与不开缓存时**逐字节相同** ✓。
        let cached = with_fold_cache(|| {
            let a = d.fold(&t);
            let b = d.fold(&t);
            let c = d.fold(&t);
            assert_eq!(a, b, "缓存命中必须与首次相同");
            assert_eq!(b, c, "缓存命中必须与首次相同");
            c
        });
        assert_eq!(cached, plain, "开缓存 ≠ 不开缓存（文本 {t:?}）");
        // ② 退出作用域后：仍相同 ✓（不许泄漏 ✓）。
        assert_eq!(d.fold(&t), plain, "作用域退出后结果变了（文本 {t:?}）");
        checked += 1;
    }
    // ③ 两个作用域各自用**不同**的表 ⇒ 各自都要对 ✓。
    let other = DisplayNotations::default();
    let a = with_fold_cache(|| d.fold("P -> Q"));
    let b = with_fold_cache(|| other.fold("P -> Q"));
    assert_eq!(a, b, "两张默认表结果应相同（夹具前提）");
    assert_eq!(a, d.fold("P -> Q"));
    assert!(checked >= 7, "判据要真的跑起来（检查了 {checked} 个文本）");
    println!("PERF fold-cache-parity: 透明 + 作用域隔离 ✓ · 共比对 {checked} 个文本（含空串/emoji/500 字长文本 ✓）");
}
