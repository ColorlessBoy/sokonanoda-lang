//! **T2-B0 的前置判据**（2026-10-09）：记法表**分段建 + 合并**必须与
//! **一次性建**（按 `库层 ++ 入口` 的完整命令序列）**逐位相同** ✓。
//!
//! 为什么先要这一条：T2-B0 要把"库层那一段"随检查点存一次、每刀只建"入口那一段" ✓
//! —— 而两张表**各自都带内建记法**（`display_notations_from_commands` 会前插
//! `builtin_notation_decls()` ✓）⇒ 天真拼接会**重复内建项** ✗ ⇒ 折叠行为可能分叉 ✗。
//! 本判据就是那条红线的守卫 ✓。

use sokonanoda_front::compile::display_notations_from_commands;
use sokonanoda_front::parse;

#[test]
fn a_segmented_notation_table_equals_the_one_shot_build() {
    // 库层：一条自定义中缀 + 它作用的声明。
    let lib = parse("infix:50 \" ⊗ \" => myop\naxiom myop : Prop → Prop → Prop\n").unwrap();
    // 入口：另一条中缀 + 它作用的声明（**名字与符号都与库层不同** ⇒ 覆盖关系干净 ✓）。
    let entry = parse("infix:60 \" ⊕ \" => otherop\naxiom otherop : Prop → Prop → Prop\n").unwrap();

    // ① 一次性：库层 ++ 入口。
    let mut all = lib.commands.clone();
    all.extend(entry.commands.clone());
    let one_shot = display_notations_from_commands(&all);

    // ② 分段：各自建 + 合并。
    let lib_only = display_notations_from_commands(&lib.commands);
    let entry_only = display_notations_from_commands(&entry.commands);
    let merged = lib_only.merged_with(&entry_only);

    assert_eq!(
        merged, one_shot,
        "**分段建 + 合并**必须与**一次性建**逐位相同 ✗ —— 不等说明内建记法被重复了、\
         或元数表的覆盖顺序不对 ⇒ 折叠会分叉 ✗"
    );

    // ③ 反向验证：**把入口那一段丢了** ⇒ 必须**不同**（否则②只是"两边都空"的假绿 ✗）。
    assert_ne!(
        lib_only, one_shot,
        "反向验证：丢掉入口那一段必须**真的**不同（否则②没有判别力 ✗）"
    );
    // ④ 反向验证（第二条牙）：**重复内建**（天真拼接）必须**不同** —— 这正是②要挡的错法 ✓。
    let mut naive = lib_only.clone();
    naive = naive.merged_with(&lib_only); // 拿库层自己当"入口"⇒ 库层那几条会被再加一遍
    assert_ne!(
        naive, one_shot,
        "反向验证：把同一段加两遍必须**不同** ⇒ 判据②分得清\"合对了\"与\"合重了\" ✓"
    );
}
