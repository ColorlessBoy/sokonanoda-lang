//! **S2/S4/S5 的跨检查载体**：元变量的**文本化句柄**（前端持有，无生命周期）。
//!
//! 为什么是文本：内核 `ExportFile::with_ctx` 每次检查新建 arena（`util.rs:883`），
//! 检查一结束里面的 `ExprPtr` 全部作废 ⇒ 前端不能持有内核项。句柄存 **source
//! `Expr`**（自有 AST），每次检查时再 elaborate 成内核项种子（`with_tc_seeded`）。
#![allow(dead_code)] // S5（by 引擎接线）之前只有单测用它
use crate::ast::Expr;
use crate::compile::meta::MetaKind;

/// 一个声明级（或 by 块级）的元变量表。
#[derive(Debug, Default, Clone)]
pub(crate) struct MetaHandle {
    /// `(id, 声明类型)`：类型是**源 AST**（渲染/elaborate 由调用方做）。
    types: Vec<(u32, Expr)>,
    /// `(id, 已赋的值)`：值也是源 AST。
    values: Vec<(u32, Expr)>,
    /// `(id, 种类)`；缺省 `Natural`。
    kinds: Vec<(u32, MetaKind)>,
}

impl MetaHandle {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 建一个元变量（重复登记 ⇒ 覆盖，保留第一次的顺序）。
    pub(crate) fn register(&mut self, id: u32, ty: Expr, kind: MetaKind) {
        match self.types.iter_mut().find(|(i, _)| *i == id) {
            Some(slot) => slot.1 = ty,
            None => self.types.push((id, ty)),
        }
        match self.kinds.iter_mut().find(|(i, _)| *i == id) {
            Some(slot) => slot.1 = kind,
            None => self.kinds.push((id, kind)),
        }
    }

    /// 记下解（可重复赋值 ⇒ 以最后一次为准；与内核"单调赋值"一致）。
    pub(crate) fn set_value(&mut self, id: u32, val: Expr) {
        match self.values.iter_mut().find(|(i, _)| *i == id) {
            Some(slot) => slot.1 = val,
            None => self.values.push((id, val)),
        }
    }

    pub(crate) fn value(&self, id: u32) -> Option<&Expr> {
        self.values.iter().find(|(i, _)| *i == id).map(|(_, v)| v)
    }

    pub(crate) fn ty_of(&self, id: u32) -> Option<&Expr> {
        self.types.iter().find(|(i, _)| *i == id).map(|(_, t)| t)
    }

    pub(crate) fn kind(&self, id: u32) -> MetaKind {
        self.kinds
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, k)| *k)
            .unwrap_or(MetaKind::Natural)
    }

    /// 种子的源 AST 形态：`(id, 类型, 种类)`（elaborate 成内核项由调用方做）。
    pub(crate) fn seeds(&self) -> Vec<(u32, Expr, MetaKind)> {
        self.types
            .iter()
            .map(|(id, ty)| (*id, ty.clone(), self.kind(*id)))
            .collect()
    }

    /// 仍未被解出的 id（声明出口要报"未解元变量"）。
    pub(crate) fn unresolved(&self) -> Vec<u32> {
        self.types
            .iter()
            .map(|(i, _)| *i)
            .filter(|id| self.value(*id).is_none())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn ident(name: &str) -> Expr {
        Expr::Ident {
            name: name.to_string(),
            span: Span::default(),
        }
    }

    #[test]
    fn handle_registers_assigns_and_reports_unresolved() {
        let mut h = MetaHandle::new();
        h.register(7, ident("Prop"), MetaKind::Natural);
        h.register(8, ident("Nat"), MetaKind::SyntheticOpaque);
        assert!(h.ty_of(7).is_some());
        assert_eq!(h.kind(8), MetaKind::SyntheticOpaque);
        assert_eq!(h.unresolved(), vec![7, 8]);
        h.set_value(7, ident("p"));
        assert!(h.value(7).is_some());
        assert_eq!(h.unresolved(), vec![8]);
        let seeds = h.seeds();
        assert_eq!(seeds.len(), 2);
        assert_eq!(seeds[0].0, 7);
        // 覆盖登记不改顺序、以最后一次为准
        h.register(7, ident("Q"), MetaKind::Natural);
        assert_eq!(h.unresolved(), vec![8]);
        assert!(matches!(&h.seeds()[0].1, Expr::Ident { name, .. } if name == "Q"));
    }
}
