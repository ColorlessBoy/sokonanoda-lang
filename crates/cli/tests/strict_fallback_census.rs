//! **P2-12 S0b 判据（端到端对账）**：内核回落之后老路的结局必须**对得上账**。
//!
//! S0 只结算了应用路径（`try_app`），记法路由 / 零元 `try_hinted_const` / `fill_late` 三处没量 ⇒ 普查里
//! `fallback_unsolved` 约 119 次对不上账（`reports/2026-10-31-strict-handoff.md` §2.1）。这条判据钉的就是那张账：
//!
//! ```text
//! fallback_unsolved == probe_cut + Σ fallback_old_*_unsolved     # P2-12 S6 **之前**（什么都回落）
//! nofallback_kept_<类> == Σ 老路在该路由上接住的那些                # P2-12 S6 **之后**（只例外回落）
//! ```
//!
//! **P2-12 S6 翻默认**把前一条口径作废（没有回落就没有探测切 ⇒ `probe_cut` 恒 0；`fallback_unsolved`
//! 里还有一部分直接走报错出口 ⇒ 原来那条等式两边不再相等）。后一条口径量的是**新的真相**：
//! 写明的例外是**唯一**还会回落的一族，且每一次都必须由老路接住 —— 判据强度不降。
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
//! 归零、差额回到 31 ⇒ 本判据判红。**只加计数**：宽松档 `KERNEL_APP` 一行都不许出现（新代码不跑）。
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

/// 跑一次用户命令，返回 `(退出码, stderr)`。`dialect` = 用户写的那一格（P2-12 S6 起 strict 档
/// **恒**是内核路径 + 不回落 ⇒ 不再有开关可设，只能换方言对照）。
fn grade(file: &str, dialect: &str) -> (i32, String, PathBuf) {
    let dir = scratch();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    for key in [
        "SOKO_STRICT_AT",
        "SOKO_KERNEL_ARG_PP",
        "SOKO_META_PATTERN",
        "SOKO_META_APPROX",
    ] {
        cmd.env_remove(key);
    }
    cmd.env("SOKO_JUDGE_STATS", "1");
    cmd.env("SOKONANODA_CACHE_DIR", &dir);
    cmd.env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1");
    cmd.env("SOKONANODA_NO_MODULE_ARTIFACTS", "1");
    let out: Output = cmd
        .args(["grade", "--dialect", dialect])
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
fn exception_fallbacks_are_accounted_and_the_flip_is_visible() {
    let (code, stderr, dir) = grade(FIXTURE, "strict");
    let m = kernel_app(&stderr).unwrap_or_else(|| {
        panic!(
            "strict 档现在恒走内核路径 ⇒ 必须有 KERNEL_APP 行（stderr 尾巴：{}）",
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
    // **P2-12 S6 翻默认**：非例外**不再回落** ⇒ 老路只接"写明的例外"这一族。
    // 口径随之改变（原文 `fallback_unsolved == probe_cut + Σ fallback_old_*_unsolved` 只在
    // "什么都回落"的旧档成立：没有回落就没有探测切，`probe_cut` 恒 0；而且 `fallback_unsolved`
    // 里有一部分直接走了报错出口）。新的对账口径是**逐例外桶**：
    //   每个 `nofallback_kept_<类>` 必须 = 老路在该路由上真正接住（`fallback_old_ok_*`）的条数。
    let kept = |k: &str| m.get(k).copied().unwrap_or(0);
    let old_ok = |k: &str| m.get(k).copied().unwrap_or(0);
    let (kept_prefix, kept_meta) = (
        kept("nofallback_kept_prefix_meta"),
        kept("nofallback_kept_input_contains_a_meta"),
    );
    assert!(
        kept_prefix > 0 && kept_meta > 0,
        "夹具必须真出现过两种**写明的例外**回落（prefix_meta={kept_prefix} meta={kept_meta}）\n{m:?}"
    );
    assert_eq!(
        kept_prefix,
        old_ok("fallback_old_ok_notation_unsolved"),
        "`prefix_meta` 例外必须 = 老路在记法路由上接住的那些（一条不多一条不少）\n{m:?}"
    );
    assert_eq!(
        kept_meta,
        old_ok("fallback_old_ok_unsupported_input_contains_a_meta"),
        "`input contains a meta` 例外必须 = 老路接住的那些\n{m:?}"
    );
    // 翻默认必须**真的**走到"不回落"出口 ⇒ 否则上面那两条会在旧档下"照样绿"（咬不住本刀）。
    assert!(
        kept("nofallback_dead") > 0,
        "strict 档必须报过错（`nofallback_dead` > 0）：否则本判据咬不住翻默认\n{m:?}"
    );
    // 记法路由必须真的结算过 —— 否则上面那两条会"两边都少"而假绿（S0b 之前正是这样）。
    assert!(
        old_ok("fallback_old_ok_notation_unsolved") > 0,
        "记法路由的 old_ok 必须被记下来（S0b 之前这里是 0，差额 31）\n{m:?}"
    );
    let _ = unsolved_old;
}

/// **反向验证（宽松档）**：内核路径一行不跑 ⇒ 没有 `KERNEL_APP` 行；判卷与 strict 相同（都是 rc=0）。
///
/// P2-12 S6 翻默认之前这里是"开关关"那一臂；翻默认之后唯一能关掉内核路径的档位就是**宽松档**
/// （`TEST_OVERRIDE` 是进程内测试钩子，子进程改不到），所以对照臂换方言、判据本身不放宽。
#[test]
fn loose_records_nothing() {
    let (code, stderr, dir) = grade(FIXTURE, "loose");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, 0, "宽松档本来就能过：{}", tail(&stderr, 400));
    assert!(
        kernel_app(&stderr).is_none(),
        "宽松档 ⇒ 一条 KERNEL_APP 都不许打：{}",
        tail(&stderr, 400)
    );
}

/// `fill_late`（域一直卡在元变量上 ⇒ 推迟的带注解 λ 位用老办法补上）也要在普查里看得见。
/// 单测层造不出这个形状（`BLOCKED_SRC` 是补不上的那一支），只能拿语料量。
const LATE_FIXTURE: &str = "courses/set-theory/units/I.4/unit12-synthesis.sokonanoda";

#[test]
fn fill_late_is_visible_in_census() {
    let (code, stderr, dir) = grade(LATE_FIXTURE, "strict");
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
