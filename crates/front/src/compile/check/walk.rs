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
    make_univ_map, params_of_ty, resolve_known, strip_lambdas_n, DefInfo, ElabCtx, ElabScope,
    HoverNode, InductiveTable, KnownName, KnownTable, UnivMap,
};
use crate::compile::error::{CompileError, ErrorKind};
use crate::compile::event::CompileOutput;
use crate::compile::goals::{expr_has_hole, open_goal, spine_without_arg, GoalTemplates};
use crate::compile::prelude::CompileOptions;
use crate::compile::report::{DeclKind, DeclState};
use crate::compile::scope::{NamespaceScope, OpenEntry};
use crate::compile::units::SourceUnit;
use crate::{Binder, Command, CtorDecl, Expr, IotaRule, OpenFilter, RecDecl, Span};
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::Declar;
use sokonanoda::util::ExprPtr;
use std::borrow::Cow;

/// 命令走查的**可变累加器**（原 `run_pass` 主循环里被 arm 改写的局部变量）。
pub(super) struct Walk<'arena> {
    /// **显示期的记法表**（线 C）：整趟建一次，给 `ty_text` 与 `by` 步进的
    /// 展示副本共用（`check/mod.rs` 的 `display_notations`）。
    pub(super) display: crate::display::DisplayNotations,
    pub(super) builder: EnvBuilder<'arena>,
    /// **影子环境**（T-K12b）：一份**只读给 judge 用**的环境，内容是"到目前为
    /// 止已经 elaborate 且**已通过内核检查**的前缀"。它从 `self.ops` **惰性重放**
    /// （[`Walk::shadow_env`]），检查序列**逐条镜像** `kernel_phase` 的主路径
    /// （主声明 `try_check_declar`＝`ByName` 形式、归纳块逐成员检查），
    /// 失败的不进环境、记进 [`Walk::shadow_failed`]。
    ///
    /// 为什么不直接用 `builder`：`builder` 最终要被 `kernel_phase` 的
    /// `finish()` **消费**，而且 walk 阶段**不往里 add** 文件声明 ✗
    /// （它只装 prelude + intern 名字）⇒ judge 拿它查不到前缀 ✓。
    pub(super) shadow: EnvBuilder<'arena>,
    /// 影子环境已重放到 `ops` 的哪个下标。
    pub(super) shadow_upto: usize,
    /// 影子重放中**内核拒绝**的那些 `ops` 下标（与 `kernel_phase` 的失败表同键：
    /// 都按"命令序"索引 ✓）。
    pub(super) shadow_failed: Vec<usize>,
    pub(super) known: KnownTable,
    pub(super) inductives: InductiveTable<'arena>,
    /// 源级 `def` 表（课程 Lean 化）：跨单元累加，`by` 引擎做一层 delta 展开用。
    pub(super) defs: crate::compile::elab::DefTable,
    pub(super) out: CompileOutput,
    pub(super) ops: Vec<PendingOp<'arena>>,
    pub(super) cmd_hovers: Vec<CmdHover<'arena>>,
    pub(super) decl_states: Vec<DeclState>,
    /// `example` 的内部名计数器（`_example_N`，按出现次序）。
    pub(super) example_idx: usize,
    /// G-05：命名空间栈 + `open` 集合。按源码顺序推进（`namespace`/`end`/`open`
    /// 三条命令的臂），单元切换处 [`NamespaceScope::reset`]——`open` 是**文件**
    /// 作用域，不跨 `import`（设计 N5）；`namespace` 由 parser 校验闭合，所以
    /// 单元边界上栈必然为空。
    pub(super) ns: NamespaceScope,
    /// **跨单元导出表**（第二刀 §N7）：`export Foo` 记在这里，单元切换时重放。
    /// 依赖按拓扑序排在入口之前，所以入口文件在文件头就能用依赖导出的短名。
    pub(super) exports: Vec<OpenEntry>,
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
    /// G-05：**本单元**用了 `namespace`/`open` ⇒ `by` 引擎的根目标先过一遍
    /// 内核 pp（`docs/design/namespace-open.md` §4.6）。没碰命名空间的文件
    /// 零额外开销、行为逐字不变。
    canonical_goal: bool,
    /// 本单元源码（`open … in …` 的合成前缀要按**源码文本**补一行 open，
    /// 见 [`Walk::open_in`]）。
    src: &'a str,
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
    /// **把影子环境推进到"当前已 elaborate 的前缀"**（T-K12b）。
    ///
    /// 惰性：只在第一次（以及每次有新 `ops` 之后）被调用时才重放新增的那几条
    /// ⇒ **不用就零成本** ✓。重放的检查序列**逐条镜像** `kernel_phase`：
    /// 主声明走 `try_check_declar`（`ByName` 形式，同 `kernel_phase.rs:251`），
    /// 归纳块逐成员检查（同 `kernel_phase.rs:315`）；**内核拒绝的不进环境**
    /// （check-then-add 语义 ✓），名字记进 `shadow_failed`。
    pub(super) fn shadow_env(&mut self) -> &mut EnvBuilder<'arena> {
        while self.shadow_upto < self.ops.len() {
            // 失败表按 **`cmd`（命令下标）** 记 —— 与 `kernel_phase` 的
            // `failed_cmds: KernelFailed` **同键**，这样两张表能逐条对照 ✓。
            match &self.ops[self.shadow_upto] {
                PendingOp::Decl { declar, cmd, .. } => {
                    let (declar, cmd) = (declar.clone(), *cmd);
                    self.shadow_check_and_add(&declar, cmd);
                }
                PendingOp::InductiveBlock { declars, cmd, .. } => {
                    let (declars, cmd) = (declars.clone(), *cmd);
                    for declar in declars {
                        self.shadow_check_and_add(&declar, cmd);
                    }
                }
                _ => {}
            }
            self.shadow_upto += 1;
        }
        &mut self.shadow
    }

    /// 影子环境的一条"检查后加入"（check-then-add，与 `kernel_phase` 同序同语义）。
    /// 检查走 `ExportFile`（`try_check_declar` 是它的方法）⇒ 借 `with_env` 一次；
    /// **内核拒绝的不进环境** ✓，只记下标。
    fn shadow_check_and_add(&mut self, declar: &Declar<'arena>, cmd: usize) {
        let declar = declar.clone();
        let ok = self
            .shadow
            .with_env(|env| env.try_check_declar(&declar).is_ok());
        if ok {
            let _ = self.shadow.add_declar(declar);
        } else {
            self.shadow_failed.push(cmd);
        }
    }

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
        // G-05：每个单元是否用了 namespace/open（`by` 引擎的根目标规范化开关，
        // 每单元算一次；没用到的文件零开销）。第二刀：`open … in` 与 `export`
        // 同样会改引用解析，所以一并计入。
        let unit_uses_namespaces: Vec<bool> = units
            .iter()
            .map(|unit| {
                unit.file.commands.iter().any(|command| {
                    matches!(
                        command,
                        Command::Namespace { .. }
                            | Command::End { .. }
                            | Command::Open { .. }
                            | Command::OpenIn { .. }
                            | Command::Export { .. }
                    )
                })
            })
            .collect();
        for (idx, &(unit_idx, command)) in flat.iter().enumerate() {
            let unit = &units[unit_idx];
            // G-05 N5：单元（文件）切换处清空作用域——`open` 与 `namespace`
            // 都是文件内的（`import` 不做模块限定，但被导入模块的**全局名**
            // 本来就可见，所以入口里的 `open Set` 对依赖的 `Set.mem` 仍然有效）。
            // 第二刀 §N7：`export` 是**唯一**跨 `import` 的那一半——清空之后
            // 重放导出表（依赖按拓扑序排在入口之前，此时它的导出已经齐了）。
            if idx == 0 || flat[idx - 1].0 != unit_idx {
                self.ns.reset();
                let exports = self.exports.clone();
                for entry in exports {
                    self.ns.open_entry(entry);
                }
            }
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
                canonical_goal: unit_uses_namespaces[unit_idx],
                src: &unit.file.src,
            };
            self.command(&c, command);
        }
    }

    /// 一条命令的分发（原 `run` 主循环里的 `match`，逐字搬过来）。
    ///
    /// 抽成方法的**唯一**原因：`open Foo in <命令>` 要把被包住的命令按同一个
    /// 上下文再走一遍（见 [`Walk::open_in`]）；arm 里的 `return` 语义不变
    /// （每个 arm 都没有内层循环，返回后 `run` 继续下一条命令）。
    fn command(&mut self, c: &CmdCtx<'_>, command: &Command) {
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
            } => self.def(c, name, universe, ty, val, *span),
            Command::Theorem {
                name,
                universe,
                ty,
                val,
                span,
            } => self.theorem(c, name, universe, ty, val, *span),
            Command::Axiom {
                name,
                universe,
                ty,
                span,
            } => self.axiom(c, name, universe, ty, *span),
            Command::Example { ty, val, span } => self.example(c, ty, val, *span),
            Command::InductiveBlock {
                name,
                params,
                ty,
                constructors,
                recursor,
                iota_rules,
                span,
            } => self.inductive_block(
                c,
                name,
                params,
                ty,
                constructors,
                recursor,
                iota_rules,
                *span,
            ),
            Command::Check { expr, span: _ } => self.check(c, expr),
            Command::Reduce { expr, span: _ } => self.reduce(c, expr),
            Command::Print { name, span } => self.print(c, name, *span),
            // 记法命令**不是声明**（设计 N6）：不 elaborate、不产
            // PendingOp、不进声明表——与 `Command::Import` 同族。
            Command::Notation { .. } => {}
            // G-05：三条作用域命令同样不是声明。声明名加前缀在 parser 里
            // 已经落定（N3），这里只维护**引用解析**用的作用域（N4）：
            // `namespace` 压栈、`end` 弹栈、`open` 进可省略前缀集合。
            // trusted 前缀也要走（否则后半段的解析会丢作用域）。
            Command::Namespace { name, .. } => self.ns.push(name),
            Command::End { .. } => self.ns.pop(),
            // `open scoped <名字>`（第三刀 §12.3）**只**打开记法作用域
            // （副作用在 parser 里已经落定），**不**打开名字前缀——与 Lean
            // 一致（`open scoped Foo` 不会让 `Foo.bar` 能写成 `bar`）。
            Command::Open {
                name,
                scoped: false,
                filter,
                ..
            } => self.ns.open_entry(OpenEntry::new(name, filter.clone())),
            Command::Open { scoped: true, .. } => {}
            // `open Foo in <命令>`（第二刀 §N7）：局部 open。
            Command::OpenIn {
                name,
                filter,
                inner,
                header,
                ..
            } => self.open_in(c, name, filter, inner, *header),
            // `export Foo`（第二刀 §N7）：本文件内与 `open` 逐字相同，额外
            // 记进导出表（跨 `import` 生效）。
            Command::Export { name, filter, .. } => {
                let entry = OpenEntry::new(name, filter.clone());
                self.ns.open_entry(entry.clone());
                if !self.exports.contains(&entry) {
                    self.exports.push(entry);
                }
            }
        }
    }

    /// `open <name> [<子句>] in <命令>`（第二刀 §N7）：把 open 压进作用域、
    /// 走一遍被包住的命令、再撤销。
    ///
    /// 被包住的命令用**同一个 `CmdCtx`**，只把合成前缀补一行 open 的源码文本
    /// （`open Foo hiding a`）——`by` 引擎的根目标规范化（`judge_render_type`）
    /// 与 `judge_terms` 都是"前缀源码 + 合成命令"再走一遍流水线，前缀里没有这
    /// 一行，短名在那里就解析不了（退回源 AST 是安全的，但 `apply` 的文本对齐
    /// 会失准）。补的是**源码原文**，不是重建的文本，所以子句逐字保真。
    fn open_in(
        &mut self,
        c: &CmdCtx<'_>,
        name: &str,
        filter: &OpenFilter,
        inner: &Command,
        header: Span,
    ) {
        let mark = self.ns.opens_mark();
        self.ns.open_entry(OpenEntry::new(name, filter.clone()));
        let header_text = c
            .src
            .get(header.start.offset..header.end.offset)
            .unwrap_or("");
        let inner_ctx = CmdCtx {
            idx: c.idx,
            templates: c.templates,
            prefix_src: Cow::Owned(format!("{}{header_text}\n", c.prefix_src)),
            trusted: c.trusted,
            env_before: c.env_before,
            options: c.options,
            skip: c.skip,
            canonical_goal: c.canonical_goal,
            src: c.src,
        };
        self.command(&inner_ctx, inner);
        self.ns.rollback_opens(mark);
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
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
        };
        let lowered = match lower_value(
            ty,
            val,
            universe,
            prefix_src,
            options,
            c.canonical_goal,
            &self.inductives,
            &self.defs,
        ) {
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
        let by_steps = by_step_states(&lowered.1, &self.display);
        // 源级 delta 表：**值完整**的 def 才登记（开练习的值是洞，展开没意义）。
        // `by` 引擎的 `intro`/`apply` 靠它看穿 `A ⊆ B` 这类 def 头。
        if open_goal(ty, val, templates, &mut Vec::new()).is_none() {
            let info = DefInfo {
                params: params_of_ty(ty),
                universes: universe.to_vec(),
                body: strip_lambdas_n(val, params_of_ty(ty).len()),
            };
            self.defs.insert(name.to_string(), info.clone());
            // **短名别名**（R2 实测）：`namespace Set` 里的 def 体是用**短名**
            // 写的（`def powerset … := fun B => subset α B A`），而 delta 展开是
            // 逐层的——第二层拿到的头是短名 `subset`，`defs` 里却只有规范名
            // `Set.subset` ⇒ 展开在第二层断掉（实测：`A ∈ 𝒫 B` 上 `intro` 报
            // 「需要一个函数目标」，而目标明明是集合成员关系）。
            //
            // 只登记**不冲突**的短名（先到先得）：同名短名在两个命名空间里都有
            // 时保持今天的行为（查不到 ⇒ 不展开 ⇒ 响亮报错），绝不猜。
            if let Some(short) = name.rsplit('.').next() {
                if short != name && !short.is_empty() {
                    self.defs.entry(short.to_string()).or_insert(info);
                }
            }
        }
        if trusted {
            // Trusted prefix: keep the environment, skip the kernel.
            // Cached failures keep the name free (check-then-add);
            // open exercises never enter the environment anyway.
            if skip.is_some_and(|s| s.contains_key(&idx))
                || open_goal(ty, val, templates, &mut Vec::new()).is_some()
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
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                let _ = self.builder.add_declar(decl);
                self.known.insert(
                    name.to_string(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::render_expr(ty)),
                    },
                );
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
        let mut redundant_spans: Vec<Span> = Vec::new();
        let open_info = open_goal(ty, val, templates, &mut redundant_spans);
        if let Some(info) = open_info {
            // G-01：签名必须先过 elaborate；`Err` 与值位 elaborate 失败同罪。
            let signature =
                match open_signature(&mut self.builder, universe, ty, &self.known, &elab_ctx) {
                    Ok(sig) => sig,
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
            self.ops.push(PendingOp::OpenExercise {
                name: Some(name.to_string()),
                kind: DeclKind::Definition,
                universe: universe.to_vec(),
                redundant_probes: build_redundant_probes(
                    &mut self.builder,
                    universe,
                    ty,
                    val,
                    &redundant_spans,
                    &self.known,
                    &elab_ctx,
                ),
                env_before: c.env_before,
                declared_ty: Some(signature.declared_ty),
                sig_probe: signature.probe,
                sig_span: signature.span,
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
            &self.known,
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
                self.known.insert(
                    name_owned.clone(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::render_expr(ty)),
                    },
                );
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
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
        };
        let lowered = match lower_value(
            ty,
            val,
            universe,
            prefix_src,
            options,
            c.canonical_goal,
            &self.inductives,
            &self.defs,
        ) {
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
        let by_steps = by_step_states(&lowered.1, &self.display);
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx))
                || open_goal(ty, val, templates, &mut Vec::new()).is_some()
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
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                let _ = self.builder.add_declar(decl);
                self.known.insert(
                    name.to_string(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::render_expr(ty)),
                    },
                );
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
        let mut redundant_spans: Vec<Span> = Vec::new();
        let open_info = open_goal(ty, val, templates, &mut redundant_spans);
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
            // G-01：签名必须先过 elaborate；`Err` 与值位 elaborate 失败同罪。
            let signature =
                match open_signature(&mut self.builder, universe, ty, &self.known, &elab_ctx) {
                    Ok(sig) => sig,
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
            self.ops.push(PendingOp::OpenExercise {
                name: Some(name.to_string()),
                kind: DeclKind::Theorem,
                universe: universe.to_vec(),
                redundant_probes: build_redundant_probes(
                    &mut self.builder,
                    universe,
                    ty,
                    val,
                    &redundant_spans,
                    &self.known,
                    &elab_ctx,
                ),
                env_before: c.env_before,
                declared_ty: Some(signature.declared_ty),
                sig_probe: signature.probe,
                sig_span: signature.span,
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
            &self.known,
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
                self.known.insert(
                    name_owned.clone(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::render_expr(ty)),
                    },
                );
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
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
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
                &self.known,
                &mut hovers,
                &elab_ctx,
            ) {
                let _ = self.builder.add_declar(decl);
                self.known.insert(
                    name.to_string(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::render_expr(ty)),
                    },
                );
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
            &self.known,
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
                self.known.insert(
                    name_owned.clone(),
                    KnownName::Decl {
                        universes: universe.to_vec(),
                        implicit_prefix: crate::compile::elab::leading_implicit_prefix(ty),
                        explicit_arity: crate::compile::elab::explicit_arity(ty),
                        signature: Some(crate::proof::render_expr(ty)),
                    },
                );
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
        let templates = c.templates;
        // 注意用 `&c.prefix_src`（Deref，借 `c` 的寿命）而不是 `Cow::as_ref()`
        // （后者返回的是 `Cow` 自身的寿命参数，会把 `ElabCtx` 逼到 `'arena`）。
        let prefix_src: &str = &c.prefix_src;
        let trusted = c.trusted;
        let options = local(c.options);
        let skip = c.skip;
        // `elab_ctx` 里的 `defs` 要**借到本次 `elab_expr` 结束**，而稍后的
        // `self.defs.insert` 需要可变借用 ⇒ 借一份快照（本次声明自己的 def 还没
        // 登记，快照正合适：def 不递归）。课程规模下克隆成本可忽略。
        let defs_for_ctx = self.defs.clone();
        let elab_ctx = ElabCtx {
            prefix_src,
            options,
            inductives: &self.inductives,
            ns: &self.ns,
            defs: &defs_for_ctx,
        };
        let lowered = match lower_value(
            ty,
            val,
            // `example` 不能声明宇宙参数 ⇒ 空切片（判定合成声明不需要带宇宙 binder）。
            &[],
            prefix_src,
            options,
            c.canonical_goal,
            &self.inductives,
            &self.defs,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.out.push_error(idx, e.clone());
                self.decl_states
                    .push(failed_state(DeclKind::Example, None, span, e, idx));
                return;
            }
        };
        let val = &lowered.0;
        let by_steps = by_step_states(&lowered.1, &self.display);
        if trusted {
            if skip.is_some_and(|s| s.contains_key(&idx))
                || open_goal(ty, val, templates, &mut Vec::new()).is_some()
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
                &self.known,
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
        let mut redundant_spans: Vec<Span> = Vec::new();
        let open_info = open_goal(ty, val, templates, &mut redundant_spans);
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
            // G-01：`example` 没有宇宙参数，签名同样必须先过 elaborate。
            let signature = match open_signature(&mut self.builder, &[], ty, &self.known, &elab_ctx)
            {
                Ok(sig) => sig,
                Err(e) => {
                    self.out.push_error(idx, e.clone());
                    self.decl_states
                        .push(failed_state(DeclKind::Example, None, span, e, idx));
                    return;
                }
            };
            self.ops.push(PendingOp::OpenExercise {
                name: None,
                kind: DeclKind::Example,
                universe: Vec::new(),
                redundant_probes: build_redundant_probes(
                    &mut self.builder,
                    &[],
                    ty,
                    val,
                    &redundant_spans,
                    &self.known,
                    &elab_ctx,
                ),
                env_before: c.env_before,
                declared_ty: Some(signature.declared_ty),
                sig_probe: signature.probe,
                sig_span: signature.span,
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
            &self.known,
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
            // 失败已经记在会话缓存里（`skip`），这里不重复报错；env 自己持有
            // 声明副本，返回的 `Declar` 只是给内核阶段用的句柄，丢弃即可。
            let mut hovers = Vec::new();
            let mut built: Vec<Declar<'_>> = Vec::new();
            let _ = install_inductive_block(
                &mut self.builder,
                &mut self.known,
                &mut self.inductives,
                prefix_src,
                options,
                &self.ns,
                name,
                params,
                ty,
                constructors,
                recursor.as_ref(),
                iota_rules,
                &mut hovers,
                &mut built,
            );
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
            &mut self.known,
            &mut self.inductives,
            prefix_src,
            options,
            &self.ns,
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
            &self.known,
            &mut hovers,
            None,
            None,
            &ElabCtx {
                prefix_src,
                options,
                inductives: &self.inductives,
                ns: &self.ns,
                defs: &self.defs,
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
            &self.known,
            &mut hovers,
            None,
            None,
            &ElabCtx {
                prefix_src,
                options,
                inductives: &self.inductives,
                ns: &self.ns,
                defs: &self.defs,
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
        // R2：`#print mk` 与 `#print Wrap.mk` 打印同一条声明（别名解析到规范名）；
        // 歧义/未知走各自稳定的错误码，与 `#check` 同源。G-05：命名空间/open
        // 的候选顺序也走这一条（`#print mem` 在 `namespace Set` 里解析到 `Set.mem`）。
        let canonical = match resolve_known(&self.known, &self.ns, name, span) {
            Ok(canonical) => canonical,
            Err(e) => {
                self.out.push_error(idx, e);
                return;
            }
        };
        let ptr = self.builder.name_from_str(&canonical);
        self.ops.push(PendingOp::Print {
            name: canonical,
            ptr,
            span,
            cmd: idx,
        });
    }
}

/// 开练习的签名检查产物（G-01 / WO-004）。
///
/// 值位是 `sorry` 不再让签名免检：签名必须先 elaborate 成内核类型
/// （`elab_expr` 的 `Err` 由调用方走既有失败通道上报），再由内核阶段用
/// [`Self::probe`] 终审「它是不是一个类型 / 是不是 Prop」。
pub(super) struct OpenSignature<'arena> {
    /// 签名 elaborate 后的内核表达式（只用来渲染 `DeclState.ty_text`）。
    pub(super) declared_ty: ExprPtr<'arena>,
    /// 「签名是不是一个类型」的探针：一条**不入环境**的同签名 axiom。
    /// 内核的 `check_declar_info_v` 先 `ensure_sort_v`，消息族与 checked
    /// 路径同源（`Declar::Axiom` 不需要值，正适合签名这种"没有值"的声明）。
    pub(super) probe: Box<Declar<'arena>>,
    /// 诊断 span：**签名**的 AST 范围（G-01 要求报在签名上；G-15 已修，内核 span 本身精确）。
    pub(super) span: Span,
}

/// 开练习的签名检查（G-01 / WO-004）：把签名 elaborate 成内核类型并造终审探针。
///
/// 与 checked 路径用**同一套** elaborate 上下文：宇宙参数在作用域里
/// （原来这里传的是空宇宙表 `UnivMap::new()`，`{u}` 签名的 `ty_text`
/// 因此渲染不出来——签名检查顺带把它对齐）。`Err` = 签名 elaborate 不过，
/// 调用方必须把它当失败上报，**不要**登记开放练习。
fn open_signature<'arena>(
    builder: &mut EnvBuilder<'arena>,
    universe: &[String],
    ty: &Expr,
    known: &KnownTable,
    ctx: &ElabCtx<'arena, '_>,
) -> Result<OpenSignature<'arena>, CompileError> {
    let univ = make_univ_map(builder, universe);
    let mut hovers: Vec<HoverNode<'arena>> = Vec::new();
    let declared_ty = elab_expr(
        builder,
        ty,
        &mut ElabScope::new(),
        &univ,
        known,
        &mut hovers,
        None,
        None,
        ctx,
    )?;
    // 探针重新 elaborate 一次签名：`build_axiom` 是现成的**无值**声明构造器，
    // 复用它的宇宙参数登记（`collect_uparams`），免得在这里重造一遍。
    let probe = build_axiom(
        builder,
        SIG_PROBE_NAME,
        universe,
        ty,
        known,
        &mut Vec::new(),
        ctx,
    )?;
    Ok(OpenSignature {
        declared_ty,
        probe: Box::new(probe),
        span: ty.span(),
    })
}

/// 签名探针的内部名（不入环境，不会与用户名字冲突；对照
/// `_soko_redundant_sorry_N`）。
const SIG_PROBE_NAME: &str = "_soko_signature_probe";

/// 「多余的 `sorry`」的 kernel 探针（`docs/design/redundant-sorry.md` §4）：
/// 对每个候选洞，把那个实参从应用 spine 上删掉、按原声明的类型合成一条
/// **不会进入环境**的声明；pass 2 用 `try_check_declar_at` 终审——过了才说明
/// "删掉这行 sorry 就通过"，也就是"它不是你要证的东西"。
/// 造不出来（elab 失败/形状不认识）就跳过：绝不猜。
///
/// **注意（§8.1）**：探针不入环境 ⇒ 它的名字没有 `decl_idx` ⇒
/// `try_check_declar` 内部的 `EnvLimit::ByName(探针名)` 取 0（空环境），
/// 终审必然 `unknown const`。修法见 §8.3（终审显式传 `EnvLimit::ByIndex(env_before)`）。
#[allow(clippy::too_many_arguments)]
fn build_redundant_probes<'arena>(
    builder: &mut EnvBuilder<'arena>,
    universe: &[String],
    ty: &Expr,
    val: &Expr,
    spans: &[Span],
    known: &KnownTable,
    ctx: &ElabCtx<'arena, '_>,
) -> Vec<(Declar<'arena>, Span)> {
    let mut probes = Vec::new();
    for (i, span) in spans.iter().enumerate() {
        let Some(modified) = spine_without_arg(val, *span) else {
            continue;
        };
        let mut hovers: Vec<HoverNode<'arena>> = Vec::new();
        let name = format!("_soko_redundant_sorry_{i}");
        if let Ok(declar) = build_def(
            builder,
            &name,
            universe,
            ty,
            &modified,
            known,
            &mut hovers,
            ctx,
        ) {
            probes.push((declar, *span));
        }
    }
    probes
}
