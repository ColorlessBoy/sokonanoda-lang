//! LSP 侧的项目缓存端到端（T-A11 / T-A12）。
//!
//! **为什么是集成测试而不是单元测试**：`crates/lsp/src/lib.rs` 里到处是
//! `cfg!(test)` 守卫（单元测试并行跑，共享真实缓存目录会互相污染）。
//! 集成测试链接的是**不带 `cfg(test)` 编译的库** ⇒ 缓存是活的，而且这里
//! 每次都把 `SOKONANODA_CACHE_DIR` 指到自己的临时目录，互不干扰。
//!
//! 跑的是**真的 `sokonanoda-lsp` 进程**（`env!("CARGO_BIN_EXE_sokonanoda-lsp")`），
//! 因为要验证的正是"跨进程复用缓存"——同进程内测不到。

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n\
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x\n\
infix:50 \" ⊆ \" => Set.subset\n";

const ENTRY: &str = "\
import SetLib\n\n\
theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n\n\
theorem open_one (α : Type) (A B : Set α) : A ⊆ B := by\n  sorry\n\n\
-- 点名引用**依赖里**的声明：跨文件 definition 用它（T-A13）。\n\
theorem uses_lib (α : Type) (a : α) (A : Set α) : Set.mem α a A -> Set.mem α a A :=\n\
  fun (h : Set.mem α a A) => h\n";

struct Fixture {
    dir: PathBuf,
    cache: PathBuf,
    entry: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("soko-lsp-cache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("SetLib.sokonanoda"), LIB).expect("write lib");
        let entry = dir.join("Canvas.sokonanoda");
        std::fs::write(&entry, ENTRY).expect("write entry");
        Self {
            cache: dir.join("cache"),
            dir,
            entry,
        }
    }

    fn uri(&self) -> String {
        format!("file://{}", self.entry.display())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 一个最小的 stdio LSP 客户端：只会 `initialize` + `didOpen` + 等诊断。
struct Client {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl Client {
    fn start(cache: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda-lsp"))
            .env("SOKONANODA_CACHE_DIR", cache)
            .env_remove("SOKONANODA_NO_CACHE")
            .env_remove("SOKONANODA_LSP_BIN")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // stderr 丢掉：管道写满会把这个进程堵死。
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sokonanoda-lsp");
        let reader = BufReader::new(child.stdout.take().expect("stdout"));
        Self { child, reader }
    }

    fn send(&mut self, message: serde_json::Value) {
        let body = serde_json::to_vec(&message).expect("serialize");
        let stdin = self.child.stdin.as_mut().expect("stdin");
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
        stdin.write_all(&body).expect("write body");
        stdin.flush().expect("flush");
    }

    fn next_message(&mut self) -> serde_json::Value {
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

    fn wait_for<F: Fn(&serde_json::Value) -> bool>(&mut self, predicate: F) -> serde_json::Value {
        for _ in 0..200 {
            let message = self.next_message();
            if predicate(&message) {
                return message;
            }
        }
        panic!("等不到期望的消息");
    }

    fn initialize(&mut self, fixture: &Fixture) {
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"processId": null, "rootUri": format!("file://{}", fixture.dir.display()),
                       "capabilities": {}},
        }));
        self.wait_for(|message| message.get("id") == Some(&serde_json::json!(1)));
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
    }

    /// `initialize` + `initialized` + `didOpen`，返回诊断正文（JSON 文本，用于逐字节比较）。
    fn open(&mut self, fixture: &Fixture) -> String {
        self.initialize(fixture);
        self.did_open(fixture);
        let published = self.wait_for(|message| {
            message.get("method") == Some(&serde_json::json!("textDocument/publishDiagnostics"))
        });
        serde_json::to_string(&published["params"]["diagnostics"]).expect("serialize diagnostics")
    }

    fn did_open(&mut self, fixture: &Fixture) {
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {
                "uri": fixture.uri(), "languageId": "sokonanoda", "version": 1, "text": ENTRY,
            }},
        }));
    }

    /// 在 `needle` 第一次出现的位置问 `textDocument/definition`，返回目标文件路径。
    fn definition_at(&mut self, fixture: &Fixture, needle: &str) -> Vec<String> {
        let offset = ENTRY.find(needle).expect("needle 必须在入口里");
        let before = &ENTRY[..offset];
        let line = before.matches('\n').count();
        let character = before.rsplit('\n').next().map(str::len).unwrap_or(0);
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "id": 7, "method": "textDocument/definition",
            "params": {"textDocument": {"uri": fixture.uri()},
                       "position": {"line": line, "character": character}},
        }));
        let answer = self.wait_for(|message| message.get("id") == Some(&serde_json::json!(7)));
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

fn cache_entries(cache: &Path) -> usize {
    std::fs::read_dir(cache.join("compiled"))
        .map(|dir| dir.filter_map(Result::ok).count())
        .unwrap_or(0)
}

/// **T-A11**：一次 LSP 打开之后，条目必须落在缓存里。
///
/// 不写的话，只有"用户先跑过 CLI `build`"才享受得到 T-A10 的命中——
/// 第一次打开仍然白编，而且那份成果没人存。
#[test]
fn opening_a_project_document_writes_a_cache_entry() {
    let fixture = Fixture::new("write");
    assert_eq!(cache_entries(&fixture.cache), 0, "夹具前提：缓存是空的");

    let mut client = Client::start(&fixture.cache);
    let diagnostics = client.open(&fixture);
    assert!(
        diagnostics.contains("sorry"),
        "夹具前提：诊断里应当有那条 sorry：{diagnostics}"
    );

    assert!(
        cache_entries(&fixture.cache) >= 1,
        "LSP 打开之后必须写出项目缓存条目（T-A11），实际 {}",
        cache_entries(&fixture.cache)
    );
}

/// **T-A12**：两个**独立**的 LSP 进程打开同一份文档，诊断**逐字节一致**。
///
/// 第二个进程会命中第一个写下的条目 ⇒ 这条同时钉住"回放出来的诊断与真编译
/// 的一模一样"。诊断是用户直接看到的东西，也是 agent 判卷的通道——回放少一条
/// 或改一个字都是"缓存让你看到另一个世界"。
#[test]
fn a_warm_process_publishes_byte_identical_diagnostics() {
    let fixture = Fixture::new("replay");

    // 冷：全新缓存，真编译，同时把条目写下（T-A11）。
    let cold = {
        let mut client = Client::start(&fixture.cache);
        client.open(&fixture)
    };
    assert!(cache_entries(&fixture.cache) >= 1, "冷跑必须写下条目");

    // 热：另一个进程，同一份缓存。
    let warm = {
        let mut client = Client::start(&fixture.cache);
        client.open(&fixture)
    };

    assert_eq!(
        cold, warm,
        "冷/热两次打开的诊断必须逐字节一致（缓存回放不许改变用户看到的东西）"
    );
    assert!(
        cold.contains("sorry"),
        "夹具前提：诊断里应当有那条 sorry：{cold}"
    );
}

/// **T-A13**：命中缓存之后，**跨文件能力仍然工作**。
///
/// 这条钉的是 T-A03 的"整份报告一起回放"：`textDocument/definition` 读的是
/// `project_modules()`（模块表）。只回放入口报告的话，冷跑能跳、**热跑跳不了**
/// ——"命中缓存的文档能显示、不能跳转"，而用户完全不知道为什么。
#[test]
fn cross_file_definition_still_works_after_a_cache_hit() {
    let fixture = Fixture::new("definition");

    // 冷：全新缓存，真编译，写下条目。
    let cold = {
        let mut client = Client::start(&fixture.cache);
        let diagnostics = client.open(&fixture);
        assert!(
            !diagnostics.contains("elab-unknown"),
            "夹具前提：这份入口必须编译得干净：{diagnostics}"
        );
        client.definition_at(&fixture, "Set.mem α a A ->")
    };
    assert_eq!(
        cold.len(),
        1,
        "冷跑的 definition 必须命中一个位置：{cold:?}"
    );
    assert!(
        cold[0].ends_with("SetLib.sokonanoda"),
        "definition 必须跳到**依赖文件**：{cold:?}"
    );

    // 热：另一个进程，同一份缓存。
    let warm = {
        let mut client = Client::start(&fixture.cache);
        client.open(&fixture);
        client.definition_at(&fixture, "Set.mem α a A ->")
    };
    assert_eq!(
        cold, warm,
        "命中缓存之后 definition 必须给出**同一个**答案（T-A03 的整份报告回放）"
    );
}
