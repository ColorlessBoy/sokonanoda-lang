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
use crate::compile::units::{unit_ranges, SourceUnit};
use crate::Span;
use sokonanoda::builder::EnvBuilder;
use sokonanoda::env::{Declar, EnvLimit};
use sokonanoda::util::{ExportFile, ExprPtr};

/// 命令走完后交给内核阶段的一切（原 `run_pass` 尾部读到的全部局部变量）。
pub(super) struct Walked<'a, 'arena> {
    /// 显示期的记法表（`run_pass` 里建一次，`walk` 与这里共用）。
    pub(super) display: crate::display::DisplayNotations,
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

/// **检查→加入**一条声明（T-D1：把这段逻辑收进单一函数）。
///
/// 从 `finish_pass` 的 `PendingOp::Decl` 分支**逐字搬过来**（纯重构、零行为变化 ✓）——
/// 目的不是"更漂亮"，而是让 T-D3 的 walk 能在 elaborate 之后**当场**做同一件事，
/// 从而消掉"走一遍再查一遍"的重复（阶段 D 的两刀都建立在"只有一个 check-then-add"上）。
///
/// 返回值 = **这条被内核拒绝了吗**。被拒绝会让第一阶段的环境变成"临时"的
/// （后续命令不再可信）⇒ 调用方必须据此关掉早期截断 ✓ —— 所以它**不能**留在函数里
/// 当副作用，必须是返回值 ✓（原代码里的 `allow_cutoff = false; op_failed = true;`）。
#[allow(clippy::too_many_arguments)]
fn check_then_add_decl<'arena>(
    env: &mut ExportFile<'arena>,
    display: &crate::display::DisplayNotations,
    out: &mut CompileOutput,
    decl_states: &mut Vec<DeclState>,
    failed_cmds: &mut KernelFailed,
    kernel_checks: &mut usize,
    j: usize,
    op: PendingOp<'arena>,
) -> bool {
    let PendingOp::Decl {
        name,
        kind,
        declar,
        by_steps,
        span,
        cmd,
    } = op
    else {
        unreachable!("check_then_add_decl 只接 PendingOp::Decl");
    };
    *kernel_checks += 1;
    let ty_res = quiet_catch(|| {
        env.with_tc(EnvLimit::Empty, |tc| {
            let ty = declar.info().ty;
            tc.with_pp(|pp| pp.pp_expr(ty))
        })
    });
    // **诊断**（`SOKO_TRACE_NOTATIONS=1`，默认零输出）：③ 那条报告（`∃` 不折）量到
    // `ty_text` 全是 `None` ✗ ⇒ 这里的 pp 失败了；打出原因才知道该修哪里，不许猜 ✗。
    if std::env::var_os("SOKO_TRACE_NOTATIONS").is_some() {
        let who = name.clone().unwrap_or_else(|| "<anon>".to_string());
        match &ty_res {
            Ok(raw) => {
                // **③ 最后一层**：`print_back` 是不是**解析失败就原样返回** ✗ ——
                // 拿同一段文本单独 parse 一次就知道 ✓（这段文本是**内核 pp 的产物**，
                // 形状未必等于源级写法 ✓）。
                eprintln!(
                    "[trace-notations] {who} parse_ok={} pp raw: {}",
                    crate::parse(raw).is_ok(),
                    raw.chars().take(96).collect::<String>().replace('\n', " ")
                );
                // **逐字节**（转义）—— 与单测夹具的字符串做字符级比对用 ✓
                if who == "exists_univ" || who == "subset_univ" {
                    eprintln!("[trace-notations] {who} RAWDBG: {raw:?}");
                }
            }
            Err(e) => eprintln!("[trace-notations] {who} ty pp FAILED: {e:?}"),
        }
    }
    let ty_text = ty_res.ok().map(|text| {
        // **走唯一接口**（T-U11，2026-09-25 ✓）：`fold` 就是 `print_back(text, self)` ✓
        // ⇒ **零行为变化** ✓（判据：T-U12 的面级判据 + front 全量 + 全语料对拍 ✓）。
        display.fold(&text)
    });
    if std::env::var_os("SOKO_TRACE_NOTATIONS").is_some() {
        let who = name.clone().unwrap_or_else(|| "<anon>".to_string());
        eprintln!("[trace-notations] {who} folded: {ty_text:?}");
    }
    // **声明的值**（T-D52）：与 `ty_text` 同一形状算一遍
    // （内核 pp + 线 C 折叠）。只有 `def`/`opaque` 有值。
    let val_text = quiet_catch(|| {
        env.with_tc(EnvLimit::Empty, |tc| {
            let val = declar.value()?;
            Some(tc.with_pp(|pp| pp.pp_expr(val)))
        })
    })
    .ok()
    .flatten()
    .map(|text| {
        // **走唯一接口**（T-U11，2026-09-25 ✓）：`fold` 就是 `print_back(text, self)` ✓
        // ⇒ **零行为变化** ✓（判据：T-U12 的面级判据 + front 全量 + 全语料对拍 ✓）。
        display.fold(&text)
    });
    match env.try_check_declar(&declar) {
        Ok(()) => {
            match kind {
                DeclKind::Example => out.push_event(cmd, CheckEvent::ExampleChecked),
                _ => {
                    if let Some(n) = &name {
                        out.push_event(cmd, CheckEvent::DeclarationChecked { name: n.clone() });
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
                val_text,
            });
            false
        }
        Err(e) => {
            let msg = format!("{e}");
            let mut err = CompileError::kernel(refine_kernel_kind(&msg), msg, span);
            if let Some((expected, actual)) = parse_def_eq_mismatch(&err.message) {
                // **用户可见文本必须折记法**（T-U11 ✓ 2026-09-25 round 157 修 ✗⇒✓）：
                // 这条消息是**给学习者看的** ✓（"类型不匹配" ✓），却漏出**原始内核 pp** ✗
                // （实测：`Set.[]` / `Set.subset.[]` / `$1` / `Sort(0)` ✓，折叠开与关**完全一样** ✗
                // ⇒ 它此前**根本没走折叠** ✓ —— `docs/design/duplication-audit.md` 的 🔴 条 ✓）。
                // ⚠ **只折消息** ✓；`err.expected`/`err.actual` 是**结构化字段** ✓（可能喂机器比对 ✓）
                // ⇒ 保持内核原值不动 ✗。
                err.message = format!(
                    "类型不匹配：期望 `{}`，实际是 `{}`",
                    display.fold(&expected),
                    display.fold(&actual)
                );
                err.expected = Some(expected);
                err.actual = Some(actual);
            }
            failed_cmds.insert(cmd, err.clone());
            out.push_error(j, err.clone());
            decl_states.push(failed_state(kind, name, span, err, cmd));
            true
        }
    }
}

pub(super) fn finish_pass(walked: Walked<'_, '_>) -> PassResult {
    let Walked {
        display,
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
    // **显示期的记法表**（线 C / T-C20）：`ty_text` 是内核 pp 出来的**点名**形式
    // （`Set.mem α a A`），而用户看的是 goal 面板 / 声明卡片——他要记法。
    // 这里**建一次**、给这一趟里每个声明共用：记法表来自闭包各单元的**已解析命令**
    // （零额外解析），元数来自源级签名 + prelude（prelude 那份 parse 一次就缓存）。
    //
    // 只作用于 `ty_text`（T-C02 的审计：它**只有给人看的消费者**）；
    // `goal` / `binders[].ty` / `sub_goals[].ty` **同时喂 judge**，一个字节都不动。
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
                    env_before,
                    redundant_probes,
                    declared_ty,
                    sig_probe,
                    sig_span,
                    goal,
                    binders,
                    holes,
                    sub_goals,
                    refine_template,
                    by_steps,
                    span,
                    cmd,
                } => {
                    // 签名终审（G-01 / WO-004）：开练习的签名先过内核的
                    // 「它是不是一个类型」（`theorem` 还要过「是不是 Prop」）。
                    // 不过 ⇒ 与值位 elaborate 失败完全同罪：报诊断、声明
                    // Failed、**不**发 `exercise.open`（`sorry` 救不回来）。
                    match open_signature_failure(
                        &env,
                        &sig_probe,
                        kind,
                        declared_ty,
                        env_before,
                        sig_span,
                    ) {
                        Some(err) => {
                            allow_cutoff = false;
                            op_failed = true;
                            failed_cmds.insert(cmd, err.clone());
                            out.push_error(j, err.clone());
                            decl_states.push(failed_state(kind, name, sig_span, err, cmd));
                        }
                        None => {
                            out.push_event(cmd, CheckEvent::ExerciseOpen { name: name.clone() });
                            // 「多余的 sorry」的终审：把候选实参删掉后，整条声明必须能被
                            // 完整内核接受。过了才报；过不了就维持"练习尚未解决"（保守）。
                            // 探针**不入环境** ⇒ 名字没有 `decl_idx`，必须显式给可见前缀
                            // `env_before`（= 该声明若补完时会占的下标）；否则
                            // `EnvLimit::ByName(探针名)` 取 0 → 空环境 → 假 `unknown const`
                            // （`docs/design/redundant-sorry.md` §8）。
                            for (declar, hole_span) in redundant_probes {
                                kernel_checks += 1;
                                if env
                                    .try_check_declar_at(&declar, EnvLimit::ByIndex(env_before))
                                    .is_ok()
                                {
                                    out.push_warning(
                                        cmd,
                                        crate::compile::warning::CompileWarning {
                                            kind: crate::compile::warning::WarningKind::RedundantSorry,
                                            message: "这一行的 sorry 是多余的：前面的项已经完成了证明，\
                                                      sorry 不能再接在这里。"
                                                .to_string(),
                                            span: hole_span,
                                        },
                                    );
                                }
                            }
                            let ty_text = declared_ty
                                .and_then(|ty| {
                                    quiet_catch(|| {
                                        env.with_tc(EnvLimit::Empty, |tc| {
                                            tc.with_pp(|pp| pp.pp_expr(ty))
                                        })
                                    })
                                    .ok()
                                })
                                .map(|text| {
                                    // **走唯一接口**（T-U11 ✓，同上）
                                    display.fold(&text)
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
                                // **Open 练习没有值**（还没证/还没写）——T-D52。
                                val_text: None,
                            });
                        }
                    }
                }
                PendingOp::Decl { .. } => {
                    if check_then_add_decl(
                        &mut env,
                        &display,
                        &mut out,
                        &mut decl_states,
                        &mut failed_cmds,
                        &mut kernel_checks,
                        j,
                        op,
                    ) {
                        // Check-then-add: a rejected declaration makes pass 1's
                        // environment provisional — stop trusting any cutoff.
                        allow_cutoff = false;
                        op_failed = true;
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
                                // 同上 ✓（归纳块那条路 ✓，见上一条注释 ✓）。
                                err.message = format!(
                                    "类型不匹配：期望 `{}`，实际是 `{}`",
                                    display.fold(&expected),
                                    display.fold(&actual)
                                );
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
                                val_text: None,
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
    // 语法级 warning 在前（顺序稳定、可断言；每个单元算一次，归属到该单元的
    // 首条命令——归因只要落到正确的单元，warning 自身没有命令下标），pass 2
    // 内核终审过的 `redundant-sorry` 追加在后；batch 输出与 report 共用同一份
    // 列表，`warning_cmds` 与 `warnings` 严格平行（跨文件归因靠它）。
    let ranges = unit_ranges(units);
    let mut warnings = Vec::new();
    let mut warning_cmds = Vec::new();
    for (i, unit) in units.iter().enumerate() {
        for warning in crate::compile::warning::collect_warnings(unit.file) {
            warning_cmds.push(ranges[i].start);
            warnings.push(warning);
        }
    }
    warnings.append(&mut out.warnings);
    warning_cmds.append(&mut out.warning_cmds);
    debug_assert_eq!(warnings.len(), warning_cmds.len());
    report.warnings = warnings;
    out.warnings = report.warnings.clone();
    out.warning_cmds = warning_cmds;
    PassResult {
        out,
        report,
        failed: failed_cmds,
        checks: kernel_checks,
        sigs,
        cutoff,
    }
}

/// 开练习的**签名终审**（G-01 / WO-004）：`None` = 签名通过。
///
/// 两步都走内核，消息/判据与 checked 路径同源：
///
/// 1. **是不是一个类型**——`try_check_declar_at(&sig_probe, ByIndex(env_before))`。
///    探针是同签名的 `Declar::Axiom`（**不入环境**），内核的
///    `check_declar_info_v` 会先 `ensure_sort_v`；失败消息原样进
///    `refine_kernel_kind`（`expected a sort…` ⇒ `kernel-expected-sort`），
///    与值位有真值时 `theorem t : 3 := 3` 收到的诊断逐字同族。
/// 2. **是不是 Prop**（只对 `theorem`）——内核公开判据
///    `TypeChecker::is_proposition`；`false` ⇒ `kernel-theorem-not-prop`，
///    消息形状与内核 `theorem type must be Prop (sort 0): …` 一致
///    （类型用内核自己的 pretty printer 渲染）。
///
/// 顺序不能反：`is_prop_type` 对**不是类型**的值会 panic
/// （`conv.rs` 的 `expected a sort in conversion`），所以第 1 步先挡。
/// 第 2 步外面套 `quiet_catch`：探针内部若 panic（不该发生——签名已经
/// 过第 1 步），按**保守**处理（判为通过），绝不让合法练习被误拒。
fn open_signature_failure<'t>(
    env: &ExportFile<'t>,
    sig_probe: &Declar<'t>,
    kind: DeclKind,
    declared_ty: Option<ExprPtr<'t>>,
    env_before: usize,
    span: Span,
) -> Option<CompileError> {
    let limit = EnvLimit::ByIndex(env_before);
    if let Err(e) = env.try_check_declar_at(sig_probe, limit) {
        let msg = format!("{e}");
        return Some(CompileError::kernel(refine_kernel_kind(&msg), msg, span));
    }
    if kind != DeclKind::Theorem {
        return None;
    }
    // 签名 elaborate 成功 ⇒ `declared_ty` 必然在（`?` 只是防御）。
    let ty = declared_ty?;
    let is_prop = quiet_catch(|| env.with_tc(limit, |tc| tc.is_proposition(ty))).ok()?;
    if is_prop {
        return None;
    }
    let rendered = quiet_catch(|| env.with_tc(limit, |tc| tc.with_pp(|pp| pp.pp_expr(ty))))
        .unwrap_or_default();
    let msg = format!("rejected: theorem type must be Prop (sort 0): {rendered}");
    Some(CompileError::kernel(refine_kernel_kind(&msg), msg, span))
}
