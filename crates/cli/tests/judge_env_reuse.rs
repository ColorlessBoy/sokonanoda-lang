//! **K1-a（T-K11）的验收判据**：`SOKO_JUDGE_ENV_REUSE` 两态下，同一批语料的
//! `grade --json` 事件流必须**逐字节相同**（含诊断的码/消息/span）。
//!
//! 设计来源 `docs/design/by-prefix-reuse.md` §4「K1-a（先做，零内核风险）」：
//!
//! > 开关：`SOKO_JUDGE_ENV_REUSE=0/1`。验收：两态下**全语料 `--json` 逐字节相同**
//! > （`assert_same_both_ways` 已具备）。
//!
//! ⚠ **那句"已具备"是错的，这条测试就是补它**（2026-09-30 实测核对）：
//! `crates/cli/tests/judge_batch.rs::assert_same_both_ways` 比的是
//! **`SOKO_NO_JUDGE_BATCH`**（乐观判定批处理），**不是**这个开关 ——
//! 换句话说，K1-a 的验收标准在文档里挂了名、却**没有任何判据在跑** ✗
//!（`grep -rn SOKO_JUDGE_ENV_REUSE crates/*/tests/` 当时只命中
//! `crates/front/tests/judge_env_vouch.rs`，而那条钉的是**反向**判据：
//! "不许多担保"，不是两态等价）。
//!
//! 为什么两态等价**必须**有判据（而不是"默认开着、没人碰"就够）：
//! `§3.C`（2026-09-30）把担保接到**主编译 pass** 之后，复用**默认就是开的**
//! ⇒ 这条路径现在**天天在跑**，而它的正确性证据此前只有一次性的手工对拍
//!（第 508 轮的"`--json` 逐行不同 0 行"）✓ —— 手工对拍不会在下次改坏时报警。
//!
//! **两条断言缺一不可**（`AGENTS.md`「判据的两条硬规矩」）：
//! 1. **两态逐字节相同**（含退出码）—— 判定的红线；
//! 2. **判据不空转**：on 态必须**真的命中**（`SOKO_JUDGE_REUSE_STATS=1` 的
//!    `JUDGE_ENV_REUSE: 命中`）—— 否则"一个永远走整份重查的实现"也能让 ① 变绿 ✗。
//!
//! ⚠⚠ **`SOKONANODA_NO_PROJECT_ARTIFACTS=1` 是这条测试的必需品**（实测踩到）：
//! 项目闭包产物落在**模块根**（`courses/set-theory/.sokonanoda/compiled/`），
//! 而它**不受 `SOKONANODA_NO_CACHE=1` 管**（那是全局缓存的开关）。产物一热，
//! 整份编译被跳过 ⇒ **主编译 pass 不跑 ⇒ 不压栈 ⇒ 复用一次都不命中** ⇒
//! 两态"相同"是**空转的相同** ✗（实测：带产物 `hits=0`、整条测试 1.4s；
//! 不带产物 `hits=30`，且**去掉夹紧那个真 bug 立刻显形**：`exit 1`、
//! `--json` 20 行 vs 9 行）⇒ 这条测试**故意**关掉产物，测的是**真编译**那条路。
//!
//! ⚠ 语料是**有代表性的子集**（全语料两态对拍是**收尾**动作，比每环节贵得多；
//! 2026-09-30 的全语料读数是：**128 个文件 · 逐字节差异 0 · on 态命中 164**）。
//! 这里的四份覆盖 **画布 / 带 `import` 的项目入口 / 开放练习 / `by` 密集解答**
//! 四种形状 —— 最后一份是让复用真命中的那种形状。
//!
//! ⚠ **一个档位一个文件**（同 `judge_inplace*` / `judge_env_vouch` 的理由）：
//! `SOKO_JUDGE_ENV_*` 走 `OnceLock` 只读一次环境 ⇒ 同文件的多个 `#[test]`
//! 会抢档位。本文件恰好一个 `#[test]` ✓（守卫 `scripts/check-test-env-isolation.py`）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-judge-env-reuse-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// 判一个文件，返回 `(退出码, stdout, stderr)`。
///
/// `reuse` 决定 `SOKO_JUDGE_ENV_REUSE`；`stats` 打开命中计数（只影响 stderr）。
/// 两个档位都用**各自**的缓存目录，并关掉**两处**缓存（全局缓存 + 模块根产物）——
/// 任一命中都会跳过编译，两态就都"没跑"，等价判据会**空转**地变绿（见文件头）。
fn grade(path: &Path, reuse: bool, stats: bool) -> (i32, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(["grade", "--json"])
        .arg(path)
        .env(
            "SOKONANODA_CACHE_DIR",
            cache_dir(if reuse { "on" } else { "off" }),
        )
        .env("SOKONANODA_NO_CACHE", "1")
        .env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")
        .env("SOKO_JUDGE_ENV_REUSE", if reuse { "1" } else { "0" })
        // 把环境**钉死**（不靠默认值）：`vouch` 是"担保"那一档，
        // 复用要真的发生就得它在 On（默认 On，但外部环境可能设成 0）。
        .env("SOKO_JUDGE_ENV_VOUCH", "1");
    if stats {
        command.env("SOKO_JUDGE_REUSE_STATS", "1");
    }
    let output = command.output().expect("spawn sokonanoda");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// **两态逐字节相同 + 判据不空转**（见文件头）。
#[test]
fn the_reuse_switch_is_byte_identical_on_the_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // 四种形状：画布 / 带 import 的项目入口 / 开放练习 / `by` 密集解答。
    let files = [
        "playground.sokonanoda",
        "course/unit11-project/Exercises.sokonanoda",
        "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda",
        "courses/set-theory/units/solutions/I.4/unit12-solution.sokonanoda",
    ];
    let mut hits = 0usize;
    for name in files {
        let path = root.join(name);
        assert!(path.exists(), "找不到语料 {path:?}");
        let (code_off, out_off, _) = grade(&path, false, false);
        let (code_on, out_on, err_on) = grade(&path, true, true);
        assert_eq!(
            code_on, code_off,
            "{name}：退出码在复用开关两态下不同（{code_on} vs {code_off}）"
        );
        assert_eq!(
            out_on, out_off,
            "{name}：`--json` 事件流在复用开关两态下不同——\n--- 开 ---\n{out_on}\n--- 关 ---\n{out_off}"
        );
        hits += err_on.matches("JUDGE_ENV_REUSE: 命中").count();
    }
    assert!(
        hits > 0,
        "复用**一次都没命中** ⇒ 判据空转：两态相同只说明「这条路根本没跑」✗。\
         夹具必须含 `by` 密集的解答（`unit12-solution` 实测命中 30 次）；\
         若命中数变 0，先查两件事：① `SOKO_JUDGE_ENV_VOUCH` 那条担保链是不是断了；\
         ② 有没有漏掉 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`（产物一热就跳过整份编译）。"
    );
}
