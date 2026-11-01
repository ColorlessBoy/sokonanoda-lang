//! **P2-12 S4a**：Pi 臂的**闭包域提示**规则（`conv.rs` 只留一行调用，见 `docs-lint` ⑨ 的 500 行软线）。
//!
//! `unify` 比两个 `Value::Pi` 时，要先合一域、再在**同一个新 binder** 下比 codomain；两个 codomain 闭包各自由
//! [`Self::right_domain_hint`] 拿自己的域当提示（`apply_closure` 的第 4 个参数只给 infer 闭包用：它要拿域去扩展
//! 类型上下文、再**惰性推断**体）。
//!
//! ## 为什么不能两边都用左边的域（S4a 的根因）
//!
//! 2026-11-01 重做的探针（`congrArg.{1} (fun (A : α → Prop) => A x) h`，课程 `unit10-cantor:89`）：
//!
//! ```text
//! [s4a]         gen=1 gen0=0 dom=Pi (?m1073741824 $4..$0), (?m1073741825 …) at=Pi (A : Pi (_ : $4)), Prop
//! [s4a-panic]  expected a pi type, got: Pi ( : $5), Sort(0)      ← $5 在 depth 5 上是越界 bvar
//! ```
//!
//! 左边的域 `dx` 可能是**赋值之前**造的值（`force_all` 不重放已赋值的 `Value::Meta`，见 `app_arg_def_eq`
//! 的注释）⇒ 用它当右闭包的类型上下文，闭包就在错误作用域里推断体 ⇒ 越界 bvar ⇒ 内层 `infer_app_v` panic
//! ⇒ 被 `is_def_eq_at` 的 `catch_unwind` 吞成「不 defeq」⇒ 假 `ArgMismatch`（`congrArg` 家族 8 次）。
//! 两个域此时已经合一过（defeq），所以喂**各自**的域既不改变语义、也不再踩陈旧值。
//!
//! ⚠ **这条也说明 `catch_unwind` 把「判不了」变成了「判成否」**（`docs/architecture.md` G-91 的反例）：
//! 把 panic 当 `Unsupported` 而不是 `false` 值得单开一刀（P2-13 候选）。

impl<'x, 't, 'p> crate::tc::TypeChecker<'x, 't, 'p> {
    /// 右 codomain 闭包的域提示：**它自己的**域 `dy`。`cfg(test)` 下可被
    /// [`set_stale_dom_hint`] 改回左边的 `dx`（反向验证 `p211a_b_*` 的修前态）。
    #[inline]
    pub(crate) fn right_domain_hint(&self, dx: crate::value::V<'t>, dy: crate::value::V<'t>) -> crate::value::V<'t> {
        #[cfg(test)]
        if !STALE_DOM_HINT.with(|c| c.get()) {
            return dx;
        }
        let _ = dx;
        dy
    }
}

#[cfg(test)]
thread_local! {
    /// `true` ⇒ 各自域（判定路径）；`false` ⇒ 左边的域（S4a 之前的行为，只给反向验证）。
    static STALE_DOM_HINT: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// 测试用：关掉/打开「右闭包用左边的域」（反向验证必须在关时判红）。
#[cfg(test)]
pub(crate) fn set_stale_dom_hint(on: bool) {
    STALE_DOM_HINT.with(|c| c.set(on));
}
