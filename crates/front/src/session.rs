//! 会话式编译：版本号、delta 事件与重编译范围日志。
//!
//! 这是服务层（LSP/`watch`/未来的 compiler service）的地基：
//! 同一份反馈按版本递增，agent 与编辑器各持游标；练习状态变化以
//! `exercise.solved` / `exercise.failed` 等稳定事件广播。
//! v1 边界：内容未变的整份文档跳过重编译（recompiled=false）；
//! 内容有变时整份重算并上报首个变化声明索引（`recompiled_from`），
//! 逐声明内核缓存（只重查受影响后缀）是下一步。

use crate::parse;
use crate::Diagnostic;
use crate::{
    compile::{check_document_with, CheckEvent, CompileOptions, DeclState, DeclStatus},
    Span,
};

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
    /// 首个被重新编译的命令索引；`None` 表示整份文档未变、零重编译。
    pub recompiled_from: Option<usize>,
    /// parse 失败时携带（report 为空）。
    pub parse_error: Option<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclKey(String);

/// 长期驻留的编译会话：持有上一版本的声明键与状态，按内容差异决定
/// 是否重编译，并产出带版本号的 delta 事件。
#[derive(Debug)]
pub struct Session {
    options: CompileOptions,
    version: u64,
    /// 每条命令的源码切片键（按索引对齐比较）。
    keys: Vec<DeclKey>,
    decls: Vec<DeclState>,
    started: bool,
}

impl Session {
    pub fn new(options: CompileOptions) -> Self {
        Self {
            options,
            version: 0,
            keys: Vec::new(),
            decls: Vec::new(),
            started: false,
        }
    }

    /// 送入文档新文本；返回带版本号与 delta 的更新。
    pub fn update(&mut self, src: &str, version: u64) -> SessionUpdate {
        self.version = version;
        let file = match parse(src) {
            Ok(file) => file,
            Err(diag) => {
                self.keys.clear();
                self.decls.clear();
                self.started = true;
                return SessionUpdate {
                    report: Default::default(),
                    events: Vec::new(),
                    delta: Vec::new(),
                    version,
                    recompiled_from: None,
                    parse_error: Some(diag),
                };
            }
        };
        let keys: Vec<DeclKey> = file
            .commands
            .iter()
            .map(|command| DeclKey(command_src(src, command.span())))
            .collect();
        // 内容完全一致（仅注释/空白之外没动过任何命令）→ 零重编译。
        if self.started && keys == self.keys {
            return SessionUpdate {
                report: crate::compile::DocumentReport {
                    decls: self.decls.clone(),
                    ..Default::default()
                },
                events: Vec::new(),
                delta: Vec::new(),
                version,
                recompiled_from: None,
                parse_error: None,
            };
        }
        let recompiled_from = first_diff(&keys, &self.keys);
        let output = crate::compile::compile_fol_with(&file, &self.options);
        let report = check_document_with(&file, &self.options);
        let delta = diff_decls(&self.decls, &report.decls, version);
        self.keys = keys;
        self.decls = report.decls.clone();
        self.started = true;
        SessionUpdate {
            report,
            events: output.events,
            delta,
            version,
            recompiled_from: Some(recompiled_from),
            parse_error: None,
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

/// 逐索引比较，返回首个差异位置（短的按缺失处理）。
fn first_diff(new_keys: &[DeclKey], old_keys: &[DeclKey]) -> usize {
    let mut i = 0;
    while i < new_keys.len() && i < old_keys.len() {
        if new_keys[i] != old_keys[i] {
            return i;
        }
        i += 1;
    }
    i
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
        let src1 = "example : Prop -> Prop := ???\n";
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

        // 改回 ??? → 重新打开。
        let u3 = update(&mut session, src1, 3);
        assert!(u3
            .delta
            .iter()
            .any(|e| e.kind == SessionEventKind::ExerciseOpened));

        // 注释变化（命令内容不变，仍是 open 状态）→ 零重编译。
        let src4 = "-- 只是加了一行讲解\nexample : Prop -> Prop := ???\n";
        let u4 = update(&mut session, src4, 4);
        assert_eq!(u4.recompiled_from, None);
        assert!(u4.delta.is_empty());
        assert_eq!(u4.version, 4);
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
        let _ = update(&mut session, "example : Prop -> Prop := ???\n", 3);
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
}
