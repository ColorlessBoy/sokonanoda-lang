//! `TcCtx::with_tc_declar_info` —— **归纳块**专用的 `with_tc` 变体（K1，2026-11-02）。
//!
//! ## 为什么单独一个文件
//!
//! `util.rs` 是 **⑨ 只许减不许增** 的存量超长文件（基线 1868 行）⇒ 新函数不能往那里加。
//! 这里只放一个 `impl` 块（Rust 允许在别的模块里给同一个类型写 `impl`），`util.rs` 因此回到基线。
//!
//! ## 它补的是什么
//!
//! `with_tc`（`util.rs`）恒传 `declar_info = None`；而 `infer` 里的宇宙参数守卫
//! （`Sort`/`Const` 两臂的 `all_uparams_defined(.., info.uparams)`）**只有 `Some` 才活**。
//! `def`/`theorem`/`axiom` 经 `check_simple_declar`（`tc.rs:134`）拿得到 `Some`，
//! 归纳块没有逐声明入口 ⇒ 守卫对块内 types/ctors/recs 是死代码。
//! Lean 4 在 `src/kernel/type_checker.cpp:76-81`（`check_level`）由 `inductive.cpp:211-229`
//! 的 `tc().check(type, m_lparams)` 打开 —— 同一个位置、同一条报错。

use crate::env::{DeclarInfo, EnvLimit};
use crate::tc::TypeChecker;
use crate::util::{TcCache, TcCtx};

impl<'t, 'p: 't> TcCtx<'t, 'p> {
    /// Like [`Self::with_tc`], but with the declaration's [`DeclarInfo`] set —
    /// which is what **arms** the universe-parameter guard in `infer`: the
    /// `Sort`/`Const` arms assert `all_uparams_defined(.., info.uparams)`
    /// ("universe parameter not declared by the current declaration").
    ///
    /// A `def`/`theorem`/`axiom` turns that guard on by being checked through
    /// `check_simple_declar` (`Some(*d.info())`); an **inductive block** has no
    /// such per-declaration entry point, so before this existed its
    /// `check_declar_info_v` ran with `declar_info = None` ⇒ the guard was dead
    /// code for every inductive type/constructor/recursor *block-internal*
    /// check, and an export could mention a level parameter the block never
    /// declared (Lean 4 rejects that in `type_checker::check_level`:
    /// `invalid reference to undefined universe level parameter 'u'`, reached
    /// from `inductive.cpp`'s `check_inductive_types` via
    /// `tc().check(type, m_lparams)`).
    pub fn with_tc_declar_info<F, A>(
        &mut self,
        declar_info: DeclarInfo<'t>,
        env_limit: EnvLimit<'p>,
        arena: &'t bumpalo::Bump,
        cache: &mut TcCache<'t, 't>,
        f: F,
    ) -> A
    where
        F: FnOnce(&mut TypeChecker<'_, 't, 'p>) -> A, {
        let env = self.export_file.new_env(env_limit);
        let mut tc = TypeChecker::new(self, &env, arena, Some(declar_info), cache);
        f(&mut tc)
    }

}
