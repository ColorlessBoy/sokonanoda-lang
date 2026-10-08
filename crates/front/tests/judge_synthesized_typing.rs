//! **T3-B2 的门槛读数（在"真实连续键入"那条臂上重量）** —— `PLAN-align-lean4.md` §11.10 的重排①。
//!
//! ## 为什么必须重量
//!
//! §11.8 的门槛读数是「冷开 37 趟 / 第 1 刀 4 趟 / **稳态 0 趟**」⇒ 判"合成趟不是主要成本"。
//! 但那次用的臂是**两份文本之间来回**——与 §11.9 推翻掉的"稳态 78ms"**是同一个假象** ✗：
//! 第二刀起命中的是第一次就喂热的缓存。**真实做题**每一刀都是**新文本**（插入/删除字符，
//! 后缀的字节偏移整体平移）。
//!
//! ⇒ 本文件在 `QueryDoc`（= LSP 每一次 `didChange` 走的那条路）上把**两臂**并排量出来：
//!
//! | 臂 | 每一刀的文本 | 对应真实场景 |
//! |---|---|---|
//! | `alternating` | 在**两份**文本之间来回 | 旧读数的形状（**假象** ✗） |
//! | `typing` | **每一刀都新**（多插一个空格） | **用户一路敲下去** ✓ |
//!
//! 读数 = `judge::synthesized_report()` 的**逐刀差量** `(趟数, Σ命令数, 回退趟数)`。
//!
//! 跑法：`cargo test -p sokonanoda-front --test judge_synthesized_typing -- --nocapture`

use sokonanoda_front::judge::synthesized_report;
use sokonanoda_front::query::QueryDoc;

fn course_unit(rel: &str) -> Option<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    let entry = root.join(rel);
    entry.is_file().then_some(entry)
}

/// 在 `Set.mem_image α β f A<k 空格>y` 里再加一个空格 ⇒ **每一刀都是没编过的新文本** ✓。
fn add_one_space(current: &str) -> String {
    let marker = "Set.mem_image α β f A";
    let at = current.find(marker).expect("夹具前提：锚点必须在");
    let rest = &current[at + marker.len()..];
    let spaces = rest.chars().take_while(|c| *c == ' ').count();
    let mut next = String::with_capacity(current.len() + 1);
    next.push_str(&current[..at + marker.len()]);
    next.push_str(&" ".repeat(spaces + 1));
    next.push_str(&rest[spaces..]);
    next
}

#[test]
fn synthesized_passes_are_measured_on_real_typing() {
    let Some(entry) = course_unit("units/I.3/unit08-images-preimages.sokonanoda") else {
        eprintln!("跳过：找不到课程单元（课程仓可分开检出）");
        return;
    };
    let text = std::fs::read_to_string(&entry).expect("read unit08");
    assert!(
        text.contains("Set.mem_image α β f A y") && text.contains("demo_mem_image"),
        "夹具前提：两个编辑锚点都必须在"
    );

    // ── 冷开：建库层检查点 + `entry_cache`（两臂共用同一份起点：各起一个 doc ✓）──
    let open = |doc: &mut QueryDoc| {
        doc.path = Some(entry.clone());
        doc.set_text(&text, 1, None);
    };
    let mut cold_doc = QueryDoc::new();
    let cold_before = synthesized_report();
    open(&mut cold_doc);
    let cold_after = synthesized_report();
    let cold = diff(&cold_before, &cold_after);
    assert!(
        cold.0 > 0,
        "夹具自检：冷开必须真的跑合成趟（否则计数器没接上/路径变了 ✗）——实测 {} 趟",
        cold.0
    );
    eprintln!(
        "PERF synthesized-typing cold-open: passes={} cmds={} fb={}",
        cold.0, cold.1, cold.2
    );

    // ── 臂 A：两文本来回（**旧读数的形状**）──
    let mut alt = QueryDoc::new();
    open(&mut alt);
    let mut alt_rounds = Vec::new();
    let mut current = text.clone();
    for round in 0..5 {
        let (from, to) = if round % 2 == 0 {
            ("demo_mem_image", "demo_mem_imagX")
        } else {
            ("demo_mem_imagX", "demo_mem_image")
        };
        current = current.replace(from, to);
        assert!(current.contains(to), "夹具前提：改名必须命中");
        let before = synthesized_report();
        alt.set_text(&current, 2 + round as u64, None);
        alt_rounds.push(diff(&before, &synthesized_report()));
    }

    // ── 臂 B：**真实连续键入**（每一刀都是新文本）──
    let mut typing = QueryDoc::new();
    open(&mut typing);
    let mut typing_rounds = Vec::new();
    let mut current = text.clone();
    for round in 0..5 {
        current = add_one_space(&current);
        let before = synthesized_report();
        typing.set_text(&current, 2 + round as u64, None);
        typing_rounds.push(diff(&before, &synthesized_report()));
    }

    for (label, rounds) in [("alternating", &alt_rounds), ("typing", &typing_rounds)] {
        let passes: Vec<u64> = rounds.iter().map(|r| r.0).collect();
        let cmds: Vec<u64> = rounds.iter().map(|r| r.1).collect();
        let fb: Vec<u64> = rounds.iter().map(|r| r.2).collect();
        eprintln!(
            "PERF synthesized-{label}: passes={passes:?} cmds={cmds:?} fallbacks={fb:?} \
             (Σpasses={} Σcmds={})",
            passes.iter().sum::<u64>(),
            cmds.iter().sum::<u64>()
        );
    }
    // **判据（T3-B2 的目标出口 · 本文件唯一的正确出口 ✓）**：真实连续键入下
    // **一趟合成都不许跑**（`typing_passes == 0`）。
    //
    // 历史：本条原先是"夹具自检：`typing_passes >= 4`"（证明这条臂**真的**产生新文本、
    // 且 §11.8 的"稳态 0 趟"是假象 ✗）。**T3-B1 第 5 条族落地后**（把"合成一份文档"
    // 换成"就地读活环境的签名"）⇒ 该出口达成 ⇒ 按本文件原来的指令（"落地后改成 0"，
    // **不许放宽** ✗）翻转 ✓。冷开仍有 `cold.0 > 0` 趟 ⇒ 计数器**没空转** ✓。
    let typing_passes: u64 = typing_rounds.iter().map(|r| r.0).sum();
    let alternating_passes: u64 = alt_rounds.iter().map(|r| r.0).sum();
    assert_eq!(
        typing_passes, 0,
        "**T3-B2 的目标出口**：真实连续键入下**一趟合成都不许跑** —— 现在 Σ={typing_passes} \
         ⇒ 还有判定点走「合成一份文档 + 重跑前缀」✗（别放宽这条断言 ✗）。\
         冷开仍有 {} 趟（对照 = `alternating` Σ={alternating_passes}）⇒ 计数器没空转 ✓",
        cold.0
    );
    eprintln!(
        "PERF synthesized-typing-verdict: typing Σpasses={typing_passes} vs alternating \
         Σpasses={alternating_passes}"
    );
}

fn diff(before: &(u64, u64, u64), after: &(u64, u64, u64)) -> (u64, u64, u64) {
    (after.0 - before.0, after.1 - before.1, after.2 - before.2)
}
