//! **`soko/goalAt` 的结构判据（第 102 轮）** —— 真子进程、真 stdio、真课程单元。
//!
//! 背景（设计 `docs/design/persistent-declarations.md` §7.6–7.9 ✓）：`soko/goals` 答的是
//! **整份入口的声明列表**（真 unit08 上 27 条 ⇒ 91 361B ✗），而"光标处的 goal"只要
//! **一条**。把整份列表当"某处的 goal"用，就是那 91KB 的来源 ✗。
//!
//! 判据（**结构计数，不用墙钟** ✓ —— `AGENTS.md`：perf 判据不许用绝对毫秒）：
//! ① **按需**：`soko/goalAt` 的响应字节 ≤ `soko/goals` 的 **1/10** ✓
//!    （实测 3 777B vs 91 361B = **1/24** ✓）；
//! ② **等价**：`goalAt` 答的那一条与 `soko/goals` 里**含同一光标**的那一条
//!    **逐字段相同** ✓（含各自的 `ty_runs`/`value_runs`/`goal_runs`/`goals_runs`
//!    —— 这是"两条入口共用同一个映射实现"的守卫 ✓）；
//! ③ **不降级**：答的那一条**仍带自己的 runs** ✓（硬约束：不许"顺手摘掉 runs" ✗
//!    —— 摘掉就是 Infoview 卡片"目标不高亮"的静默降级，R-2 同形 ✗）。
//!
//! 墙钟只作**读数**打印（不进门槛 ✓）。

mod common;

use common::Client;

const FIXTURE: &str = "courses/set-theory/units/I.3/unit08-images-preimages.sokonanoda";

#[test]
fn goal_at_is_one_decl_and_matches_the_goals_entry_field_by_field() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.join("courses/set-theory");
    let path = root.join("units/I.3/unit08-images-preimages.sokonanoda");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {FIXTURE}");
        return;
    };
    let cache = std::env::temp_dir().join(format!("soko-goal-at-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start(&cache);
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text);

    let mut ask =
        |method: &str, params: serde_json::Value, id: i64| -> (usize, f64, serde_json::Value) {
            let t = std::time::Instant::now();
            client.send(serde_json::json!({
                "jsonrpc": "2.0", "method": method, "id": id, "params": params,
            }));
            let answered = client.wait_for(|m| m.get("id") == Some(&serde_json::json!(id)));
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            let size = serde_json::to_string(&answered)
                .map(|s| s.len())
                .unwrap_or(0);
            (size, ms, answered)
        };

    // 与 Lean 对拍**同一个光标**（`lsp_bench.py` 的 `marker_position` ✓）。
    let (line, character) = Client::marker_position(&text);
    let doc = serde_json::json!({"textDocument": {"uri": uri}});

    // 预热一次（报告已落 ⇒ 量的是稳态 ✓）。
    let _ = ask("soko/goals", doc.clone(), 9000);
    let (all_b, all_ms, all) = ask("soko/goals", doc.clone(), 9001);

    let decls = all
        .pointer("/result/decls")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        decls.len() > 1,
        "夹具前提：unit08 必须有多条声明（否则'按需'判据空转 ✗）：{} 条",
        decls.len()
    );

    // **两个**光标（⚠ 咬得住的前提 ✓）：① 对拍那一格（落在**第一条**声明里）；
    // ② 第三条声明里。只测 ① 的话，"恒答第一条"也能过 ✗ —— 实测过：unit08 的第一条
    // 声明恰好就是锚点所在的那条，单点判据**咬不住**选择错误 ✗（2026-10-09 反向验证）。
    let probes: [(&str, (u32, u32)); 2] = [
        ("对拍锚点 y", (line, character)),
        (
            "第三条声明 preimage_union",
            position_of(&text, "theorem preimage_union"),
        ),
    ];

    let reconstruct = |runs: &serde_json::Value| -> String {
        runs.as_array()
            .map(|a| {
                a.iter()
                    .map(|r| r["text"].as_str().unwrap_or_default())
                    .collect::<String>()
            })
            .unwrap_or_default()
    };
    let inside = |d: &serde_json::Value, at: (u32, u32)| -> bool {
        let r = &d["range"];
        let (sl, sc) = (
            r["start"]["line"].as_u64().unwrap_or(u64::MAX),
            r["start"]["character"].as_u64().unwrap_or(u64::MAX),
        );
        let (el, ec) = (
            r["end"]["line"].as_u64().unwrap_or(0),
            r["end"]["character"].as_u64().unwrap_or(0),
        );
        let p = (at.0 as u64, at.1 as u64);
        // **两端都闭**（与真相层 `state_at`/`goal_at` 的包含语义一致 ✓ —— 半开的话
        // 光标恰在声明末尾时两边会各答一条，判据自己就先错了 ✗）。
        p >= (sl, sc) && p <= (el, ec)
    };

    let mut at_max = 0usize;
    let mut at_ms_max = 0.0f64;
    let mut names: Vec<String> = Vec::new();
    for (label, at) in probes {
        let mut at_params = doc.clone();
        at_params["position"] = serde_json::json!({"line": at.0, "character": at.1});
        let _ = ask("soko/goalAt", at_params.clone(), 9002);
        let (at_b, at_ms, answer) = ask("soko/goalAt", at_params, 9003);
        at_max = at_max.max(at_b);
        at_ms_max = at_ms_max.max(at_ms);

        let at_decl = answer
            .pointer("/result/decl")
            .cloned()
            .expect("`decl` 必须在响应里（null 也是值 ✓）");
        assert!(
            !at_decl.is_null(),
            "{label} 的光标 {}:{} 必须落在某条声明里（否则判据空转 ✗）：{answer:?}",
            at.0,
            at.1
        );

        // ② 等价：在整份列表里按**光标落在谁的范围里**找那一条（服务端选择语义的对照 ✓
        // —— 不是按名字找 ✗：按名字找的话，"恒答第一条"也能过 ✗）。
        let expected = decls.iter().find(|d| inside(d, at)).unwrap_or_else(|| {
            panic!(
                "整份列表里没有含光标 {}:{} 的声明（夹具前提失败 ✗）：{all:?}",
                at.0, at.1
            )
        });
        // ⚠ 用 `assert!` 而不是 `assert_eq!`：后者的 `left:`/`right:` 会把两个几十条 runs 的
        // 对象整份打印 ✗（实测刷屏 —— `AGENTS.md`：长命令输出不许灌进会话）。
        assert!(
            at_decl == *expected,
            "{label}：goalAt 的条目必须与 soko/goals 里含同一光标的那条**逐字段相同**（含全部 runs ✓）\
             —— 不一致的字段：{:?}（goalAt 答 `{}`，整份列表里那条是 `{}`）",
            mismatched_fields(&at_decl, expected),
            at_decl["name"].as_str().unwrap_or("?"),
            expected["name"].as_str().unwrap_or("?")
        );
        // 两次光标必须落在**不同**的声明上（否则这条判据是同一格的重复 ✗）。
        let name = at_decl["name"].as_str().unwrap_or_default().to_string();
        assert!(
            !names.contains(&name),
            "两个探针光标必须落在不同声明上（都答了 `{name}` ⇒ 判据退化 ✗）"
        );
        names.push(name);

        // ③ 不降级：那一份**自己的** runs 必须在、且能重建它自己的文本 ✓。
        let ty = at_decl["ty"].as_str().unwrap_or_default().to_string();
        assert_eq!(
            reconstruct(&at_decl["ty_runs"]),
            ty,
            "{label}：`ty_runs` 必须逐字节重建 `ty`（否则卡片类型行只剩纯文本 ✗）"
        );
        assert!(
            at_decl["ty_runs"]
                .as_array()
                .map(|a| a.iter().any(|r| r["kind"].is_string()))
                .unwrap_or(false),
            "{label}：`ty_runs` 必须有 kind（着色的唯一来源）：{:?}",
            at_decl["ty_runs"]
        );
        if let Some(goal) = at_decl["goal"].as_str() {
            assert_eq!(
                reconstruct(&at_decl["goal_runs"]),
                goal,
                "{label}：`goal_runs` 必须逐字节重建 `goal`（R-2 同形的静默降级守卫 ✓）"
            );
            assert!(
                at_decl["goal_runs"]
                    .as_array()
                    .map(|a| a.iter().any(|r| r["kind"].is_string()))
                    .unwrap_or(false),
                "{label}：`goal_runs` 必须有 kind（否则目标行不高亮 ✗）：{:?}",
                at_decl["goal_runs"]
            );
        }
    }
    assert_eq!(names.len(), 2, "两个探针都要答上");

    // ① 按需：结构比值（噪声免疫 ✓ —— 不写绝对字节，夹具长大了判据也不假红 ✓）。
    assert!(
        at_max * 10 <= all_b,
        "goalAt 必须 ≤ soko/goals 的 1/10（结构判据）：goalAt {at_max}B vs goals {all_b}B \
         （{:.1}%）—— 变大了说明它又开始回整份列表 ✗",
        100.0 * at_max as f64 / all_b as f64
    );

    println!(
        "PERF goals-at: soko/goals {all_b}B/{all_ms:.1}ms（{} 条）· \
         soko/goalAt ≤{at_max}B/{at_ms_max:.1}ms（1 条，{:.1}%，命中 {:?}）⇒ 结构比 ≥10× ✓ · cache={}",
        decls.len(),
        100.0 * at_max as f64 / all_b as f64,
        names,
        cache.display()
    );
    let _ = std::fs::remove_dir_all(&cache);
}

/// 文本里 `needle` 首次出现的 LSP 位置（1-based 行 → 0-based；列按 **UTF-16 码元** ✓）。
fn position_of(text: &str, needle: &str) -> (u32, u32) {
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("夹具前提：找不到 {needle:?}"));
    let before = &text[..at];
    let line = before.rsplit('\n').next().unwrap_or_default();
    (
        before.matches('\n').count() as u32,
        line.encode_utf16().count() as u32,
    )
}

/// 两个 wire 条目里**值不同**的字段名（只报名字 ✓ —— 一个 goal 声明的 JSON 有几十条
/// runs，整份 dump 会把会话刷爆 ✗；`AGENTS.md`：长命令的输出不许灌进会话）。
fn mismatched_fields(a: &serde_json::Value, b: &serde_json::Value) -> Vec<String> {
    match (a.as_object(), b.as_object()) {
        (Some(a), Some(b)) => {
            let mut keys: Vec<&str> = a.keys().chain(b.keys()).map(String::as_str).collect();
            keys.sort_unstable();
            keys.dedup();
            keys.into_iter()
                .filter(|k| a.get(*k) != b.get(*k))
                .map(str::to_string)
                .collect()
        }
        _ => vec!["<不是对象>".to_string()],
    }
}
