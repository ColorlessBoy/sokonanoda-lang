//! **HO3b（2026-10-26）**：pattern 赋值的**生产 checker** —— `fun args => t : m.ty` 的
//! **完整内核检查**（设计 `docs/design/metavar-pattern.md` §3.3）。
//!
//! ## 为什么必须是 `judge_terms`（不是 `InferOnly`）
//!
//! 近似的**唯一**安全保障是「抽象出来的 λ 在**内核**里真的成立」：`InferOnly` 只推类型、
//! **不查实参类型** ⇒ 一个类型错的 λ 也能"推出"一个类型 ✗（设计 §7 的 ④）。
//! ⇒ 走 [`judge_terms_strict`]（`InferOnly` 不算；`by` 的 `have` 用同一条通道 ✓）。
//!
//! ## 为什么要 `judge_terms_strict`（不是 `judge_terms`）
//!
//! `judge_terms` 在 `by` 块里走**乐观批处理**（先答 `Match`、块末一次判）—— 我们**现在**
//! 就要答案（要拿它决定 `assign` 写不写）⇒ 用**当场判**那条入口（`judge.rs` 的注释原话：
//! 「少数 tactic 需要**当场**拿到结论才能继续」）。乐观档在这里会**静默放松**健全性 ✗。
//!
//! ## 合成文档的形状
//!
//! ```text
//! <prefix 原文>
//! def _soko_judge_0 : (b₁ : T₁) → … → <m.ty> := fun (b₁ : T₁) … => <lambda>
//! ```
//!
//! * `bᵢ` 取 `ElabScope::local_binders()`（**与 `judge_binders` 同一条筛选**：有名字 +
//!   有书写类型；渲染延迟到真要判定时 —— 求解点不付渲染成本 ✓）；
//! * `universe` 取**空**：当前声明的宇宙参数此刻还没登记（`ElabCtx` 里没有它）⇒ 带
//!   宇宙参数的声明上判卷文档会 parse/elaborate 失败 ⇒ **弃权**（fail-closed ✓）。
//!
//! ## 三条 fail-closed 纪律（缺一条都会把"没判"变成"判过了"）
//!
//! 1. **没有 checker ⇒ 规则弃权**（`pattern.rs` 的 ⑨；不是"放行"）。
//! 2. **渲染出 `\0` 名字**（元载体 `\0soko_m*` / 望远镜 fresh 名 `\0soko_mp*`）⇒ 直接
//!    弃权（合成文档必然 parse 不过；省掉一次注定失败的整份前缀重编 ✓）。
//! 3. **重入**（判定里再判定）⇒ 弃权：`judge_terms` 会**再跑一遍完整流水线**（含
//!    implicit 求解 ⇒ 可能又走到 pattern 规则）⇒ 没有这道闸就**无界递归** ✗。
//!
//! ## 前缀缓存（性能纪律）
//!
//! 求解点在**每一次**带隐式实参的应用上都会 `note_ctx`（`elab.rs` 的 5 处）；每次
//! 克隆一份前缀（几 KB～几十 KB）是纯浪费 ⇒ 判定基座（前缀 + options）按**来源身份**
//! 缓存：同一个 `ElabCtx` 的前缀切片地址与长度不变 ⇒ 命中 ⇒ **不克隆**。
//! ⚠ 身份**指针**是安全的：`DeclMVarStore` 是**每条声明**新建的（`new_meta_table`），
//! 而一条声明里的 `prefix_src` 是**同一个 `&str`**（`ElabCtx` 只造一次）⇒ 一个 store
//! 的生命周期里前缀不可能被换成另一份文本 ✓（跨声明不可能复用同一个 store ✓）。

use std::cell::Cell;
use std::rc::Rc;

use crate::ast::Expr;
use crate::compile::elab::ElabCtx;
use crate::compile::prelude::CompileOptions;
use crate::judge::{judge_terms_strict, GoalBinderSpec, Judgement, OpenGoalSpec};

use super::{DeclMVarStore, LocalBinder};

/// 这个项里是否有 **`@` 全显式脊**（`expr` 的 `explicit_spine`）：有 ⇒ pattern 规则
/// 一律弃权（Lean 的 `@` 已经关掉隐式插入，pattern 不该改写它；unit12 `project_chain`
/// 全 `@And.intro` 的误配即由此挡住）。
pub(crate) fn has_explicit_spine(e: &Expr) -> bool {
    if let Expr::App {
        explicit_spine: true,
        ..
    } = e
    {
        return true;
    }
    let mut found = false;
    let _ = crate::spine::map_children(e, &mut |child| {
        if !found && has_explicit_spine(child) {
            found = true;
        }
        child.clone()
    });
    found
}

/// quasi 的捕获闸（pc2f）：实参名在 `t` 里**自由出现** ⇒ `fun x => t` 会把外层
/// 那个局部一起绑掉（`?p` 丢依赖）⇒ 弃权。Lean 条件 2 的 fail-closed 近似。
pub(crate) fn quasi_arg_free(args: &[String], used: &[String]) -> bool {
    args.iter().any(|a| used.iter().any(|u| u == a))
}

/// **顶层 binder 名**（类型最外层的 `Forall`/`Lambda`/`Let`，不含参数里的嵌套 binder）：
/// B3 守卫用它区分「常量签名自己的 binder 泄漏」（`{α : Sort 1} -> …`）与
/// 「参数里的 `fun (x y : Nat) => …`」（G-73）—— 后者是嵌套 binder，必须放行。
pub(crate) fn top_binders(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(e: &Expr, out: &mut Vec<String>) {
        match e {
            Expr::Forall { binders, body, .. } | Expr::Lambda { binders, body, .. } => {
                for b in binders {
                    out.push(b.name.clone());
                }
                walk(body, out);
            }
            Expr::Let { binder, body, .. } => {
                out.push(binder.name.clone());
                walk(body, out);
            }
            Expr::Arrow { codomain, .. } => walk(codomain, out),
            _ => {}
        }
    }
    walk(e, &mut out);
    out
}

/// 判定基座：**一份前缀 + 一份 options**（同一条声明内不变 ⇒ 只克隆一次）。
pub(crate) struct Base {
    prefix_src: String,
    /// 来源切片的**地址/长度**（身份判据，见模块头「前缀缓存」）。
    src_ptr: usize,
    src_len: usize,
    options: CompileOptions,
    /// **唯一记法接口**（`DisplayNotations`）：渲染必须走它，`audit-notation-paths` 会抓绕过。
    notations: crate::display::DisplayNotations,
}

impl Base {
    fn new(
        prefix_src: &str,
        options: CompileOptions,
        notations: Option<&crate::display::DisplayNotations>,
    ) -> Self {
        Base {
            prefix_src: prefix_src.to_string(),
            src_ptr: prefix_src.as_ptr() as usize,
            src_len: prefix_src.len(),
            options,
            notations: notations.cloned().unwrap_or_default(),
        }
    }

    /// 项 → 文本：**只经唯一记法接口**（`fold(render_expr(e))`）。
    fn render(&self, e: &Expr) -> String {
        self.notations.render(e)
    }

    /// 同一个 `ElabCtx`（同一份前缀 + 同一档 options）⇒ `true`。
    fn matches(
        &self,
        prefix_src: &str,
        options: &CompileOptions,
        notations: Option<&crate::display::DisplayNotations>,
    ) -> bool {
        self.src_ptr == prefix_src.as_ptr() as usize
            && self.src_len == prefix_src.len()
            && self.options.prelude == options.prelude
            && self.options.dialect == options.dialect
            && self.notations == notations.cloned().unwrap_or_default()
    }
}

/// **求解点**的基座（`pattern::note_ctx` 调）：store 上已缓存的能复用就复用
/// （**不克隆前缀**），否则新建一份。
pub(super) fn base_for(store: &DeclMVarStore, ctx: &ElabCtx<'_, '_>) -> Rc<Base> {
    let cached = {
        let scope = store.scope.borrow();
        scope.base.clone()
    };
    let base = match cached {
        Some(base) if base.matches(ctx.prefix_src, ctx.options, ctx.notations) => base,
        _ => Rc::new(Base::new(ctx.prefix_src, *ctx.options, ctx.notations)),
    };
    // 写回缓存（`note_ctx` 之后会用同一份 `Scope` 覆盖 `lctx`/`check`，`base` 不变 ✓）。
    store.scope.borrow_mut().base = Some(base.clone());
    base
}

thread_local! {
    /// 判定**重入**闸（见模块头第 3 条）。
    static IN_CHECK: Cell<bool> = const { Cell::new(false) };
}

/// 重入闸的 RAII（panic 展开也弹栈 ⇒ 不会把后续判定永久锁死）。
struct Enter;

impl Enter {
    fn try_enter() -> Option<Enter> {
        IN_CHECK.with(|c| {
            if c.get() {
                None
            } else {
                c.set(true);
                Some(Enter)
            }
        })
    }
}

impl Drop for Enter {
    fn drop(&mut self) {
        IN_CHECK.with(|c| c.set(false));
    }
}

/// **判定**：`lambda : ty` 在内核里成立吗？（成立 ⇒ `true`；判不出 / 判不了 ⇒ `false`）
///
/// 每一次调用都是一整份合成文档的走查（`judge_terms_strict` 自带缓存，同一条约束
/// 重复问只付一次）。
pub(super) fn judge(base: &Base, binders: &[LocalBinder], lambda: &Expr, ty: &Expr) -> bool {
    // ③ 重入 ⇒ 弃权（无界递归的唯一闸）。
    let Some(_enter) = Enter::try_enter() else {
        return false;
    };
    let term = base.render(lambda);
    let goal = base.render(ty);
    // ② 元载体 / fresh 名的 `\0` 前缀在合成文档里**词法非法** ⇒ 弃权（不猜）。
    if term.contains('\u{0}') || goal.contains('\u{0}') {
        return false;
    }
    let Some(binders) = binder_specs(base, binders) else {
        return false;
    };
    let spec = OpenGoalSpec {
        // 见模块头：声明自己的宇宙参数此刻还不存在 ⇒ 空（带 `u` 的形状 ⇒ 弃权）。
        universe: Vec::new(),
        ty: goal,
        binders,
        // `metas: []`（设计 §3.3：这条判定不携带元变量表 —— 载体已在 ⑨ 之前 zonk 过）。
        metas: Vec::new(),
    };
    matches!(
        judge_terms_strict(&base.prefix_src, &base.options, &spec, &[&term])
            .into_iter()
            .next(),
        Some(Judgement::Match)
    )
}

/// `OpenGoalSpec` 的 binder 表：**有名字 + 有书写类型**（与 `judge_binders` 同一条筛选）。
///
/// 延迟渲染是**有意的**：求解点每次都要 `note_ctx`，但真正要判定的只有极少数
/// ⇒ 渲染（每个 binder 一次 `render_expr`）只在判定时付 ✓。
///
/// `None` = 某个 binder 的类型渲染出 `\0` 名字（元载体）⇒ 合成文档必然失败 ⇒ 弃权。
fn binder_specs(base: &Base, binders: &[LocalBinder]) -> Option<Vec<GoalBinderSpec>> {
    let mut out: Vec<GoalBinderSpec> = Vec::with_capacity(binders.len());
    for binder in binders.iter() {
        if binder.name.is_empty() {
            continue;
        }
        let Some(ty) = binder.ty.as_ref() else {
            continue;
        };
        let text = base.render(ty);
        if text.contains('\u{0}') {
            return None;
        }
        out.push(GoalBinderSpec {
            name: binder.name.clone(),
            ty: Some(text),
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests;
