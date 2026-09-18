//! 会话式编译：版本号、delta 事件与真增量（I8）。
//!
//! 复用不变式（学 Lean4 / coq-lsp 的快照模型）：命令源码文本不变 ⇒ 它所在的
//! 前缀环境语义不变 ⇒ 该命令的内核结果（状态/hover/事件）可以复用，只有首个
//! 文本变化之后的后缀才重新内核重查（`run_incremental` 的 `TrustPlan`）。
//!
//! 正确性依据：prelude 决策（explicit-Nat 探测、Eq all-or-nothing、防遮蔽）
//! 依赖整文件内容，本会话两种路径都能看到整文件，因此与全量重编译严格等价；
//! prelude 模式注释指令变化时整体重建。
//!
//! span 生命周期：文本相同但位置漂移（注释/空白编辑）时，前缀快照按
//! "新命令起点 − 旧命令起点" 重映射到新坐标（本会话保存原文，重映射后
//! 重新计算行列），避免零重编译路径上的过期位置。

use crate::parse;
use crate::Diagnostic;
use crate::{
    compile::{
        prelude_mode_from_source, run_incremental, CheckEvent, CompileError, CompileOptions,
        CompileStats, DeclState, DeclStatus, DocumentReport, HoverType, PreludeMode, TrustPlan,
    },
    Pos, Span,
};
use std::collections::HashMap;

/// 稳定的会话事件名（与 docs/protocol.md 的未来事件词汇对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEventKind {
    DeclChecked,
    DeclFailed,
    ExerciseOpened,
    ExerciseSolved,
    ExerciseFailed,
}

impl SessionEventKind {
    pub fn code(self) -> &'static str {
        match self {
            SessionEventKind::DeclChecked => "decl.checked",
            SessionEventKind::DeclFailed => "decl.failed",
            SessionEventKind::ExerciseOpened => "exercise.opened",
            SessionEventKind::ExerciseSolved => "exercise.solved",
            SessionEventKind::ExerciseFailed => "exercise.failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEvent {
    pub kind: SessionEventKind,
    pub name: Option<String>,
    pub version: u64,
}

/// 一次会话更新的结构化结果。
#[derive(Debug, Clone, Default)]
pub struct SessionUpdate {
    pub report: crate::compile::DocumentReport,
    pub events: Vec<CheckEvent>,
    /// 与上一版本相比的状态变化（agent/编辑器增量视图）。
    pub delta: Vec<SessionEvent>,
    /// 本轮文档版本号。
    pub version: u64,
    /// 首个被重新编译的命令索引；`None` 表示整份文档文本未变、零重编译。
    pub recompiled_from: Option<usize>,
    /// 本轮实际执行的内核检查次数（增量可验证性：改第 i 个声明 ≈ 重查 n-i）。
    pub stats: CompileStats,
    /// parse 失败时携带（report 为空）。
    pub parse_error: Option<Diagnostic>,
}

/// 一条命令的内容键：源码切片文本 + 起点 offset。
/// 文本相同 = 内核结果可复用；起点用于把缓存的 span 重映射到新坐标。
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclKey {
    text: String,
    start: usize,
}

/// 一条命令的跨版本快照（I8 缓存单元）。
#[derive(Debug, Clone, Default)]
struct CmdSnapshot {
    /// 声明类命令的状态；`#check`/`#reduce`/`#print` 等非声明命令为 `None`。
    state: Option<DeclState>,
    hovers: Vec<HoverType>,
    events: Vec<CheckEvent>,
    /// 归属本命令、但不属于声明状态的错误（如 `#check` 的 elab 失败）。
    errors: Vec<CompileError>,
    /// 归属本命令的**内核终审** warning（`redundant-sorry`）。语法级 warning
    /// 每次 update 由 `collect_warnings` 重算，不进快照（否则重复报）。
    warnings: Vec<crate::compile::CompileWarning>,
    /// I8 依赖精确化（early cutoff）：本命令对环境的贡献签名（已过内核的
    /// 声明非空）。open/失败/非声明命令为 `None`（无环境贡献）。
    signature: Option<String>,
}

/// 影响 prelude 安装决策的整文件特征：模式、是否自带 `inductive Nat`、
/// 是否自带 `inductive Bool`、是否自带 `Eq` 三件套。任一变化都必须整体
/// 重编译（决策看整文件）。
type PreludeShape = (PreludeMode, bool, bool, bool);

/// 长期驻留的编译会话：持有上一版本的命令键与逐命令快照，按内容差异决定
/// 复用范围，并产出带版本号的 delta 事件与内核检查统计。
#[derive(Debug)]
pub struct Session {
    options: CompileOptions,
    version: u64,
    keys: Vec<DeclKey>,
    snaps: Vec<CmdSnapshot>,
    /// 上一版本文本（span 重映射时重新计算行列）。
    src: String,
    started: bool,
    /// 上一版本的 prelude 决策特征（变化时整体重编译）。
    prelude_shape: Option<PreludeShape>,
}

impl Session {
    pub fn new(options: CompileOptions) -> Self {
        Self {
            options,
            version: 0,
            keys: Vec::new(),
            snaps: Vec::new(),
            src: String::new(),
            started: false,
            prelude_shape: None,
        }
    }

    /// 送入文档新文本；返回带版本号、delta 与内核检查统计的更新。
    pub fn update(&mut self, src: &str, version: u64) -> SessionUpdate {
        self.version = version;
        // prelude 注释指令可能中途出现/消失：决策依赖整文件内容，变化时重建。
        let mode = prelude_mode_from_source(src);
        if self.started && mode != self.options.prelude {
            self.options.prelude = mode;
            self.started = false;
            self.keys.clear();
            self.snaps.clear();
        }
        let file = match parse(src) {
            Ok(file) => file,
            Err(diag) => {
                self.keys.clear();
                self.snaps.clear();
                self.src.clear();
                self.started = true;
                return SessionUpdate {
                    report: Default::default(),
                    events: Vec::new(),
                    delta: Vec::new(),
                    version,
                    recompiled_from: None,
                    stats: CompileStats::default(),
                    parse_error: Some(diag),
                };
            }
        };
        // prelude 决策看整文件（explicit-Nat / Eq all-or-nothing）：特征变化
        // 时缓存快照全部失效，整体重建（与全量语义严格一致）。这也保证
        // early cutoff 的前提——会话前缀环境相同——在 prelude 层面成立。
        let shape = prelude_shape(&file, self.options.prelude);
        if self.started && self.prelude_shape != Some(shape) {
            self.started = false;
            self.keys.clear();
            self.snaps.clear();
        }
        self.prelude_shape = Some(shape);
        let new_keys: Vec<DeclKey> = file
            .commands
            .iter()
            .map(|command| DeclKey {
                text: command_src(src, command.span()),
                start: command.span().start.offset,
            })
            .collect();
        let new_spans: Vec<Span> = file.commands.iter().map(|c| c.span()).collect();
        // 旧命令 span 已不单独保存：用 (start, start+text.len()) 重建
        // ——文本相同 ⇒ 长度相同。
        let old_spans: Vec<Span> = self
            .keys
            .iter()
            .map(|k| span_from_offsets(&self.src, k.start, k.start + k.text.len()))
            .collect();

        // 文本完全一致（注释/空白可能变化）→ 零重编译：重映射缓存坐标。
        if self.started
            && self.keys.len() == new_keys.len()
            && self
                .keys
                .iter()
                .zip(new_keys.iter())
                .all(|(a, b)| a.text == b.text)
        {
            remap_snapshots(&mut self.snaps, &old_spans, &new_spans, src);
            self.keys = new_keys;
            self.src = src.to_string();
            let mut report = assemble_report(&self.snaps);
            // 提示阶梯是注释级数据：零重编译路径也要按当前文本刷新
            // （hint 指令的增删只移动 span，不触发重编译）。
            crate::compile::hints::attach_hints_to_report(src, &mut report);
            let mut warnings = crate::compile::collect_warnings(&file);
            warnings.append(&mut report.warnings);
            report.warnings = warnings;
            let events = all_events(&self.snaps);
            return SessionUpdate {
                report,
                events,
                delta: Vec::new(),
                version,
                recompiled_from: None,
                stats: CompileStats::default(),
                parse_error: None,
            };
        }

        let recompiled_from = if self.started {
            first_diff(&new_keys, &self.keys)
        } else {
            0
        };
        // 信任前缀的坐标重映射（文本相同，起点可能因前文编辑而漂移）。
        if self.started {
            let count = recompiled_from
                .min(self.snaps.len())
                .min(old_spans.len())
                .min(new_spans.len());
            remap_snapshots(
                &mut self.snaps[..count],
                &old_spans[..count],
                &new_spans[..count],
                src,
            );
        }

        let prefix_failures: HashMap<usize, CompileError> = self.snaps
            [..recompiled_from.min(self.snaps.len())]
            .iter()
            .enumerate()
            .filter_map(|(j, snap)| {
                let state = snap.state.as_ref()?;
                if state.status != DeclStatus::Failed {
                    return None;
                }
                let error = state.error.clone()?;
                Some((j, error))
            })
            .collect();

        // I8 依赖精确化（early cutoff）：仅当命令数与上次一致、且改动点之后
        // 所有命令源码文本未变时才允许截断（复用尾段要求文本逐条未变）。
        let prev_signatures: Vec<Option<String>> =
            self.snaps.iter().map(|s| s.signature.clone()).collect();
        let allow_cutoff = self.started
            && self.keys.len() == new_keys.len()
            && prev_signatures.len() == new_keys.len()
            && recompiled_from < new_keys.len()
            && (recompiled_from + 1..new_keys.len()).all(|j| new_keys[j].text == self.keys[j].text);
        let mut text_unchanged = vec![false; new_keys.len()];
        if allow_cutoff {
            for (j, unchanged) in text_unchanged.iter_mut().enumerate() {
                *unchanged = new_keys[j].text == self.keys[j].text;
            }
        }
        let trust = TrustPlan {
            before: recompiled_from,
            prev_signatures: if allow_cutoff {
                prev_signatures
            } else {
                Vec::new()
            },
            text_unchanged,
            allow_cutoff,
        };
        let (out, fresh_report, checks, sigs, cutoff) =
            run_incremental(&file, &self.options, &trust, &prefix_failures);

        // 组装快照：信任前缀来自缓存、新鲜段 `[i, cutoff)` 来自本轮运行、
        // 复用尾段 `[cutoff, n)` 来自缓存（仅坐标重映射）。
        let old_states: Vec<DeclState> =
            self.snaps.iter().filter_map(|s| s.state.clone()).collect();
        let mut new_snaps: Vec<CmdSnapshot> =
            self.snaps[..recompiled_from.min(self.snaps.len())].to_vec();
        let mid_base = recompiled_from.min(file.commands.len());
        let mid_end = cutoff.min(file.commands.len());
        new_snaps.extend(build_suffix_snapshots(
            &file.commands[mid_base..mid_end],
            mid_base,
            &out,
            &fresh_report,
            &sigs,
        ));
        if cutoff < new_keys.len() && cutoff < self.snaps.len() {
            let mut tail: Vec<CmdSnapshot> = self.snaps[cutoff..].to_vec();
            let tail_len = tail.len();
            remap_snapshots(
                &mut tail,
                &old_spans[cutoff..cutoff + tail_len],
                &new_spans[cutoff..cutoff + tail_len],
                src,
            );
            new_snaps.extend(tail);
        }

        let mut report = assemble_report(&new_snaps);
        crate::compile::hints::attach_hints_to_report(src, &mut report);
        let mut warnings = crate::compile::collect_warnings(&file);
        warnings.append(&mut report.warnings);
        report.warnings = warnings;
        let events = all_events(&new_snaps);
        let delta = diff_decls(&old_states, &report.decls, version);
        self.keys = new_keys;
        self.snaps = new_snaps;
        self.src = src.to_string();
        self.started = true;
        SessionUpdate {
            report,
            events,
            delta,
            version,
            recompiled_from: Some(recompiled_from),
            stats: CompileStats {
                kernel_checks: checks,
            },
            parse_error: None,
        }
    }
}

/// 把快照数组的 span 从旧坐标重映射到新坐标（按命令索引对齐）。
fn remap_snapshots(
    snaps: &mut [CmdSnapshot],
    old_spans: &[Span],
    new_spans: &[Span],
    new_src: &str,
) {
    for j in 0..snaps.len().min(old_spans.len()).min(new_spans.len()) {
        let (old_c, new_c) = (old_spans[j], new_spans[j]);
        if old_c == new_c {
            continue;
        }
        let snap = &mut snaps[j];
        if let Some(state) = &mut snap.state {
            state.span = new_c;
            if let Some(err) = &mut state.error {
                err.span = remap_span(err.span, old_c, new_c, new_src);
            }
            for step in &mut state.by_steps {
                step.span = remap_span(step.span, old_c, new_c, new_src);
            }
            for hole in &mut state.holes {
                *hole = remap_span(*hole, old_c, new_c, new_src);
            }
            for sub in &mut state.sub_goals {
                sub.span = remap_span(sub.span, old_c, new_c, new_src);
            }
        }
        for h in &mut snap.hovers {
            h.span = remap_span(h.span, old_c, new_c, new_src);
        }
        for ev in &mut snap.events {
            match ev {
                CheckEvent::TypeChecked { span, .. } | CheckEvent::Reduced { span, .. } => {
                    *span = remap_span(*span, old_c, new_c, new_src);
                }
                _ => {}
            }
        }
        for err in &mut snap.errors {
            err.span = remap_span(err.span, old_c, new_c, new_src);
        }
        for warning in &mut snap.warnings {
            warning.span = remap_span(warning.span, old_c, new_c, new_src);
        }
    }
}

/// 影响 prelude 安装的整文件特征（与 `run_pass` 的判定一致）：模式、
/// 是否自带 `inductive Nat`、是否自带 `inductive Bool`、是否自带
/// `Eq`/`Eq.refl`/`Eq.subst` 之一。
fn prelude_shape(file: &crate::FolFile, mode: PreludeMode) -> PreludeShape {
    let mut explicit_nat = false;
    let mut explicit_bool = false;
    let mut eq_taken = false;
    for command in &file.commands {
        let name = match command {
            crate::Command::Def { name, .. }
            | crate::Command::Theorem { name, .. }
            | crate::Command::Axiom { name, .. }
            | crate::Command::InductiveBlock { name, .. } => Some(name),
            _ => None,
        };
        if let Some(name) = name {
            if matches!(command, crate::Command::InductiveBlock { .. }) && name == "Nat" {
                explicit_nat = true;
            }
            if matches!(command, crate::Command::InductiveBlock { .. }) && name == "Bool" {
                explicit_bool = true;
            }
            if matches!(name.as_str(), "Eq" | "Eq.refl" | "Eq.subst") {
                eq_taken = true;
            }
        }
    }
    (mode, explicit_nat, explicit_bool, eq_taken)
}

/// 一条命令的源码切片（内容键：文本相同 = 未变）。
fn command_src(src: &str, span: Span) -> String {
    if span.start.offset <= span.end.offset && span.end.offset <= src.len() {
        src[span.start.offset..span.end.offset].to_string()
    } else {
        String::new()
    }
}

/// 从 (start, end) offset 重建带行列的 span（旧文本坐标）。
fn span_from_offsets(src: &str, start: usize, end: usize) -> Span {
    let (sl, sc) = line_col(src, start);
    let (el, ec) = line_col(src, end);
    Span {
        start: Pos {
            offset: start,
            line: sl,
            column: sc,
        },
        end: Pos {
            offset: end,
            line: el,
            column: ec,
        },
    }
}

/// 把旧命令内的 span 平移到新命令坐标系，并按新文本重算行列。
fn remap_span(span: Span, old_cmd: Span, new_cmd: Span, new_src: &str) -> Span {
    let old_width = old_cmd.end.offset.saturating_sub(old_cmd.start.offset);
    let rel_start = span
        .start
        .offset
        .saturating_sub(old_cmd.start.offset)
        .min(old_width);
    let rel_end = span
        .end
        .offset
        .saturating_sub(old_cmd.start.offset)
        .min(old_width);
    let base = new_cmd.start.offset;
    let s_off = (base + rel_start).min(new_src.len());
    let e_off = (base + rel_end).min(new_src.len());
    let (sl, sc) = line_col(new_src, s_off);
    let (el, ec) = line_col(new_src, e_off);
    Span {
        start: Pos {
            offset: s_off,
            line: sl,
            column: sc,
        },
        end: Pos {
            offset: e_off,
            line: el,
            column: ec,
        },
    }
}

fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in src.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// 逐索引比较文本，返回首个差异位置（短的按缺失处理）。
/// 索引对齐是保守的：它恒 ≤ 真实公共前缀，只会多查、不会漏查。
fn first_diff(new_keys: &[DeclKey], old_keys: &[DeclKey]) -> usize {
    let mut i = 0;
    while i < new_keys.len() && i < old_keys.len() {
        if new_keys[i].text != old_keys[i].text {
            return i;
        }
        i += 1;
    }
    i
}

/// 从本轮运行结果构建后缀命令的快照（按 cmd 归属状态/hover/事件，
/// 错误按 span 包含关系归属到命令）。
fn build_suffix_snapshots(
    commands: &[crate::Command],
    cmd_base: usize,
    out: &crate::compile::CompileOutput,
    report: &DocumentReport,
    signatures: &[Option<String>],
) -> Vec<CmdSnapshot> {
    let mut snaps: Vec<CmdSnapshot> = (0..commands.len())
        .map(|_| CmdSnapshot::default())
        .collect();
    for state in &report.decls {
        if state.cmd >= cmd_base {
            if let Some(s) = snaps.get_mut(state.cmd - cmd_base) {
                s.state = Some(state.clone());
                s.signature = signatures.get(state.cmd).cloned().flatten();
            }
        }
    }
    for (i, hover) in report.hovers.iter().enumerate() {
        let cmd = report.hover_cmds.get(i).copied().unwrap_or(usize::MAX);
        if cmd >= cmd_base {
            if let Some(s) = snaps.get_mut(cmd - cmd_base) {
                s.hovers.push(hover.clone());
            }
        }
    }
    for (i, event) in out.events.iter().enumerate() {
        let cmd = out.event_cmds.get(i).copied().unwrap_or(usize::MAX);
        if cmd >= cmd_base {
            if let Some(s) = snaps.get_mut(cmd - cmd_base) {
                s.events.push(event.clone());
            }
        }
    }
    // 非声明错误（#check/#reduce/#print 的 elab 失败等）：按 span 归属命令。
    for err in &out.errors {
        if let Some(j) = containing_command(commands, err.span.start.offset) {
            snaps[j].errors.push(err.clone());
        }
    }
    // 内核终审过的 warning 同法归属（`out.warnings` 里还混着语法级的，
    // 那些由 `collect_warnings` 每轮重算，不能进快照）。
    for warning in out.warnings.iter().filter(|w| w.kind.is_kernel_verified()) {
        if let Some(j) = containing_command(commands, warning.span.start.offset) {
            snaps[j].warnings.push(warning.clone());
        }
    }
    snaps
}

/// 找到包含 offset 的命令索引（教学文件命令按行分布，线性扫描足够）。
fn containing_command(commands: &[crate::Command], offset: usize) -> Option<usize> {
    let mut best = None;
    for (j, command) in commands.iter().enumerate() {
        let span = command.span();
        if span.start.offset <= offset && offset < span.end.offset.max(span.start.offset + 1) {
            best = Some(j);
        }
    }
    best
}

/// 从快照组装整份报告（声明、hover、错误、#check 结果；来源顺序即命令顺序）。
fn assemble_report(snaps: &[CmdSnapshot]) -> DocumentReport {
    let mut decls = Vec::new();
    let mut hovers = Vec::new();
    let mut hover_cmds = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut checks = Vec::new();
    for (j, snap) in snaps.iter().enumerate() {
        if let Some(state) = &snap.state {
            decls.push(state.clone());
        }
        for hover in &snap.hovers {
            hovers.push(hover.clone());
            hover_cmds.push(j);
        }
        errors.extend(snap.errors.iter().cloned());
        warnings.extend(snap.warnings.iter().cloned());
        for event in &snap.events {
            if let CheckEvent::TypeChecked { text, span } = event {
                checks.push(crate::compile::CheckInfo {
                    span: *span,
                    text: text.clone(),
                    // 会话快照按命令归属重建；`cmd` 在这里没有意义（单文档）。
                    cmd: 0,
                });
            }
        }
    }
    DocumentReport {
        decls,
        hovers,
        hover_cmds,
        errors,
        checks,
        // 内核终审过的 warning 来自快照（跨版本复用）；语法级的由调用方
        // 用 `collect_warnings` 在整文件上重算后拼在前面。
        warnings,
    }
}

fn all_events(snaps: &[CmdSnapshot]) -> Vec<CheckEvent> {
    snaps
        .iter()
        .flat_map(|s| s.events.iter().cloned())
        .collect()
}

/// 按身份配对（名字；匿名 example 按出现序号），产出状态变化 delta。
fn diff_decls(old: &[DeclState], new: &[DeclState], version: u64) -> Vec<SessionEvent> {
    fn ident(d: &DeclState, ordinal: usize) -> String {
        match &d.name {
            Some(n) => n.clone(),
            None => format!("<anon:{}:{}>", d.kind.as_str(), ordinal),
        }
    }
    let old_map: std::collections::HashMap<String, &DeclState> = old
        .iter()
        .enumerate()
        .map(|(i, d)| (ident(d, i), d))
        .collect();
    let mut delta = Vec::new();
    for (i, d) in new.iter().enumerate() {
        let id = ident(d, i);
        let event = |kind: SessionEventKind| SessionEvent {
            kind,
            name: d.name.clone(),
            version,
        };
        match old_map.get(&id) {
            None => match d.status {
                DeclStatus::Open => delta.push(event(SessionEventKind::ExerciseOpened)),
                DeclStatus::Checked => delta.push(event(SessionEventKind::DeclChecked)),
                DeclStatus::Failed => delta.push(event(SessionEventKind::DeclFailed)),
            },
            Some(prev) => match (prev.status, d.status) {
                (DeclStatus::Open, DeclStatus::Checked) => {
                    delta.push(event(SessionEventKind::ExerciseSolved))
                }
                (DeclStatus::Checked, DeclStatus::Open) => {
                    delta.push(event(SessionEventKind::ExerciseOpened))
                }
                (DeclStatus::Open, DeclStatus::Failed) => {
                    delta.push(event(SessionEventKind::ExerciseFailed))
                }
                (DeclStatus::Checked, DeclStatus::Failed) => {
                    delta.push(event(SessionEventKind::DeclFailed))
                }
                (DeclStatus::Failed, DeclStatus::Checked) => {
                    delta.push(event(SessionEventKind::DeclChecked))
                }
                _ => {}
            },
        }
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(session: &mut Session, src: &str, version: u64) -> SessionUpdate {
        session.update(src, version)
    }

    #[test]
    fn session_reports_solved_and_reopen_deltas() {
        let mut session = Session::new(CompileOptions::default());
        let src1 = "example : Prop -> Prop := sorry\n";
        let u1 = update(&mut session, src1, 1);
        assert_eq!(u1.delta.len(), 1);
        assert_eq!(u1.delta[0].kind, SessionEventKind::ExerciseOpened);
        assert_eq!(u1.delta[0].version, 1);
        assert_eq!(u1.recompiled_from, Some(0));

        // 学习者填入答案 → solved。
        let src2 = "example : Prop -> Prop := fun (x : Prop) => x\n";
        let u2 = update(&mut session, src2, 2);
        assert_eq!(
            u2.delta,
            vec![SessionEvent {
                kind: SessionEventKind::ExerciseSolved,
                name: None,
                version: 2,
            }]
        );
        assert_eq!(u2.recompiled_from, Some(0));

        // 改回 sorry → 重新打开。
        let u3 = update(&mut session, src1, 3);
        assert!(u3
            .delta
            .iter()
            .any(|e| e.kind == SessionEventKind::ExerciseOpened));

        // 注释变化（命令内容不变，仍是 open 状态）→ 零重编译。
        let src4 = "-- 只是加了一行讲解\nexample : Prop -> Prop := sorry\n";
        let u4 = update(&mut session, src4, 4);
        assert_eq!(u4.recompiled_from, None);
        assert!(u4.delta.is_empty());
        assert_eq!(u4.version, 4);
    }

    #[test]
    fn session_attaches_hints_and_refreshes_on_comment_edit() {
        let mut session = Session::new(CompileOptions::default());
        let src1 = "-- soko:hint 先看目标形状\nexample : Prop -> Prop := sorry\n";
        let u1 = update(&mut session, src1, 1);
        let hints = u1
            .report
            .decls
            .first()
            .map(|d| d.hints.clone())
            .unwrap_or_default();
        assert_eq!(hints, vec!["先看目标形状".to_string()]);

        // 提示注释是纯注释编辑：零重编译，但阶梯按新文本刷新。
        let src2 = "-- soko:hint 第一层\n-- soko:hint 第二层\nexample : Prop -> Prop := sorry\n";
        let u2 = update(&mut session, src2, 2);
        assert_eq!(u2.recompiled_from, None, "hint edits must not recompile");
        assert!(u2.delta.is_empty());
        let hints = u2
            .report
            .decls
            .first()
            .map(|d| d.hints.clone())
            .unwrap_or_default();
        assert_eq!(hints, vec!["第一层".to_string(), "第二层".to_string()]);
    }

    #[test]
    fn session_reports_failed_exercise_and_parse_errors() {
        let mut session = Session::new(CompileOptions::default());
        let _ = update(
            &mut session,
            "example : Prop -> Prop := fun (x : Prop) => x\n",
            1,
        );
        // 填错答案 → checked → failed。
        let u2 = update(
            &mut session,
            "example : Prop -> Prop := fun (x : Prop) => 1\n",
            2,
        );
        assert!(u2
            .delta
            .iter()
            .any(|e| e.kind == SessionEventKind::DeclFailed));
        // open → 填错 → failed。
        let _ = update(&mut session, "example : Prop -> Prop := sorry\n", 3);
        let u4 = update(
            &mut session,
            "example : Prop -> Prop := fun (x : Prop) => 1\n",
            4,
        );
        assert!(u4
            .delta
            .iter()
            .any(|e| e.kind == SessionEventKind::ExerciseFailed));
        // 语法错误：report 为空，parse_error 携带。
        let u3 = update(&mut session, "def broken : Prop :=\n", 3);
        assert!(u3.parse_error.is_some());
        // 修好 → 重新出现为 checked。
        let u4 = update(
            &mut session,
            "def broken : Prop -> Prop := fun (x : Prop) => x\n",
            4,
        );
        assert!(u4
            .delta
            .iter()
            .any(|e| e.kind == SessionEventKind::DeclChecked));
    }

    // ---- I8 真增量 ----

    const FIVE: &str = "\
def one : Nat := 1
def two : Nat := 2
def three : Nat := 3
def four : Nat := 4
def five : Nat := 5
";

    #[test]
    fn session_rechecks_only_the_affected_suffix() {
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, FIVE, 1);
        assert_eq!(u1.recompiled_from, Some(0));
        assert_eq!(u1.stats.kernel_checks, 5);

        // 只改第 4 个声明（索引 3）→ 只内核重查第 4、5 个。
        let edited = FIVE.replace("def four : Nat := 4", "def four : Nat := 40");
        assert_ne!(edited, FIVE);
        let u2 = update(&mut session, &edited, 2);
        assert_eq!(u2.recompiled_from, Some(3), "first changed command index");
        assert_eq!(
            u2.stats.kernel_checks, 2,
            "only the suffix is kernel-rechecked"
        );
        assert!(
            u2.report
                .decls
                .iter()
                .any(|d| d.name.as_deref() == Some("four") && d.status == DeclStatus::Checked),
            "four is still checked after the edit"
        );

        // 追加一个声明 → 只重查新增者。
        let appended = format!("{edited}def six : Nat := 6\n");
        let u3 = update(&mut session, &appended, 3);
        assert_eq!(u3.recompiled_from, Some(5));
        assert_eq!(u3.stats.kernel_checks, 1);

        // 删除中间声明 → 从删除点重查到结尾。
        let removed = appended.replace("def two : Nat := 2\n", "");
        let u4 = update(&mut session, &removed, 4);
        assert_eq!(u4.recompiled_from, Some(1));
        assert_eq!(u4.stats.kernel_checks, 4, "two..six are rechecked");
    }

    #[test]
    fn session_zero_recompile_keeps_prefix_results() {
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, FIVE, 1);
        let hovers_before = u1.report.hovers.len();
        assert!(hovers_before > 0, "hover map is populated on first run");

        // 注释插入在文件头：所有命令文本不变 → 零重编译，且 hover 坐标平移。
        let with_comment = format!("-- 讲解注释\n{FIVE}");
        let shift = "-- 讲解注释\n".len();
        let u2 = update(&mut session, &with_comment, 2);
        assert_eq!(u2.recompiled_from, None);
        assert_eq!(u2.stats.kernel_checks, 0);
        assert_eq!(u2.report.hovers.len(), hovers_before);
        assert!(u2
            .report
            .hovers
            .iter()
            .zip(u1.report.hovers.iter())
            .all(|(new, old)| {
                new.text == old.text && new.span.start.offset == old.span.start.offset + shift
            }));
        // 声明 span 同步平移。
        let first = u2
            .report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("one"))
            .expect("decl one");
        assert_eq!(first.span.start.line, 2, "one now starts on line 2");

        // 事件流同样来自缓存（全量视图语义不变）。
        assert_eq!(u2.events.len(), u1.events.len());
    }

    #[test]
    fn session_whitespace_between_commands_zero_recompiles_with_remap() {
        let mut session = Session::new(CompileOptions::default());
        let _ = update(&mut session, FIVE, 1);
        // 在两条命令之间插入空行：命令文本不变、起点漂移。
        let spaced = FIVE.replace("def three", "\ndef three");
        let u2 = update(&mut session, &spaced, 2);
        assert_eq!(u2.recompiled_from, None, "no command text changed");
        assert_eq!(u2.stats.kernel_checks, 0);
        let three = u2
            .report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("three"))
            .expect("decl three");
        assert_eq!(three.span.start.line, 4, "three moved down one line");
    }

    #[test]
    fn session_prefix_failure_keeps_name_free_for_suffix() {
        let mut session = Session::new(CompileOptions::default());
        // 第一版：第二条声明内核拒绝（Prop 值的 universe 不匹配）。
        let bad = "def ok : Prop -> Prop := fun (x : Prop) => x\ndef bad : Prop -> Type := fun (x : Prop) => x\n";
        let u1 = update(&mut session, bad, 1);
        assert!(u1
            .report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("bad") && d.status == DeclStatus::Failed));
        // 只修好第一条 → `bad` 仍失败（缓存复用），后缀语义一致。
        let u2 = update(&mut session, bad, 2);
        assert_eq!(u2.stats.kernel_checks, 0);
        // 改坏第一条文本 → bad 从删除点重查并复用缓存失败。
        let worse = bad.replace("def ok", "def okk");
        let u3 = update(&mut session, &worse, 3);
        assert!(u3
            .report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("okk") && d.status == DeclStatus::Checked));
        assert!(u3
            .report
            .decls
            .iter()
            .any(|d| d.name.as_deref() == Some("bad") && d.status == DeclStatus::Failed));
    }

    #[test]
    fn session_remaps_by_step_spans_on_comment_edit() {
        // by_steps 随快照缓存；注释级编辑零重编译，但 span 必须平移，
        // 否则 soko/stateAt 会把光标位置对到错误的目标上。
        let mut session = Session::new(CompileOptions::default());
        let src =
            "axiom True : Prop\naxiom True.intro : True\ntheorem t : True := by exact True.intro\n";
        let u1 = update(&mut session, src, 1);
        let before = u1
            .report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("t"))
            .expect("decl t")
            .by_steps[0]
            .span;
        let with_comment = format!("-- 讲解\n{src}");
        let shift = "-- 讲解\n".len();
        let u2 = update(&mut session, &with_comment, 2);
        assert_eq!(u2.recompiled_from, None, "comment-only edit");
        assert_eq!(u2.stats.kernel_checks, 0);
        let after = u2
            .report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some("t"))
            .expect("decl t")
            .by_steps[0]
            .span;
        assert_eq!(after.start.offset, before.start.offset + shift);
        assert_eq!(after.start.line, before.start.line + 1);
    }

    #[test]
    fn session_remaps_hole_and_sub_goal_spans_on_comment_edit() {
        // 洞 span 随快照缓存：注释级编辑零重编译后 holes/sub_goals 坐标必须
        // 平移，否则 inlay / nextHole / code action 会指到错误位置。
        let mut session = Session::new(CompileOptions::default());
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   axiom True : Prop\n\
                   example : And True True := And.intro True True sorry sorry\n";
        let u1 = update(&mut session, src, 1);
        let before = u1
            .report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Open)
            .expect("open example");
        assert_eq!(before.holes.len(), 2);
        assert_eq!(before.sub_goals.len(), 2);
        let hole = before.holes[0];
        let sub = before.sub_goals[0].span;
        assert_eq!(hole, sub, "each sub-goal span matches its hole");

        let with_comment = format!("-- 讲解\n{src}");
        let shift = "-- 讲解\n".len();
        let u2 = update(&mut session, &with_comment, 2);
        assert_eq!(u2.recompiled_from, None, "comment-only edit");
        assert_eq!(u2.stats.kernel_checks, 0);
        let after = u2
            .report
            .decls
            .iter()
            .find(|d| d.status == DeclStatus::Open)
            .expect("open example");
        assert_eq!(after.holes[0].start.offset, hole.start.offset + shift);
        assert_eq!(after.holes[0].start.line, hole.start.line + 1);
        assert_eq!(
            after.sub_goals[0].span.start.offset,
            sub.start.offset + shift
        );
        assert_eq!(after.sub_goals[0].span.start.line, sub.start.line + 1);
    }

    #[test]
    fn session_keeps_check_results_on_zero_recompile() {
        // #check 结果随快照缓存：注释级编辑零重编译时仍可用，且 span 平移。
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, "#check Nat\n", 1);
        assert_eq!(u1.report.checks.len(), 1);
        assert_eq!(u1.report.checks[0].text, "Type 0");
        let with_comment = "-- 讲解\n#check Nat\n";
        let u2 = update(&mut session, with_comment, 2);
        assert_eq!(u2.recompiled_from, None, "comment-only edit");
        assert_eq!(u2.stats.kernel_checks, 0);
        assert_eq!(u2.report.checks.len(), 1);
        assert_eq!(
            u2.report.checks[0].span.start.offset,
            u1.report.checks[0].span.start.offset + "-- 讲解\n".len()
        );
    }

    #[test]
    fn session_keeps_warnings_on_zero_recompile() {
        // 语法级 warning 每轮从文件现算：注释级编辑零重编译后仍在，且 span
        // 随前文平移。
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, "axiom Prop : Sort 1\n", 1);
        assert_eq!(u1.report.warnings.len(), 1);
        assert_eq!(u1.report.warnings[0].code(), "reserved-declaration-name");
        let with_comment = "-- 讲解\naxiom Prop : Sort 1\n";
        let u2 = update(&mut session, with_comment, 2);
        assert_eq!(u2.recompiled_from, None, "comment-only edit");
        assert_eq!(u2.report.warnings.len(), 1);
        assert_eq!(
            u2.report.warnings[0].span.start.offset,
            u1.report.warnings[0].span.start.offset + "-- 讲解\n".len()
        );
    }

    /// 「多余的 `sorry`」是**内核终审**过的 warning：会话路径必须把它带出来
    /// （`session.rs` 曾经只重算语法级 warning，把它丢了 → LSP 看不到），
    /// 而且增量编辑后要随快照活下来（信任前缀不会重跑探针），span 随前文平移。
    #[test]
    fn session_keeps_kernel_verified_warnings_across_edits() {
        let mut session = Session::new(CompileOptions::default());
        let src1 = "axiom A : Prop\n\
                    axiom B : Prop\n\
                    axiom f : A -> B\n\
                    theorem t (h : A) : B := f h\n\
                    \x20 sorry\n";
        let u1 = update(&mut session, src1, 1);
        let codes: Vec<&str> = u1.report.warnings.iter().map(|w| w.code()).collect();
        assert_eq!(codes, vec!["redundant-sorry"], "{:?}", u1.report.warnings);
        let span1 = u1.report.warnings[0].span;
        assert_eq!(&src1[span1.start.offset..span1.end.offset], "sorry");

        // 注释级编辑（命令内容不变 → 零重编译）：warning 仍在，span 平移。
        let comment = "-- 讲解：这一行的 sorry 是多余的\n";
        let src2 = format!("{comment}{src1}");
        let u2 = update(&mut session, &src2, 2);
        assert_eq!(u2.recompiled_from, None, "comment-only edit");
        let codes: Vec<&str> = u2.report.warnings.iter().map(|w| w.code()).collect();
        assert_eq!(codes, vec!["redundant-sorry"], "{:?}", u2.report.warnings);
        let span2 = u2.report.warnings[0].span;
        assert_eq!(span2.start.offset, span1.start.offset + comment.len());
        assert_eq!(&src2[span2.start.offset..span2.end.offset], "sorry");

        // 删掉那一行 → 声明通过内核，warning 消失。
        let fixed = src2.replace(" sorry\n", "\n");
        let u3 = update(&mut session, &fixed, 3);
        assert!(
            u3.report.warnings.is_empty(),
            "固定后不该再有 warning：{:?}",
            u3.report.warnings
        );
    }

    #[test]
    fn session_prelude_directive_change_rebuilds() {
        let mut session = Session::new(CompileOptions::default());
        let _ = update(&mut session, "def two : Nat := 2\n", 1);
        // 追加 bare 指令 → prelude 决策变化 → 整体重建，Nat 消失。
        let bare = "-- sokonanoda:prelude none\ndef two : Nat := 2\n";
        let u2 = update(&mut session, bare, 2);
        assert!(u2
            .report
            .errors
            .iter()
            .any(|e| e.code() == "elab-unknown-identifier"));
    }

    // ---- I8 依赖精确化（early cutoff 签名比较）----

    const FOUR: &str = "\
def one : Nat := 1
def two : Nat := 2
def three : Nat := 3
def four : Nat := 4
";

    /// (a) 改第一个命令的源码写法（加括号），elaboration 后签名不变：
    /// 改动点之后文本未变的全部命令被复用，`kernel_checks` 从 4 降到 1。
    #[test]
    fn session_early_cutoff_reuses_suffix_when_signature_unchanged() {
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, FOUR, 1);
        assert_eq!(u1.stats.kernel_checks, 4);

        let edited = FOUR.replace("def one : Nat := 1", "def one : Nat := (1)");
        assert_ne!(edited, FOUR);
        let u2 = update(&mut session, &edited, 2);
        assert_eq!(u2.recompiled_from, Some(0), "first changed command index");
        assert_eq!(
            u2.stats.kernel_checks, 1,
            "signature unchanged → only the edited command is rechecked"
        );
        // 报告仍带全部已证声明，且复用尾段坐标正确。
        for (name, line) in [("one", 1), ("two", 2), ("three", 3), ("four", 4)] {
            let d = u2
                .report
                .decls
                .iter()
                .find(|d| d.name.as_deref() == Some(name))
                .unwrap_or_else(|| panic!("decl {name} missing after cutoff"));
            assert_eq!(d.status, DeclStatus::Checked);
            assert_eq!(d.span.start.line, line);
        }
        assert!(
            u2.report.errors.is_empty(),
            "cutoff must not drop or add errors"
        );
    }

    /// (b) 改 def 的 body（type 不变）：签名变化 → 依赖的后缀必须重查，
    /// 且 `#reduce` 结果反映新 body（不许复用陈旧快照）。
    #[test]
    fn session_early_cutoff_rechecks_after_def_body_change() {
        let src = "def one : Nat := 1\ndef two : Nat := one\n#reduce two\n";
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, src, 1);
        assert_eq!(u1.stats.kernel_checks, 2);
        let reduced = |u: &SessionUpdate| {
            u.events.iter().find_map(|e| match e {
                CheckEvent::Reduced { text, .. } => Some(text.clone()),
                _ => None,
            })
        };
        assert_eq!(reduced(&u1).as_deref(), Some("1"));

        // body 1 → 2（type 不变）：one 的签名变，two / #reduce 全部重查。
        let edited = src.replace("def one : Nat := 1", "def one : Nat := 2");
        let u2 = update(&mut session, &edited, 2);
        assert_eq!(u2.recompiled_from, Some(0));
        assert_eq!(
            u2.stats.kernel_checks, 2,
            "changed def body must invalidate the suffix"
        );
        assert_eq!(
            reduced(&u2).as_deref(),
            Some("2"),
            "no stale #reduce result from a reused snapshot"
        );
    }

    /// (c) 仅公理的文件：改动源码写法但签名不变 → 截断后缀重查。
    #[test]
    fn session_early_cutoff_axiom_only() {
        let src = "axiom A : Prop\naxiom B : A\naxiom C : A -> A\n";
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, src, 1);
        assert_eq!(u1.stats.kernel_checks, 3);

        let edited = src.replace("axiom A : Prop", "axiom A : (Prop)");
        assert_ne!(edited, src);
        let u2 = update(&mut session, &edited, 2);
        assert_eq!(u2.recompiled_from, Some(0));
        assert_eq!(
            u2.stats.kernel_checks, 1,
            "axiom name/type unchanged → suffix reused"
        );
        assert_eq!(u2.report.decls.len(), 3);
        assert!(u2
            .report
            .decls
            .iter()
            .all(|d| d.status == DeclStatus::Checked));
    }

    /// prelude 决策特征（此处 `Eq` 被文件占用）变化时整体重建：改动点
    /// 之前、原本依赖 prelude `Eq` 的命令必须重查，不能复用陈旧状态。
    #[test]
    fn session_early_cutoff_prelude_shape_change_rebuilds() {
        let old = "axiom A : Prop\naxiom a : A\naxiom eqa : Eq A a a\naxiom B : Prop\n";
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, old, 1);
        assert!(
            u1.report
                .decls
                .iter()
                .any(|d| d.name.as_deref() == Some("eqa") && d.status == DeclStatus::Checked),
            "eqa is checked against the Eq prelude"
        );

        // 末条改名为 `Eq` → Eq prelude 整体不再安装（all-or-nothing）。
        // 改动点之前的 `eqa`（引用 Eq）必须重查，不能复用陈旧 Checked 状态。
        let new = "axiom A : Prop\naxiom a : A\naxiom eqa : Eq A a a\naxiom Eq : Prop\n";
        let u2 = update(&mut session, new, 2);
        assert_eq!(
            u2.recompiled_from,
            Some(0),
            "prelude decision change rebuilds from scratch"
        );
        assert!(
            u2.report
                .decls
                .iter()
                .any(|d| d.name.as_deref() == Some("eqa") && d.status == DeclStatus::Failed),
            "eqa can no longer use the (removed) Eq prelude"
        );
    }

    /// 签名必须包含声明的宇宙参数（arity）：未使用的 `{v}` 也改变声明的
    /// universe 参数个数，是可观测的环境贡献，不能因 type/body 文本相同
    /// 而被误判为未变。
    #[test]
    fn session_early_cutoff_sees_universe_arity_change() {
        let src = "def id {u} : {A : Sort u} -> A -> A := fun (A : Sort u) (a : A) => a\ndef one : Nat := 1\n";
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, src, 1);
        assert_eq!(u1.stats.kernel_checks, 2);

        // 增加一个未使用的宇宙参数 → arity 1 → 2，签名必须变化。
        let edited = src.replace("def id {u} :", "def id {u, v} :");
        assert_ne!(edited, src);
        let u2 = update(&mut session, &edited, 2);
        assert_eq!(u2.recompiled_from, Some(0));
        assert_eq!(
            u2.stats.kernel_checks, 2,
            "universe arity change must invalidate the suffix"
        );
    }

    /// 归纳块（inductive/ctor/recursor + iota 规则）的签名路径：整块作为一
    /// 条命令参与签名比较，改动块后的等价 def 仍可截断复用。
    #[test]
    fn session_early_cutoff_handles_inductive_blocks() {
        let src = "\
inductive Nat : Type
ctor zero : Nat
ctor succ (n : Nat) : Nat
rec Nat.rec {u} :
  (motive : (n : Nat) -> Sort u) ->
  (mz : motive zero) ->
  (ms : (n : Nat) -> motive n -> motive (succ n)) ->
  (n : Nat) -> motive n
iota zero :=
  fun (motive : (n : Nat) -> Sort u) =>
  fun (mz : motive zero) =>
  fun (ms : (n : Nat) -> motive n -> motive (succ n)) => mz
iota succ :=
  fun (motive : (n : Nat) -> Sort u) =>
  fun (mz : motive zero) =>
  fun (ms : (n : Nat) -> motive n -> motive (succ n)) =>
  fun (n : Nat) => ms n (Nat.rec.{u} motive mz ms n)
end
def one : Nat := succ zero
def two : Nat := succ one
";
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, src, 1);
        assert!(
            u1.report.errors.is_empty(),
            "inductive block compiles: {:?}",
            u1.report.errors
        );
        // 4 block declarations (spine/zero/succ/rec) + 2 defs.
        assert_eq!(u1.stats.kernel_checks, 6);

        let edited = src.replace("def one : Nat := succ zero", "def one : Nat := (succ zero)");
        assert_ne!(edited, src);
        let u2 = update(&mut session, &edited, 2);
        assert_eq!(u2.recompiled_from, Some(1));
        assert_eq!(
            u2.stats.kernel_checks, 1,
            "inductive block + unchanged def signatures → suffix reused"
        );
        assert_eq!(u2.report.decls.len(), 3);
        assert!(u2
            .report
            .decls
            .iter()
            .all(|d| d.status == DeclStatus::Checked));
    }

    /// 多条命令同时改动：不做跨改动命令的复用（`allow_cutoff` 关闭），
    /// 行为与当前保守后缀重查一致。
    #[test]
    fn session_early_cutoff_disabled_for_multiple_edits() {
        let src = "def one : Nat := 1\ndef two : Nat := 2\ndef three : Nat := 3\n";
        let mut session = Session::new(CompileOptions::default());
        let u1 = update(&mut session, src, 1);
        assert_eq!(u1.stats.kernel_checks, 3);

        // 两处等价编辑：第二处 text 也变 → 不允许截断。
        let edited = src
            .replace("def one : Nat := 1", "def one : Nat := (1)")
            .replace("def three : Nat := 3", "def three : Nat := (3)");
        let u2 = update(&mut session, &edited, 2);
        assert_eq!(u2.recompiled_from, Some(0));
        assert_eq!(
            u2.stats.kernel_checks, 3,
            "multiple changed commands disable cutoff"
        );
    }
}
