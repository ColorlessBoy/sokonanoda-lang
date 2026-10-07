use crate::expr::Expr::*;
use crate::level::Level;
use crate::name::Name;
use crate::util::{ExprPtr, LevelPtr, NamePtr, TcCtx};

pub struct DebugPrinter<'x, 't, 'p, A> {
    pub(crate) ctx: &'x TcCtx<'t, 'p>,
    pub(crate) elem_to_print: A,
    /// **G-49**：被打印项**外围**的 binder 名字（从外到内，`len()` = 打印起点的
    /// 深度 ✓）。空 = 旧行为（松散变量打印成 `$k` ✓）。名字非空时松散变量打印成
    /// `$k(名字)` ✓ —— 前端据此渲染「第 k 个绑元（名字）」✓。
    ///
    /// 只服务**报错渲染**（`conv.rs` 的 def-eq 失败路径 ✓），判定路径一个字不动 ✓。
    pub(crate) names: Vec<NamePtr<'t>>,
}

impl<'x, 't: 'x, 'p: 't> TcCtx<'t, 'p> {
    pub fn debug_print<A>(&'x self, elem_to_print: A) -> DebugPrinter<'x, 't, 'p, A> {
        DebugPrinter { ctx: self, elem_to_print, names: Vec::new() }
    }

    /// **G-49**：带外围 binder 名字的调试打印（松散变量 `$k` ⇒ `$k(名字)` ✓）。
    pub fn debug_print_named<A>(
        &'x self,
        elem_to_print: A,
        names: &[NamePtr<'t>],
    ) -> DebugPrinter<'x, 't, 'p, A> {
        DebugPrinter { ctx: self, elem_to_print, names: names.to_vec() }
    }
}

impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, NamePtr<'t>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Name::*;
        match self.ctx.read_name(self.elem_to_print) {
            Anon => Ok(()),
            Str(pfx, sfx, _) => {
                let sfx = self.ctx.read_string(sfx);
                match self.ctx.read_name(pfx) {
                    Anon => write!(f, "{}", sfx),
                    _ => write!(f, "{:?}.{}", self.ctx.debug_print(pfx), sfx),
                }
            }
            Num(pfx, sfx, _) => match self.ctx.read_name(pfx) {
                Anon => write!(f, "{}", sfx),
                _ => write!(f, "{:?}.{}", self.ctx.debug_print(pfx), sfx),
            },
        }
    }
}

use std::fmt;
impl<'x, 't, 'p, A, B> std::fmt::Debug for DebugPrinter<'x, 't, 'p, (A, B)>
where
    A: Copy,
    B: Copy,
    DebugPrinter<'x, 't, 'p, A>: std::fmt::Debug,
    DebugPrinter<'x, 't, 'p, B>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({:?}, {:?})",
            self.ctx.debug_print(self.elem_to_print.0),
            self.ctx.debug_print(self.elem_to_print.1)
        )
    }
}
impl<'x, 't, 'p, A> std::fmt::Debug for DebugPrinter<'x, 't, 'p, &[A]>
where
    A: Copy,
    DebugPrinter<'x, 't, 'p, A>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.elem_to_print.iter().copied().map(|x| self.ctx.debug_print(x))).finish()
    }
}

impl<'x, 't, 'p, A> std::fmt::Debug for DebugPrinter<'x, 't, 'p, Vec<A>>
where
    A: Clone,
    DebugPrinter<'x, 't, 'p, A>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.elem_to_print.clone().into_iter().map(|x| self.ctx.debug_print(x))).finish()
    }
}
impl<'x, 't, 'p, A> std::fmt::Debug for DebugPrinter<'x, 't, 'p, std::rc::Rc<A>>
where
    A: Clone,
    DebugPrinter<'x, 't, 'p, A>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self.ctx.debug_print(self.elem_to_print.as_ref().clone()))
    }
}
impl<'x, 't, 'p, A> std::fmt::Debug for DebugPrinter<'x, 't, 'p, Option<A>>
where
    A: Copy,
    DebugPrinter<'x, 't, 'p, A>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.elem_to_print {
            None => write!(f, "None"),
            Some(ref x) => write!(f, "Some({:?})", self.ctx.debug_print(*x)),
        }
    }
}

impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, LevelPtr<'t>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Level::*;
        match self.ctx.read_level(self.elem_to_print) {
            Zero => write!(f, "0"),
            // **R1a**：层元变量的显示 ✓（对齐 Lean 的 `?u.1` 形状 ✓ —— 我们用 `?m{id}` ✓）。
            MVar(id, _) => write!(f, "?m{id}"),
            Succ(..) => {
                let (val, n) = self.ctx.level_succs(self.elem_to_print);
                if self.ctx.read_level(val) == Zero {
                    write!(f, "{}", n)
                } else {
                    write!(f, "{:?} + {}", self.ctx.debug_print(val), n)
                }
            }
            Max(l, r, _) => {
                write!(f, "max{:?}", self.ctx.debug_print((l, r)))
            }
            IMax(l, r, _) => {
                write!(f, "imax{:?}", self.ctx.debug_print((l, r)))
            }
            Param(name, _) => {
                write!(f, "{:?}", self.ctx.debug_print(name))
            }
        }
    }
}

impl<'x, 't, 'p> DebugPrinter<'x, 't, 'p, ExprPtr<'t>> {
    /// 同一份名字栈下打印**子项**（`App` 的两边、`Proj` 的结构……同深度 ✓）。
    fn sub<B>(&self, e: B) -> DebugPrinter<'x, 't, 'p, B> {
        DebugPrinter { ctx: self.ctx, elem_to_print: e, names: self.names.clone() }
    }

    /// 进一层 binder（`Pi`/`Lambda` 的体 ✓）：名字栈**进一层**再打印
    /// —— 被打印项内部的 binder 会让 de Bruijn 编号整体上移 ✓，
    /// 名字栈必须同步，否则会错位到外层的名字上 ✗。
    fn under_binder<B>(&self, name: NamePtr<'t>, e: B) -> DebugPrinter<'x, 't, 'p, B> {
        let mut names = self.names.clone();
        names.push(name);
        DebugPrinter { ctx: self.ctx, elem_to_print: e, names }
    }

    /// **G-49**：松散变量（`$k`）对应的外围 binder 名字；拿不到/匿名 ⇒ `None`
    /// （保持 `$k`，前端会渲染成「第 k 个绑元」✓）。
    fn loose_binder_name(&self, dbj_idx: u16) -> Option<NamePtr<'t>> {
        let pos = self.names.len().checked_sub(1 + dbj_idx as usize)?;
        let name = *self.names.get(pos)?;
        if matches!(self.ctx.read_name(name), Name::Anon) {
            return None;
        }
        Some(name)
    }
}

impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, ExprPtr<'t>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ctx.read_expr(self.elem_to_print) {
            // **K1**：调试打印里**明确显示**它 ✓（`pp` 那条路另算 —— 见 `pretty_printer` ✓）。
            Meta { id, .. } => write!(f, "?m{}", id),
            // **G-49**：名字拿得到就带上（`$4(β)` ✓ —— 前端渲染成「第 4 个绑元（β）」✓）；
            // 拿不到（匿名 binder / 名字栈与深度不齐）就保持 `$4` ✓。
            Var { dbj_idx, .. } => match self.loose_binder_name(dbj_idx) {
                Some(name) => write!(f, "${}({:?})", dbj_idx, self.ctx.debug_print(name)),
                None => write!(f, "${}", dbj_idx),
            },
            Sort { level, .. } => write!(f, "Sort({:?})", self.ctx.debug_print(level)),
            Const { name, levels, .. } => {
                let levels = self.ctx.read_levels(levels);
                write!(f, "{:?}.{:?}", self.sub(name), self.sub(levels.as_ref()))
            }
            App { fun, arg, .. } => write!(f, "({:?} {:?})", self.sub(fun), self.sub(arg)),
            Let { data: &crate::expr::LetData { binder_name, val, binder_type: binder, body, .. }, .. } => {
                write!(
                    f,
                    "let {:?} : {:?} := {:?} in {:?}",
                    self.sub(binder_name),
                    self.sub(binder),
                    self.sub(val),
                    self.under_binder(binder_name, body)
                )
            }
            Pi { binder_name, binder_type, body, .. } => {
                write!(
                    f,
                    "Pi ({:?} : {:?}), {:?}",
                    self.sub(binder_name),
                    self.sub(binder_type),
                    self.under_binder(binder_name, body)
                )
            }
            Lambda { binder_name, binder_type, body, .. } => {
                write!(
                    f,
                    "fun ({:?} : {:?}) => {:?}",
                    self.sub(binder_name),
                    self.sub(binder_type),
                    self.under_binder(binder_name, body)
                )
            }
            Proj { idx, structure, .. } => {
                write!(f, "%({:?}).{}", self.sub(structure), idx)
            }
            NatLit { ptr, .. } => write!(f, "NLit({})", self.ctx.read_bignum(ptr).unwrap()),
            StringLit { ptr, .. } => write!(f, "SLit({})", self.ctx.read_string(ptr)),
        }
    }
}

impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, crate::util::LevelsPtr<'t>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.ctx.debug_print(self.ctx.read_levels(self.elem_to_print).as_ref()))
    }
}
impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, crate::util::StringPtr<'t>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.ctx.read_string(self.elem_to_print))
    }
}
impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, crate::util::BigUintPtr<'t>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.ctx.read_bignum(self.elem_to_print).unwrap())
    }
}

impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, &crate::env::DeclarInfo<'t>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeclarInfo")
            .field("name", &self.ctx.debug_print(self.elem_to_print.name))
            .field("ty", &self.ctx.debug_print(self.elem_to_print.ty))
            .field("uparams", &self.ctx.debug_print(self.ctx.read_levels(self.elem_to_print.uparams).as_ref()))
            .finish()
    }
}

impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, crate::env::RecRule<'t>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecRule")
            .field("ctor_name", &self.ctx.debug_print(self.elem_to_print.ctor_name))
            .field("ctor_telescope_size_wo_params", &self.elem_to_print.ctor_telescope_size_wo_params)
            .field("val", &self.ctx.debug_print(self.elem_to_print.val))
            .finish()
    }
}

impl<'x, 't, 'p> std::fmt::Debug for DebugPrinter<'x, 't, 'p, crate::env::ReducibilityHint> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self.elem_to_print) }
}
