//! `sokonanoda watch`：常驻监控教学画布，产出带版本号的 JSON Lines 事件流。
//!
//! 这是 L1 服务层的 CLI 形态（也是 agent 通道的直连形态）。第一行永远是
//! 服务握手 `service.hello`；之后文件每次变化都会以 `file.didChange` 事件
//! 开场（`file.changed` 是弃用别名，本实现只发射规范名），随后是本轮 delta
//! （exercise.opened/solved/failed、decl.checked/failed）与诊断。
//!
//! 单文档：`sokonanoda watch <file>` 或 `watch --doc <file>`。
//! 工作区：`sokonanoda watch --workspace <root>` 递归监控每个 `*.sokonanoda`
//! （跳过 `target`/`.git`/`node_modules`），每文件一个独立会话与版本号，
//! 事件带稳定的 `file` 字段；跨文件无全序。
//!
//! 控制面：watch 同时按 JSON Lines 从 stdin 读取客户端命令（`ping` /
//! `subscribe` / `unsubscribe`），后台线程读入、主循环非阻塞消费，因此
//! 300ms 轮询不被阻塞；stdin EOF 只断开命令通道，监控继续。

use crate::json_report::{print_json_line, span_json};
use serde_json::Value;
use sokonanoda_front::compile::{CompileError, CompileOptions};
use sokonanoda_front::session::{Session, SessionUpdate};
use sokonanoda_front::Diagnostic;
use std::collections::HashSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// 轮询间隔（事件流是服务形态，客户端按行消费；无需文件系统通知）。
const POLL: Duration = Duration::from_millis(300);

/// 每文件待发送事件的有界缓冲上界。溢出时合并到最新版本并标
/// `recompiled_from: 0`，客户端必须视为全量重同步（docs/protocol.md）。
const PENDING_BOUND: usize = 64;

/// 控制面协议版本（与 `service.hello` 的 `protocol` 字段同源）。
const PROTOCOL: u64 = 1;

/// 服务握手：镜像 LSP 的 `soko/version`，客户端据此判断协议兼容与重启。
/// 必须是 watch 流的第一条 JSON 行。
fn print_handshake() {
    print_json_line(&serde_json::json!({
        "type": "service.hello",
        "protocol": PROTOCOL,
        "engine": env!("CARGO_PKG_VERSION"),
        "pid": std::process::id(),
    }));
}

/// 客户端→服务控制通道：stdin 的 JSON Lines 命令在后台线程读取，经 mpsc
/// 送达主循环，轮询永不阻塞。stdin EOF 只断开通道（`try_recv` 返回
/// `Disconnected`），watch 继续监控。
struct Control {
    commands: Receiver<String>,
    /// `None` = 每个被监控文件都发事件（默认，向后兼容）；`Some(set)` =
    /// 白名单，仅名单内文件发事件。首个 `subscribe`/`unsubscribe` 进入白名单
    /// 模式：`subscribe` 从空集开始，`unsubscribe` 从「当前已知文件」开始。
    filter: Option<HashSet<String>>,
}

impl Control {
    fn spawn() -> Self {
        let (tx, commands) = mpsc::channel();
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                match line {
                    Ok(line) => {
                        if tx.send(line).is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        });
        Self {
            commands,
            filter: None,
        }
    }

    /// 非阻塞地处理当前排队的全部命令。
    fn drain(&mut self, files: &mut [FileWatch]) {
        while let Ok(line) = self.commands.try_recv() {
            self.handle(&line, files);
        }
    }

    /// 该路径当前是否应发事件（默认全部）。
    fn emits(&self, path: &str) -> bool {
        match &self.filter {
            None => true,
            Some(set) => set.contains(path),
        }
    }

    fn handle(&mut self, line: &str, files: &mut [FileWatch]) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let command: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                self.error(format!("invalid JSON command: {error}"));
                return;
            }
        };
        match command.get("type").and_then(Value::as_str) {
            Some("ping") => print_json_line(&serde_json::json!({
                "type": "pong",
                "id": command.get("id").cloned().unwrap_or(Value::Null),
                "protocol": PROTOCOL,
                "engine": env!("CARGO_PKG_VERSION"),
            })),
            Some(ty @ ("subscribe" | "unsubscribe")) => {
                let subscribe = ty == "subscribe";
                match command.get("file").and_then(Value::as_str) {
                    Some(file) => self.set_filter(file, subscribe, files),
                    None => self.error("subscribe/unsubscribe require a string `file`"),
                }
            }
            Some(other) => self.error(format!("unknown command type {other:?}")),
            None => self.error("command is missing a string `type`"),
        }
    }

    fn set_filter(&mut self, file: &str, subscribe: bool, files: &[FileWatch]) {
        let filter = self.filter.get_or_insert_with(|| {
            if subscribe {
                HashSet::new()
            } else {
                files.iter().map(|watch| watch.path.clone()).collect()
            }
        });
        if subscribe {
            filter.insert(file.to_string());
        } else {
            filter.remove(file);
        }
    }

    /// 结构化错误事件：坏命令不崩流，客户端可继续发命令。
    fn error(&self, message: impl Into<String>) {
        print_json_line(&serde_json::json!({
            "type": "error",
            "message": message.into(),
        }));
    }
}

/// 单文档 watch（`watch <file>` 或 `watch --doc <file>`）。
pub(crate) fn watch_doc(path: &str) -> std::process::ExitCode {
    print_handshake();
    let mut control = Control::spawn();
    let mut files = vec![FileWatch::new(path.to_string())];
    loop {
        control.drain(&mut files);
        tick(&mut files, &control);
        std::thread::sleep(POLL);
    }
}

/// 工作区 watch（`watch --workspace <root>`）：递归监控根下每个
/// `*.sokonanoda`。新出现的文件会在后续轮询中被纳入；跨文件无全序。
pub(crate) fn watch_workspace(root: &str) -> std::process::ExitCode {
    print_handshake();
    let mut control = Control::spawn();
    let root = Path::new(root);
    let mut files: Vec<FileWatch> = Vec::new();
    let mut known: HashSet<PathBuf> = HashSet::new();
    loop {
        let mut discovered = Vec::new();
        discover_files(root, &mut discovered);
        for path in discovered {
            if known.insert(path.clone()) {
                files.push(FileWatch::new(path.to_string_lossy().into_owned()));
            }
        }
        control.drain(&mut files);
        tick(&mut files, &control);
        std::thread::sleep(POLL);
    }
}

fn tick(files: &mut [FileWatch], control: &Control) {
    for file in files.iter_mut() {
        file.poll();
    }
    for file in files.iter_mut() {
        let emit = control.emits(&file.path);
        file.flush(emit);
    }
}

/// 递归收集 `*.sokonanoda`，跳过构建/版本控制/依赖目录。
fn discover_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(name.as_ref(), "target" | ".git" | "node_modules") {
                continue;
            }
            discover_files(&path, out);
        } else if path
            .extension()
            .map(|ext| ext == "sokonanoda")
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

/// 一个被监控文件的独立会话：自己的版本号、自己的待发送缓冲。
struct FileWatch {
    path: String,
    session: Session,
    version: u64,
    last_content: Option<String>,
    pending: Vec<Value>,
}

impl FileWatch {
    fn new(path: String) -> Self {
        Self {
            path,
            session: Session::new(CompileOptions::default()),
            version: 0,
            last_content: None,
            pending: Vec::new(),
        }
    }

    fn poll(&mut self) {
        let Ok(src) = std::fs::read_to_string(&self.path) else {
            // 文件可能正在被编辑器原子替换；下一轮再试。
            return;
        };
        if self.last_content.as_deref() == Some(src.as_str()) {
            return;
        }
        self.last_content = Some(src.clone());
        self.version += 1;
        let update = self.session.update(&src, self.version);
        self.enqueue(&update);
    }

    fn enqueue(&mut self, update: &SessionUpdate) {
        let opener = self.opener(update.version, update.recompiled_from);
        self.push(opener);
        for event in &update.delta {
            let event = serde_json::json!({
                "type": event.kind.code(),
                "file": self.path,
                "name": event.name,
                "version": event.version,
            });
            self.push(event);
        }
        if let Some(diag) = &update.parse_error {
            let diag = self.diagnostic(diag, update.version);
            self.push(diag);
        }
        for err in &update.report.errors {
            let diag = self.error_diagnostic(err, update.version);
            self.push(diag);
        }
    }

    fn opener(&self, version: u64, recompiled_from: Option<usize>) -> Value {
        serde_json::json!({
            "type": "file.didChange",
            "file": self.path,
            "version": version,
            "recompiled_from": recompiled_from,
        })
    }

    fn diagnostic(&self, diag: &Diagnostic, version: u64) -> Value {
        serde_json::json!({
            "type": "diagnostic",
            "file": self.path,
            "stage": diag.stage_code(),
            "code": diag.code(),
            "message": diag.message,
            "hint": diag.hint(),
            "span": span_json(diag.span),
            "version": version,
        })
    }

    fn error_diagnostic(&self, err: &CompileError, version: u64) -> Value {
        serde_json::json!({
            "type": "diagnostic",
            "file": self.path,
            "stage": err.stage().code(),
            "code": err.code(),
            "message": err.message,
            "hint": err.hint(),
            "span": span_json(err.span),
            "version": version,
        })
    }

    /// 有界入队：超过上界时合并 —— 丢弃已缓冲的旧事件，只保留最新的
    /// `file.didChange` 并把 `recompiled_from` 改写成 0（全量重同步提示）。
    fn push(&mut self, event: Value) {
        if self.pending.len() >= PENDING_BOUND {
            self.coalesce();
        }
        self.pending.push(event);
    }

    fn coalesce(&mut self) {
        let latest = self
            .pending
            .iter()
            .rposition(|e| e.get("type").and_then(Value::as_str) == Some("file.didChange"));
        match latest {
            Some(pos) => {
                let version = self.pending[pos]
                    .get("version")
                    .cloned()
                    .unwrap_or(Value::from(0));
                self.pending.clear();
                self.pending.push(serde_json::json!({
                    "type": "file.didChange",
                    "file": self.path,
                    "version": version,
                    "recompiled_from": 0,
                }));
            }
            None => self.pending.clear(),
        }
    }

    fn flush(&mut self, emit: bool) {
        if emit {
            for event in self.pending.drain(..) {
                print_json_line(&event);
            }
        } else {
            self.pending.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_coalesces_to_a_resync_opener() {
        let mut file = FileWatch::new("canvas.sokonanoda".to_string());
        file.version = 1;
        let opener = file.opener(1, Some(0));
        file.push(opener);
        for _ in 0..PENDING_BOUND {
            file.push(serde_json::json!({
                "type": "decl.checked",
                "file": "canvas.sokonanoda",
                "version": 1,
            }));
        }
        assert!(
            file.pending.len() < PENDING_BOUND,
            "buffer stays bounded: {}",
            file.pending.len()
        );
        let coalesced = file
            .pending
            .iter()
            .find(|e| e.get("type").and_then(Value::as_str) == Some("file.didChange"))
            .expect("a coalesced opener survives");
        assert_eq!(
            coalesced["recompiled_from"], 0,
            "coalescing marks the stream for full resync: {coalesced}"
        );
    }

    #[test]
    fn within_bound_keeps_the_real_recompiled_from() {
        let mut file = FileWatch::new("canvas.sokonanoda".to_string());
        let opener = file.opener(2, Some(3));
        file.push(opener);
        assert_eq!(file.pending.len(), 1);
        assert_eq!(file.pending[0]["recompiled_from"], 3);
    }

    fn control() -> Control {
        let (_tx, commands) = mpsc::channel();
        Control {
            commands,
            filter: None,
        }
    }

    #[test]
    fn default_filter_emits_every_file() {
        let control = control();
        assert!(control.emits("a.sokonanoda"));
        assert!(control.emits("b.sokonanoda"));
    }

    #[test]
    fn subscribe_narrows_to_an_allowlist() {
        let files = vec![
            FileWatch::new("a.sokonanoda".to_string()),
            FileWatch::new("b.sokonanoda".to_string()),
        ];
        let mut control = control();
        control.set_filter("a.sokonanoda", true, &files);
        assert!(control.emits("a.sokonanoda"));
        assert!(!control.emits("b.sokonanoda"));
        control.set_filter("b.sokonanoda", true, &files);
        assert!(control.emits("a.sokonanoda"));
        assert!(control.emits("b.sokonanoda"));
        control.set_filter("a.sokonanoda", false, &files);
        assert!(!control.emits("a.sokonanoda"));
        assert!(control.emits("b.sokonanoda"));
    }

    #[test]
    fn unsubscribe_before_any_subscribe_keeps_the_other_files() {
        let files = vec![
            FileWatch::new("a.sokonanoda".to_string()),
            FileWatch::new("b.sokonanoda".to_string()),
        ];
        let mut control = control();
        control.set_filter("a.sokonanoda", false, &files);
        assert!(!control.emits("a.sokonanoda"));
        assert!(control.emits("b.sokonanoda"));
    }
}
