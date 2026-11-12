//! `match` 的**模式编译器**（从 `elab.rs` 拆出：`scripts/docs-lint.py` 判据 ⑨
//! 的源码行数红线 —— `elab.rs` 在存量冻结清单里，**只许减不许增**）。
//! 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：
//! 要读原文用 `git log --all -- <path>`。doc-refs: history-only
//!
//! 职责不变（原文照搬）：把用户写的 `| <pattern> [if <guard>] => body`（可能含
//! 通配、绑定、嵌套构造子、Nat 字面量）**源到源** canonical 化成「每构造子恰好
//! 一条 arm、参数全是绑定」的 `Expr::Match` 树；嵌套匹配生成在该 arm 的 body
//! 里，交回 `elab.rs` 的 lowering 逐层处理。好处：不手搓 de Bruijn，守卫复用
//! prelude `Bool.rec`。设计 ⇒ `docs/design/match-patterns.md` §4。
//!
//! **纯源级**：本模块不碰内核（不 elaborate、不建声明）⇒ 拆出去零行为变化，
//! 判据是 `crates/front` 的既有 match 全量单测。

use std::collections::HashMap;

use crate::ast::{Expr, MatchArm, Pattern};
use crate::compile::elab::{head_ident, src_spine, ElabCtx, InductiveInfo};
use crate::compile::error::{CompileError, ErrorKind};
use crate::span::Span;

// ---- 模式编译器（docs/design/match-patterns.md §4）----
//
// 把用户写的 `| <pattern> [if <guard>] => body`（可能含通配、绑定、嵌套构造子、
// Nat 字面量）**源到源** canonical 化成「每构造子恰好一条 arm、参数全是绑定」
// 的 `Expr::Match` 树；嵌套匹配生成在该 arm 的 body 里，交回本模块既有的
// lowering 逐层处理。好处：不手搓 de Bruijn，守卫复用 prelude `Bool.rec`。

/// 待匹配的一列（顶层是用户写的 scrutinee；嵌套是字段 binder 名）。
#[derive(Clone)]
pub(crate) struct ColVar<'c, 'a> {
    pub(crate) expr: Expr,
    /// 该列类型的归纳元数据（`None` = 参数/未知 → 只能绑定或通配）。
    pub(crate) ind: Option<&'c InductiveInfo<'a>>,
    pub(crate) ind_name: Option<String>,
    /// 该列类型的参数实例（参数名 → 书写实参），用于把字段类型 `A` 代换成
    /// `Option Nat` 里的 `Nat`（否则嵌套模式看不到内层归纳）。
    pub(crate) subst: HashMap<String, Expr>,
}

/// 编译器的一行：模式串（长度 = 列数）+ body + 守卫。
#[derive(Clone)]
pub(crate) struct PatternRow {
    pub(crate) pats: Vec<Pattern>,
    pub(crate) guard: Option<Expr>,
    pub(crate) body: Expr,
}

/// 模式在某一列的解析结果。
enum Resolved {
    Ctor { ci: usize, args: Vec<Pattern> },
    Bind(String),
    Wild,
}

fn bad_arm(message: &str, span: Span) -> CompileError {
    CompileError::elab(ErrorKind::ElabMatchBadArm, message.to_string(), span)
}

fn ident_expr(name: &str, span: Span) -> Expr {
    Expr::Ident {
        name: name.to_string(),
        span,
    }
}

/// 错误消息里列出的构造子拼写：源名与规范名都给（R1 之后源名已不是内核名，
/// 学员按提示写哪一个都能过 —— R3）。
fn ctor_names_text_from(info: &InductiveInfo) -> String {
    info.ctors
        .iter()
        .map(|c| {
            if c.name == c.canonical {
                format!("`{}`", c.name)
            } else {
                format!("`{}`（或 `{}`）", c.canonical, c.name)
            }
        })
        .collect::<Vec<_>>()
        .join("、")
}

/// 构造子下标：源名（`succ`，R3 的源级写法）、规范名（`Nat.succ`，R1）与
/// 该归纳内唯一的裸后缀都命中。
fn ctor_index(info: &InductiveInfo, name: &str) -> Option<usize> {
    if let Some(i) = info.ctors.iter().position(|c| c.name == name) {
        return Some(i);
    }
    if let Some(i) = info.ctors.iter().position(|c| c.canonical == name) {
        return Some(i);
    }
    if name.contains('.') {
        return None;
    }
    let hits: Vec<usize> = info
        .ctors
        .iter()
        .enumerate()
        .filter(|(_, c)| c.canonical.rsplit('.').next() == Some(name))
        .map(|(i, _)| i)
        .collect();
    if hits.len() == 1 {
        Some(hits[0])
    } else {
        None
    }
}

/// Nat 形状：零构造子（0 字段）+ succ 构造子（1 字段，类型回到自身）。
fn nat_shape(info: &InductiveInfo, name: &str) -> Option<(usize, usize)> {
    let zero = info.ctors.iter().position(|c| c.fields.is_empty())?;
    let succ = info.ctors.iter().position(|c| {
        c.fields.len() == 1
            && c.fields[0].src_ty.as_ref().and_then(head_ident).as_deref() == Some(name)
    })?;
    Some((zero, succ))
}

fn resolve_pattern(pat: &Pattern, col: &ColVar) -> Result<Resolved, CompileError> {
    match pat {
        Pattern::Wild { .. } => Ok(Resolved::Wild),
        Pattern::Num { value, span } => {
            let info = col.ind.ok_or_else(|| {
                bad_arm(
                    "数字字面量模式只能用在 Nat 上：这一列不是已知的归纳类型",
                    *span,
                )
            })?;
            let name = col.ind_name.as_deref().unwrap_or("");
            let (zero, succ) = nat_shape(info, name).ok_or_else(|| {
                bad_arm(
                    "数字字面量模式只能用在 Nat 上（0 元零构造子 + 一元 succ 构造子）",
                    *span,
                )
            })?;
            let n: u64 = value.parse().unwrap_or(0);
            if n == 0 {
                Ok(Resolved::Ctor {
                    ci: zero,
                    args: Vec::new(),
                })
            } else {
                Ok(Resolved::Ctor {
                    ci: succ,
                    args: vec![Pattern::Num {
                        value: (n - 1).to_string(),
                        span: *span,
                    }],
                })
            }
        }
        Pattern::Ident { name, args, span } => {
            if let Some(info) = col.ind {
                if let Some(ci) = ctor_index(info, name) {
                    let want = info.ctors[ci].fields.len();
                    if args.len() != want {
                        return Err(bad_arm(
                            &format!(
                                "构造子 `{}` 有 {} 个字段，但这一支写了 {} 个子模式；请写满字段",
                                info.ctors[ci].canonical,
                                want,
                                args.len()
                            ),
                            *span,
                        ));
                    }
                    return Ok(Resolved::Ctor {
                        ci,
                        args: args.clone(),
                    });
                }
            }
            if args.is_empty() {
                return Ok(Resolved::Bind(name.clone()));
            }
            let ty = col.ind_name.clone().unwrap_or_default();
            let available = col.ind.map(ctor_names_text_from).unwrap_or_default();
            Err(bad_arm(
                &format!("`{ty}` 没有构造子 `{name}`；可用的是：{available}"),
                *span,
            ))
        }
    }
}

/// 守卫链：`| p if g1 => b1 | …` 在「模式都已匹配」后按顺序判定，第一个为真
/// 的 body 胜；为假落到下一行；最后一行若仍有守卫 → 没有兜底。
fn guard_chain(rows: &[PatternRow], span: Span) -> Result<Expr, CompileError> {
    let Some(row) = rows.first() else {
        return Err(CompileError::elab(
            ErrorKind::ElabMatchNonExhaustive,
            "`match` 的守卫为假时没有兜底分支：请在后面补一条不加守卫的分支".to_string(),
            span,
        ));
    };
    match &row.guard {
        None => Ok(row.body.clone()),
        Some(g) => {
            let fallback = guard_chain(&rows[1..], span)?;
            let gspan = g.span();
            Ok(Expr::Match {
                scrutinee: Box::new(g.clone()),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Ident {
                            name: "Bool.true".to_string(),
                            args: Vec::new(),
                            span: gspan,
                        },
                        guard: None,
                        body: row.body.clone(),
                        span: gspan,
                    },
                    MatchArm {
                        pattern: Pattern::Ident {
                            name: "Bool.false".to_string(),
                            args: Vec::new(),
                            span: gspan,
                        },
                        guard: None,
                        body: fallback,
                        span: gspan,
                    },
                ],
                span,
            })
        }
    }
}

/// 子模式的列变量：字段的书写类型给出嵌套归纳。
fn field_col<'c, 'a>(
    name: &str,
    src_ty: Option<&Expr>,
    parent_subst: &HashMap<String, Expr>,
    ctx: &'c ElabCtx<'a, '_>,
) -> ColVar<'c, 'a> {
    // 参数化归纳：字段类型里的参数名先代入（`some (a : A)` 在 `Option Nat`
    // 下 → `Nat`），嵌套模式才能解析内层构造子。
    let substituted =
        src_ty.map(|t| super::goals::substitute_names(t, parent_subst, &HashMap::new()));
    let spine = substituted.as_ref().and_then(src_spine);
    let head = spine.as_ref().map(|(h, _)| h.clone());
    let args = spine.map(|(_, a)| a).unwrap_or_default();
    let ind = head.as_deref().and_then(|h| ctx.inductives.get(h));
    let subst = match ind {
        Some(info) => info
            .param_names
            .iter()
            .cloned()
            .zip(args)
            .collect::<HashMap<String, Expr>>(),
        None => HashMap::new(),
    };
    ColVar {
        expr: ident_expr(name, Span::default()),
        ind,
        ind_name: head,
        subst,
    }
}

/// 编译一层的 `match`（`vars` 都已在作用域里），返回一个 body 表达式
/// （可能是生成出来的嵌套 `Expr::Match`，也可能直接就是叶子 body）。
pub(crate) fn compile_pattern_body<'c, 'a>(
    ctx: &'c ElabCtx<'a, '_>,
    vars: &[ColVar<'c, 'a>],
    rows: &[PatternRow],
    span: Span,
) -> Result<Expr, CompileError> {
    if rows.is_empty() {
        return Err(CompileError::elab(
            ErrorKind::ElabMatchNonExhaustive,
            "`match` 的分支不完整：有些取值没有对应分支".to_string(),
            span,
        ));
    }
    let mut resolved: Vec<Vec<Resolved>> = Vec::with_capacity(rows.len());
    let mut all_irrefutable = true;
    for row in rows {
        let mut this = Vec::with_capacity(vars.len());
        for (j, pat) in row.pats.iter().enumerate() {
            let res = resolve_pattern(pat, &vars[j])?;
            if !matches!(res, Resolved::Bind(_) | Resolved::Wild) {
                all_irrefutable = false;
            }
            this.push(res);
        }
        resolved.push(this);
    }
    if all_irrefutable {
        // 这一层所有模式都不可反驳：顺序 + 守卫决定，无需再造 match。
        return guard_chain(rows, span);
    }
    // 选第一处含可反驳模式的列。
    let col = (0..vars.len())
        .find(|&j| {
            resolved
                .iter()
                .any(|r| matches!(r[j], Resolved::Ctor { .. }))
        })
        .expect("at least one refutable pattern exists");
    let info = vars[col]
        .ind
        .ok_or_else(|| bad_arm("无法确定被匹配类型的归纳信息", span))?;
    let mut arms: Vec<MatchArm> = Vec::with_capacity(info.ctors.len());
    for (ci, ctor) in info.ctors.iter().enumerate() {
        let k = ctor.fields.len();
        // 每个字段的嵌套归纳（用于判定某个子模式是不是构造子）。
        let field_inds: Vec<Option<&InductiveInfo<'a>>> = ctor
            .fields
            .iter()
            .map(|f| f.src_ty.as_ref().and_then(head_ident))
            .map(|head| head.and_then(|h| ctx.inductives.get(&h)))
            .collect();
        // 子模式是「绑定」吗（无子模式，且名字不是该字段类型的构造子）。
        let is_bind_arg = |j: usize, name: &str| -> bool {
            !name.contains('.') && field_inds[j].is_none_or(|ind| ctor_index(ind, name).is_none())
        };
        // 1) 字段名：某行在该字段是「绑定」时优先沿用它的名字（canonical 输入
        //    因此保持名字不变 → 编译幂等）；否则用新鲜名。
        let mut field_names: Vec<String> = Vec::with_capacity(k);
        for j in 0..k {
            let mut chosen: Option<String> = None;
            for (ri, _) in rows.iter().enumerate() {
                if let Resolved::Ctor { ci: rci, args } = &resolved[ri][col] {
                    if *rci == ci {
                        if let Pattern::Ident { name, args: a, .. } = &args[j] {
                            if a.is_empty() && is_bind_arg(j, name) && !field_names.contains(name) {
                                chosen = Some(name.clone());
                                break;
                            }
                        }
                    }
                }
            }
            field_names.push(chosen.unwrap_or_else(|| format!("__soko_m{col}_{ci}_{j}")));
        }
        // 2) 逐行特化（去掉 col，换成该构造子的 k 个子模式）。
        let mut sub_rows: Vec<PatternRow> = Vec::new();
        for (ri, row) in rows.iter().enumerate() {
            let (args, bound_column) = match &resolved[ri][col] {
                Resolved::Ctor { ci: rci, args } if *rci == ci => (args.clone(), None),
                Resolved::Ctor { .. } => continue,
                Resolved::Wild => (
                    vec![
                        Pattern::Wild {
                            span: row.pats[col].span(),
                        };
                        k
                    ],
                    None,
                ),
                Resolved::Bind(name) => (
                    vec![
                        Pattern::Wild {
                            span: row.pats[col].span(),
                        };
                        k
                    ],
                    Some(name.clone()),
                ),
            };
            let mut map: HashMap<String, Expr> = HashMap::new();
            for (j, arg) in args.iter().enumerate() {
                if let Pattern::Ident {
                    name, args: sub, ..
                } = arg
                {
                    if sub.is_empty() && is_bind_arg(j, name) && field_names[j] != *name {
                        map.insert(name.clone(), ident_expr(&field_names[j], arg.span()));
                    }
                }
            }
            if let Some(name) = bound_column {
                map.insert(name, vars[col].expr.clone());
            }
            let mut pats = row.pats.clone();
            pats.splice(col..=col, args.iter().cloned());
            let body = super::goals::substitute_names(&row.body, &map, &HashMap::new());
            let guard = row
                .guard
                .as_ref()
                .map(|g| super::goals::substitute_names(g, &map, &HashMap::new()));
            sub_rows.push(PatternRow { pats, guard, body });
        }
        // 3) 新列：去掉 col、在 col 处插入 k 个字段列。
        let mut next_vars: Vec<ColVar<'c, 'a>> = Vec::with_capacity(vars.len() - 1 + k);
        for (j, v) in vars.iter().enumerate() {
            if j != col {
                next_vars.push(v.clone());
            }
        }
        let field_vars: Vec<ColVar<'c, 'a>> = (0..k)
            .map(|j| {
                field_col(
                    &field_names[j],
                    ctor.fields[j].src_ty.as_ref(),
                    &vars[col].subst,
                    ctx,
                )
            })
            .collect();
        next_vars.splice(col..col, field_vars);
        let body = compile_pattern_body(ctx, &next_vars, &sub_rows, span)?;
        arms.push(MatchArm {
            pattern: Pattern::Ident {
                name: ctor.name.clone(),
                args: field_names
                    .iter()
                    .map(|n| Pattern::Ident {
                        name: n.clone(),
                        args: Vec::new(),
                        span,
                    })
                    .collect(),
                span,
            },
            guard: None,
            body,
            span,
        });
    }
    Ok(Expr::Match {
        scrutinee: Box::new(vars[col].expr.clone()),
        arms,
        span,
    })
}
