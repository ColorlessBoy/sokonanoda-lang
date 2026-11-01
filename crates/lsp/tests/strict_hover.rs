//! **P2-12 S5 ③**：strict 报错处的 **hover 不许丢** —— **用户动作级**判据（设计稿 §Q4.2）。
//!
//! ## 判据是什么（为什么不是"字段存在"）
//!
//! 两个用例拧的都是**用户在编辑器里做的动作**：起一个**真的** `sokonanoda-lsp` 子进程
//! （`common::Client`），打开一份**被判红**的文件，等 `publishDiagnostics` 落地，然后把鼠标
//! **放到报错那一行的具体光标位置**上，读 `textDocument/hover` 的答复。
//!
//! 它**不是**"`report.hovers` 非空"那种断言 —— 那种在"行都落在别的声明上"时也绿 ✗；这里问的是
//! **wire 请求在哪个位置**答了什么。
//!
//! ## 两个用例，一个都不能少
//!
//! | 用例 | 夹具 | 钉住的是什么 |
//! |---|---|---|
//! | `hover_at_the_error_site_of_a_rejected_open_exercise_is_never_null` | **c1**（设计稿 §Q1 逐字：`theorem t1 : g 0 := sorry`） | 实施单 §2③ 的字面判据：在 `g` 与 `0` 上**都要拿到**非空 hover |
//! | `hover_at_a_failed_value_shows_the_argument_type` | c1 的**值位**版本（`def bad : Prop := g 0`） | **本刀 `finish` 的牙**：`0` 上必须是 `0 : Nat`（改之前那里只有"未通过，见诊断"的声明卡片） |
//!
//! ### 为什么 c1 那一条只能钉"非空"
//!
//! 实测（`reports/2026-11-01-p212-s5-readings.md` §3）：c1 的失败发生在**开放练习的签名**里，
//! 而 `walk.rs` 的开放练习臂在 `open_signature` 回 `Err` 时**只 push 错误、不 push hover 行**
//! （`open_signature` 把行收在**自己的局部** `Vec` 里，`Err` 一并丢掉）⇒ 那两行**到不了**
//! `report.hovers`，用户看到的是声明卡片「未通过，见诊断」。那是**既有**的另一处缺口
//! （不是本刀引入的），修它要改 `walk.rs`（**2887 行 = `docs-lint` ⑨ 的基线**，一行都不许涨）
//! ⇒ 留给下一刀；本刀**如实记录**，不把"声明卡片"伪装成"类型 hover" ✓。
//!
//! ### 为什么值位版本是**真的牙**
//!
//! 值位失败走 `walk.rs` 的 G-108 错误臂：那里**会**把本趟收的 `hovers` 交给报告。于是：
//!
//! * `finish` 在报错出口 `hovers.truncate(mark)`（S3 的形状）⇒ `0` 的行被撤掉 ⇒ 用户只拿到
//!   「未通过，见诊断」的声明卡片；
//! * 本刀改成**报错出口不撤** ⇒ `0 : Nat`。
//!
//! 反向验证（把 `finish` 那一行改回去 ⇒ 本用例判红）见读数 §3。
//!
//! ## 为什么是**集成**测试（真子进程）
//!
//! 方言走**编辑器设置**（`initializationOptions.dialect = strict`）—— 那才是用户改的那一处。
//! 真 server 进程 + 真 `textDocument/hover` 才能断言**用户动作级**的结果：hover 行从 `finish` 出发，
//! 经 `decl_info` 映射进 wire，再由扩展渲染 —— 字段漏映射这类 bug 只活在**接缝**上，单元测试看不见。

mod common;

use common::Client;
use serde_json::json;
use std::path::{Path, PathBuf};

/// c1（设计稿 §Q1 的漏名用例，逐字）。
const C1: &str = "\
axiom g : {A : Type} -> Nat -> Prop
theorem t1 : g 0 := sorry
";

/// c1 的**值位**版本：同一个失败的应用，但这次它在 `def` 的**值**里（不是开放练习的签名里）
/// ⇒ 走 `walk.rs` 的 G-108 错误臂（那条**会**把 hover 行交给报告）。
const DEF: &str = "\
axiom g : {A : Type} -> Nat -> Prop
def bad : Prop := g 0
";

/// 一条 JSON-RPC 版本号（与 `common` 里的写法一致）。
const RPC: &str = "2.0";

/// 每个用例一份独立目录与缓存（cargo 的测试是同进程多线程跑的，共用路径会互相覆盖）。
fn fixture(tag: &str, text: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("soko-s5-hover-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture dir");
    let cache = dir.join("cache");
    std::fs::create_dir_all(&cache).expect("cache dir");
    let file = dir.join("Probe.sokonanoda");
    std::fs::write(&file, text).expect("write fixture");
    (dir, cache, file)
}

/// `anchor` **之后**第一处 `needle` 的 **0-based** LSP 位置（`character` 按 **UTF-16 码元**）。
///
/// 锚点是为了打中**用户会点的那个**出现点（c1 里 `g` 出现两次：`axiom g :` 与 `g 0` ✗）。
fn pos_after(text: &str, anchor: &str, needle: &str) -> (u32, u32) {
    let at = text
        .find(anchor)
        .unwrap_or_else(|| panic!("锚点 {anchor:?}"));
    let offset = at
        + text[at..]
            .find(needle)
            .unwrap_or_else(|| panic!("锚点之后的 {needle:?}"));
    let before = &text[..offset];
    let line = before.matches('\n').count() as u32;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = text[line_start..offset].encode_utf16().count() as u32;
    (line, character)
}

/// 起一个会话：`initialize` 用**编辑器设置**（`initializationOptions.dialect = strict`）——
/// 那才是用户改的那一处；`SOKO_STRICT_AT` 恒清空（它**高于**设置档，开发机设过就会把这一档顶掉 ✗）。
fn start(cache: &Path, extra_env: &[(&str, &str)]) -> Client {
    let mut env: Vec<(&str, &str)> = vec![("SOKO_STRICT_AT", "")];
    env.extend_from_slice(extra_env);
    let mut client = Client::start_traced_with_env(cache, &env);
    client.send(json!({
        "jsonrpc": RPC,
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "capabilities": {},
            "initializationOptions": {"dialect": "strict"},
        },
    }));
    client.wait_for(|message| message.get("id") == Some(&json!(1)));
    client.send(json!({"jsonrpc": RPC, "method": "initialized", "params": {}}));
    client
}

/// 在 `(line, character)` 上做一次**用户动作** `textDocument/hover`，返回非空的 markdown 正文。
fn hover_at(client: &mut Client, id: i64, uri: &str, pos: (u32, u32), what: &str) -> String {
    let answer = client.request(
        id,
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": pos.0, "character": pos.1},
        }),
    );
    let result = &answer["result"];
    assert!(
        !result.is_null(),
        "在 `{what}` 上（{pos:?}）hover 必须拿到结果，实测 `null` —— \
         报错处的 hover 丢了 ✗（设计稿 §Q4.2）：{answer}"
    );
    let value = result["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("`{what}` 的 hover 必须是 markdown 正文：{answer}"));
    assert!(
        !value.trim().is_empty(),
        "`{what}` 的 hover 正文是空的 —— 用户看到的是个空框 ✗：{answer}"
    );
    value.to_string()
}

/// 读一份文档发布出来的诊断（`publishDiagnostics`）。
fn published(client: &mut Client, uri: &str) -> Vec<serde_json::Value> {
    let text = client.diagnostics_for(uri);
    serde_json::from_str(&text).expect("diagnostics JSON")
}

/// 打开 + 断言"这一份真的被判红了，且是**内核路径**的文字码"。
///
/// 后半句是判据的**前提**：若看到 `elab-unknown-identifier`，说明两个过渡开关没在这个 server
/// 进程里生效 —— 那测的就不是 S5 那一档（判据会静默退化 ✗）。
fn open_rejected(client: &mut Client, uri: &str, text: &str) -> Vec<serde_json::Value> {
    client.did_open(uri, text);
    let diagnostics = published(client, uri);
    assert_eq!(
        diagnostics.len(),
        1,
        "夹具必须恰好报一条错（否则\"报错处\"不成立）：{diagnostics:?}"
    );
    assert_eq!(
        diagnostics[0]["code"], "elab-implicit-argument-unsolved",
        "这一条必须是**内核路径**的文字码 —— 若看到 `elab-unknown-identifier`，说明两个过渡开关\
         没在这个 server 进程里生效：{diagnostics:?}"
    );
    diagnostics
}

/// **实施单 §2③ 的字面判据**：c1（开放练习，类型位失败）上，`g` 与 `0` 两处 hover **都要拿到**。
///
/// ⚠ 如实：这两处在**本刀之前**也非空 —— 拿到的是声明卡片「未通过，见诊断」。本用例钉的是
/// **"用户在那个位置上一定有反馈"**（防漂移：不许哪天退化成 `null`），**不是**类型 hover；
/// 类型 hover 的那把牙在下一个用例（值位版本）里。
#[test]
fn hover_at_the_error_site_of_a_rejected_open_exercise_is_never_null() {
    let (_dir, cache, file) = fixture("c1", C1);
    let uri = Client::file_uri(&file);
    // **P2-12 S6 起 strict 档默认就是内核路径 + 不回落**（环境变量闸门已删 ⇒ 不必再注入）。
    let mut client = start(&cache, &[]);
    open_rejected(&mut client, &uri, C1);

    // 动作 1：鼠标放到报错应用的**头** `g` 上。
    let (line, col) = pos_after(C1, "theorem t1 :", "g");
    let on_g = hover_at(&mut client, 10, &uri, (line, col), "g");
    assert!(
        on_g.contains("t1"),
        "`g` 上的 hover 必须说的是**这条声明**（用户得知道哪一条没过）：{on_g}"
    );

    // 动作 2：鼠标放到同一行的**实参** `0` 上。
    let (line0, col0) = pos_after(C1, "theorem t1 : g", "0");
    let on_zero = hover_at(&mut client, 11, &uri, (line0, col0), "0");
    assert!(
        on_zero.contains("t1"),
        "`0` 上的 hover 必须说的是**这条声明**：{on_zero}"
    );
}

/// **本刀 `finish` 的牙**：同一个失败应用放在 `def` 的**值**里 ⇒ 报错处必须能 hover 出**类型**。
///
/// * `g` 上：`axiom g : ∀ {A : Type}, Nat → Prop`（声明卡片 —— 名字解析得出）；
/// * `0` 上：**`0 : Nat`**（内核路径第一趟真的把这位于 elaborate 了，报错出口**没有**再撤）。
///
/// 改之前（`finish` 里无条件 `hovers.truncate(mark)`）`0` 上只剩「未通过，见诊断」⇒ 判红。
#[test]
fn hover_at_a_failed_value_shows_the_argument_type() {
    let (_dir, cache, file) = fixture("def", DEF);
    let uri = Client::file_uri(&file);
    let mut client = start(&cache, &[]);
    open_rejected(&mut client, &uri, DEF);

    // `g`：名字上 ⇒ 声明卡片（与"报错不报错"无关，那是 G-108 的词法行/解析）。
    let (line, col) = pos_after(DEF, "def bad : Prop :=", "g");
    let on_g = hover_at(&mut client, 10, &uri, (line, col), "g");
    assert!(
        on_g.contains("axiom g") || on_g.contains("g :"),
        "`g` 上的 hover 必须给出它的签名：{on_g}"
    );

    // `0`：**报错前已经 elaborate 过的实参** ⇒ 必须有类型行（本刀那一行改动的唯一用户可见效果）。
    let (line0, col0) = pos_after(DEF, "def bad : Prop := g", "0");
    let on_zero = hover_at(&mut client, 11, &uri, (line0, col0), "0");
    assert!(
        on_zero.contains("0 : Nat"),
        "`0` 上的 hover 必须是 `0 : Nat` —— 若只看到「未通过，见诊断」，说明报错出口又把这一趟的\
         hover 行撤掉了（`kernel_app_fallback::finish` 的 `hovers.truncate(mark)`）：{on_zero}"
    );
}
