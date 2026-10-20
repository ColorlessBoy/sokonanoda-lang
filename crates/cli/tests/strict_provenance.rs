//! **P3 —— 严格档 v2 的安全腿：CLI canary**（设计 `docs/design/strict-mode-v2.md`
//! §4/§5/§6）：用**用户那条命令** `sokonanoda grade --json <file>` 钉住「两档判定相同」
//! 的语料基线，外加一条**有咬合力**的正对照。
//!
//! ## 三条牙
//!
//! 1. `lib_prod_is_dual_dialect` —— `courses/set-theory/lib/Prod.sokonanoda`：
//!    `SOKO_STRICT_AT=0` 与 `=1` 的 stdout **逐字节相同**、退出码相同（HEAD = 0）；
//! 2. `unit05_is_dual_dialect` —— 同上，
//!    `courses/set-theory/units/I.2/unit05-pairs-products.sokonanoda`；
//! 3. `b2_is_rejected_only_under_strict` —— **正对照**：`h Nat Nat …`（正常形式喂位置
//!    隐式实参）在 loose 下 checked、在 strict 下 exit 1，且唯一诊断码是
//!    `elab-implicit-argument-unsolved`。**没有它，①② 的「两档相同」可能只是
//!    「两档都坏成一样」** ✗（判据没有咬合力）。
//!
//! ## ⚠ ①② 是 **canary：HEAD 绿、R3 后预期变红（这是有意的）**
//!
//! HEAD `f6a27097`（S3）上这两份语料是**双方言**的：两档都 exit 0、事件流逐字节一致 ✓。
//! 严格档 v2 的**第 3 棒（R3：provenance 收口 / 删 `operand_type_expr` 里那条
//! `explicit = strict_at_enabled()`）**会改严格档经**引擎合成文本**走的判定通路 ——
//! 届时这两份语料若在新通路上仍依赖旧读法，strict 就会开始判红 ⇒ **①② 失败是设计好的
//! 警报，不是回归** ✓。
//!
//! 处置：按 `strict-mode-v2.md` §4 迁移语料（**删多余隐式实参**，不是加 `@`），
//! **不许**为了让测试变绿而放宽本条断言 ✗（放宽 = 把 canary 拆掉）。
//!
//! ## 环境隔离
//!
//! 每次 spawn 都先 `env_remove("SOKO_STRICT_AT")` 再显式注入本用例那一档 —— `Command`
//! 默认**继承**父环境，开发机/CI 只要外部设过开关，判据量到的就是环境值而不是本档 ✗
//! （假红/假绿）。`SOKO_KERNEL_ARG_PP` 同族（见 `implicit_at_alignment.rs`）一并摘掉。
//!
//! 两份语料都额外设 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`：**每次真编译**、且不在
//! `courses/` 下落产物 ⇒ 判据不依赖上一次的缓存状态（热产物会跳过整份编译，canary 就
//! 可能被旧缓存蒙住 ✗），测试并行时也不互相写同一棵课程树 ✓。

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

/// 一条 CLI 结果：退出码 + **原始字节** stdout（逐字节比较用）+ stderr 摘要。
struct GradeRun {
    code: i32,
    stdout: Vec<u8>,
    stderr: String,
}

/// 跑一次用户命令 `sokonanoda grade --json <path>`。
///
/// `strict_at` = `"0"`（loose）或 `"1"`（strict）。**先摘继承环境、再注入本档**。
fn run_grade(path: &Path, strict_at: &str) -> GradeRun {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.env_remove("SOKO_STRICT_AT");
    cmd.env_remove("SOKO_KERNEL_ARG_PP");
    cmd.env("SOKO_STRICT_AT", strict_at);
    cmd.env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1");
    let out: Output = cmd
        .args(["grade", "--json"])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("跑不起 `sokonanoda grade --json {}`：{e}", path.display()));
    GradeRun {
        code: out
            .status
            .code()
            .unwrap_or_else(|| panic!("子进程被信号终止（没有退出码）：{out:?}")),
        stdout: out.stdout,
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&root).unwrap_or(root)
}

/// 解析 `--json` 事件流（一行一个 JSON 对象）；解析不了的行走 `filter_map` 丢掉 ——
/// 这里只给**失败信息**与**正对照**用，canary 本身比的是原始字节。
fn events(run: &GradeRun) -> Vec<Value> {
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn checked_names(run: &GradeRun) -> Vec<String> {
    events(run)
        .iter()
        .filter(|e| e["type"] == "decl.checked")
        .filter_map(|e| e["name"].as_str().map(str::to_owned))
        .collect()
}

fn diag_codes(run: &GradeRun) -> Vec<String> {
    events(run)
        .iter()
        .filter(|e| e["type"] == "diagnostic")
        .filter_map(|e| e["code"].as_str().map(str::to_owned))
        .collect()
}

/// 失败信息里的 stdout/stderr 摘要：**不灌整条事件流**（课程库的 `--json` 有几十 KB），
/// 但足够定位（退出码、字节数、首末行、诊断码、stderr 尾巴）。
fn brief(run: &GradeRun) -> String {
    let text = String::from_utf8_lossy(&run.stdout);
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("<空>");
    let last = lines.last().unwrap_or(first);
    let truncate = |s: &str, n: usize| -> String {
        if s.chars().count() <= n {
            s.to_string()
        } else {
            let head: String = s.chars().take(n).collect();
            format!("{head}…（截断）")
        }
    };
    format!(
        "exit={} · stdout={}B · 诊断码={:?} · stdout 首行={} · 末行={} · stderr={}",
        run.code,
        run.stdout.len(),
        diag_codes(run),
        truncate(first, 160),
        truncate(last, 160),
        if run.stderr.trim().is_empty() {
            "<空>".to_string()
        } else {
            truncate(run.stderr.trim(), 800)
        },
    )
}

fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .or_else(|| {
            if a.len() != b.len() {
                Some(a.len().min(b.len()))
            } else {
                None
            }
        })
}

/// canary 失败时提醒**为什么这是有意的**（只进失败信息，不影响绿）。
const CANARY_HINT: &str = "若这发生在 R3 之后：①② 变红是**有意**的 canary，\
处置是按 `docs/design/strict-mode-v2.md` §4 迁移语料（删多余隐式实参），\
**不许**放宽断言；若发生在 R3 之前，先查是不是继承了外部 `SOKO_STRICT_AT`。";

/// ①② 的共用判据：同一份语料，两档 stdout **逐字节**相同 + 退出码相同（HEAD 都是 0）。
fn assert_dual_dialect(rel: &str) {
    let path = repo_root().join(rel);
    assert!(
        path.is_file(),
        "找不到语料 `{rel}`（解析到 {}；仓库根 = {}）",
        path.display(),
        repo_root().display()
    );

    let loose = run_grade(&path, "0");
    let strict = run_grade(&path, "1");

    // **canary 不许空转**：两档都真的编了、真的产出了声明事件（否则"逐字节相同"
    // 可能只是"两边都什么都没输出"）。
    let checked = checked_names(&loose);
    assert!(
        !loose.stdout.is_empty() && !checked.is_empty(),
        "`{rel}`：loose 档的 `grade --json` 没有产出任何 `decl.checked` ⇒ canary 空转。\n\
         loose: {}\nstrict: {}",
        brief(&loose),
        brief(&strict)
    );

    assert_eq!(
        loose.code,
        0,
        "`{rel}`：HEAD 上 loose 档应 exit 0。\nloose: {}\n{CANARY_HINT}",
        brief(&loose)
    );
    assert_eq!(
        strict.code,
        0,
        "`{rel}`：HEAD 上 strict 档应 exit 0（两档都是 0）。\
         这里变红如果发生在 R3 之后，就是 canary 按设计触发。\nstrict: {}\n{CANARY_HINT}",
        brief(&strict)
    );
    assert_eq!(
        loose.code,
        strict.code,
        "`{rel}`：两档退出码必须相同。\nloose: {}\nstrict: {}",
        brief(&loose),
        brief(&strict)
    );
    assert_eq!(
        loose.stdout,
        strict.stdout,
        "`{rel}`：`grade --json` 的 stdout 必须**逐字节**相同（双方言语料）。\
         首个差异字节偏移 = {:?}（loose {}B / strict {}B）。\n\
         loose: {}\nstrict: {}\n{CANARY_HINT}",
        first_difference(&loose.stdout, &strict.stdout),
        loose.stdout.len(),
        strict.stdout.len(),
        brief(&loose),
        brief(&strict)
    );
}

/// ① **R3 的 canary（课程库）**：`lib/Prod.sokonanoda` 在 HEAD 上是双方言语料。
#[test]
fn lib_prod_is_dual_dialect() {
    assert_dual_dialect("courses/set-theory/lib/Prod.sokonanoda");
}

/// ② **R3 的 canary（课程单元）**：
/// `units/I.2/unit05-pairs-products.sokonanoda` 在 HEAD 上是双方言语料。
#[test]
fn unit05_is_dual_dialect() {
    assert_dual_dialect("courses/set-theory/units/I.2/unit05-pairs-products.sokonanoda");
}

/// ③ **正对照（咬合力）**：同一个 `b2` 在 loose 下 checked、在 strict 下被拒，
/// 且诊断码可归属 —— 证明 ①② 用的 `SOKO_STRICT_AT` 通道**真的能改变判定**。
///
/// 源文本逐字取自 `implicit_at_alignment.rs` 的用户原案（`b2` 那一行）。
const B2_SRC: &str = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a
def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
";

/// 临时画布：`std::env::temp_dir()` + 进程号/计数器唯一名；`Drop` 里清理
/// （断言失败 panic 时也清理 ✓）。
struct TempCanvas {
    dir: PathBuf,
    path: PathBuf,
}

impl TempCanvas {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "soko-p3-strict-provenance-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建临时画布目录");
        let path = dir.join("Probe.sokonanoda");
        std::fs::write(&path, B2_SRC).expect("写临时画布");
        Self { dir, path }
    }
}

impl Drop for TempCanvas {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn b2_is_rejected_only_under_strict() {
    let canvas = TempCanvas::new();

    let loose = run_grade(&canvas.path, "0");
    assert_eq!(
        loose.code,
        0,
        "正对照第一半：loose 档下 `b2` 必须被接受（旧读法）。\nloose: {}",
        brief(&loose)
    );
    assert!(
        checked_names(&loose).iter().any(|name| name == "b2"),
        "正对照第一半：loose 档下 `b2` 必须出现在 `decl.checked` 里。\nloose: {}",
        brief(&loose)
    );

    let strict = run_grade(&canvas.path, "1");
    assert_eq!(
        strict.code,
        1,
        "正对照第二半：strict 档下 `b2`（正常形式喂位置隐式实参）必须判红并 exit 1。\nstrict: {}",
        brief(&strict)
    );
    let codes = diag_codes(&strict);
    assert_eq!(
        codes,
        vec!["elab-implicit-argument-unsolved".to_string()],
        "strict 档必须报且只报 `elab-implicit-argument-unsolved`（码在 `code` 字段，不在文案里）。\
         \nstrict: {}",
        brief(&strict)
    );
    assert!(
        !checked_names(&strict).iter().any(|name| name == "b2"),
        "strict 档下 `b2` 不许出现在 `decl.checked` 里（不是「报个警告」那种放过）。\nstrict: {}",
        brief(&strict)
    );
}
