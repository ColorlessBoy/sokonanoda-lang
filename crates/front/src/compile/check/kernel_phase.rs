//! 待执行操作（`PendingOp`）的**内核阶段**：环境和 `EnvBuilder` 定稿后，按命令
//! 序把每个操作送进内核（check-then-add）、产生事件与错误、给增量会话算签名与
//! early cutoff，最后组装 `DocumentReport`。
//!
//! 从 `run_pass` 尾部整体切出（I16 batch 3 收尾的模块化），行为逐字节不变：
//! 这里的代码就是把原来那个 1000+ 行函数的后半段原样搬过来，只把外层局部变量
//! 变成 `Walked` 的字段。

use super::{
    declar_signature, failed_state, inductive_signature, op_cmd, quiet_catch, resolve_hovers,
    top_level_def_spans_over, CmdHover, KernelFailed, PassResult, PendingOp, TrustPlan,
};
use crate::compile::error::{parse_def_eq_mismatch, refine_kernel_kind, CompileError, ErrorKind};
use crate::compile::event::{CheckEvent, CompileOutput};
use crate::compile::report::{DeclKind, DeclState, DeclStatus, DocumentReport, ResolvedTarget};
use crate::compile::units::SourceUnit;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::EnvLimit;

/// 命令走完后交给内核阶段的一切（原 `run_pass` 尾部读到的全部局部变量）。
pub(super) struct Walked<'a, 'arena> {
    pub(super) units: &'a [SourceUnit<'a>],
    /// 每个扁平命令所属的单元下标（报告排序用）。
    pub(super) unit_of_cmd: &'a [usize],
    /// 扁平命令总数（签名表的长度）。
    pub(super) n_commands: usize,
    pub(super) collect: bool,
    pub(super) trust: Option<&'a TrustPlan>,
    pub(super) builder: EnvBuilder<'arena>,
    pub(super) out: CompileOutput,
    pub(super) report: DocumentReport,
    pub(super) ops: Vec<PendingOp<'arena>>,
    pub(super) cmd_hovers: Vec<CmdHover<'arena>>,
    pub(super) decl_states: Vec<DeclState>,
    pub(super) failed_cmds: KernelFailed,
    pub(super) kernel_checks: usize,
}

pub(super) fn finish_pass(walked: Walked<'_, '_>) -> PassResult {
    let Walked {
        units,
        unit_of_cmd,
        n_commands: n,
        collect,
        trust,
        builder,
        mut out,
        mut report,
        ops,
        mut cmd_hovers,
        mut decl_states,
        mut failed_cmds,
        mut kernel_checks,
    } = walked;
    let mut env = builder.finish();
    // Print proof terms as terms instead of suppressing them to `_`; the
    // suppression path would try to infer types of open binder bodies.
    env.config.pp_options.proofs = true;

    let want_sigs = trust.is_some();
    let before = trust.map_or(0, |t| t.before);
    let mut allow_cutoff = trust.is_some_and(|t| t.allow_cutoff);
    let old_sigs: &[Option<String>] = trust.map_or(&[][..], |t| t.prev_signatures.as_slice());
    let text_unchanged: &[bool] = trust.map_or(&[][..], |t| t.text_unchanged.as_slice());
    let mut sigs: Vec<Option<String>> = vec![None; n];
    let mut acc_new: Vec<Option<String>> = Vec::new();
    let mut acc_old: Vec<Option<String>> = Vec::new();
    let mut cutoff = n;

    let mut ops = ops.into_iter().peekable();
    for (j, sig_slot) in sigs.iter_mut().enumerate() {
        // Early cutoff (I8 依赖精确化): the environment contribution of
        // `[before, j)` matches the previous session AND command `j`'s text is
        // unchanged, so `[j, n)` may be reused from the session cache without
        // re-checking. `j > before` because the changed command itself must
        // always be recompiled (its own spans/hovers may have moved).
        if allow_cutoff
            && j > before
            && text_unchanged.get(j).copied().unwrap_or(false)
            && acc_new == acc_old
        {
            cutoff = j;
            break;
        }
        let op = match ops.peek() {
            Some(o) if op_cmd(o) == j => ops.next(),
            _ => None,
        };
        let mut contribution = if want_sigs {
            op.as_ref().and_then(|op| match op {
                PendingOp::Decl { declar, .. } => Some(declar_signature(&mut env, declar)),
                PendingOp::InductiveBlock { declars, .. } => {
                    Some(inductive_signature(&mut env, declars))
                }
                _ => None,
            })
        } else {
            None
        };
        // A kernel-rejected declaration never enters the environment
        // (check-then-add), so its environment contribution is empty.
        let mut op_failed = false;
        if let Some(op) = op {
            match op {
                PendingOp::OpenExercise {
                    name,
                    kind,
                    universe,
                    declared_ty,
                    goal,
                    binders,
                    holes,
                    sub_goals,
                    refine_template,
                    by_steps,
                    span,
                    cmd,
                } => {
                    out.push_event(cmd, CheckEvent::ExerciseOpen { name: name.clone() });
                    let ty_text = declared_ty.and_then(|ty| {
                        quiet_catch(|| {
                            env.with_tc(EnvLimit::Empty, |tc| tc.with_pp(|pp| pp.pp_expr(ty)))
                        })
                        .ok()
                    });
                    decl_states.push(DeclState {
                        kind,
                        name,
                        span,
                        status: DeclStatus::Open,
                        error: None,
                        goal,
                        binders,
                        cmd,
                        universe,
                        holes,
                        sub_goals,
                        refine_template,
                        by_steps,
                        hints: Vec::new(),
                        ty_text,
                    });
                }
                PendingOp::Decl {
                    name,
                    kind,
                    declar,
                    by_steps,
                    span,
                    cmd,
                } => {
                    kernel_checks += 1;
                    let ty_text = quiet_catch(|| {
                        env.with_tc(EnvLimit::Empty, |tc| {
                            let ty = declar.info().ty;
                            tc.with_pp(|pp| pp.pp_expr(ty))
                        })
                    });
                    let ty_text = ty_text.ok();
                    match env.try_check_declar(&declar) {
                        Ok(()) => {
                            match kind {
                                DeclKind::Example => {
                                    out.push_event(cmd, CheckEvent::ExampleChecked)
                                }
                                _ => {
                                    if let Some(n) = &name {
                                        out.push_event(
                                            cmd,
                                            CheckEvent::DeclarationChecked { name: n.clone() },
                                        );
                                    } else {
                                        out.push_event(cmd, CheckEvent::ExampleChecked);
                                    }
                                }
                            }
                            decl_states.push(DeclState {
                                kind,
                                name,
                                span,
                                status: DeclStatus::Checked,
                                error: None,
                                goal: None,
                                binders: Vec::new(),
                                cmd,
                                universe: Vec::new(),
                                holes: Vec::new(),
                                sub_goals: Vec::new(),
                                refine_template: None,
                                by_steps,
                                hints: Vec::new(),
                                ty_text,
                            });
                        }
                        Err(e) => {
                            let msg = format!("{e}");
                            let mut err = CompileError::kernel(refine_kernel_kind(&msg), msg, span);
                            if let Some((expected, actual)) = parse_def_eq_mismatch(&err.message) {
                                err.message =
                                    format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`");
                                err.expected = Some(expected);
                                err.actual = Some(actual);
                            }
                            // Check-then-add: a rejected declaration makes pass 1's
                            // environment provisional — stop trusting any cutoff.
                            allow_cutoff = false;
                            op_failed = true;
                            failed_cmds.insert(cmd, err.clone());
                            out.push_error(j, err.clone());
                            decl_states.push(failed_state(kind, name, span, err, cmd));
                        }
                    }
                }
                PendingOp::InductiveBlock {
                    name,
                    declars,
                    span,
                    cmd,
                } => {
                    let mut failure = None;
                    for declar in &declars {
                        kernel_checks += 1;
                        if let Err(e) = env.try_check_declar(declar) {
                            let msg = format!("{e}");
                            let mut err = CompileError::kernel(refine_kernel_kind(&msg), msg, span);
                            if let Some((expected, actual)) = parse_def_eq_mismatch(&err.message) {
                                err.message =
                                    format!("类型不匹配：期望 `{expected}`，实际是 `{actual}`");
                                err.expected = Some(expected);
                                err.actual = Some(actual);
                            }
                            failure = Some(err);
                            break;
                        }
                    }
                    match failure {
                        None => {
                            out.push_event(
                                cmd,
                                CheckEvent::DeclarationChecked { name: name.clone() },
                            );
                            decl_states.push(DeclState {
                                kind: DeclKind::Inductive,
                                name: Some(name),
                                span,
                                status: DeclStatus::Checked,
                                error: None,
                                goal: None,
                                binders: Vec::new(),
                                cmd,
                                universe: Vec::new(),
                                holes: Vec::new(),
                                sub_goals: Vec::new(),
                                refine_template: None,
                                by_steps: Vec::new(),
                                hints: Vec::new(),
                                ty_text: None,
                            });
                        }
                        Some(err) => {
                            allow_cutoff = false;
                            op_failed = true;
                            failed_cmds.insert(cmd, err.clone());
                            out.push_error(j, err.clone());
                            decl_states.push(failed_state(
                                DeclKind::Inductive,
                                Some(name),
                                span,
                                err,
                                cmd,
                            ));
                        }
                    }
                }
                PendingOp::Check {
                    expr,
                    env_at,
                    span,
                    cmd,
                } => {
                    // #check/#reduce 直通内核求值路径：panic（如对非函数应用）
                    // 必须降级为诊断，绝不能崩掉编译/LSP 进程。
                    match quiet_catch(|| {
                        env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                            let ty = tc.infer_closed_type(expr);
                            tc.with_pp(|pp| pp.pp_expr(ty))
                        })
                    }) {
                        Ok(text) => out.push_event(cmd, CheckEvent::TypeChecked { text, span }),
                        Err(msg) => out.push_error(
                            j,
                            CompileError::kernel(
                                refine_kernel_kind(&msg),
                                format!("类型检查失败：{msg}"),
                                span,
                            ),
                        ),
                    }
                }
                PendingOp::Reduce {
                    expr,
                    env_at,
                    span,
                    cmd,
                } => {
                    match quiet_catch(|| {
                        env.with_tc(EnvLimit::ByIndex(env_at), |tc| {
                            let reduced = tc.reduce_closed(expr);
                            tc.with_pp(|pp| pp.pp_expr(reduced))
                        })
                    }) {
                        Ok(text) => out.push_event(cmd, CheckEvent::Reduced { text, span }),
                        Err(msg) => out.push_error(
                            j,
                            CompileError::kernel(
                                refine_kernel_kind(&msg),
                                format!("化简失败：{msg}"),
                                span,
                            ),
                        ),
                    }
                }
                PendingOp::Print {
                    name,
                    ptr,
                    span,
                    cmd,
                } => {
                    let printed = env.with_pp(|pp| pp.pp_declar(ptr));
                    match printed {
                        Some(text) => out.push_event(cmd, CheckEvent::Printed { name, text }),
                        None => out.push_error(
                            j,
                            CompileError::elab(
                                ErrorKind::ElabUnknownIdentifier,
                                format!("unknown declaration `{name}`"),
                                span,
                            ),
                        ),
                    }
                }
            }
        }
        if op_failed {
            contribution = None;
        }
        *sig_slot = contribution.clone();
        if j >= before {
            acc_new.push(contribution);
            acc_old.push(old_sigs.get(j).cloned().flatten());
        }
    }

    if collect {
        // Open/failed states are recorded during the command walk while
        // checked states come from the kernel phase; keep source order.
        let mut states = decl_states;
        // 多个单元时先按单元、再按文件内 offset 排序：不同文件的 offset 不在
        // 同一个坐标空间里，混排会把入口的声明插到依赖的声明之间。
        states.sort_by_key(|d| (unit_of_cmd[d.cmd], d.span.start.offset));
        report.decls = states;
        // Name use → definition: top-level targets were recorded with a
        // placeholder span during elaboration; backfill them from the file's
        // name → def-span map (prelude names resolve to nothing).
        let defs = top_level_def_spans_over(units);
        for cmd in &mut cmd_hovers {
            for node in &mut cmd.nodes {
                if let Some(ResolvedTarget::Declaration { name, .. }) = &node.resolution {
                    let name = name.clone();
                    node.resolution = defs
                        .get(&name)
                        .map(|&span| ResolvedTarget::Declaration { name, span });
                }
            }
        }
        let mut hover_cmds = Vec::new();
        resolve_hovers(&env, cmd_hovers, &mut report.hovers, &mut hover_cmds);
        report.hover_cmds = hover_cmds;
        report.checks = out
            .events
            .iter()
            .zip(out.event_cmds.iter())
            .filter_map(|(event, cmd)| match event {
                CheckEvent::TypeChecked { text, span } => Some(crate::compile::report::CheckInfo {
                    span: *span,
                    text: text.clone(),
                    cmd: *cmd,
                }),
                _ => None,
            })
            .collect();
    }
    // 报告里的错误**始终**与 `out.errors` 平行（即使 `collect == false`，
    // 报告会被丢弃）：`split_report` 依赖 `errors[i] ↔ error_cmds[i]` 的严格
    // 平行关系做跨文件归因。
    report.errors = out.errors.clone();
    // Syntax-level warnings are independent of the kernel pass: compute them
    // once for the whole file so every return path (batch output + report)
    // carries the same list.
    report.warnings = units
        .iter()
        .flat_map(|unit| crate::compile::warning::collect_warnings(unit.file))
        .collect();
    out.warnings = report.warnings.clone();
    PassResult {
        out,
        report,
        failed: failed_cmds,
        checks: kernel_checks,
        sigs,
        cutoff,
    }
}
