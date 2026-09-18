//! 命令走查（`run_pass` 的 `parse → elab → check-then-add` 前两段）：把每个
//! `Command` elaborate 成 `PendingOp` / `DeclState` / 错误，**不**调用内核。
//!
//! 批次 3 第三刀：从 `check/mod.rs` 的 `run_pass` 主循环整体切出，每个
//! `Command` 变体一个方法，方法体就是原来的 match arm——只动位置不动语义：
//! 外层局部变量变成 `Walk` 的字段（可变累加器）、`run` 的参数（只读上下文）
//! 或 `CmdCtx` 的字段（每个命令派生一次的前缀/模板/信任位）；arm 里的
//! `continue` 改 `return`（每个 arm 都没有内层循环，语义等价）。
//!
//! 单文件模式下 `prefix_src` 仍是**借用**本文件前缀（`Cow::Borrowed`），
//! 不产生额外分配：A1（无 `import` 的文件逐字节不变）不受影响。

use super::{
    by_step_states, failed_state, lower_value, render_expr, skipped, CmdHover, KernelFailed,
    PendingOp, TrustPlan,
};
use crate::compile::elab::{
    build_axiom, build_def, build_example, build_theorem, elab_expr, install_inductive_block,
    ElabCtx, ElabScope, InductiveTable, UnivMap,
};
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::event::CompileOutput;
use crate::compile::goals::{expr_has_hole, open_goal, GoalTemplates};
use crate::compile::prelude::CompileOptions;
use crate::compile::report::{DeclKind, DeclState};
use crate::compile::units::SourceUnit;
use crate::{Binder, Command, CtorDecl, Expr, IotaRule, RecDecl, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::Declar;
use std::borrow::Cow;
use std::collections::HashMap;

/// 命令走查的**可变累加器**（原 `run_pass` 主循环里被 arm 改写的局部变量）。
pub(super) struct Walk<'arena> {
    pub(super) builder: EnvBuilder<'arena>,
    pub(super) known_universes: HashMap<String, Vec<String>>,
    pub(super) inductives: InductiveTable<'arena>,
    pub(super) out: CompileOutput,
    pub(super) ops: Vec<PendingOp<'arena>>,
    pub(super) cmd_hovers: Vec<CmdHover<'arena>>,
    pub(super) decl_states: Vec<DeclState>,
    /// `example` 的内部名计数器（`_example_N`，按出现次序）。
    pub(super) example_idx: usize,
    /// 归纳块装进 env 的声明（与 `ops` 里的同源，只为延长生命周期）。
    pub(super) built_inductives: Vec<Declar<'arena>>,
}

/// 单个命令的派生上下文：每个命令算一次，arm 里按需取用。
struct CmdCtx<'a> {
    idx: usize,
    templates: &'a GoalTemplates,
    /// 合成前缀源码：单文件 = 本文件前缀（借用）；闭包 = 依赖声明 + 本文件前缀。
    prefix_src: Cow<'a, str>,
    /// `[0, before)` 的命令已在上一会话判过：只 elaborate，不判。
    trusted: bool,
    env_before: usize,
    options: &'a CompileOptions,
    skip: Option<&'a KernelFailed>,
}

/// 把一个引用重借成**局部寿命**。
///
/// 为什么需要：`CmdCtx` 里的字段是 `&'x T`（`'x` 是 `c` 的寿命参数）。直接拷出来
/// 会让 `ElabCtx<'arena, 'b>` 的 `'b` 被统一到 `'x`，于是 `'arena: 'x` 变成方法
/// 签名上的义务——那是调用方（甚至 `run`）无法证明的。过一次这个恒等函数，寿命
/// 就回到方法体内的局部推断变量，`'arena: 'b` 自然成立。
#[inline]
fn local<T: ?Sized>(r: &T) -> &T {
    r
}

impl<'arena> Walk<'arena> {
    /// 扁平命令序走查。`flat` 是 `(单元下标, 命令)`，单文件时只有一个单元。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn run<'src>(
        &mut self,
        units: &[SourceUnit<'src>],
        flat: &[(usize, &'src Command)],
        options: &CompileOptions,
        skip: Option<&KernelFailed>,
        trust: Option<&TrustPlan>,
        all_templates: &[GoalTemplates],
        closure_prefixes: &[String],
    ) {
        for (idx, &(unit_idx, command)) in flat.iter().enumerate() {
            let unit = &units[unit_idx];
            let trusted = trust.is_some_and(|t| idx < t.before);
            let env_before = self.builder.declaration_count();
            // `match` 的宇宙查询用前缀源码（与 `by` 同一条合成 `#check` 路线）：
            // 闭包模式 = 依赖声明文本 + 本文件到当前命令为止的前缀。
            let own_prefix = unit
                .file
                .src
                .get(..command.span().start.offset)
                .unwrap_or("");
            let prefix_src: Cow<'_, str> = match closure_prefixes.get(unit_idx) {
                Some(deps) if !deps.is_empty() => {
                    // 本文件前缀里的 `import` 行也要去掉：合成文件里它已经不在
                    // 文件开头，留着会让合成文件解析失败（那正是上一次尝试踩的坑）。
                    Cow::Owned(format!(
                        "{deps}{}",
                        crate::project::importless_source(own_prefix)
                    ))
                }
                _ => Cow::Borrowed(own_prefix),
            };
            let c = CmdCtx {
                idx,
                templates: &all_templates[unit_idx],
                prefix_src,
                trusted,
                env_before,
                options,
                skip,
            };
            match command {
                // `import` 自身不产生声明：被导入模块的命令由项目层按拓扑序
                // 先送进同一个 EnvBuilder（docs/design/imports-and-projects.md §4.5）。
                Command::Import { .. } => {}
                Command::Def {
                    name,
                    universe,
                    ty,
                    val,
                    span,
                } => self.def(&c, name, universe, ty, val, *span),
                Command::Theorem {
                    name,
                    universe,
                    ty,
                    val,
                    span,
                } => self.theorem(&c, name, universe, ty, val, *span),
                Command::Axiom {
                    name,
                    universe,
                    ty,
                    span,
                } => self.axiom(&c, name, universe, ty, *span),
                Command::Example { ty, val, span } => self.example(&c, ty, val, *span),
                Command::InductiveBlock {
                    name,
                    params,
                    ty,
                    constructors,
                    recursor,
                    iota_rules,
                    span,
                } => self.inductive_block(
                    &c,
                    name,
                    params,
                    ty,
                    constructors,
                    recursor,
                    iota_rules,
                    *span,
                ),
                Command::Check { expr, span: _ } => self.check(&c, expr),
                Command::Reduce { expr, span: _ } => self.reduce(&c, expr),
                Command::Print { name, span } => self.print(&c, name, *span),
            }
        }
    }

    /// `def name : T := v`：elaborate 成 `PendingOp::Decl`（或开练习）。
    #[allow(clippy::too_many_arguments)]
    fn def(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        universe: &[String],
        ty: &Expr,
        val: &Expr,
        span: Span,
    ) {
        let idx = c.idx;
        // 空中缀宇宙表：`HashMap::new()` 不分配，逐命令建一张的代价是零。
        let no_universe: UnivMap<'_> = UnivMap::new();
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
        };
        let lowered = match lower_value(ty, val, prefix_src, options) {
            Ok(v) => v,
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Definition,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
                return;
            }
        };
        let val = &lowered.0;
        let by_steps = by_step_states(&lowered.1);
        if trusted {
            // Trusted prefix: keep the environment, skip the kernel.
            // Cached failures keep the name free (check-then-add);
            // open exercises never enter the environment anyway.
            if skip.is_some_and(|s| s.contains_key(&idx)) || open_goal(ty, val, templates).is_some()
            {
                return;
            }
            let mut hovers = Vec::new();
            if let Ok(decl) = build_def(
                &mut self.builder,
                name,
                universe,
                ty,
                val,
                &self.known_universes,
                &mut hovers,
                &elab_ctx,
            ) {
                let _ = self.builder.add_declar(decl);
                self.known_universes
                    .insert(name.to_string(), universe.to_vec());
            }
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Definition,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        let open_info = open_goal(ty, val, templates);
        if let Some(info) = open_info {
            let declared_ty = elab_expr(
                &mut self.builder,
                ty,
                &mut ElabScope::new(),
                &no_universe,
                &self.known_universes,
                &mut Vec::new(),
                None,
                None,
                &elab_ctx,
            )
            .inspect(|_| {
                // hover 行也要：类型子表达式进 hover 表
                self.cmd_hovers.push(CmdHover {
                    env_at: self.builder.declaration_count(),
                    nodes: Vec::new(),
                    cmd: idx,
                });
            })
            .ok();
            let _ = &declared_ty;
            self.ops.push(PendingOp::OpenExercise {
                name: Some(name.to_string()),
                kind: DeclKind::Definition,
                universe: universe.to_vec(),
                declared_ty,
                goal: Some(info.goal),
                binders: info.binders,
                holes: info.holes,
                sub_goals: info.sub_goals,
                refine_template: info.refine_template,
                by_steps: by_steps.clone(),
                span,
                cmd: idx,
            });
            return;
        }
        let mut hovers = Vec::new();
        match build_def(
            &mut self.builder,
            name,
            universe,
            ty,
            val,
            &self.known_universes,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                let name_owned = name.to_string();
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    self.decl_states.push(failed_state(
                        DeclKind::Definition,
                        Some(name_owned.clone()),
                        span,
                        err,
                        idx,
                    ));
                    return;
                }
                self.known_universes
                    .insert(name_owned.clone(), universe.to_vec());
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: Some(name_owned),
                    kind: DeclKind::Definition,
                    declar: decl,
                    by_steps: by_steps.clone(),
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Definition,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
            }
        }
    }

    /// `theorem name : T := v`：与 `def` 同形，只差 `DeclKind` 与构造器。
    #[allow(clippy::too_many_arguments)]
    fn theorem(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        universe: &[String],
        ty: &Expr,
        val: &Expr,
        span: Span,
    ) {
        let idx = c.idx;
        // 空中缀宇宙表：`HashMap::new()` 不分配，逐命令建一张的代价是零。
        let no_universe: UnivMap<'_> = UnivMap::new();
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
        };
        let lowered = match lower_value(ty, val, prefix_src, options) {
            Ok(v) => v,
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Theorem,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
                return;
            }
        };
        let val = &lowered.0;
        let by_steps = by_step_states(&lowered.1);
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx)) || open_goal(ty, val, templates).is_some()
            {
                return;
            }
            let mut hovers = Vec::new();
            if let Ok(decl) = build_theorem(
                &mut self.builder,
                name,
                universe,
                ty,
                val,
                &self.known_universes,
                &mut hovers,
                &elab_ctx,
            ) {
                let _ = self.builder.add_declar(decl);
                self.known_universes
                    .insert(name.to_string(), universe.to_vec());
            }
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Theorem,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        // 尾部复用 + fallback（I13-S5b）：open_goal 的 spine 走查
        // 无法分解时（如超量应用、def 展开间接调用），如果值里有
        // 洞 → 生成 **generic open exercise**（整值 = 一个洞，目标 =
        // 声明类型）。学习者看到的是一个可填充的练习而不是报错。
        let open_info = open_goal(ty, val, templates);
        let open_info = match open_info {
            Some(info) => Some(info),
            None if expr_has_hole(val) => {
                // spine 走查无法分解，但值有洞 → generic open exercise
                Some(crate::compile::goals::OpenGoalInfo {
                    goal: render_expr(ty),
                    binders: Vec::new(),
                    holes: vec![val.span()],
                    sub_goals: Vec::new(),
                    refine_template: None,
                })
            }
            _ => None,
        };
        if let Some(info) = open_info {
            let declared_ty = elab_expr(
                &mut self.builder,
                ty,
                &mut ElabScope::new(),
                &no_universe,
                &self.known_universes,
                &mut Vec::new(),
                None,
                None,
                &elab_ctx,
            )
            .ok();
            self.ops.push(PendingOp::OpenExercise {
                name: Some(name.to_string()),
                kind: DeclKind::Theorem,
                universe: universe.to_vec(),
                declared_ty,
                goal: Some(info.goal),
                binders: info.binders,
                holes: info.holes,
                sub_goals: info.sub_goals,
                refine_template: info.refine_template,
                by_steps: by_steps.clone(),
                span,
                cmd: idx,
            });
            return;
        }
        let mut hovers = Vec::new();
        match build_theorem(
            &mut self.builder,
            name,
            universe,
            ty,
            val,
            &self.known_universes,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                let name_owned = name.to_string();
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    self.decl_states.push(failed_state(
                        DeclKind::Theorem,
                        Some(name_owned.clone()),
                        span,
                        err,
                        idx,
                    ));
                    return;
                }
                self.known_universes
                    .insert(name_owned.clone(), universe.to_vec());
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: Some(name_owned),
                    kind: DeclKind::Theorem,
                    declar: decl,
                    by_steps: by_steps.clone(),
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Theorem,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
            }
        }
    }

    /// `axiom name : T`：只 elaborate 类型，没有值。
    #[allow(clippy::too_many_arguments)]
    fn axiom(&mut self, c: &CmdCtx<'_>, name: &str, universe: &[String], ty: &Expr, span: Span) {
        let idx = c.idx;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
        };
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx)) {
                return;
            }
            let mut hovers = Vec::new();
            if let Ok(decl) = build_axiom(
                &mut self.builder,
                name,
                universe,
                ty,
                &self.known_universes,
                &mut hovers,
                &elab_ctx,
            ) {
                let _ = self.builder.add_declar(decl);
                self.known_universes
                    .insert(name.to_string(), universe.to_vec());
            }
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Axiom,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        let mut hovers = Vec::new();
        match build_axiom(
            &mut self.builder,
            name,
            universe,
            ty,
            &self.known_universes,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                let name_owned = name.to_string();
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    self.decl_states.push(failed_state(
                        DeclKind::Axiom,
                        Some(name_owned.clone()),
                        span,
                        err,
                        idx,
                    ));
                    return;
                }
                self.known_universes
                    .insert(name_owned.clone(), universe.to_vec());
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: Some(name_owned),
                    kind: DeclKind::Axiom,
                    declar: decl,
                    by_steps: Vec::new(),
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Axiom,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
            }
        }
    }

    /// `example : T := v`：没有名字，内部名按出现次序编号（`_example_N`）。
    #[allow(clippy::too_many_arguments)]
    fn example(&mut self, c: &CmdCtx<'_>, ty: &Expr, val: &Expr, span: Span) {
        let idx = c.idx;
        // 空中缀宇宙表：`HashMap::new()` 不分配，逐命令建一张的代价是零。
        let no_universe: UnivMap<'_> = UnivMap::new();
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
        };
        let lowered = match lower_value(ty, val, prefix_src, options) {
            Ok(v) => v,
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states
                    .push(failed_state(DeclKind::Example, None, span, e, idx));
                return;
            }
        };
        let val = &lowered.0;
        let by_steps = by_step_states(&lowered.1);
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx)) || open_goal(ty, val, templates).is_some()
            {
                return;
            }
            self.example_idx += 1;
            let internal_name = format!("_example_{}", self.example_idx);
            let mut hovers = Vec::new();
            if let Ok(decl) = build_example(
                &mut self.builder,
                &internal_name,
                ty,
                val,
                &self.known_universes,
                &mut hovers,
                &elab_ctx,
            ) {
                let _ = self.builder.add_declar(decl);
            }
            return;
        }
        if let Some(err) = skipped(skip, &mut self.out, idx, DeclKind::Example, None, span) {
            self.decl_states.push(err);
            return;
        }
        // 尾部复用 + fallback（I13-S5b）：open_goal 的 spine 走查
        // 无法分解时（如超量应用、def 展开间接调用），如果值里有
        // 洞 → 生成 **generic open exercise**（整值 = 一个洞，目标 =
        // 声明类型）。学习者看到的是一个可填充的练习而不是报错。
        let open_info = open_goal(ty, val, templates);
        let open_info = match open_info {
            Some(info) => Some(info),
            None if expr_has_hole(val) => {
                // spine 走查无法分解，但值有洞 → generic open exercise
                Some(crate::compile::goals::OpenGoalInfo {
                    goal: render_expr(ty),
                    binders: Vec::new(),
                    holes: vec![val.span()],
                    sub_goals: Vec::new(),
                    refine_template: None,
                })
            }
            _ => None,
        };
        if let Some(info) = open_info {
            let declared_ty = elab_expr(
                &mut self.builder,
                ty,
                &mut ElabScope::new(),
                &no_universe,
                &self.known_universes,
                &mut Vec::new(),
                None,
                None,
                &elab_ctx,
            )
            .ok();
            self.ops.push(PendingOp::OpenExercise {
                name: None,
                kind: DeclKind::Example,
                universe: Vec::new(),
                declared_ty,
                goal: Some(info.goal),
                binders: info.binders,
                holes: info.holes,
                sub_goals: info.sub_goals,
                refine_template: info.refine_template,
                by_steps: by_steps.clone(),
                span,
                cmd: idx,
            });
            return;
        }
        self.example_idx += 1;
        let internal_name = format!("_example_{}", self.example_idx);
        let mut hovers = Vec::new();
        match build_example(
            &mut self.builder,
            &internal_name,
            ty,
            val,
            &self.known_universes,
            &mut hovers,
            &elab_ctx,
        ) {
            Ok(decl) => {
                if let Err(e) = self.builder.add_declar(decl.clone()) {
                    let err = CompileError::elab(ErrorKind::ElabDuplicateDeclaration, e, span);
                    self.out.push_error(idx, err.clone());
                    self.decl_states
                        .push(failed_state(DeclKind::Example, None, span, err, idx));
                    return;
                }
                let env_after = self.builder.declaration_count();
                self.ops.push(PendingOp::Decl {
                    name: None,
                    kind: DeclKind::Example,
                    declar: decl,
                    by_steps: by_steps.clone(),
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_after,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states
                    .push(failed_state(DeclKind::Example, None, span, e, idx));
            }
        }
    }

    /// `inductive … end`：整块进 `InductiveTable` 与 env，作为最小增量单元。
    #[allow(clippy::too_many_arguments)]
    fn inductive_block(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        params: &[Binder],
        ty: &Expr,
        constructors: &[CtorDecl],
        recursor: &Option<RecDecl>,
        iota_rules: &[IotaRule],
        span: Span,
    ) {
        let idx = c.idx;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        if trusted {
            // The whole block is the minimal incremental unit: it was
            // kernel-validated together when first checked.
            if skip.is_some_and(|s| s.contains_key(&idx)) {
                return;
            }
            let mut hovers = Vec::new();
            let mut built: Vec<Declar<'_>> = Vec::new();
            if install_inductive_block(
                &mut self.builder,
                &mut self.known_universes,
                &mut self.inductives,
                prefix_src,
                options,
                name,
                params,
                ty,
                constructors,
                recursor.as_ref(),
                iota_rules,
                &mut hovers,
                &mut built,
            )
            .is_ok()
            {
                self.built_inductives.extend(built);
            }
            return;
        }
        if let Some(err) = skipped(
            skip,
            &mut self.out,
            idx,
            DeclKind::Inductive,
            Some(name.to_string()),
            span,
        ) {
            self.decl_states.push(err);
            return;
        }
        let mut hovers = Vec::new();
        let mut built: Vec<Declar<'_>> = Vec::new();
        match install_inductive_block(
            &mut self.builder,
            &mut self.known_universes,
            &mut self.inductives,
            prefix_src,
            options,
            name,
            params,
            ty,
            constructors,
            recursor.as_ref(),
            iota_rules,
            &mut hovers,
            &mut built,
        ) {
            Ok(()) => {
                self.built_inductives.extend(built.iter().cloned());
                self.ops.push(PendingOp::InductiveBlock {
                    name: name.to_string(),
                    declars: built,
                    span,
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: self.builder.declaration_count(),
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states.push(failed_state(
                    DeclKind::Inductive,
                    Some(name.to_string()),
                    span,
                    e,
                    idx,
                ));
            }
        }
    }

    /// `#check e`：只 elaborate，求值在内核阶段（`env_before` 快照）。
    #[allow(clippy::too_many_arguments)]
    fn check(&mut self, c: &CmdCtx<'_>, expr: &Expr) {
        let idx = c.idx;
        // 空中缀宇宙表：`HashMap::new()` 不分配，逐命令建一张的代价是零。
        let no_universe: UnivMap<'_> = UnivMap::new();
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let options = local(c.options);
        let env_before = c.env_before;
        let mut hovers = Vec::new();
        match elab_expr(
            &mut self.builder,
            expr,
            &mut ElabScope::new(),
            &no_universe,
            &self.known_universes,
            &mut hovers,
            None,
            None,
            &ElabCtx {
                prefix_src,
                options,
                inductives: &self.inductives,
            },
        ) {
            Ok(e) => {
                self.ops.push(PendingOp::Check {
                    expr: e,
                    env_at: env_before,
                    span: expr.span(),
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_before,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => self.out.push_error(idx, e),
        }
    }

    /// `#reduce e`：同 `#check`，内核阶段做化简。
    #[allow(clippy::too_many_arguments)]
    fn reduce(&mut self, c: &CmdCtx<'_>, expr: &Expr) {
        let idx = c.idx;
        // 空中缀宇宙表：`HashMap::new()` 不分配，逐命令建一张的代价是零。
        let no_universe: UnivMap<'_> = UnivMap::new();
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let options = local(c.options);
        let env_before = c.env_before;
        let mut hovers = Vec::new();
        match elab_expr(
            &mut self.builder,
            expr,
            &mut ElabScope::new(),
            &no_universe,
            &self.known_universes,
            &mut hovers,
            None,
            None,
            &ElabCtx {
                prefix_src,
                options,
                inductives: &self.inductives,
            },
        ) {
            Ok(e) => {
                self.ops.push(PendingOp::Reduce {
                    expr: e,
                    env_at: env_before,
                    span: expr.span(),
                    cmd: idx,
                });
                self.cmd_hovers.push(CmdHover {
                    env_at: env_before,
                    nodes: hovers,
                    cmd: idx,
                });
            }
            Err(e) => self.out.push_error(idx, e),
        }
    }

    /// `#print name`：只记名字指针，打印在内核阶段。
    #[allow(clippy::too_many_arguments)]
    fn print(&mut self, c: &CmdCtx<'_>, name: &str, span: Span) {
        let idx = c.idx;
        let ptr = self.builder.name_from_str(name);
        self.ops.push(PendingOp::Print {
            name: name.to_string(),
            ptr,
            span,
            cmd: idx,
        });
    }
}
