//! **闭包环境的结构化相等**（从 `conv.rs` 拆出 —— ⑨ 基线 `crates/kernel/src/conv.rs`
//! **只许减不许增**；K4 的修复承载在这里）。
//!
//! ## 为什么需要这一层
//!
//! `conv.rs` 的 `unify` 对 `Pi`/`Lam` 有一条捷径：同一个 body expr + 同一个 domain
//! 指针 + 环境相等 + 同一种闭包（eval / infer）⇒ 直接判等，不必把 body 展开。
//! 这里的"环境相等"就是 [`TypeChecker::envs_ptr_equal`]。
//!
//! ## Lean 4 对照（`src/kernel/*.cpp`，与 4.34/4.35 同源）
//!
//! * 判等入口 `src/kernel/type_checker.cpp:1047` `type_checker::is_def_eq_core`；
//! * 宇宙层 `src/kernel/level.h:139` `is_equivalent`（结构等价）—— Lean 把宇宙实例
//!   **烘进表达式**（`Expr.sort` / `Expr.const` 的 levels），没有"环境里的层代换"
//!   这一层，所以 `Sort 1` 与 `Sort 0` **永远**不判等。
//!
//! ## 我们原来的洞（K4，LKA `bad/bugs/closure-level-subst`）
//!
//! 我们的求值器把 `ks ↦ vs` 烘进**环境**：`eval.rs::lsub_base` 用
//! `Env::Nil { lsub, .. }` 承载宇宙代换（`value.rs::LevelSub { ks, vs }`）。但
//! `envs_ptr_equal` 对两个 `Env::Nil` **无条件返回 `true`**，`lsub` 连看都不看 ⇒
//! 同一个 body 在 `u := 1` 与 `u := 0` 下的两个闭包被判等
//! （`good : Family.{1}` 被当成 `Family.{0}`），`boom : False` 于是"成立"。
//!
//! ## 判据：`lsub` 的指针等价 = 内容等价（不是"比指针赌运气"）
//!
//! `LevelSub` **只有** `ks`/`vs` 两个 `LevelsPtr`，构造点唯一：
//! `eval.rs::intern_level_sub` 按 `(ks, vs)` 在 `tc_cache.level_subs` 里去重 ⇒
//! 同一个 `TcCtx` 内，**同一代换只有一个 `LevelSub` 指针**。而 level 切片本身在
//! `TcCtx` 里按内容 intern（`util.rs::alloc_level`/`alloc_levels_slice` 先查
//! export-file 的 interner、再进会话 interner），所以 `(ks, vs)` 的指针也由内容
//! 唯一决定。⇒ **指针不同 ⇔ 内容不同**（同一会话内），不存在"内容等价但指针不同"
//! 的误拒来源；反过来，指针相同必然内容相同 ✓。
//!
//! 即使退一步讲：`lsub` 不同 ⇒ **只是不走捷径** ⟹ 回到逐值判等（权威路径）。
//! 这条捷径只负责"允许判等"，从不用来"判不等" ⇒ 最坏也只是多算，不会多拒 ✓。
use crate::tc::TypeChecker;
use crate::value::{Env, E};

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    /// 两个**环境**是否等价 —— 闭包捷径用（`conv.rs` 的 `Pi`/`Lam` 两处）。
    ///
    /// `Env::Nil` 是"空环境 + 宇宙代换"的载体：只有 `lsub` 也相同的两个空环境
    /// 才可互换。`Env::Cons` 链的 `hash` 已经把 `lsub`（根的指针）混进去
    /// （`value.rs::env_extend` + `eval.rs::lsub_base`），所以链上不需要再比一次。
    #[inline]
    pub(crate) fn envs_ptr_equal(e1: E<'t>, e2: E<'t>) -> bool {
        let mut a = e1;
        let mut b = e2;
        loop {
            if std::ptr::eq(a, b) {
                return true;
            }
            match (a, b) {
                // **K4**：两个空环境只有带**同一份**宇宙代换时才相等。
                // （`None` = 无代换；`Some` = 非恒等代换 —— `eval_inst` 对空/恒等
                //   代换直接返回 `empty_env`，所以不会出现"内容其实是恒等"的 `Some`。）
                (Env::Nil { lsub: lx, .. }, Env::Nil { lsub: ly, .. }) => {
                    let (lx, ly) = (*lx, *ly);
                    return match (lx, ly) {
                        (None, None) => true,
                        (Some(x), Some(y)) => std::ptr::eq(x, y),
                        _ => false,
                    };
                }
                (
                    Env::Cons { v: va, parent: pa, hash: ha, .. },
                    Env::Cons { v: vb, parent: pb, hash: hb, .. },
                ) => {
                    if ha != hb {
                        return false;
                    }
                    if !std::ptr::eq(*va, *vb) {
                        return false;
                    }
                    a = *pa;
                    b = *pb;
                }
                _ => return false,
            }
        }
    }
}
