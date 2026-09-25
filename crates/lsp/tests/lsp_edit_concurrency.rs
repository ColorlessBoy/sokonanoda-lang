//! 编辑并发：**编译不阻塞消息循环**（T-A30）。
//!
//! 判据来自 clangd 的 threads 设计（`docs/design/lsp-edit-concurrency.md` §1）：
//! 「**methods should not block**」。一次长编译进行中，只读请求
//! （`soko/stateAt`）必须立刻答——用手上那份已完成的报告，而不是等编译跑完。
//!
//! 实测（改前，真进程）：冷编译 unit12 期间第一个 `soko/stateAt` 等了
//! **8907ms**（另一次 unit12-solution 是 **24393ms**）——整个编辑器像死了一样。
//!
//! **为什么必须是集成测试**：进程内的 `LspService` 是 `&mut` 串行调用的，
//! 测试没法在"通知正在被处理"的同时插一个请求进去。只有真的 stdio 客户端
//! 能把 `didOpen` 与 `soko/stateAt` 背靠背发出去、量后者的往返。

mod common;

use common::Client;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 一份**编译得够久**的文档：1200 条声明在 debug 下约 1.2 秒，是判据阈值
/// （100ms）的十倍以上 ⇒ "被挡住"与"没被挡住"在数字上不可能混淆。
///
/// 为什么不用课程里的大文件：集成测试的夹具要自足（不依赖课程内容随轮次变化），
/// 而且这里量的是**机制**，不是某个文件的绝对耗时。
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

fn long_compile_source() -> String {
    let mut text = String::new();
    for i in 0..1200 {
        text.push_str(&format!("theorem t{i:04} (P : Prop) (h : P) : P := h\n"));
    }
    text
}

struct Fixture {
    dir: PathBuf,
    cache: PathBuf,
    entry: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("soko-lsp-conc-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let entry = dir.join("Big.sokonanoda");
        std::fs::write(&entry, long_compile_source()).expect("write entry");
        Self {
            cache: dir.join("cache"),
            dir,
            entry,
        }
    }

    fn uri(&self) -> String {
        file_uri(&self.entry)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// **T-A30 的判据**：长编译进行中，只读请求仍能在 <100ms 内应答。
///
/// 两段断言，缺一不可：
/// 1. `soko/stateAt` 的往返 **< 100ms**（编译**没**挡住消息循环）；
/// 2. 随后诊断**确实**到了（编译**真的**在跑，不是因为压根没编才"快"）。
#[test]
fn read_only_requests_answer_while_a_long_compile_is_running() {
    let fixture = Fixture::new("stateat");
    let uri = fixture.uri();
    let mut client = Client::start(&fixture.cache);
    client.initialize(&fixture.dir);

    // 背靠背：`didOpen` 之后**不等**诊断，立刻问只读请求。
    client.did_open(&uri, &long_compile_source());
    let started = Instant::now();
    let answer = client.request(
        99,
        "soko/stateAt",
        serde_json::json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": 0}}),
    );
    let latency = started.elapsed();

    assert!(
        answer.get("error").is_none(),
        "soko/stateAt 必须答上（哪怕文档还没编完）：{answer:?}"
    );
    assert!(
        latency < Duration::from_millis(100),
        "长编译进行中，soko/stateAt 等了 {latency:?}——编译把消息循环挡住了（T-A30 判据是 <100ms）"
    );

    // 编译是真的在跑（否则上面那条"快"没有意义）：诊断最终必须到。
    let diagnostics = client.diagnostics_for(&uri);
    assert!(
        diagnostics.starts_with('['),
        "夹具前提：编译完成后应当发一份诊断数组，实际 {diagnostics}"
    );
}

/// **连打 5 个键、不等中间结果**：编译必须被**合并**，次数不随击键数线性增长。
///
/// 这是用户 2026-09-21 的原话：「不管怎么样，**多次编辑不应该导致性能变差**」。
/// 机制：同一份文档**同时只有一个**编译任务（`Compiler::inflight`），编辑期间
/// 来的新版本只是把 `pending` 换成最新的那一份，任务下一轮取走——所以 N 次
/// 快速编辑最多跑 2 趟（在飞的那一趟 + 合并后的那一趟），而不是 N 趟。
///
/// 判据用**服务端的编译次数**（stderr 的 `LSP_TRACE compile` 行数），不是墙钟：
/// 墙钟受负载影响，而"跑了几趟"是结构量。
#[test]
fn rapid_edits_coalesce_instead_of_queueing() {
    let fixture = Fixture::new("coalesce");
    let uri = fixture.uri();
    let mut client = Client::start_traced(&fixture.cache);
    client.initialize(&fixture.dir);
    client.did_open(&uri, &long_compile_source());
    let _ = client.diagnostics_for(&uri);

    // 连打 5 个键，**不等**任何中间结果：每次都在上一次的诊断回来之前把新文本发出去。
    let base = long_compile_source();
    let mut version = 1i32;
    for round in 0..5 {
        version += 1;
        let edited = format!("{base}-- 第 {round} 次连打\n");
        client.send(serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didChange",
            "params": {"textDocument": {"uri": uri, "version": version},
                       "contentChanges": [{"text": edited}]},
        }));
    }
    // 等最后那一版的诊断落地（版本 = 最后那次）。
    let published = client.wait_for(|message| {
        message.get("method") == Some(&serde_json::json!("textDocument/publishDiagnostics"))
            && message["params"]["uri"] == serde_json::json!(uri)
            && message["params"]["version"] == serde_json::json!(version)
    });
    assert!(
        published["params"]["diagnostics"].is_array(),
        "最后一版必须发诊断：{published:?}"
    );

    // stderr 的读取线程可能还落后于写入（追踪行在 publish 之前就写了，但读者
    // 要等调度）——给它一点时间把行数读完，否则断言会因为"读得慢"而假通过。
    std::thread::sleep(std::time::Duration::from_millis(300));
    let compiles = client.compile_count();
    assert!(
        compiles <= 3,
        "打开 + 连打 5 个键一共跑了 {compiles} 趟编译——编辑没有被合并（上限：打开 1 趟 + 在飞 1 趟 + 合并后 1 趟）"
    );
}
