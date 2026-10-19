//! **命名实参**（`g (α := Nat) 5`）—— 用户 2026-10-16 第 ② 条
//! （队列 `SUB-implicit-named-args`）。
//!
//! 用户原话（逐字）：「……真不如**对齐 lean**：**只有 `@` 形式接收隐式参数，正常形式
//! 不接受隐式参数**，并且**支持显式地指定参数名字**。」
//!
//! **Lean 4 对照**（本机源码 `~/Documents/lean/lean4/src/Lean/Elab/App.lean` ✓）：
//! * `:753` `main` 的主循环**先**问 `findBinderName? s.namedArgs binderName`
//!   （`:87` = 按 binder 名线性查找）—— **命中就用它、且不消耗位置实参**，
//!   **无论 binder 是隐式、strict 隐式、instance 隐式还是显式**（那条 `match binfo`
//!   在 `| none =>` 分支里 ⇒ 命名实参**优先于风格** ✓）；
//! * `:1073` `getNextArg?` 同款：**先命名、后位置**，且位置只在 `binderInfo.isExplicit`
//!   时才吃（这条注释正是「隐式实参**不是用命名实参给的**就拿不到」的出处）；
//! * `:238` 在 `namedArgs` 有剩余时抛 `throwInvalidNamedArg`（`:39`：一个码 + 列出
//!   **可用形参名** + code action）⇒ 我们沿用同一条契约（见 `implicit_msg`）✓。
//!
//! **我们怎么做**：形参名取**源级签名文本**（`implicit_msg::source_layer_names`，
//! 与 `implicit::telescope` 同序 ✓）；把命名层从望远镜里**摘掉**、值代换进后面各层，
//! 剩下的事**完全复用**既有求解器（`implicit::solve_prefix_with_args`）⇒ 隐式求解
//! 那套安全性质（解不出就报错、不猜不搜索）**一个字节都不动** ✓。
//!
//! **与 Lean 的差异（如实 ✗）**：
//! ① 头必须是**已知常量**（有源级签名可查）—— 局部假设（`intro`/`have` 进来的）
//!    还绑不了名（Lean 用 `forall` 的 binder 名，我们还没有那条路）；
//! ② 富余位置实参（落到**结果类型**上的那一档，路线 ③）暂不与命名实参组合
//!    ⇒ 那种形状 `return Ok(None)` 交回老路（老路给它自己的诊断 ✓，**不静默**）。
//!
//! 两条差异都**只影响"命名实参 + 那种形状"**：没有命名实参时本模块**一次都不进** ✓
//! ⇒ 既有语料逐字节不变 ✓。

use std::collections::HashMap;

use crate::ast::Expr;
use crate::compile::elab::{elab_expr, ElabCtx, ElabScope, HoverNode, KnownTable, UnivMap};
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::implicit::{self, Layer};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::util::ExprPtr;

/// 这条应用脊上有没有命名实参（**零成本**：没有就是一次线性扫描 ✓）。
pub(crate) fn has_named(args: &[&Expr]) -> bool {
    args.iter().any(|a| matches!(a, Expr::NamedArg { .. }))
}

/// 见 [`has_named`]：直接从**整条应用节点**问（钩子在 `spine_of` 之前要用它 ✓）。
pub(crate) fn spine_has_named(expr: &Expr) -> bool {
    has_named(&crate::spine::spine_of(expr).1)
}

/// 这条脊**必须走老路**吗（一次隐式实参都不插）：① `@` 形态且**没有**命名实参 ✓
/// （`@` 说"脊逐位显式"）；② **或**脊上跨过一条 `@` 边界（`(@f a) x` —— a17 实测：
/// 摊平后 `x` 会被当成位置实参 ⇒ 严格档误判 ✗）。
pub(crate) fn spine_needs_old_path(explicit_spine: bool, expr: &Expr) -> bool {
    (explicit_spine && !spine_has_named(expr)) || crate::spine::spine_crosses_explicit(expr)
}

/// **`@` 脊上「实参自己的期望类型」**（2026-10-19，族 (A)）—— `None` ⇒ 今天的行为 ✓。
///
/// ## 它补的是哪一格
///
/// 老路（[`spine_needs_old_path`] 判真那条）里每个实参的期望类型只在
/// `needs_expected_type(arg)`（`SetLiteral` / 零元记法 / `Lambda`）时才算 ⇒
/// **裸常量实参**（`@Set.ext A (A ∪ Set.univ) Set.univ h` 的第 3 位）拿到
/// `None` ⇒ 它按"函数值"elaborate ⇒ 内核看到的是那个 **Pi**
/// （`∀(α : Sort(1)), Set α`）而不是 `Set (A ∪ Set.univ)` ✗
/// ⇒ `类型不匹配：期望 (Set α)，实际是 ∀(α : Sort(1)), (Set α)`（p17 / d2 实测 ✓）。
///
/// ## 与 Lean 4 同构（本机源码 `d0493e4c1e` ✓，逐条自己读过）
///
/// * `Elab/App.lean:263` `getArgExpectedType := (← get).fType.bindingDomain!` ——
///   `fType` 是**当前部分应用**的类型（`:253` `normalizeFunType` 先 `whnfForall` ✓）；
/// * `:708` `processImplicitArg`：`if (← read).explicit then processExplicitArg`
///   —— `@` 档下**隐式形参也走显式那条** ✓；`:592` `processExplicitArg` 取
///   `getArgExpectedType` ✓；
/// * `:284` `elabAndAddNewArg`：`elabTerm stx expectedType` ⇒ **实参自己的
///   elaborate 拿到该层的域** ✓；`:281` `addNewArg`：
///   `fType := s.fType.bindingBody!.instantiate1 arg` ⇒ 下一个实参的期望类型
///   就是**吃完这个实参之后**的第 0 个 Π 域 ✓；
/// * `@` 的开关在 `:1881`（`| `(@$id:ident) => … (explicit := true)` ✓）。
///
/// ⇒ Lean 里「`@` 只改实参绑到哪个 binder」与「每个实参都吃该层的域」**同时成立** ✓
/// —— 我们缺的正是后半条 ✓。
///
/// ## 三条安全边界（宁可 `None` = 今天的行为，绝不返回**错的**期望类型 ✗）
///
/// ① **只在 `@` 脊上跑**（`explicit_spine`）：这条脊的实参**逐位**对应望远镜
///    （`@` 的语义就是"不插隐式实参" ✓）⇒ 偏应用的第 0 个 Π 域就是这一位的域 ✓；
///    没有 `@` 时实参↔形参的对应**不保证逐位** ⇒ 一个字节都不碰 ✓。
/// ② **便宜闸门**：实参的头必须是**已知常量**且 `implicit_prefix() > 0` ——
///    只有这种形状**真的需要**期望类型（前导隐式只能从期望类型反解 ✓）；
///    别的形状自己解得出来 ⇒ 闸门之外**一次内核调用都不付** ✓。
/// ③ **答不出 ⇒ `None`**（内核也答不出 / 文本回读不了 / 望远镜是空的 ✓）。
pub(crate) fn at_arg_expected<'a>(
    fun: &Expr,
    arg: &Expr,
    explicit_spine: bool,
    scope: &ElabScope<'a>,
    ctx: &ElabCtx<'a, '_>,
    known: &KnownTable,
    env: Option<&mut crate::compile::elab::InplaceEnv<'_, 'a>>,
) -> Option<Expr> {
    if !explicit_spine {
        return None;
    }
    let arg_name = match arg {
        Expr::Ident { name, .. } | Expr::UniverseApp { name, .. } => name.as_str(),
        _ => return None,
    };
    // 遮蔽：局部名不是常量（局部那一档已经答过 ✓，绝不回落到同名常量 ✗）。
    if scope.find_last(arg_name).is_some() {
        return None;
    }
    // ⚠ `locals` 传**空**：遮蔽上面已经判过 ⇒ 这里要的就是**常量**那一档
    //（`resolve_known` 的局部回落正是我们要避免的 ✗）。
    let canonical =
        crate::compile::elab::resolve_known(known, ctx.ns, arg_name, arg.span(), &[]).ok()?;
    if known.get(&canonical)?.implicit_prefix() == 0 {
        return None;
    }
    next_arg_expected(fun, scope, ctx, env, None)
}

/// **偏应用的「下一个实参」的期望类型**（= 第 0 个 Π 域）—— **`@` 脊与 `apply` 共用** ✓。
///
/// ⚠ **必须取「全显式 pp」形态** —— 默认档的 pp **只丢第一个隐式实参**
/// （`img {α β} f A` 打成 `img β f A` ✓，见 `judge::EXPLICIT_PP` 的长注释与 G-71）✗
/// ⇒ 那份文本**回读成项**时会**错位对齐**（`α := β, β := f, f := A`）✗ ⇒ 期望类型是错的 ✗
/// （`apply` 那条实测过：`mem (f x) (img β f A)` ⇒ 展开成 `A a` ⇒ `[clash] 左=Ex ｜ 右=A a` ✗）。
/// 全显式档打的是 `@img α β f A` ✓ ⇒ 回读**逐位对齐** ✓。
/// **逃生门**（反向验证 ✓）：`SOKO_KERNEL_ARG_PP=default` ⇒ 退回**默认 pp 档** ✓。
pub(crate) fn next_arg_expected<'a>(
    fun: &Expr,
    scope: &ElabScope<'a>,
    ctx: &ElabCtx<'a, '_>,
    env: Option<&mut crate::compile::elab::InplaceEnv<'_, 'a>>,
    // 调用方有**真宇宙表**就传进来（`apply` 那条有 ✓）；`@` 脊那条没有 ⇒ `None` ✓。
    univ: Option<&UnivMap<'a>>,
) -> Option<Expr> {
    let explicit_pp = std::env::var("SOKO_KERNEL_ARG_PP").ok().as_deref() != Some("default");
    let ty_text = crate::compile::elab::infer_type_text(ctx, scope, fun, env, univ, explicit_pp)?;
    let (layers, _) = crate::compile::elab::notation_telescope(&ty_text)?;
    let (_, domain) = layers.first()?;
    Some(domain.clone())
}

/// 把命名实参应用落到内核项上。返回 `Ok(None)` ⇒ 交回老路（**绝不静默**）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply<'a>(
    builder: &mut EnvBuilder<'a>,
    expr: &Expr,
    head_src: &Expr,
    ty_text: &str,
    layers: &[Layer],
    result: &Expr,
    k: usize,
    args: &[&Expr],
    scope: &mut ElabScope<'a>,
    univ: &UnivMap<'a>,
    known: &KnownTable,
    hovers: &mut Vec<HoverNode<'a>>,
    expected_src: Option<&Expr>,
    ctx: &ElabCtx<'a, '_>,
    mut solved_out: Option<&mut Vec<Expr>>,
) -> Result<Option<ExprPtr<'a>>, CompileError> {
    let app_span = expr.span();
    let Some(names) = crate::compile::implicit_msg::source_layer_names(ty_text) else {
        return Ok(None);
    };
    // ── 1) 按**源级形参名**分配（Lean `App.lean:753` 的 `findBinderName?` ✓）──────
    let mut given: Vec<Option<Expr>> = vec![None; layers.len()];
    let mut positional: Vec<Expr> = Vec::new();
    for a in args {
        match a {
            Expr::NamedArg { name, value, span } => {
                let target = names.iter().position(|(n, _)| n == name);
                let Some(j) = target.filter(|j| *j < layers.len()) else {
                    return Err(CompileError::elab(
                        ErrorKind::ElabInvalidNamedArgument,
                        crate::compile::implicit_msg::unknown_named_arg_message(
                            name, &names, ty_text,
                        ),
                        *span,
                    ));
                };
                if given[j].is_some() {
                    return Err(CompileError::elab(
                        ErrorKind::ElabInvalidNamedArgument,
                        crate::compile::implicit_msg::duplicate_named_arg_message(name, ty_text),
                        *span,
                    ));
                }
                given[j] = Some((**value).clone());
            }
            other => positional.push((*other).clone()),
        }
    }
    // ── 2) 缩简望远镜：命名层**摘掉**，值代换进后面各层 ✓ ──────────────────────
    let mut sigma: HashMap<String, Expr> = HashMap::new();
    let mut red: Vec<Layer> = Vec::new();
    let mut red_src: Vec<usize> = Vec::new();
    for j in 0..layers.len() {
        let domain = crate::spine::substitute(&layers[j].domain, &sigma);
        match given[j].clone() {
            Some(v) => {
                if !layers[j].name.is_empty() {
                    sigma.insert(layers[j].name.clone(), v);
                }
            }
            None => {
                red.push(Layer {
                    name: layers[j].name.clone(),
                    domain,
                    style: layers[j].style.clone(),
                });
                red_src.push(j);
            }
        }
    }
    let result_sub = crate::spine::substitute(result, &sigma);
    let red_k = red_src.iter().take_while(|&&j| j < k).count();
    if positional.len() > red.len() - red_k {
        return Ok(None); // 富余实参（路线 ③）暂不与命名实参组合 ⇒ 交回老路 ✓
    }
    // ── 3) 解**剩下**的隐式前缀（原料 = 位置实参的类型；求解器本体零改动 ✓）──────
    let pos_refs: Vec<&Expr> = positional.iter().collect();
    let solved: Vec<Expr> = if red_k == 0 {
        Vec::new()
    } else {
        let arg_tys =
            crate::compile::pp_shape::arg_types(&pos_refs, ctx, scope, known, &mut *builder);
        let is_inductive = |n: &str| ctx.inductives.contains_key(n);
        match implicit::solve_prefix_with_args(
            &red,
            &result_sub,
            red_k,
            &arg_tys,
            &pos_refs,
            expected_src,
            ctx.defs,
            &is_inductive,
        ) {
            Some(v) => v,
            None => {
                return Err(CompileError::elab(
                    ErrorKind::ElabImplicitArgumentUnsolved,
                    crate::compile::implicit_msg::named_arg_unsolved_message(ty_text, k),
                    app_span,
                ));
            }
        }
    };
    // ── 4) 逐层定值：命名 ✓ → 隐式（解出来的）✓ → 位置（**按剩余显式层的顺序** ✓）──
    let mut values: Vec<Option<Expr>> = given.clone();
    for (i, j) in red_src.iter().enumerate() {
        if *j < k {
            values[*j] = solved.get(i).cloned();
        }
    }
    let mut it = positional.iter();
    for (j, slot) in values.iter_mut().enumerate() {
        if slot.is_none() && j >= k {
            if let Some(v) = it.next() {
                *slot = Some((*v).clone());
            }
        }
    }
    let last_filled = match values.iter().rposition(|v| v.is_some()) {
        Some(j) => j,
        None => return Ok(None),
    };
    if values[..=last_filled].iter().any(|v| v.is_none()) {
        return Ok(None); // 中间有洞（位置实参不够够到后面的命名层）⇒ 交回老路 ✓
    }
    // ── 5) 装配：与主路径**同一口径**（每层都吃「代入后」的期望类型 ✓）───────────
    let mut out = elab_expr(
        builder, head_src, scope, univ, known, hovers, None, None, ctx,
    )?;
    let mut sigma2: HashMap<String, Expr> = HashMap::new();
    let mut prefix_vals: Vec<Expr> = Vec::new();
    for j in 0..=last_filled {
        let Some(value) = values[j].clone() else {
            return Ok(None);
        };
        let expected = crate::spine::substitute(&layers[j].domain, &sigma2);
        let term = elab_expr(
            builder,
            &value,
            scope,
            univ,
            known,
            hovers,
            None,
            Some(&expected),
            ctx,
        )?;
        out = builder.mk_app(out, term);
        if !layers[j].name.is_empty() {
            sigma2.insert(layers[j].name.clone(), value.clone());
        }
        if j < k {
            prefix_vals.push(value);
        }
    }
    if let Some(sink) = solved_out.as_mut() {
        sink.extend(prefix_vals);
    }
    Ok(Some(out))
}
