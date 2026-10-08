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
    /// **原样的** trace 行（同样的后台线程顺手攒下来）。
    ///
    /// 为什么要留下正文而不只是计数：那一行自带**差量结构计数**
    /// （`modules=` / `by=` / `infer=<未命中>/<调用>` / `prefix=`），
    /// 而"判据不许用绝对毫秒"（`AGENTS.md`）⇒ 判据要读的正是这些字段 ✓。
    /// 进程级计数器在**单元测试**里会串味（140+ 用例并行）；这里是**集成测试**、
    /// 服务端还是**独立进程** ⇒ 这些差量天然隔离 ✓。
    traces: Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>,
    /// **A5**：`LSP_TRACE warm-library …` 行（产物命中后的后台库层预热）。
    /// 与 `compile` 行**分开收**：`compile_count()`/`last_trace()` 那套判据读的是
    /// "编译了几次"，预热不是编译 ⇒ 不许把它的行混进去 ✗。
    warm_traces: Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>,
}

impl Client {
    pub fn start(cache: &Path) -> Self {
        Self::spawn(cache, false, &[])
    }

    /// 同 [`Self::start`]，但让服务端带 `SOKO_LSP_TRACE=1` 并把 stderr 收进
    /// 计数器（[`Self::compile_count`]）。
    pub fn start_traced(cache: &Path) -> Self {
        Self::spawn(cache, true, &[])
    }

    /// 同 [`Self::start_traced`]，外加**环境变量**（A5 的反向验证要
    /// `SOKO_NO_LIB_WARMUP=1` ⇒ 需要一条能注入 env 的入口 ✓）。
    pub fn start_traced_with_env(cache: &Path, env: &[(&str, &str)]) -> Self {
        Self::spawn(cache, true, env)
    }

    fn spawn(cache: &Path, trace: bool, extra_env: &[(&str, &str)]) -> Self {
        // **可换被子进程**（`SOKO_TEST_LSP_BIN`）：默认仍是 cargo 交给本测试的那个
        // debug 构建 ✓（既有用例**一个字节都不变** ✓）；设了就用它 —— 用途是
        // **北极星墙钟探针**（`perf_keystroke_wallclock.rs`）：debug 与 release 的
        // 绝对毫秒差好几倍，跨机/跨构建的读数只有**同一档构建**才可比
        // （`AGENTS.md` 的探针纪律：读数自带构建身份 ✓）。
        let bin = std::env::var("SOKO_TEST_LSP_BIN")
            .unwrap_or_else(|_| env!("CARGO_BIN_EXE_sokonanoda-lsp").to_string());
        let mut command = Command::new(bin);
        command
            .env("SOKONANODA_CACHE_DIR", cache)
            // ⚠ **必须在这里设**：服务端只有看到 `SOKO_LSP_TRACE` 才会打那一行 ✗。
            // 以前 `start_traced` 只**管道 stderr**、没设开关 ⇒ 除非调用方碰巧在父进程
            // 里设过，`compile_count()` **恒为 0** ⇒ 读它的判据**空转** ✗
            // （「咬不住的守卫等于没有」——2026-10-03 实测：等 trace 行等到超时 ✓）。
            .env("SOKO_LSP_TRACE", if trace { "1" } else { "0" })
            .env_remove("SOKONANODA_NO_CACHE")
            .env_remove("SOKONANODA_LSP_BIN");
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let mut child = command
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
        let mut warm_traces = None;
        let compiled = if trace {
            let stderr = child.stderr.take().expect("stderr");
            let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let sink = std::sync::Arc::clone(&count);
            let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
            let into = std::sync::Arc::clone(&lines);
            let warm = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
            let warm_into = std::sync::Arc::clone(&warm);
            warm_traces = Some(warm);
            std::thread::spawn(move || {
                use std::io::BufRead;
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    // ⚠ 先判**两个**前缀、再 move（`line` 只能被搬走一次 ✗）。
                    let is_warm = line.starts_with("LSP_TRACE warm-library");
                    let is_compile = line.starts_with("LSP_TRACE compile");
                    if is_warm {
                        if let Ok(mut guard) = warm_into.lock() {
                            guard.push(line);
                        }
                    } else if is_compile {
                        sink.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if let Ok(mut guard) = into.lock() {
                            guard.push(line);
                        }
                    }
                }
            });
            Some((count, lines))
        } else {
            None
        };
        let (compiled, traces) = match compiled {
            Some((count, lines)) => (Some(count), Some(lines)),
            None => (None, None),
        };
        Self {
            child,
            reader,
            compiled,
            traces,
            warm_traces,
        }
    }

    /// **A5**：已经收到几行 `LSP_TRACE warm-library`。
    pub fn warm_trace_len(&self) -> usize {
        self.warm_traces
            .as_ref()
            .and_then(|lines| lines.lock().ok())
            .map(|guard| guard.len())
            .unwrap_or(0)
    }

    /// **A5**：等 `LSP_TRACE warm-library` 行数 **> `after`**（返回新的行数）。
    ///
    /// 为什么必须等：stderr 由**另一个线程**读 ⇒ 诊断到了不等于预热那行已经收进
    /// `Vec`（同 [`Self::wait_for_trace_after`] 的教训 ✓）。
    pub fn wait_for_warm_after(&self, after: usize) -> usize {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            let len = self.warm_trace_len();
            if len > after {
                return len;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "等 warm-library 行超时（已有 {len} 行，在等第 {} 行）",
                after + 1
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// **A5**：最近一行 `LSP_TRACE warm-library`。
    pub fn last_warm_trace(&self) -> String {
        self.warm_traces
            .as_ref()
            .and_then(|lines| lines.lock().ok())
            .and_then(|guard| guard.last().cloned())
            .expect("last_warm_trace 需要 start_traced，且必须已经预热过至少一次")
    }

    /// 到目前为止服务端**真的编译了几次**（需要 `start_traced`）。
    pub fn compile_count(&self) -> usize {
        self.compiled
            .as_ref()
            .map(|count| count.load(std::sync::atomic::Ordering::Relaxed))
            .expect("compile_count 需要 start_traced")
    }

    /// 最近一次编译的 trace 行（`LSP_TRACE compile …`，需要 `start_traced`）。
    ///
    /// 自带差量结构计数：`modules=` / `by=` / `infer=<未命中>/<调用>` / `prefix=`。
    pub fn last_trace(&self) -> String {
        self.traces
            .as_ref()
            .and_then(|lines| lines.lock().ok())
            .and_then(|guard| guard.last().cloned())
            .expect("last_trace 需要 start_traced，且必须已经编译过至少一次")
    }

    /// 已经收到几行 trace。
    pub fn trace_len(&self) -> usize {
        self.traces
            .as_ref()
            .and_then(|lines| lines.lock().ok())
            .map(|guard| guard.len())
            .unwrap_or(0)
    }

    /// 等编译计数**落定**（连续 `quiet` 没有新行）并返回它 —— 取"基线"要用这个 ✓。
    ///
    /// **为什么不能直接读 `trace_len()` 当基线** ✗✓（2026-10-08 **CI 实测**）：
    /// `LSP_TRACE` 走 **stderr**、由**另一个线程**读 ⇒ `open()`/`did_change()`
    /// 返回（= 那一版诊断已经到了，走的是 **stdout**）**不等于**那一行已经进了
    /// `Vec` ✗ —— 两条管道之间**没有顺序保证** ✓。CI 上 `changing_a_statement_…`
    /// 就是这样判红的：`left: 2, right: 1`（`before` 读到 **0** —— 开档那一趟的行
    /// 还在读线程手里 ✗），而本机 **6/6 不复现**：本机 `courses/` 下**有**模块根
    /// 产物 ⇒ 开档走产物命中、**根本不编**、没有那一行 ✗ —— 同一个夹具在不同机器上
    /// 走了**两条不同的路** ⇒ 读数不可比 ✗。
    /// ⇒ 基线一律取"落定值"：等计数在 `quiet` 窗口里不再增长 ✓。
    pub fn settled_compile_count(&self) -> usize {
        self.settle_compile_count(500)
    }

    fn settle_compile_count(&self, quiet_ms: u64) -> usize {
        let quiet = std::time::Duration::from_millis(quiet_ms);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        let mut last = self.compile_count();
        let mut stable_since = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let now = self.compile_count();
            if now != last {
                last = now;
                stable_since = std::time::Instant::now();
            } else if stable_since.elapsed() >= quiet {
                return last;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "等编译计数落定超时（已有 {last} 行，静默窗口 {quiet_ms}ms）"
            );
        }
    }

    /// 等到 trace 行数 **> `after`**（返回新的行数）。
    ///
    /// **为什么必须等**：stderr 是**另一个线程**在读 ✗ —— 诊断到了不等于那一行已经
    /// 被收进 `Vec` ✓。直接读会偶发地拿到上一行（实测：按键后立刻读 ⇒ 差量 0 ✗）。
    pub fn wait_for_trace_after(&self, after: usize) -> usize {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            let len = self.trace_len();
            if len > after {
                return len;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "等 trace 行超时（已有 {len} 行，在等第 {} 行）",
                after + 1
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// 从 trace 行里取一个 `key=<数字>` 字段（取不到就 panic —— 判据不许静默退化 ✗）。
    pub fn trace_field(line: &str, key: &str) -> u64 {
        let needle = format!("{key}=");
        let at = line
            .find(&needle)
            .unwrap_or_else(|| panic!("trace 行里没有 `{key}=`：{line}"));
        let rest = &line[at + needle.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits
            .parse()
            .unwrap_or_else(|_| panic!("`{key}=` 后面不是数字：{line}"))
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

    /// **一次按键**：`didChange`（全量文本）+ 等这份文档的诊断落地。
    ///
    /// 返回值就是"**用户看到 solved 之前**"那一份诊断正文 —— 与用户 2026-10-03 的
    /// 验收口径（「输入正确答案后，VSCode 显示 solved、并且对应的 problems 消失的
    /// 时间」）同一条链 ✓：这个调用**返回**即那一刻 ✓。
    pub fn did_change(&mut self, uri: &str, version: i64, text: &str) -> String {
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": version},
                "contentChanges": [{"text": text}],
            },
        }));
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
