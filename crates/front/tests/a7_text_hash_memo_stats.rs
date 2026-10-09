//! **A7 的判据读数**（2026-10-09 · 第 62 轮）：`judge` 的前缀键 memo **命中 / 未命中**各多少。
//!
//! ## 为什么先加这条读数，而不是直接改 memo
//!
//! §8.9 把"judge 缓存键对整份前缀跑 SipHash"记成 **22.5% 编译样本** —— 但**那之后**这条路上
//! 已经加了**小 LRU**（[`canonical_text_key`] 的 `MEMO = 4` ✓）⇒ **旧百分比已经不可用** ✗。
//! 而"4 条够不够"从来没人量过 ⇒ 先量再动 ✓（动它的红线：**哈希函数一字不改** ✗）。
//!
//! 跑法：`cargo test -p sokonanoda-front --test a7_text_hash_memo_stats -- --nocapture`

use sokonanoda_front::judge::text_hash_memo_stats;
use sokonanoda_front::query::QueryDoc;

fn module_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory")
}

/// 在 `Set.mem_image α β f A<k 空格>y` 里再加一个空格 ⇒ 每一刀都是没编过的新文本 ✓。
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
fn the_text_hash_memo_hit_rate_is_measured() {
    let root = module_root();
    let entry = root.join("units/I.3/unit08-images-preimages.sokonanoda");
    if !entry.is_file() {
        eprintln!("跳过：找不到课程单元（课程仓可分开检出）");
        return;
    }
    // 冷开前先清产物（§32 的纪律 ✓）。
    let _ = sokonanoda_front::project::cache::clean_at(&root);

    let text = std::fs::read_to_string(&entry).expect("read unit08");
    assert!(
        text.contains("Set.mem_image α β f A y"),
        "夹具前提：编辑锚点必须在"
    );

    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());
    let (h0, m0) = text_hash_memo_stats();
    doc.set_text(&text, 1, None);
    let cold = {
        let (h, m) = text_hash_memo_stats();
        (h - h0, m - m0)
    };

    let mut current = text.clone();
    let mut per: Vec<(u64, u64)> = Vec::new();
    for v in 2..6u64 {
        current = add_one_space(&current);
        let (h, m) = text_hash_memo_stats();
        doc.set_text(&current, v, None);
        let (h2, m2) = text_hash_memo_stats();
        per.push((h2 - h, m2 - m));
    }
    let total_h: u64 = cold.0 + per.iter().map(|(h, _)| h).sum::<u64>();
    let total_m: u64 = cold.1 + per.iter().map(|(_, m)| m).sum::<u64>();
    let rate = if total_h + total_m == 0 {
        0.0
    } else {
        100.0 * total_h as f64 / (total_h + total_m) as f64
    };
    println!(
        "PERF a7 text-hash-memo: 冷开={cold:?} 逐刀={per:?} \
         合计 命中={total_h} 未命中={total_m} ⇒ 命中率={rate:.1}%"
    );

    // ⚠ 这里**只报读数、不断言阈值**（先量后动 ✓）：判据等下一轮按数据定。
    assert!(
        total_h + total_m > 0,
        "夹具自检：这条读数必须真的被走到（否则计数器没接上 / 路径变了 ✗）—— 合计 0 次"
    );
}
