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
        CompileStats, DeclState, DeclStatus, DocumentReport, HoverType, TrustPlan,
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
}

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
        let new_keys: Vec<DeclKey> = file
            .commands
            .iter()
            .map(|command| DeclKey {
                text: command_src(src, command.span()),
                start: command.span().start.offset,
            })
            .collect();
        let new_spans: Vec<Span> = file.commands.iter().map(|c| c.span()).collect();

        // 文本完全一致（注释/空白可能变化）→ 零重编译：重映射缓存坐标。
        if self.started
            && self.keys.len() == new_keys.len()
            && self
                .keys
                .iter()
                .zip(new_keys.iter())
                .all(|(a, b)| a.text == b.text)
        {
            let old_spans: Vec<Span> = self
                .keys
                .iter()
                .map(|k| {
                    // 旧命令 span 已不单独保存；从旧快照状态取不到时用
                    // (start, start+text.len()) 重建——文本相同 ⇒ 长度相同。
                    let end = k.start + k.text.len();
                    span_from_offsets(&self.src, k.start, end)
                })
                .collect();
            self.remap_prefix(new_keys.len(), &old_spans, &new_spans, src);
            self.keys = new_keys;
            self.src = src.to_string();
            let mut report = assemble_report(&self.snaps);
            // 提示阶梯是注释级数据：零重编译路径也要按当前文本刷新
            // （hint 指令的增删只移动 span，不触发重编译）。
            crate::compile::hints::attach_hints_to_report(src, &mut report);
            report.warnings = crate::compile::collect_warnings(&file);
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
            let old_spans: Vec<Span> = self.keys[..recompiled_from.min(self.keys.len())]
                .iter()
                .map(|k| {
                    let end = k.start + k.text.len();
                    span_from_offsets(&self.src, k.start, end)
                })
                .collect();
            let prefix_new_spans = &new_spans[..recompiled_from.min(new_spans.len())];
            self.remap_prefix(
                recompiled_from.min(self.snaps.len()),
                &old_spans,
                prefix_new_spans,
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

        let trust = TrustPlan {
            before: recompiled_from,
        };
        let (out, fresh_report, checks) =
            run_incremental(&file, &self.options, &trust, &prefix_failures);

        // 组装快照：信任前缀来自缓存，后缀来自本轮运行。
        let old_states: Vec<DeclState> =
            self.snaps.iter().filter_map(|s| s.state.clone()).collect();
        let mut new_snaps: Vec<CmdSnapshot> =
            self.snaps[..recompiled_from.min(self.snaps.len())].to_vec();
        new_snaps.extend(build_suffix_snapshots(
            &file.commands[recompiled_from.min(file.commands.len())..],
            recompiled_from,
            &out,
            &fresh_report,
        ));

        let mut report = assemble_report(&new_snaps);
        crate::compile::hints::attach_hints_to_report(src, &mut report);
        report.warnings = crate::compile::collect_warnings(&file);
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

    /// 把 `[0, count)` 的缓存快照从旧坐标重映射到新坐标。
    fn remap_prefix(
        &mut self,
        count: usize,
        old_spans: &[Span],
        new_spans: &[Span],
        new_src: &str,
    ) {
        for j in 0..count
            .min(self.snaps.len())
            .min(old_spans.len())
            .min(new_spans.len())
        {
            let (old_c, new_c) = (old_spans[j], new_spans[j]);
            if old_c == new_c {
                continue;
            }
            let snap = &mut self.snaps[j];
            if let Some(state) = &mut snap.state {
                state.span = new_c;
                if let Some(err) = &mut state.error {
                    err.span = remap_span(err.span, old_c, new_c, new_src);
                }
                for step in &mut state.by_steps {
                    step.span = remap_span(step.span, old_c, new_c, new_src);
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
        }
    }
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
) -> Vec<CmdSnapshot> {
    let mut snaps: Vec<CmdSnapshot> = (0..commands.len())
        .map(|_| CmdSnapshot::default())
        .collect();
    for state in &report.decls {
        if state.cmd >= cmd_base {
            if let Some(s) = snaps.get_mut(state.cmd - cmd_base) {
                s.state = Some(state.clone());
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
        for event in &snap.events {
            if let CheckEvent::TypeChecked { text, span } = event {
                checks.push(crate::compile::CheckInfo {
                    span: *span,
                    text: text.clone(),
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
        // Warnings are recomputed per document update from the parsed file
        // (`collect_warnings`); snapshots do not cache them.
        warnings: Vec::new(),
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
}
