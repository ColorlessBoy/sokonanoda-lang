//! `crates/lsp/tests/` 下各集成测试共享的**最小 stdio LSP 客户端**。
//!
//! 为什么是集成测试：`crates/lsp/src/lib.rs` 里到处是 `cfg!(test)` 守卫
//! （单元测试并行跑，共享真实缓存目录会互相污染）。集成测试链接的是**不带
//! `cfg(test)` 编译的库**，跑的是真的 `sokonanoda-lsp` 进程。
//!
//! 每个集成测试文件是**独立 crate**，所以这里只被用到的那部分会被另一个文件
//! 当成死代码 —— 统一 `allow(dead_code)`，别让 `warnings = "deny"` 把测试
//! 编译变成"用不到就不许写"。
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct Client {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
    /// 服务端 stderr 里 `LSP_TRACE compile …` 的行数（T-A30 的"合并"判据）。
    ///
    /// 服务端带 `SOKO_LSP_TRACE=1` 起，每次编译打一行；这里用一个后台线程数它。
    /// **为什么用行数而不是墙钟**：墙钟受负载影响，而"连打 5 个键跑了几次编译"
    /// 是**结构**量——重编的次数不该随击键次数线性增长。
    compiled: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
}

impl Client {
    pub fn start(cache: &Path) -> Self {
        Self::spawn(cache, false)
    }

    /// 同 [`Self::start`]，但让服务端带 `SOKO_LSP_TRACE=1` 并把 stderr 收进
    /// 计数器（[`Self::compile_count`]）。
    pub fn start_traced(cache: &Path) -> Self {
        Self::spawn(cache, true)
    }

    fn spawn(cache: &Path, trace: bool) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda-lsp"))
            .env("SOKONANODA_CACHE_DIR", cache)
            .env_remove("SOKONANODA_NO_CACHE")
            .env_remove("SOKONANODA_LSP_BIN")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // stderr 要有人读：管道写满会把这个进程堵死（trace 模式由一个
            // 后台线程读它并计数；其余情况直接丢掉）。
            .stderr(
                if trace || std::env::var_os("SOKO_LSP_TEST_STDERR").is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                },
            )
            .spawn()
            .expect("spawn sokonanoda-lsp");
        let reader = BufReader::new(child.stdout.take().expect("stdout"));
        let compiled = if trace {
            let stderr = child.stderr.take().expect("stderr");
            let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let sink = std::sync::Arc::clone(&count);
            std::thread::spawn(move || {
                use std::io::BufRead;
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    if line.starts_with("LSP_TRACE compile") {
                        sink.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });
            Some(count)
        } else {
            None
        };
        Self {
            child,
            reader,
            compiled,
        }
    }

    /// 到目前为止服务端**真的编译了几次**（需要 `start_traced`）。
    pub fn compile_count(&self) -> usize {
        self.compiled
            .as_ref()
            .map(|count| count.load(std::sync::atomic::Ordering::Relaxed))
            .expect("compile_count 需要 start_traced")
    }

    pub fn send(&mut self, message: serde_json::Value) {
        let body = serde_json::to_vec(&message).expect("serialize");
        let stdin = self.child.stdin.as_mut().expect("stdin");
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
        stdin.write_all(&body).expect("write body");
        stdin.flush().expect("flush");
    }

    pub fn next_message(&mut self) -> serde_json::Value {
        let mut length = 0usize;
        loop {
            let mut line = String::new();
            let read = self.reader.read_line(&mut line).expect("read header");
            assert!(read > 0, "LSP 在回答之前退出了");
            if let Some(rest) = line.strip_prefix("Content-Length:") {
                length = rest.trim().parse().expect("content length");
            }
            if line == "\r\n" {
                break;
            }
        }
        let mut body = vec![0u8; length];
        self.reader.read_exact(&mut body).expect("read body");
        serde_json::from_slice(&body).expect("parse message")
    }

    pub fn wait_for<F: Fn(&serde_json::Value) -> bool>(
        &mut self,
        predicate: F,
    ) -> serde_json::Value {
        for _ in 0..200 {
            let message = self.next_message();
            if predicate(&message) {
                return message;
            }
        }
        panic!("等不到期望的消息");
    }

    // **不要手拼 `file://{}`**（审计 #22，2026-09-25 ✓）：手拼出来的可能带 `..`，
    // 而服务端发的是 `Url::from_file_path` 规范化后的字符串 ✗ ⇒ 两边**字符串不同** ✓
    // ⇒ 谁也认不出谁（这个形状**已经咬过人** ✓：`perf_course.rs` 的注释里记着它 ✓，
    // 而 2026-09-25 的 ubuntu e2e flake 是同族的 JS 版本 ✗）。
    // 这里用 `canonicalize` 把路径规范化后再拼 ✓（不引新依赖 ✓，与 `from_file_path`
    // 对**已规范化路径**的输出一致 ✓）。若 canonicalize 失败就退回原路径 ✓。
    pub fn file_uri(path: &std::path::Path) -> String {
        let p = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        format!("file://{}", p.display())
    }

    pub fn initialize(&mut self, root: &Path) {
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"processId": null, "rootUri": Self::file_uri(root),
                       "capabilities": {}},
        }));
        self.wait_for(|message| message.get("id") == Some(&serde_json::json!(1)));
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
    }

    pub fn did_open(&mut self, uri: &str, text: &str) {
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": uri, "languageId": "sokonanoda", "version": 1, "text": text,
            }},
        }));
    }

    /// 等某份文档的诊断，返回**诊断正文**的 JSON 文本（用于逐字节比较）。
    pub fn diagnostics_for(&mut self, uri: &str) -> String {
        let published = self.wait_for(|message| {
            message.get("method") == Some(&serde_json::json!("textDocument/publishDiagnostics"))
                && message["params"]["uri"] == serde_json::json!(uri)
        });
        serde_json::to_string(&published["params"]["diagnostics"]).expect("serialize diagnostics")
    }

    /// `initialize` + `initialized` + `didOpen`，返回诊断正文。
    pub fn open(&mut self, root: &Path, uri: &str, text: &str) -> String {
        self.initialize(root);
        self.did_open(uri, text);
        self.diagnostics_for(uri)
    }

    /// 发一个请求并等它的响应（按 `id` 过滤）。
    pub fn request(
        &mut self,
        id: i64,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params,
        }));
        self.wait_for(|message| message.get("id") == Some(&serde_json::json!(id)))
    }

    /// 在 `needle` 第一次出现的位置问 `textDocument/definition`，返回目标文件路径。
    pub fn definition_at(&mut self, uri: &str, text: &str, needle: &str) -> Vec<String> {
        let offset = text.find(needle).expect("needle 必须在入口里");
        let before = &text[..offset];
        let line = before.matches('\n').count();
        let character = before.rsplit('\n').next().map(str::len).unwrap_or(0);
        let answer = self.request(
            7,
            "textDocument/definition",
            serde_json::json!({"textDocument": {"uri": uri},
                               "position": {"line": line, "character": character}}),
        );
        let result = &answer["result"];
        let items = match result {
            serde_json::Value::Array(items) => items.clone(),
            serde_json::Value::Null => vec![],
            other => vec![other.clone()],
        };
        items
            .iter()
            .filter_map(|item| {
                item.get("uri")
                    .and_then(|uri| uri.as_str())
                    .map(str::to_string)
            })
            .collect()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
