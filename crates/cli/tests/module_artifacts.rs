//! **刀 2（模块产物的磁盘层）在 CLI 边界的判据**：**跨进程**复用 · 完整性三道 · 逃生门。
//!
//! 契约 = `docs/design/module-artifacts.md` **§8.1–§8.4**（存哪 / 完整性 / 安全模型 /
//! 离线降级）。实现侧在 `crates/front/src/project/artifacts.rs` + `project/session.rs`
//! （T1-B 批 2/3）；**本文件补的是"真进程"那一层**。
//!
//! ## 为什么必须有一条 CLI 层的真进程判据（既有守卫够不着这里）
//!
//! | 既有守卫 | 它量的是 | 够不着什么 |
//! |---|---|---|
//! | `crates/front/src/project/artifacts.rs` 单测 | `read`/`write` 的三道本身 | **没人消费**它 —— "读出来当不存在"≠"会话真的回退了" ✗ |
//! | `crates/front/tests/t1b_module_artifacts.rs` | 同一进程里清掉线程局部检查点 | 那只是"另一个进程"的**等价物**（它自己这么写 ✓）；**真进程**的冷启动没人量 ✗ |
//! | `crates/lsp/tests/lsp_artifact_warmup.rs` | LSP 那条路（`reuse=artifact` / `modules`） | CLI 那条路（`build <dir>` 的**多入口会话**）✗ |
//! | `crates/cli/tests/artifacts.rs` | **项目**产物 `<模块根>/.sokonanoda/compiled/` | **模块**产物 `<模块根>/.sokonanoda/artifacts/` ✗（两者同格式不同层 ✓） |
//!
//! ⇒ 本文件的判据全部走 **spawn 真进程**（`build <dir>`），且每臂都是**新进程** ✓。
//!
//! ## 读数为什么是 `by_calls`（结构计数）
//!
//! `AGENTS.md` 判据纪律②：**不许用绝对毫秒**（同一份代码在共享 runner 上量到过
//! 44ms ↔ 2431ms ✗）。夹具的**库层恰好 1 个 `by` 证明**、两个入口**都是项风格**（0 个 `by`）
//! ⇒ `SOKO_STAGE_STATS=1` 的 `by_calls` 就是"**库层被 elaborate 了几次**"的直接读数 ✓
//! （与 `scripts/check-recompile-factor.py` 同一条纪律、同一把尺子 ✓）。
//!
//! ## ⚠ 每臂都必须先丢掉**报告缓存**（`compiled/`）
//!
//! 这是本判据最容易假绿的地方（实测踩过 ✗）：`build <dir>` 会先查**项目产物**
//! `<模块根>/.sokonanoda/compiled/<digest>.json` —— 它一命中就**整趟跳过**，
//! 模块产物（`artifacts/`）**根本没被咨询** ⇒ 两臂读数都变成 `hit:3`、`by_calls` 恒 0 ✗
//! ⇒ "跨进程复用"变成空转 ✗。⇒ 每臂跑之前 `rm -rf <root>/.sokonanoda/compiled`
//! （**只丢报告层、留住产物层** ✓），`--json` 的 `build.summary` 因此每臂都是
//! `hit:0` ⇒ 各臂的 stdout 才**逐字节可比** ✓。
//!
//! ## 逃生门必须自己控制
//!
//! 继承来的 `SOKONANODA_NO_MODULE_ARTIFACTS=1` / `SOKONANODA_NO_PROJECT_ARTIFACTS=1`
//! （本地为绕开受限环境而给 gate 开的 ✓）会把**被测功能本身**关掉 ⇒ 判据假红 ✗。
//! 与 `crates/cli/tests/artifacts.rs` 同一条纪律：显式 `env_remove`，再由用例自己设回来 ✓。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// 库层：**恰好 1 个 `by` 证明** —— 判据的分度就是它 ✓（入口侧一个 `by` 都没有）。
const LIB_SRC: &str = "\
theorem shared_and (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
";

/// 入口：`import` 库 + **项风格**（0 个 `by`）⇒ 入口那一趟不贡献读数 ✓。
const ENTRY_SRC: &str = "\
import lib.Shared

theorem {name}_thm (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := shared_and P Q hp hq
";

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-knife2-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).expect("create scratch");
    std::fs::write(dir.join("sokonanoda.toml"), "name = \"knife2\"\n").unwrap();
    std::fs::write(dir.join("lib/Shared.sokonanoda"), LIB_SRC).unwrap();
    for name in ["A", "B"] {
        std::fs::write(
            dir.join(format!("{name}.sokonanoda")),
            ENTRY_SRC.replace("{name}", &name.to_lowercase()),
        )
        .unwrap();
    }
    dir
}

/// 产物目录（`<模块根>/.sokonanoda/artifacts/`，设计 §8.1）。
fn artifacts(root: &Path) -> PathBuf {
    root.join(".sokonanoda").join("artifacts")
}

/// 目录里的文件（**排序**后返回 ⇒ 可逐字节比较 ✓；目录不在 ⇒ 空 ✓）。
fn files(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<PathBuf> = read
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    names.sort();
    names
}

/// 唯一的一对产物：`(<key>.bin, <key>.meta.json)`（判据要能指名"那一份产物" ✓）。
fn pair(root: &Path) -> (PathBuf, PathBuf) {
    let all = files(&artifacts(root));
    let bins: Vec<PathBuf> = all
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "bin"))
        .cloned()
        .collect();
    let metas: Vec<PathBuf> = all
        .iter()
        .filter(|p| p.to_string_lossy().ends_with(".meta.json"))
        .cloned()
        .collect();
    assert_eq!(
        (bins.len(), metas.len()),
        (1, 1),
        "夹具应当恰好一对产物（设计 §8.1）：{all:?}"
    );
    (bins[0].clone(), metas[0].clone())
}

/// 载荷（`<key>.bin`）。
fn payload(root: &Path) -> PathBuf {
    pair(root).0
}

/// 凭据（`<key>.meta.json`）。
fn credential(root: &Path) -> PathBuf {
    pair(root).1
}

/// 把 `meta.json` 的 `bytes` 字段**加一**（凭据与载荷不符 —— 设计 §8.2 第②道的前半 ✓）。
///
/// **为什么要这一手**：翻载荷的字节**通常**会先把 NDJSON 解析弄坏 ⇒ 回落是"解析失败"
/// 挡下来的，**证明不了摘要那一道有牙** ✗（实测：把第②道整段拆掉，翻字节那一臂
/// 仍然绿 ✗ —— 咬不住的守卫等于没有）。改凭据则**载荷完好可解析** ⇒ 若不命中，
/// 只可能是第②道拦下的 ✓。
fn bump_meta_bytes(meta: &str) -> String {
    const FIELD: &str = "\"bytes\":";
    let at = meta.find(FIELD).expect("凭据里必须有 `bytes` 字段");
    let start = at + FIELD.len();
    let end = meta[start..]
        .find(|c: char| !c.is_ascii_digit())
        .map(|i| start + i)
        .expect("`bytes` 后面必须跟数字");
    let value: u64 = meta[start..end].parse().expect("`bytes` 必须是数字");
    format!("{}{}{}", &meta[..start], value + 1, &meta[end..])
}

struct Run {
    stdout: String,
    /// `SOKO_STAGE_STATS=1` 的 `STAGE_STATS` 行里 `by_calls` = 库层被 elaborate 的次数 ✓。
    by_calls: u64,
    /// 同一行的 `passes`（诊断用；判据不依赖它 ✓）。
    passes: u64,
}

fn stat_field(stderr: &str, key: &str) -> u64 {
    let line = stderr
        .lines()
        .find(|line| line.starts_with("STAGE_STATS"))
        .unwrap_or_else(|| panic!("`SOKO_STAGE_STATS=1` 必须打一行 STAGE_STATS：{stderr}"));
    line.split_whitespace()
        .find_map(|field| field.strip_prefix(&format!("{key}=")))
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("STAGE_STATS 行里没有可解析的 `{key}=`：{line}"))
}

/// **一个真·冷进程**跑一次 `build --json <模块根>`。
///
/// * 先丢掉**报告缓存**（`compiled/`）—— 否则它一命中就整趟跳过，模块产物根本不被咨询 ✗
///   （见文件头的「最容易假绿的地方」✓）；**产物层（`artifacts/`）原样留着** ✓；
/// * 缓存目录每臂一份**新**的（`SOKONANODA_CACHE_DIR`）⇒ 全局缓存也不参与 ✓；
/// * `SOKO_STAGE_STATS=1` ⇒ `by_calls` 读数 ✓。
fn build(root: &Path, tag: &str, env: &[(&str, &str)]) -> Run {
    let _ = std::fs::remove_dir_all(root.join(".sokonanoda").join("compiled"));
    let cache = std::env::temp_dir().join(format!(
        "sokonanoda-knife2-cache-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&cache);
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(["build", "--json"])
        .arg(root)
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKO_STAGE_STATS", "1")
        // 被测功能**就是**模块产物 ⇒ 继承来的逃生门必须摘掉（再由 `env` 设回来 ✓）。
        .env_remove("SOKONANODA_NO_MODULE_ARTIFACTS")
        .env_remove("SOKONANODA_NO_PROJECT_ARTIFACTS")
        .env_remove("SOKONANODA_NO_CACHE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let out = command
        .spawn()
        .expect("spawn sokonanoda")
        .wait_with_output()
        .expect("wait sokonanoda");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "`build <dir>` 必须成功（stderr：{stderr}）");
    Run {
        by_calls: stat_field(&stderr, "by_calls"),
        passes: stat_field(&stderr, "passes"),
        stdout,
    }
}

/// ⭐ **判据 1（跨进程复用命中）** —— 同一模块连续两次**冷进程**编译，第二次命中产物。
///
/// 三臂（同一夹具、同一二进制、同一命令）：
///
/// | 臂 | 前置 | 期望 |
/// |---|---|---|
/// | ① 冷 | 模块根干净（无产物） | `by_calls = cold ≥ 1`（夹具真的跑了库层 `by` ✓）+ 写出 `.bin`/`.meta.json` ✓ |
/// | ② 冷进程 | **新进程**、新缓存目录、只丢 `compiled/` | `by_calls < cold`（库层没重 elaborate ⇒ 产物被用上了 ✓）+ **stdout 逐字节相同** ✓ |
/// | ③ 逃生门 | 同 ②，另加 `SOKONANODA_NO_MODULE_ARTIFACTS=1` | `by_calls == cold`（**回到基线** ✓）+ stdout 逐字节相同 ✓ |
///
/// ③ 是 ② 的**反向验证**：若产物从来没被用上，② 与 ① 会一样忙 ⇒ ② 判红 ✓；
/// 而 ③ 证明"② 少的那些活"**确实来自产物这条开关**，不是别的偶然 ✓。
#[test]
fn a_second_cold_process_reuses_the_library_artifact() {
    let root = scratch("reuse");

    // ── ① 冷：模块根干净 ⇒ 整条库层重编 + 写出产物 ──────────────────────
    let cold = build(&root, "cold", &[]);
    assert!(
        cold.by_calls >= 1,
        "夹具的库层必须真的跑过 `by`（否则判据没牙）：by_calls={} · passes={}",
        cold.by_calls,
        cold.passes
    );
    let pairs = files(&artifacts(&root));
    assert!(
        pairs
            .iter()
            .any(|p| p.extension().is_some_and(|e| e == "bin"))
            && pairs.iter().any(|p| p
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".meta.json"))),
        "冷编之后必须落下 `.bin` + `.meta.json`（设计 §8.1）：{pairs:?}"
    );
    let good = std::fs::read(payload(&root)).expect("read payload");

    // ── ② 冷进程（新进程 / 新缓存目录 / 只丢报告缓存）⇒ 必须命中产物 ────
    let warm = build(&root, "warm", &[]);
    assert!(
        warm.by_calls < cold.by_calls,
        "第二个**冷进程**必须命中磁盘产物 ⇒ 库层不再 elaborate（冷 {} vs 命中 {}）—— \
         若两者相等 ⇒ 产物根本没被用上 ✗",
        cold.by_calls,
        warm.by_calls
    );
    assert_eq!(
        warm.stdout, cold.stdout,
        "命中产物那条路的 `--json` 必须与冷跑**逐字节相同**（判定红线，设计 §5 判据 3）"
    );
    assert_eq!(
        std::fs::read(payload(&root)).expect("read payload"),
        good,
        "命中 ⇒ **不重写**产物（内容寻址：同名必然同内容 ✓）—— 变了说明这一臂其实又写了一遍 ✗"
    );

    // ── ③ 反向验证：逃生门关掉模块产物 ⇒ 读数回到基线 ────────────────────
    let escape = build(&root, "escape", &[("SOKONANODA_NO_MODULE_ARTIFACTS", "1")]);
    assert_eq!(
        escape.by_calls, cold.by_calls,
        "`SOKONANODA_NO_MODULE_ARTIFACTS=1` ⇒ **回到今天**（整条库层重编）—— \
         若它仍低于冷跑 ⇒ 逃生门没关掉那条路 ✗"
    );
    assert_eq!(
        escape.stdout, cold.stdout,
        "逃生门只该改**快慢**，绝不该改判定 ⇒ `--json` 必须逐字节相同"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// ⭐ **判据 2 + 3（完整性校验 · 篡改一个字节 ⇒ 必须拒绝并回落）** —— **反向验证**。
///
/// 两臂，**逐字节各改一处**（设计 §8.2 第②道的两半）：
///
/// | 臂 | 改哪 | 若不拦会怎样 |
/// |---|---|---|
/// | ④a **载荷** | `<key>.bin` **正中间翻一位**（长度不变） | 摘要不符却照用 ⇒ 坏数据进内核 ✗ |
/// | ④b **凭据** | `<key>.meta.json` 的 `bytes` **加一** | 载荷**完好可解析**、只是与凭据不符 ⇒ 照用 ✗ |
///
/// ④b 是**有牙的那一臂**：④a 翻字节常常会先把 NDJSON 解析弄坏（⇒ 回落由"解析失败"挡下，
/// **证明不了摘要那一道** ✗ —— 实测把第②道整段拆掉，只留 ④a 时本用例**仍然绿** ✗）。
/// ④b 的载荷**一个字节都没动** ⇒ 只有第②道能拦 ✓。
///
/// 每臂期望：① `by_calls` 回到冷跑水平（拒绝使用 + 回落重编 ✓）；② `--json` 与冷跑
/// **逐字节相同**（拒绝使用**不该**改判定 ✓ —— 与 §8.4「离线降级 = 只是慢，不是错」同口径 ✓）；
/// ③ 回落后**重写出一份完好的**（坏字节/坏凭据不许留在盘上 ✓）。
#[test]
fn a_tampered_payload_is_rejected_and_the_run_falls_back() {
    let root = scratch("tamper");
    let cold = build(&root, "cold", &[]);
    let bin = payload(&root);
    let meta = credential(&root);
    let good = std::fs::read(&bin).expect("read payload");
    let good_meta = std::fs::read(&meta).expect("read meta");

    // ── ④a 载荷翻一位 ────────────────────────────────────────────────────
    let mut bytes = good.clone();
    let at = bytes.len() / 2;
    bytes[at] ^= 0x01;
    assert_ne!(bytes, good, "夹具必须真的改动了载荷（否则判据空转 ✗）");
    std::fs::write(&bin, &bytes).expect("tamper payload");

    let after = build(&root, "tamper-bin", &[]);
    assert_eq!(
        after.by_calls, cold.by_calls,
        "被篡改的**载荷**必须被拒绝（设计 §8.2 第②道）⇒ 回落重编 ⇒ 读数回到冷跑水平 \
         （冷 {} vs 篡改后 {}）",
        cold.by_calls, after.by_calls
    );
    assert_eq!(
        after.stdout, cold.stdout,
        "回落重编的 `--json` 必须与冷跑逐字节相同（降级只是慢，不是错 ✓）"
    );
    assert_eq!(
        std::fs::read(&bin).expect("read payload"),
        good,
        "回落重编之后必须**重写出一份完好的**载荷（坏字节不许留在盘上 ✗）"
    );

    // ── ④b 凭据翻一位（`bytes` 加一；载荷**完好可解析**）──────────────────
    let text = String::from_utf8(good_meta.clone()).expect("meta 是文本");
    let patched = bump_meta_bytes(&text);
    assert_ne!(patched, text, "夹具必须真的改动了凭据（否则判据空转 ✗）");
    std::fs::write(&meta, &patched).expect("tamper meta");

    let after = build(&root, "tamper-meta", &[]);
    assert_eq!(
        after.by_calls, cold.by_calls,
        "凭据与载荷不符（`bytes` 对不上）⇒ **必须**当不存在（设计 §8.2 第②道）\
         （冷 {} vs 篡改后 {}）—— 这一臂的载荷完好可解析 ⇒ 若仍命中，只可能是第②道没牙 ✗",
        cold.by_calls, after.by_calls
    );
    assert_eq!(
        after.stdout, cold.stdout,
        "凭据被篡改也只该改**快慢**，不该改判定 ⇒ `--json` 必须逐字节相同"
    );
    assert_eq!(
        std::fs::read(&meta).expect("read meta"),
        good_meta,
        "回落重编之后必须**重写出一份完好的**凭据（坏凭据不许留在盘上 ✗）"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// ⭐ **设计 §5 判据 1（改依赖 ⇒ 必须 miss 且重编）**：库层源文本变一个字 ⇒
/// 它的键变 ⇒ 旧产物**再也匹配不上** ✓（§8.2 第③道"键可重算"）。
///
/// 改的是**惰性**的一行（只加注释）⇒ 判定逐字节不变 ✓、而**产物键必须换** ✓
/// —— 这两条同时成立才说明"键真的折进了源文本"，而不是"反正结果一样就复用了" ✗。
#[test]
fn a_changed_dependency_source_invalidates_the_key() {
    let root = scratch("invalidate");
    let cold = build(&root, "cold", &[]);
    let warm = build(&root, "warm", &[]);
    assert!(
        warm.by_calls < cold.by_calls,
        "前置：这一臂必须先命中（否则下面的 miss 无从判起 ✗）"
    );

    // 库层源文本**惰性**变一行（注释 ⇒ 判定的输入没变、Merkle 键的输入变了 ✓）。
    let lib = root.join("lib/Shared.sokonanoda");
    let mut text = std::fs::read_to_string(&lib).expect("read lib");
    text.push_str("-- 惰性改一行（注释）：只该让键失效，不该改判定\n");
    std::fs::write(&lib, text).expect("write lib");

    let after = build(&root, "changed", &[]);
    assert_eq!(
        after.by_calls, cold.by_calls,
        "库层源文本变了 ⇒ 键变 ⇒ **必须** miss 重编（设计 §5 判据 1）\
         （冷 {} vs 改后 {}）—— 若仍是命中的读数 ⇒ 键没折进源文本 ✗",
        cold.by_calls, after.by_calls
    );
    assert_eq!(
        after.stdout, cold.stdout,
        "惰性改动 ⇒ 判定必须逐字节不变（改前/改后同一个 `--json`）"
    );
    assert_eq!(
        files(&artifacts(&root))
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "bin"))
            .count(),
        2,
        "内容寻址 ⇒ 新旧两份产物**并存**（文件名即内容寻址，设计 §8.1）：{:?}",
        files(&artifacts(&root))
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// **`clean` 一并清产物**（`AGENTS.md`：`clean` = **只清不编**）。
///
/// **屏幕上会多/少什么**：`sokonanoda clean <dir>` 之后 `<模块根>/.sokonanoda/artifacts/`
/// 里**一个文件都不剩**（`.gitignore` 留着 —— 它是自忽略的锚，不是产物 ✓）。
#[test]
fn clean_removes_the_module_artifacts_without_compiling() {
    let root = scratch("clean");
    let cold = build(&root, "cold", &[]);
    assert!(cold.by_calls >= 1, "前置：冷跑必须真的编过");
    assert!(
        !files(&artifacts(&root)).is_empty(),
        "前置：冷跑之后产物目录必须非空"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
        .args(["clean", "--json"])
        .arg(&root)
        .env_remove("SOKONANODA_NO_MODULE_ARTIFACTS")
        .env_remove("SOKONANODA_NO_PROJECT_ARTIFACTS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sokonanoda clean")
        .wait_with_output()
        .expect("wait sokonanoda clean");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "`clean` 必须成功：{stdout}"
    );
    assert!(
        files(&artifacts(&root)).is_empty(),
        "`clean` 必须把模块产物一并清掉（设计 §8.1 / `AGENTS.md`）：{:?}",
        files(&artifacts(&root))
    );
    // **只清不编**：清完之后目录里**没有新产物**（编了就会有 ✗）。
    assert!(
        files(&root.join(".sokonanoda").join("compiled")).is_empty(),
        "`clean` = 只清不编 ⇒ 报告缓存也必须空着（不许顺手预热 ✗）"
    );

    let _ = std::fs::remove_dir_all(&root);
}
