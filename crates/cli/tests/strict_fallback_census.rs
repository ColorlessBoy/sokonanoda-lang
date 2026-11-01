//! **P2-12 S0b 判据（端到端对账）**：内核回落之后老路的结局必须**对得上账**。
//!
//! S0 只结算了应用路径（`try_app`），记法路由 / 零元 `try_hinted_const` / `fill_late` 三处没量 ⇒ 普查里
//! `fallback_unsolved` 约 119 次对不上账（`reports/2026-10-31-strict-handoff.md` §2.1）。这条判据钉的就是那张账：
//!
//! ```text
//! fallback_unsolved == probe_cut + Σ fallback_old_*_unsolved
//! ```
//!
//! ## 为什么判据在这一层
//!
//! 触发记法路由（`kernel_app_notation` 的 `missing > 0` 分支）的形状要课程库的 `Set.Le`（`A ≼ B`）——
//! prelude 里的记法目标要么带前导隐式（走 `elab_notation_implicit`）、要么没有前导参数 ⇒ 单元层够不着。
//! 这里跑**用户那条命令**：`grade --dialect strict <文件>`，只比 stderr 的 `KERNEL_APP` 一行。
//!
//! ## 咬合力（反向验证过一次）
//!
//! 把 `elab_notation` 里的 `settle.ok()` 去掉（或让 `arm()` 恒 `false`）⇒ `fallback_old_ok_notation_unsolved`
//! 归零、差额回到 31 ⇒ 本判据判红。**只加计数**：闸门关时 `KERNEL_APP` 一行都不许出现（新代码不跑）。
//!
//! ## 环境隔离
//!
//! 每次 spawn 用**新的** `SOKONANODA_CACHE_DIR`（全局编译缓存命中会整份回放、一行 `KERNEL_APP` 都不打 ⇒
//! 判据会"绿得没道理"）；`SOKO_STRICT_AT` / `SOKO_KERNEL_ARG_PP` / `SOKO_META_PATTERN` 一并摘掉，
//! 不让外部环境把读数改掉。两份 `NO_*_ARTIFACTS` 关掉产物 ⇒ 不在 `courses/` 下落东西。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// 该文件上记法路由命中过（`Set.Le` 记法 `A ≼ B`）、且老路把 31 次 `unsolved` 都接住了。
const FIXTURE: &str = "courses/set-theory/units/I.8/unit19-cardinal-le.sokonanoda";

static SEQ: AtomicUsize = AtomicUsize::new(0);

fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&root).unwrap_or(root)
}

fn scratch() -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("s0b-census-{}-{nanos}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录");
    dir
}

/// 跑一次用户命令，返回 `(退出码, stderr)`。
fn grade(file: &str, switch_on: bool) -> (i32, String, PathBuf) {
    let dir = scratch();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    for key in [
        "SOKO_STRICT_AT",
        "SOKO_KERNEL_ARG_PP",
        "SOKO_META_PATTERN",
        "SOKO_META_APPROX",
        "SOKO_STRICT_KERNEL_APP",
    ] {
        cmd.env_remove(key);
    }
    if switch_on {
        cmd.env("SOKO_STRICT_KERNEL_APP", "1");
    }
    cmd.env("SOKO_JUDGE_STATS", "1");
    cmd.env("SOKONANODA_CACHE_DIR", &dir);
    cmd.env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1");
    cmd.env("SOKONANODA_NO_MODULE_ARTIFACTS", "1");
    let out: Output = cmd
        .args(["grade", "--dialect", "strict"])
        .arg(repo_root().join(file))
        .arg("--json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("跑不起 `sokonanoda grade --dialect strict {file}`：{e}"));
    (
        out.status
            .code()
            .unwrap_or_else(|| panic!("子进程被信号终止：{out:?}")),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        dir,
    )
}

/// `KERNEL_APP` 行的 `k=v` 计数（`KERNEL_APP_MISS` 是另一行，不混进来）。
fn kernel_app(stderr: &str) -> Option<BTreeMap<String, u64>> {
    let line = stderr
        .lines()
        .find(|l| l.starts_with("KERNEL_APP ") && !l.starts_with("KERNEL_APP_MISS"))?;
    Some(
        line.split_whitespace()
            .filter_map(|tok| {
                let (k, v) = tok.split_once('=')?;
                Some((k.to_string(), v.parse::<u64>().ok()?))
            })
            .collect(),
    )
}

fn unsolved_old(m: &BTreeMap<String, u64>) -> u64 {
    m.iter()
        .filter(|(k, _)| k.starts_with("fallback_old_") && k.ends_with("_unsolved"))
        .map(|(_, v)| *v)
        .sum()
}

#[test]
fn unsolved_fallbacks_are_fully_accounted() {
    let (code, stderr, dir) = grade(FIXTURE, true);
    let m = kernel_app(&stderr).unwrap_or_else(|| {
        panic!(
            "开关开 ⇒ 必须有 KERNEL_APP 行（stderr 尾巴：{}）",
            tail(&stderr, 600)
        )
    });
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        code,
        0,
        "课程语料必须通过：stderr 尾巴 {}",
        tail(&stderr, 600)
    );
    let cut = m.get("probe_cut").copied().unwrap_or(0);
    let unsolved = m.get("fallback_unsolved").copied().unwrap_or(0);
    let old = unsolved_old(&m);
    assert!(
        unsolved > 0 && cut > 0,
        "夹具必须真出现过两种 unsolved 回落（unsolved={unsolved} cut={cut}）"
    );
    assert_eq!(
        unsolved,
        cut + old,
        "对不上账：fallback_unsolved={unsolved} 应 = probe_cut={cut} + Σ fallback_old_*_unsolved={old}\n{m:?}"
    );
    // 记法路由必须真的结算过 —— 否则上一条会"两边都少"而假绿（S0b 之前正是这样）。
    let notation_ok = m
        .get("fallback_old_ok_notation_unsolved")
        .copied()
        .unwrap_or(0);
    assert!(
        notation_ok > 0,
        "记法路由的 old_ok 必须被记下来（S0b 之前这里是 0，差额 31）\n{m:?}"
    );
}

/// **反向验证（开关关）**：新代码一行不跑 ⇒ 没有 `KERNEL_APP` 行；判卷与开关开相同（都是 rc=0）。
#[test]
fn switch_off_records_nothing() {
    let (code, stderr, dir) = grade(FIXTURE, false);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, 0, "strict 档本来就能过：{}", tail(&stderr, 400));
    assert!(
        kernel_app(&stderr).is_none(),
        "闸门关 ⇒ 一条 KERNEL_APP 都不许打：{}",
        tail(&stderr, 400)
    );
}

/// `fill_late`（域一直卡在元变量上 ⇒ 推迟的带注解 λ 位用老办法补上）也要在普查里看得见。
/// 单测层造不出这个形状（`BLOCKED_SRC` 是补不上的那一支），只能拿语料量。
const LATE_FIXTURE: &str = "courses/set-theory/units/I.4/unit12-synthesis.sokonanoda";

#[test]
fn fill_late_is_visible_in_census() {
    let (code, stderr, dir) = grade(LATE_FIXTURE, true);
    let _ = std::fs::remove_dir_all(&dir);
    let m =
        kernel_app(&stderr).unwrap_or_else(|| panic!("缺 KERNEL_APP 行：{}", tail(&stderr, 600)));
    assert_eq!(code, 0, "语料必须通过：{}", tail(&stderr, 600));
    assert!(
        m.get("fill_late").copied().unwrap_or(0) > 0,
        "该文件上 fill_late 必须 > 0（S0b 之前这条看不见）\n{m:?}"
    );
}

fn tail(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let start = chars.len().saturating_sub(n);
    chars[start..].iter().collect()
}
