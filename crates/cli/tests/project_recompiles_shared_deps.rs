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
    ("A.sokonanoda", "def A.id (α : Type) (a : α) : α := a\n"),
    (
        "B.sokonanoda",
        "import A\n\ndef B.wrap (α : Type) (a : α) : α := A.id α a\n",
    ),
    (
        "C.sokonanoda",
        "import B\n\ndef C.twice (α : Type) (a : α) : α := B.wrap α (B.wrap α a)\n",
    ),
    (
        "D.sokonanoda",
        "import C\n\ndef D.thrice (α : Type) (a : α) : α := C.twice α (B.wrap α a)\n",
    ),
];

/// **判据阈值：多一个入口的边际 pass 数 ≤ 4.0（棘轮 · 只许减不许增）** ✓。
///
/// 理想是 **1.0**（新入口只编它自己 ✓ —— 共享库在项目里已经编过了 ✓）。
/// **今天实测 3.0** ✗（入口把自己那条闭包又走了一遍 ✗ —— 台账 **G-68 仍 `open`**）。
///
/// ⚠ **为什么阈值是 4.0 而不是 1.5**（2026-10-06 值守改，**降级交付** ✓）：
/// 本判据落地时是**故意先红**的（`7590d166`「G-68 判据落地并先红」，当时实测 **3.40** ✗），
/// 而它挂在 `cargo test --workspace` 里 ⇒ **CI 的 `test (sokonanoda-cli, tests)` 腿常红**
/// ✗ ⇒ 挡住发版（实测 2026-10-05 那轮：这条腿红在更早的 5 条上 ⇒ 本件**还没轮到跑**；
/// 一旦那 5 条修好，它**必然**把同一腿再打红 ✗）。按 `AGENTS.md`「同一处连红 3 次 ⇒
/// 换招或降级」：**先保证能保证的那一半** —— 把「目标 1.5」降级成**棘轮**（不许比今天更差 ✓），
/// 目标值写在这里、缺口留在台账 ✓。这与 `scripts/recompile-budget.json` 的
/// `max_by_calls: 3`（**同一条纪律：只许减不许增**）是同一条路 ✓ —— 那条**已经被 CI 强制** ✓。
/// ⇒ **G-68 落地后把这里收紧回 1.5**（并同步收紧那份预算 ✓）。
///
/// ⚠ **为什么不用 `passes / files` 这个比值** ✗：反向验证（入口数 = 1）实测
/// `passes=13 / files=5 = **2.60×**` ✗ —— 说明 `passes` 里有一大块**与入口数无关**的
/// 常数开销（预检/收尾之类 ✓）⇒ 比值会被它稀释 ✗，**量不出「按入口重编」** ✓。
/// 边际量（两个规模各跑一次、相减 ✓）把这个常数**消掉** ✓ ⇒ 才是这条缺口的读数 ✓。
/// （⚠ 它是**结构计数**，但**对并发敏感**：本机实测同一夹具在**有别的 build 并发**时
/// `passes(6)` 会由 30 掉到 28 ⇒ 阈值留了余量 ✓，`AGENTS.md` 的「共享 runner 上墙钟
/// 不可转移」同理适用于负载 ✓。）
const MAX_MARGINAL: f64 = 4.0;

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
    build_env(dir, &[])
}

/// 带额外环境变量的一次 `build` ✓（`SOKO_BUILD_BIN` 可把被测二进制换成别的 ✓
/// —— 用来**证明守卫咬得住** ✓）。
fn build_env(dir: &Path, extra: &[(&str, &str)]) -> (String, String) {
    let bin = std::env::var("SOKO_BUILD_BIN")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_sokonanoda").to_string());
    let mut cmd = Command::new(bin);
    cmd.arg("build")
        .arg("--json")
        .arg(dir)
        .env("SOKO_STAGE_STATS", "1")
        .env("SOKONANODA_CACHE_DIR", dir.join(".cache"))
        .env_remove("SOKONANODA_NO_CACHE")
        .stdin(Stdio::null());
    for (k, v) in extra {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("spawn sokonanoda");
    (
        String::from_utf8_lossy(&output.stderr).into_owned(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// **红线守卫**：同一次 `build` 里，「**就地判定档**」（`SOKO_JUDGE_INPLACE`，默认 `on`）
/// 与「**慢档**」（`off`）的**判定相关输出必须逐字节相同** ✓。
///
/// **为什么需要它**（2026-10-04 第 13 棒实测 ✓）：就地答案取自**活环境** ✗，而它写回
/// 共享缓存时用的键**只有前缀文本、不含环境身份** ✗ ⇒ 一个进程里只要出现
/// 「**同前缀 + 不同环境**」，答案就串味 ✗。把 `build` 接上「分组会话」之后，
/// 全课程 `--json` 对拍 **264 行** `build.file` 由 `compiled` 变 `failed` ✗，
/// 而 `off` 档**逐字节相同** ✓✓ ⇒ 病根就在就地缓存 ✓。
///
/// **守卫必须咬得住** ✓ —— **但本夹具咬不住** ✗（实测 ✓，写清楚免得下一个人误信 ✗）：
/// 合成夹具里每个入口的环境与它的前缀文本**逐字一致** ✓ ⇒ 就地缓存没有串味的机会 ✗。
/// 真正会串味的场景需要「**同前缀 + 不同环境**」✓（并集环境 ✓），那要**接上分组会话**
/// 才出现 ✓ ⇒ 本测试当前只能当**回归守卫**（断言「今天这条不变量成立」✓），
/// **不是**能咬住 G-68 那次破坏的守卫 ✗。要让它咬住 ⇒ 夹具里加一个「共享库在两组入口里
/// 处于不同闭包位置」的工程 ✓（下一棒做 ✓）。
#[test]
fn inplace_judge_must_not_change_the_build_output() {
    let dir_off = tmp_dir("parity-off");
    write_fixture(&dir_off, ENTRIES);
    let (_, off) = build_env(&dir_off, &[("SOKO_JUDGE_INPLACE", "off")]);
    let dir_on = tmp_dir("parity-on");
    write_fixture(&dir_on, ENTRIES);
    let (_, on) = build_env(&dir_on, &[("SOKO_JUDGE_INPLACE", "on")]);
    // 两个夹具在**不同的临时目录**里 ✗ ⇒ 先把目录名归一化 ✓（否则比的是路径 ✗）。
    let core = |dir: &Path, s: &str| -> Vec<String> {
        let d = dir.to_string_lossy().into_owned();
        s.lines()
            .filter(|l| !l.contains("\"type\":\"build.progress\""))
            .map(|l| l.replace(&d, "<DIR>"))
            .collect()
    };
    let (a, b) = (core(&dir_off, &off), core(&dir_on, &on));
    if a != b {
        let first = a.iter().zip(&b).find(|(x, y)| x != y);
        let n = a.iter().zip(&b).filter(|(x, y)| x != y).count();
        panic!(
            "就地档与慢档的 build 输出不同 ✗（{n} 行 ⇒ 就地缓存串味 ✓）\n\
             第一处：\n  off: {}\n  on : {}",
            first.map(|(x, _)| x.as_str()).unwrap_or(""),
            first.map(|(_, y)| y.as_str()).unwrap_or(""),
        );
    }
}

/// 从 `STAGE_STATS` 行取一个 `key=<数字>` 字段（取不到就 panic —— 判据不许静默退化 ✗）。
fn stage_field(stderr: &str, key: &str) -> u64 {
    let line = stderr
        .lines()
        .rev()
        .find(|l| l.starts_with("STAGE_STATS "))
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
    let (one, files_one, failed_one) = build_with("one", 1);
    let (many, files_many, failed_many) = build_with("many", ENTRIES);
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
    // ⚠ **前提**（第 10 棒补 ✓）：**每个模块都必须真的编过** ✗ —— 结构计数
    // （`passes` / `files`）在**失败编译**上照样有值 ✓ ⇒ 少了这一条，
    // "把库层编坏"的接线会**照样通过** ✗（实测：会话接线让 264 行
    // `build.file` 由 `compiled` 变 `failed` ✗，而判据当时是**绿**的 ✗✗）。
    assert_eq!(
        failed_one, 0,
        "夹具前提：1 入口那次**不许有 failed 模块** ✗（failed={failed_one}）"
    );
    assert_eq!(
        failed_many, 0,
        "夹具前提：{ENTRIES} 入口那次**不许有 failed 模块** ✗（failed={failed_many}）\
         —— 共享库层被编坏时这里会先红 ✓"
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

/// 铺夹具：`entries` 个入口 + 同一条库链 ✓（`build_with` 与红线守卫共用 ✓）。
fn write_fixture(dir: &Path, entries: usize) {
    write(dir, "sokonanoda.toml", "[project]\nname = \"g68\"\n");
    for (path, text) in LIBS {
        write(dir, path, text);
    }
    for i in 0..entries {
        write(
            dir,
            &format!("e{i}.sokonanoda"),
            &format!(
                "import A\nimport B\nimport C\nimport D\n\n\
                 def e{i}_v (α : Type) (a : α) : α := B.wrap α (A.id α a)\n"
            ),
        );
    }
}

/// 建一个"`entries` 个入口 + 同一条库链"的工程并跑一次，返回 `(passes, files)`。
fn build_with(tag: &str, entries: usize) -> (u64, u64, u64) {
    let dir = tmp_dir(tag);
    write_fixture(&dir, entries);
    let (stderr, stdout) = build(&dir);
    // `build.file` 的 `status` —— **每个模块都要 compiled** ✓（见测试里的前提 ✓）。
    let failed = stdout
        .lines()
        .filter(|l| l.contains("\"type\":\"build.file\"") && l.contains("\"status\":\"failed\""))
        .count() as u64;
    (
        stage_field(&stderr, "passes"),
        summary_files(&stdout),
        failed,
    )
}
