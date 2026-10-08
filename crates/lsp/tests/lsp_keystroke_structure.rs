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
//! ## 判据（**两向** ✓）：改**证明** ⇒ `prefix=0` ✓；改**陈述** ⇒ `prefix>0` ✓
//!
//! `prefix` = `JUDGE_PREFIX runs` = `judge_infer` **未命中**后把**整段前缀**合成
//! 一份文件、交 `check_document_with` **从零重跑**的趟数 ✗。它是 O(N²) 的放大源
//! （设计 `docs/design/incremental-environment.md` §1：前缀随声明序号线性变长
//! ⇒ 总字节随 N² 涨）。修法是让判定**就地查当前环境**而不是重跑前缀 ✓ ——
//! **as-built 是 `InplaceEnv` + `infer_type_text_inplace`**（设计 §0.2 #2 / §32；
//! ⚠ 那个 `EnvProvider` trait **至今零接线** ✗，别再按它的名字找实现）。
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

/// **改陈述（= 改定理标识符）⇒ 后面必须重新失效** ✓（**反向验证** ✓）。
///
/// ⚠ **这条期望在 2026-10-04 被翻转** ✓（值守口径更正 ✓）：
/// * 原期望 `prefix == 0` ✗ —— 那是把「**陈述**变了 ⇒ 缓存全 miss」当成**缺陷** ✗；
/// * 现口径 ✓：**改证明** ⇒ 后面**不许**失效（`prefix == 0` ✓，见
///   [`proof_body_keystroke`] 的两条用例 ✓）；**改陈述** ⇒ 引用它的**必须**失效
///   （`prefix > 0` ✓）—— 两个方向都要有判据 ✓（S6 那批的教训：信任边界的漏洞都长成
///   「**文本没变但语义变了**」✗；这里钉的是反方向：「**文本变了但语义没变**」不许失效 ✓）。
///
/// 改名同时改了**接口**（名字是接口的一部分 ✓）⇒ 后面**依赖这个名字**的查询必须重跑 ✓；
/// 本用例只断言**方向**（`> 0` ✓），**不锁具体趟数**（实现会演进 ✓）。
#[test]
fn changing_a_statement_must_invalidate_the_prefixes_after_it() {
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
        "sokonanoda-lsp-keystroke-structure-statement-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&cache);
    // **同一个二进制里的 3 个用例是并行跑的** ✓（libtest 默认）⇒ 各自的缓存目录必须
    // **互不相同** ✗✓：以前三条都用 `…-structure-<pid>`（同一个 pid ⇒ 同一个目录），
    // 每个用例开头 `remove_dir_all` 会把**别人正在用**的缓存删掉 ✗（2026-10-08 发版点
    // 才暴露：CI 上这条判红，本机 6/6 绿 —— 并行度与机器速度都不同）。
    let mut client = Client::start_traced(&cache);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text); // 开档（冷编译；这一步本来就贵，不判它）

    // ⚠ **基线取"落定值"** ✗✓（2026-10-08 CI 实测 `left: 2, right: 1`）：开档那一趟
    // 走冷编还是产物命中**因机器而异**（本机 `courses/` 下有产物 ⇒ 不编 ✗），而
    // `open()` 返回只保证**诊断**（stdout）到了、**不保证** trace 行（stderr，另一个
    // 线程在读）已经进 `Vec` ✗ ⇒ 直接读会把开档那一行算进按键的差量 ✗。
    let before = client.settled_compile_count();
    let _ = client.did_change(&uri, 2, &edited);
    let after = client.settled_compile_count();
    assert_eq!(
        after,
        before + 1,
        "一次按键必须恰好编译一次（多了 = 防抖失效，少了 = 没编）"
    );
    let line = client.last_trace();
    let prefix = Client::trace_field(&line, "prefix");
    let modules = Client::trace_field(&line, "modules");
    println!("PERF keystroke-structure {rel}: modules={modules} prefix={prefix}\n  {line}");
    // **2026-10-08 翻转（A2a 落地之后）**：改陈述**不必**再重跑前缀（`prefix == 0` ✓）
    // —— 就地判定直接答：它的查表键**含前缀文本** ⇒ 陈述一变必然 miss ⇒ 走就地路重算 ✓。
    // ⇒ "不许给旧答案"这条**不变量没丢**，但守卫**换到了答案层**（下面直接问
    // `soko/goals` ✓）。**两条一起**才完整：成本读数（`prefix`）+ 答案读数（新名字）✓
    // —— 只钉 `prefix > 0` 是**钉实现** ✗（A2a 之后恒红，实测这条守卫从 A2a 起一直红 ✗）；
    // 只钉 `prefix == 0` 又漏掉"答案陈旧" ✗。
    assert_eq!(
        prefix, 0,
        "**A2a 之后**：改陈述（改名 = 改接口 ✓）由**就地判定**答，**不再重跑前缀** ✓ \
         （`prefix == 0`）。若这里 > 0 ⇒ 就地路没生效（退回「整份前缀重跑」✗ = 成本回归）。\n  {line}"
    );
    // **答案层判据**（这条守卫的**目的**所在 ✓）：改名之后 `soko/goals` 必须答**新名字** ✓
    // —— 答旧名字（或答不出）= 缓存给了**陈旧答案** ✗，那才是它真正要防的东西 ✓。
    let request_id = 4242;
    client.send(serde_json::json!({
        "jsonrpc": "2.0",
        "method": "soko/goals",
        "id": request_id,
        "params": {"textDocument": {"uri": uri}, "position": null},
    }));
    let answered =
        client.wait_for(|message| message.get("id") == Some(&serde_json::json!(request_id)));
    assert!(
        answered.to_string().contains(&format!("{name}_a")),
        "**答案层**：改名之后 `soko/goals` 必须答**新名字** `{name}_a` ✓ —— \
         答不出/还答旧名字 = 陈旧答案 ✗：{answered:?}"
    );
}

/// 第一条 `theorem` 的**证明体那一行**（缩进**恰好两格**、且不以 `:=` 结尾 ✓）。
///
/// 为什么这么找：课程单元里 `theorem` 的陈述常跨两行（`… :` 换行后 `… :=` ✓），
/// 证明体是**再下一行** ✓；用"恰好两格缩进 + 不是 `:=` 结尾"就能稳定点到它 ✓。
fn first_theorem_body_line(text: &str) -> Option<&str> {
    let mut in_theorem = false;
    for line in text.lines() {
        if line.starts_with("theorem ") {
            in_theorem = true;
            continue;
        }
        if in_theorem
            && line.starts_with("  ")
            && !line.starts_with("   ")
            && !line.trim_end().ends_with(":=")
        {
            return Some(line);
        }
    }
    None
}

/// **只改证明体**（陈述一字不动）⇒ 后面**一条都不该重跑前缀**（`prefix=0`）。
///
/// 用户 2026-10-04 点名的特性：「只改证明，后面不需要重编」✓。`theorem` 的值是证明，
/// **证明不参与 `def_eq`** ⇒ 改它不该让后面任何判定失效 ✓。
///
/// **为什么要两刀**：第一刀是**冷态**（开档后第一次编辑，缓存还空着 ⇒ `prefix` 必然大 ✗），
/// 第二刀才是**被量的那一刀** ✓ —— 判据量的是"**复用**有没有生效" ✓，不是"第一次有多贵" ✓。
///
/// `shift` 决定**等长**还是**不等长**：
/// * `false` —— 等长（总字节不变 ⇒ 后面命令**不平移** ✓）；
/// * `true` —— 不等长（后面命令**整体平移** ✗ ⇒ 同时考"脏集按字节偏移判"那条 ✓）。
fn proof_body_keystroke(shift: bool) {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml（课程仓可分开检出）");
        return;
    };
    let rel = "units/I.3/unit08-images-preimages.sokonanoda";
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("读不到 {}：{error}", path.display()));
    let body = first_theorem_body_line(&text)
        .expect("夹具前提：课程单元里第一条 theorem 必须有证明体那一行")
        .to_string();

    // ⚠ `lines()` 给的是**整行**（含缩进 ✓）⇒ 这里 `body` 就是那一行的原文 ✓。
    let trimmed = body.trim_start().to_string();
    // 两刀都**必须真改**（改回去会命中磁盘缓存 ⇒ 读数把"没生效"看成"很快" ✗）。
    let (warm, mid, measured) = if shift {
        // 不等长：+2 字节（加一对括号 ✓，语义相同 ✓）。
        (
            body.clone(),
            format!("  ({trimmed})"),
            format!("  (({trimmed}))"),
        )
    } else {
        // 等长：缩进与行尾空白对调 ⇒ 总字节数不变 ✓。
        (body.clone(), format!("{trimmed}  "), format!(" {trimmed} "))
    };
    let after_warm = text.replacen(&warm, &mid, 1);
    assert_ne!(after_warm, text, "夹具前提：预热那一刀必须真的改变文本");
    let after_measured = after_warm.replacen(&mid, &measured, 1);
    assert_ne!(
        after_measured, after_warm,
        "夹具前提：被量的那一刀必须真的改变文本"
    );
    if !shift {
        assert_eq!(
            after_measured.len(),
            text.len(),
            "夹具前提：等长那一刀必须让**总字节数不变**（后面命令不平移 ✓）"
        );
    } else {
        assert_eq!(
            after_measured.len(),
            text.len() + 4,
            "夹具前提：不等长那一刀必须让后面命令**整体平移** ✓"
        );
    }

    let cache = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-proof-body-{}-{}",
        std::process::id(),
        if shift { "shift" } else { "same" }
    ));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start_traced(&cache);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text);
    let _ = client.did_change(&uri, 2, &after_warm); // 预热（冷态，不判它）
                                                     // 同 statement 那条：基线取**落定值**（`did_change` 返回 ≠ trace 行已进 `Vec` ✗）。
    let before = client.settled_compile_count();
    let _ = client.did_change(&uri, 3, &after_measured);
    let after = client.settled_compile_count();
    assert_eq!(after, before + 1, "一次按键必须恰好编译一次");
    let line = client.last_trace();
    let prefix = Client::trace_field(&line, "prefix");
    let modules = Client::trace_field(&line, "modules");
    println!(
        "PERF proof-body-{} {rel}: modules={modules} prefix={prefix}\n  {line}",
        if shift { "shift" } else { "same" }
    );
    assert_eq!(
        prefix, 0,
        "**只改证明体**（陈述一字不动）⇒ 后面一条都不该重跑前缀 ✗\n  {line}"
    );
}

/// **等长**改证明体（总字节不变 ⇒ 后面命令不平移）。
#[test]
fn changing_a_proof_body_without_shifting_bytes_must_not_rerun_prefixes() {
    proof_body_keystroke(false);
}

/// **不等长**改证明体（后面命令整体平移 ⇒ 同时考"脏集按偏移判"）。
#[test]
fn changing_a_proof_body_with_shifting_bytes_must_not_rerun_prefixes() {
    proof_body_keystroke(true);
}
