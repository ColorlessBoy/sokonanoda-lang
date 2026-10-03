//! **G-29 的验收判据（编辑器里的编辑延迟）—— 结构计数版**。
//!
//! 用户 2026-10-03 22:37 的验收口径（原话）：
//! 「**输入正确答案后，VSCode 显示 solved、并且对应的 problems 消失的时间**」。
//! 它对应"一次按键 → 一次编译 → 一次 `publishDiagnostics`"这条链 ✓
//! （[`common::Client::did_change`] 返回的那一刻就是它 ✓）。
//!
//! ## 判据为什么不是毫秒
//!
//! `AGENTS.md`（2026-09-29，**第三次**同一种病）：共享 runner 上墙钟不可转移
//! —— 同一份代码量到过 **44ms ↔ 2431ms** ⇒ 一律用**结构计数** ✓。
//! 服务端 `SOKO_LSP_TRACE=1` 的 `LSP_TRACE compile` 行自带**差量**结构计数
//! （`modules=` / `by=` / `infer=<未命中>/<调用>` / `prefix=`）✓，本用例读它 ✓。
//!
//! ## 判据：一次按键**不许重跑整份前缀**（`prefix=0`）
//!
//! `prefix` = `JUDGE_PREFIX runs` = `judge_infer` **未命中**后把**整段前缀**合成
//! 一份文件、交 `check_document_with` **从零重跑**的趟数 ✗。它是 O(N²) 的放大源
//! （设计 `docs/design/incremental-environment.md` §2：前缀随声明序号线性变长
//! ⇒ 总字节随 N² 涨）。修法是让 `judge_infer` **就地查当前环境**（`EnvProvider`
//! 就是为此定义的 ✓）而不是重跑前缀 ✓。
//!
//! ⚠ **夹具必须用真课程**：合成夹具（`crates/front/tests/keystroke_structure.rs`）
//! 实测 `infer_miss=0 / prefix_runs=0` ⇒ 它**根本不触发**这条 ✗（那些 `def`
//! 不走 `judge_infer`）。触发它的是**带源级记法**的真课程单元 ✓。
//!
//! ## 为什么是集成测试（不是 `crates/lsp/src/tests/`）
//!
//! 那里的 140+ 用例在**同一个进程**里并行跑，而这些计数器是**进程级**的
//! ⇒ 差量会串味 ✗。这里跑的是**真的 `sokonanoda-lsp` 进程**（`CARGO_BIN_EXE_…` ✓）
//! ⇒ 计数器天然隔离 ✓（同 `lsp_cache.rs` 的理由 ✓）。

mod common;

use common::Client;

/// 真课程根（相对 `crates/lsp/`）。找不到就跳过 —— 课程仓与语言仓可以分开检出
/// （同 `crates/lsp/src/tests/perf_course.rs` 的口径 ✓）。
fn course_root() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    dir.join("sokonanoda.toml").is_file().then_some(dir)
}

/// 文件里**第一条** `theorem` 的标识符（探针 `G29b` 用的同一个形状 ✓）。
fn first_theorem_name(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("theorem "))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|name| !name.is_empty())
}

/// **一次按键不许重跑整份前缀**（`prefix=0`）。
///
/// 编辑落在文件**前面**是关键 ✓：它让**后缀**进脏集，而后缀里那些 `judge_infer`
/// 查询的前缀文本跟着变了 ⇒ 缓存必 miss ⇒ 才走到"重跑整份前缀"那条路 ✗
/// （改**最后**一条时 `prefix=0` ✓ —— 那正是今天的快路 ✓）。
#[test]
fn a_keystroke_must_not_rerun_the_whole_prefix() {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml（课程仓可分开检出）");
        return;
    };
    let rel = "units/I.3/unit08-images-preimages.sokonanoda";
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("读不到 {}：{error}", path.display()));

    // 改**第一条** `theorem` 的标识符，**全部出现处一起改**（后面的引用跟着走 ⇒
    // 文件仍然可编 ✓）。⚠ 不许变成空操作（T-A21 在 `perf_course.rs` 踩过：
    // 名字不在文件里 ⇒ `replace` 无操作 ⇒ 量的其实是"同文本通知" ✗）。
    let name = first_theorem_name(&text).expect("夹具前提：课程单元里必须有 `theorem`");
    let edited = text.replace(&name, &format!("{name}_a"));
    assert_ne!(
        edited, text,
        "夹具前提：这一刀必须真的改变文本（改名 {name}）"
    );

    let cache = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-keystroke-structure-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start_traced(&cache);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text); // 开档（冷编译；这一步本来就贵，不判它）

    let before = client.trace_len();
    let _ = client.did_change(&uri, 2, &edited);
    // stderr 由**另一个线程**读 ⇒ 诊断到了不等于那一行已经收到 ✓（实测踩过 ✗）。
    let after = client.wait_for_trace_after(before);
    assert_eq!(
        after,
        before + 1,
        "一次按键必须恰好编译一次（多了 = 防抖失效，少了 = 没编）"
    );
    let line = client.last_trace();
    let prefix = Client::trace_field(&line, "prefix");
    let modules = Client::trace_field(&line, "modules");
    println!("PERF keystroke-structure {rel}: modules={modules} prefix={prefix}\n  {line}");
    assert_eq!(
        prefix, 0,
        "一次按键**不许重跑整份前缀**（`prefix` = judge_infer 未命中后从零重跑前缀的趟数 ✗）\n  {line}"
    );
}
