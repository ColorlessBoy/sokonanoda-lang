//! **G-68 的验收判据（结构计数版）**：项目 `build` **不许**把共享依赖按入口各编一遍。
//!
//! ## 缺口
//!
//! 项目 `build`/`rebuild` 的缓存键是 **per-entry-closure** ⇒ 每个入口都把共享依赖
//! **重新编一遍** ✗（台账 G-68：0.78.0 实测 `courses/set-theory` rebuild **222.1s** ·
//! Σ闭包 174 次 vs 去重 42 个模块 = **4.14×** ✓；2026-10-04 在 250 文件语料上复测
//! Σ闭包 **1366** vs **248** = **5.51×** ✗）。
//!
//! ## 判据为什么是"倍率"而不是毫秒
//!
//! `AGENTS.md`（2026-09-29 **第三次**同一种病）：共享 runner 上墙钟不可转移
//! ⇒ 一律用**结构计数**或**比值** ✓。这里两个数都来自**同一次跑** ✓：
//!
//! * `passes` —— `SOKO_STAGE_STATS=1` 的 `STAGE_STATS` 行 ✓ = **Σ 闭包模块编译次数**
//!   （实测：5 个文件 × 3 个入口 = **15** ✓ —— 即每个入口把**整份项目**各编一遍 ✗）；
//! * `files` —— `build --json` 的 `build.summary` ✓ = **去重后的模块数** ✓。
//!
//! ⇒ **倍率 `passes / files`** 就是"共享依赖被编了几遍" ✓：理想 ≈ **1** ✓，
//! 现在 = **入口数** ✗。
//!
//! ## 夹具
//!
//! `lib/A` ← `lib/B` ← 4 个入口（每个入口 `import` 两个库 ✓）。**共享依赖只有 2 个** ✓，
//! 所以"编几遍"与入口数直接相关 ✓ —— 合成夹具能在**一秒内**跑完 ✓（真课程要 4 分钟 ✗，
//! 而且课程仓可以分开检出 ✗）。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 入口数。每个入口 `import` 整条**链式**库（`A ← B ← C ← D` ✓）⇒ 每个入口的闭包
/// 都是 5 个模块（4 库 + 自己 ✓）⇒ 倍率 ≈ `E × 5 / (E + 4)` ✓（E=6 ⇒ ≈ **3.5×** ✗）。
const ENTRIES: usize = 6;

/// 共享库的**链**（每个 `import` 上一个 ✓）：闭包更深 ⇒ "按入口各编一遍"的代价更大 ✓，
/// 判据的余量也更清楚 ✓。
const LIBS: &[(&str, &str)] = &[
    ("lib/A.sokonanoda", "def A.id (α : Type) (a : α) : α := a\n"),
    (
        "lib/B.sokonanoda",
        "import A\n\ndef B.wrap (α : Type) (a : α) : α := A.id α a\n",
    ),
    (
        "lib/C.sokonanoda",
        "import B\n\ndef C.twice (α : Type) (a : α) : α := B.wrap α (B.wrap α a)\n",
    ),
    (
        "lib/D.sokonanoda",
        "import C\n\ndef D.thrice (α : Type) (a : α) : α := C.twice α (B.wrap α a)\n",
    ),
];

/// **判据阈值：多一个入口的边际 pass 数 ≤ 1.5** ✓。
///
/// 理想是 **1.0**（新入口只编它自己 ✓ —— 共享库在项目里已经编过了 ✓）；留 0.5 余量 ✓。
/// 现在实测 ≈ **3.0** ✗（入口把自己那条闭包又走了一遍 ✗）。
///
/// ⚠ **为什么不用 `passes / files` 这个比值** ✗：反向验证（入口数 = 1）实测
/// `passes=13 / files=5 = **2.60×**` ✗ —— 说明 `passes` 里有一大块**与入口数无关**的
/// 常数开销（预检/收尾之类 ✓）⇒ 比值会被它稀释 ✗，**量不出「按入口重编」** ✓。
/// 边际量（两个规模各跑一次、相减 ✓）把这个常数**消掉** ✓ ⇒ 才是这条缺口的读数 ✓。
const MAX_MARGINAL: f64 = 1.5;

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-g68-recompile-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, text).expect("write file");
}

/// 跑一次 `build --json`，返回 `(stderr, stdout)`。
///
/// **必须开 `SOKO_STAGE_STATS=1`** ✓（`passes` 只在那行里 ✓）；缓存目录指到自己的
/// 临时目录 ✓（同 `imports.rs` 的理由：互不干扰 ✓）。
fn build(dir: &Path) -> (String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .arg("build")
        .arg("--json")
        .arg(dir)
        .env("SOKO_STAGE_STATS", "1")
        .env("SOKONANODA_CACHE_DIR", dir.join(".cache"))
        .env_remove("SOKONANODA_NO_CACHE")
        .stdin(Stdio::null())
        .output()
        .expect("spawn sokonanoda");
    (
        String::from_utf8_lossy(&output.stderr).into_owned(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// 从 `STAGE_STATS` 行取一个 `key=<数字>` 字段（取不到就 panic —— 判据不许静默退化 ✗）。
fn stage_field(stderr: &str, key: &str) -> u64 {
    let line = stderr
        .lines()
        .filter(|l| l.starts_with("STAGE_STATS "))
        .next_back()
        .unwrap_or_else(|| panic!("stderr 里没有 `STAGE_STATS ` 行：\n{stderr}"));
    let needle = format!("{key}=");
    let at = line
        .find(&needle)
        .unwrap_or_else(|| panic!("`STAGE_STATS` 行里没有 `{key}=`：{line}"));
    let rest = &line[at + needle.len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("`{key}=` 后面不是数字：{line}"))
}

/// 从 `build.summary` 事件取 `files=`。
fn summary_files(stdout: &str) -> u64 {
    let line = stdout
        .lines()
        .find(|l| l.contains("\"type\":\"build.summary\""))
        .unwrap_or_else(|| panic!("stdout 里没有 `build.summary`：\n{stdout}"));
    let at = line.find("\"files\":").expect("summary 里必须有 files");
    let rest = &line[at + "\"files\":".len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().expect("files 必须是数字")
}

/// **项目 `build` 不许把共享依赖按入口各编一遍**。
///
/// 判据 = **边际**：`(passes(N) − passes(1)) / (N − 1) ≤ 1.5` ✓ ——
/// 即"多一个入口只该多编**它自己**" ✓；现在 ≈ 3.0 ✗（把那条闭包又走了一遍 ✓）。
#[test]
fn a_project_build_must_not_recompile_shared_deps_once_per_entry() {
    let (one, files_one) = build_with("one", 1);
    let (many, files_many) = build_with("many", ENTRIES);
    assert_eq!(
        files_one,
        (1 + LIBS.len()) as u64,
        "夹具前提：1 入口的模块数"
    );
    assert_eq!(
        files_many,
        (ENTRIES + LIBS.len()) as u64,
        "夹具前提：{ENTRIES} 入口的模块数"
    );
    let marginal = (many - one) as f64 / (ENTRIES - 1) as f64;
    println!(
        "PERF g68 marginal-passes: passes(1)={one} passes({ENTRIES})={many} \
         files={files_many} marginal={marginal:.2}（阈值 {MAX_MARGINAL}）"
    );
    assert!(
        marginal <= MAX_MARGINAL,
        "项目 build 把共享依赖**按入口各编了一遍** ✗：多一个入口多 **{marginal:.2}** 次 \
         pass（判据 ≤ {MAX_MARGINAL}，理想 1.0）—— passes(1)={one} ⇒ passes({ENTRIES})={many} ✓\n\
         G-68：缓存键是 per-entry-closure ⇒ 共享依赖在每个入口里各编一遍。"
    );
}

/// 建一个"`entries` 个入口 + 同一条库链"的工程并跑一次，返回 `(passes, files)`。
fn build_with(tag: &str, entries: usize) -> (u64, u64) {
    let dir = tmp_dir(tag);
    write(&dir, "sokonanoda.toml", "[project]\nname = \"g68\"\n");
    for (path, text) in LIBS {
        write(&dir, path, text);
    }
    for i in 0..entries {
        write(
            &dir,
            &format!("e{i}.sokonanoda"),
            &format!(
                "import A\nimport B\nimport C\nimport D\n\n\
                 theorem e{i}_t (α : Type) (a : α) : D.thrice α a = a := rfl\n"
            ),
        );
    }
    let (stderr, stdout) = build(&dir);
    (stage_field(&stderr, "passes"), summary_files(&stdout))
}
