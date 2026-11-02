//! **η 的两条判等捷径**（`unify_cold` 的最后一站）—— 从 `conv.rs` 拆出（⑨ 基线
//! `crates/kernel/src/conv.rs` **只许减不许增** ✓，K3 的修复承载在这里 ✓）。
//!
//! ## Lean 4 对照（本机 `~/Documents/lean/lean4`，`src/kernel/type_checker.cpp`）
//!
//! * **结构 η**：`:784` `try_eta_struct_core` —— `s` 是构造子应用、参数个数 =
//!   `nparams + nfields`、`is_structure_like`、逐字段比 `mk_proj`；调用点 `:1112`
//!   `try_eta_struct(t_n, s_n)`（两向 ✓）。我们的对应物 = `try_struct_eta` +
//!   `try_eta_struct_v`。
//! * **unit-like 捷径**：`:1035-1047` `is_def_eq_unit_like` —— **单构造子 + 零字段**
//!   只是**前提** ✓；放行前**必须** `is_def_eq_core(t_type, infer_type(s))`（`:1047`）
//!   ⇒ 我们的对应物 = `is_unit_inductive` + 下面那道**两边类型判等**。
//!
//! ## K3（LKA `bad/bugs/unit-rec-rule`，2026-11-02 ✓）
//!
//! 以前 unit-like 分支**只要有一边是 unit-like 就直接 `true`**（另一边连看都不看），
//! 漏掉 Lean `:1047` 那句 ⇒ `PUnit.unit` 与**任何东西**判等 —— 冒烟读数
//! `def_eq(Prop, PUnit.unit) == true`。后果不止一条用例：`inductive.rs`
//! `assert_imported_expr_matches`（「导入的 iota 规则必须等于自推的规则」）被吞成
//! **空操作** ✗（`bugs/unit-rec-rule` 就是这么被接受的）。
//!
//! 现在：unit-like 分支**必须两边类型判等** ✓。方向安全（只会多拒 ✓）；代价只在
//! **类型真是 unit-like** 这一支上多一次 `conv_types_at`（类型本来就已经推断过 ✓）。

use crate::tc::TypeChecker;
use crate::util::NamePtr;
use crate::value::{RigidHead, Value, V};

impl<'x, 't, 'p> TypeChecker<'x, 't, 'p> {
    pub(crate) fn try_struct_eta(&mut self, depth: u32, x: V<'t>, y: V<'t>) -> bool {
        let xt = self.value_type_opt(depth, x);
        let yt = self.value_type_opt(depth, y);
        for ty in [xt, yt].into_iter().flatten() {
            let ty_f = self.force_all(depth, ty);
            if let Value::Rigid { head: RigidHead::Inductive(ind_name, _), .. } = ty_f {
                let ind_name = *ind_name;
                if self.is_unit_inductive(ind_name) {
                    // **Lean `:1047`**：unit-like 只是前提 —— `t_type` 必须与
                    // `infer_type(s)` 判等才放行（K3）。缺这句 ⇒ 任取一边的
                    // 类型是 unit-like，两个**任意**的值就判等 ✗。
                    return match (xt, yt) {
                        (Some(tx), Some(ty)) => self.conv_types_at(depth, tx, ty),
                        _ => false,
                    };
                }
                if self.can_be_struct_memo(ind_name)
                    && (self.try_eta_struct_v(depth, ind_name, x, y) || self.try_eta_struct_v(depth, ind_name, y, x))
                {
                    return true;
                }
            }
        }
        false
    }

    fn value_type_opt(&mut self, depth: u32, v: V<'t>) -> Option<V<'t>> {
        match v {
            Value::Pi { .. } | Value::Lam { .. } => None,
            _ => Some(self.value_type(depth, v)),
        }
    }

    fn try_eta_struct_v(&mut self, depth: u32, ind_name: NamePtr<'t>, x: V<'t>, y: V<'t>) -> bool {
        let (yname, yspine) = match y {
            Value::Rigid { head: RigidHead::Ctor(name, _), spine, .. } => (*name, *spine),
            _ => return false,
        };
        let (num_params, num_fields, inductive_name) = match self.ctor_shape(yname) {
            Some(t) => t,
            None => return false,
        };
        if inductive_name != ind_name {
            return false;
        }
        let yargs = match self.spine_apps(depth, yspine) {
            Some(v) if v.len() == usize::from(num_params) + usize::from(num_fields) => v,
            _ => return false,
        };
        for i in 0..num_fields {
            let proj = self.do_proj(depth, ind_name, i, x);
            let rhs = yargs[usize::from(num_params) + usize::from(i)];
            if !self.unify::<true>(depth, proj, rhs) {
                return false;
            }
        }
        true
    }

    fn is_unit_inductive(&self, ind_name: NamePtr<'t>) -> bool {
        let ind = match self.env.get_structure(&ind_name, false) {
            Some(i) => i,
            None => return false,
        };
        let ctor = match self.env.get_constructor(&ind.all_ctor_names[0]) {
            Some(c) => c,
            None => return false,
        };
        ctor.num_fields == 0
    }
}
