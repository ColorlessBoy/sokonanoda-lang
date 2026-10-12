//! **长尾 ④ / C4 的真相层判据**：`judge_print_all`（批量 `#print`）与逐条
//! `judge_print_of` **同源同形** —— 它是补全把 50+ 个 prelude 名连同签名一起
//! 摆出来的**唯一**数据源（一条一次的话实测 15–20s ✗，见 `crates/lsp/src/completion.rs`）。
//!
//! 判据三条，缺一不算：
//!
//! 1. **逐位对齐**：`names` 的顺序 = 结果的顺序（补全按位取名，错位就是给错签名 ✗）；
//! 2. **与逐条版逐字相同**：同一个前缀/模式下，批量拿到的文本与
//!    `judge_print_of` 的单条结果**逐字节相等**（批量是加速，不是第二条真相）；
//! 3. **一条坏不拖垮全批**：名字不存在时那一位是 `Err`，**其余位照旧有值**
//!    —— 补全少一条签名可以忍，"整批消失"会静默降级 ✗。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::judge::{judge_print_all, judge_print_of};

/// 取内核渲染里的**签名**段（第一个顶层 ` := ` 之前）——与补全侧同一条切分
/// （`crates/lsp/src/completion.rs::signature_of`）。这里只用它比"两边同源"。
fn signature(text: &str) -> &str {
    text.trim().split(" := ").next().unwrap_or(text)
}

#[test]
fn batch_print_matches_one_at_a_time_and_survives_a_bad_name() {
    let options = CompileOptions::default();
    let names = [
        "Eq.subst",
        "And.intro",
        "Iff.mp",
        "Classical.em",
        // 不存在 ⇒ 这一位必须是 `Err`，且**不影响**其余位。
        "No.Such.Prelude.Name",
        "Or.elim",
    ];
    let batch = judge_print_all("", &options, &names);
    assert_eq!(batch.len(), names.len(), "结果必须与名字逐位对齐");

    for (index, name) in names.iter().enumerate() {
        let single = judge_print_of("", &options, name);
        match (&batch[index], single) {
            (Some(Ok(from_batch)), Ok(from_single)) => assert_eq!(
                signature(from_batch),
                signature(&from_single),
                "`{name}`：批量与逐条必须给同一份内核渲染（附注：批量是加速，不是第二条真相）"
            ),
            (Some(Err(_)), Err(_)) => {}
            (batch, single) => {
                panic!("`{name}`：批量与逐条的成功/失败必须一致 —— 批量={batch:?} 逐条={single:?}")
            }
        }
    }

    // 坏名字那一位是错，**好名字那几位仍然有值**（"整批消失"是静默降级 ✗）。
    assert!(
        matches!(batch[4], Some(Err(_))),
        "不存在的名字必须报错，实际 {:?}",
        batch[4]
    );
    for index in [0, 1, 2, 3, 5] {
        assert!(
            matches!(batch[index], Some(Ok(_))),
            "`{}` 的签名不翼而飞（批量被一个坏名字拖垮了）：{:?}",
            names[index],
            batch[index]
        );
    }

    // 空名单：不跑流水线、不 panic。
    assert!(judge_print_all("", &options, &[]).is_empty());
}
