//! `Expr` 的**遍历/查询**方法（从 `ast.rs` 拆出来，2026-10-16）。  // 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
//!
//! **为什么单独一个文件**：`ast.rs` 在 `scripts/docs-lint.py` 判据 ⑨ 的**存量冻结**
//! 清单里（**只许减不许增** ✗）。本轮要给它加 `Expr::NamedArg`（用户 2026-10-16 的
//! 第 ② 条「支持显式地指定参数名字」）⇒ 先把这 107 行**搬出去**腾出预算 ✓
//! （`AGENTS.md` 硬规则 5「文件接近 ~500 行即拆分」；先例：`prefix_recovery.rs`）。

use crate::ast::Expr;
use crate::span::Span;

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Sort { span, .. }
            | Expr::Ident { span, .. }
            | Expr::UniverseApp { span, .. }
            | Expr::Num { span, .. }
            | Expr::Hole { span }
            | Expr::App { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Forall { span, .. }
            | Expr::Arrow { span, .. }
            | Expr::Plus { span, .. }
            | Expr::Let { span, .. }
            | Expr::By { span, .. }
            | Expr::Match { span, .. }
            | Expr::Notation { span, .. }
            | Expr::SetLiteral { span, .. }
            | Expr::AnonCtor { span, .. }
            | Expr::NamedArg { span, .. } => *span,
        }
    }

    /// 遍历这棵表达式树里**每一个标识符出现点**（名字 + 它自己的 span）。
    ///
    /// **为什么要它**（G-108，2026-10-10 用户实测）：`by` 块里**没能走到
    /// elaborate 的那部分**（证明体以 `sorry` 收尾 ⇒ 走"开放练习"那条路；
    /// 或某条 tactic 报错 ⇒ elaborate 中途 `Err`）**不会产生 `HoverNode`**
    /// ⇒ 那段源码里所有名字都没有 hover 行 ⇒ **F12 整个失效** ✗
    /// （用户原话：「加了 `sorry`（或其他引起报错的 tactic），就会导致
    /// `Eq.refl` 无法跳转」）。词法行就是给这段"补座位"：**不改判定**
    /// （判定仍由 kernel 终审），只让导航/悬停认得这些名字 ✓。
    ///
    /// ⚠ 只收**值/证明体里**的出现点（`Ident`）；`match` 的模式、binder 名字
    /// 是**绑定点**不是使用点 ⇒ 不收（它们本来就不该往外跳）。
    /// `UniverseApp`（`f.{u}`）的 span 覆盖整段 `f.{u}`，给它造行会让 F12
    /// 落在错误宽度上 ⇒ 不收（正常路径 elaborate 时本来就有它的行 ✓）。
    ///
    /// ⚠ **穷尽 match**：新增 `Expr` 变体时编译器会在这里报错 ⇒ 不会漏 ✓。
    pub fn for_each_ident(&self, f: &mut impl FnMut(&str, Span)) {
        match self {
            Expr::Ident { name, span } => f(name, *span),
            Expr::Sort { .. } | Expr::UniverseApp { .. } | Expr::Num { .. } | Expr::Hole { .. } => {
            }
            Expr::App { fun, arg, .. } => {
                fun.for_each_ident(f);
                arg.for_each_ident(f);
            }
            Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
                for binder in binders {
                    if let Some(ty) = binder.ty.as_deref() {
                        ty.for_each_ident(f);
                    }
                }
                body.for_each_ident(f);
            }
            Expr::Arrow {
                domain, codomain, ..
            }
            | Expr::Plus {
                lhs: domain,
                rhs: codomain,
                ..
            } => {
                domain.for_each_ident(f);
                codomain.for_each_ident(f);
            }
            Expr::Let {
                binder, val, body, ..
            } => {
                if let Some(ty) = binder.ty.as_deref() {
                    ty.for_each_ident(f);
                }
                val.for_each_ident(f);
                body.for_each_ident(f);
            }
            Expr::By { tactics, .. } => {
                for tactic in tactics {
                    tactic.for_each_ident(f);
                }
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                scrutinee.for_each_ident(f);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        guard.for_each_ident(f);
                    }
                    arm.body.for_each_ident(f);
                }
            }
            Expr::Notation { lhs, rhs, .. } => {
                if let Some(lhs) = lhs {
                    lhs.for_each_ident(f);
                }
                if let Some(rhs) = rhs {
                    rhs.for_each_ident(f);
                }
            }
            Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
                for element in elements {
                    element.for_each_ident(f);
                }
            }
            Expr::NamedArg { value, .. } => value.for_each_ident(f),
        }
    }

    /// `expr` 里有没有出现标识符 `name`（**只做名字出现性检查，不问作用域**）。
    ///
    /// 从 `spine.rs` 搬过来（判据 ⑨ 的行数预算腾挪，2026-10-16）：`Expr::NamedArg`
    /// 落地要往 `spine.rs` 加一行，而它在冻结清单上 ⇒ 按 `prefix_recovery.rs` 的先例，
    /// 把这一整段搬到 `expr.rs`，`spine.rs` 只留一行转发 ✓。
    pub(crate) fn mentions(&self, name: &str) -> bool {
        match self {
            Expr::Ident { name: n, .. } => n == name,
            Expr::UniverseApp { .. } | Expr::Sort { .. } | Expr::Num { .. } | Expr::Hole { .. } => {
                false
            }
            Expr::App { fun, arg, .. } => fun.mentions(name) || arg.mentions(name),
            Expr::SetLiteral { elements, .. } | Expr::AnonCtor { elements, .. } => {
                elements.iter().any(|e| e.mentions(name))
            }
            Expr::Lambda { binders, body, .. } | Expr::Forall { binders, body, .. } => {
                binders
                    .iter()
                    .any(|b| b.ty.as_deref().is_some_and(|t| t.mentions(name)))
                    || body.mentions(name)
            }
            Expr::Arrow {
                domain, codomain, ..
            } => domain.mentions(name) || codomain.mentions(name),
            Expr::Plus { lhs, rhs, .. } => lhs.mentions(name) || rhs.mentions(name),
            Expr::Let {
                binder, val, body, ..
            } => {
                binder.ty.as_deref().is_some_and(|t| t.mentions(name))
                    || val.mentions(name)
                    || body.mentions(name)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                scrutinee.mentions(name)
                    || arms.iter().any(|arm| {
                        arm.guard.as_ref().is_some_and(|g| g.mentions(name))
                            || arm.body.mentions(name)
                    })
            }
            // 记号节点（G-04 / WO-011）：符号与目标名不是标识符，只走操作数。
            Expr::Notation { lhs, rhs, .. } => {
                lhs.as_deref().is_some_and(|e| e.mentions(name))
                    || rhs.as_deref().is_some_and(|e| e.mentions(name))
            }
            Expr::By { .. } => false,
            Expr::NamedArg { value, .. } => value.mentions(name),
        }
    }

    /// Clone of `expr` with its root span replaced: every variant carries a
    /// span field, so each variant is rebuilt with the given span.
    ///
    /// 从 `compile/goals.rs` 搬过来（判据 ⑨ 的行数预算腾挪，2026-10-16）——
    /// 那边在冻结清单上，而 `Expr::NamedArg` 要往它的 match 里加一行 ✓。
    pub(crate) fn with_root_span(self, span: Span) -> Expr {
        match self {
            Expr::Sort { sort, .. } => Expr::Sort { sort, span },
            Expr::Ident { name, .. } => Expr::Ident { name, span },
            Expr::UniverseApp { name, levels, .. } => Expr::UniverseApp { name, levels, span },
            Expr::Num { value, .. } => Expr::Num { value, span },
            Expr::Hole { .. } => Expr::Hole { span },
            Expr::SetLiteral { elements, .. } => Expr::SetLiteral { elements, span },
            Expr::AnonCtor { elements, .. } => Expr::AnonCtor { elements, span },
            Expr::NamedArg { name, value, .. } => Expr::NamedArg { name, value, span },
            Expr::App { fun, arg, .. } => Expr::App {
                fun,
                arg,
                explicit_spine: false,
                span,
            },
            Expr::Lambda { binders, body, .. } => Expr::Lambda {
                binders,
                body,
                span,
            },
            Expr::Forall { binders, body, .. } => Expr::Forall {
                binders,
                body,
                span,
            },
            Expr::Arrow {
                domain, codomain, ..
            } => Expr::Arrow {
                domain,
                codomain,
                span,
            },
            Expr::Plus { lhs, rhs, .. } => Expr::Plus { lhs, rhs, span },
            Expr::Let {
                binder, val, body, ..
            } => Expr::Let {
                binder,
                val,
                body,
                span,
            },
            Expr::Match {
                scrutinee, arms, ..
            } => Expr::Match {
                scrutinee,
                arms,
                span,
            },
            Expr::By { tactics, .. } => Expr::By { tactics, span },
            // 记号节点（G-04 / WO-011）：换根 span，保留符号/目标/结合性与操作数。
            Expr::Notation {
                symbol,
                target,
                assoc,
                lhs,
                rhs,
                alternatives,
                symbol_span,
                ..
            } => Expr::Notation {
                symbol,
                target,
                assoc,
                lhs,
                rhs,
                alternatives,
                span,
                symbol_span,
            },
        }
    }
}
