//! **刀 2 的「下载那一半」（设计 §8.3/§8.4）在 CLI 边界的判据** —— 离线降级 ·
//! 不完整产物 · 有界淘汰 · 两层逃生门。
//!
//! 契约 = `docs/design/module-artifacts.md` **§8.3**（安全模型：默认关 / 两层逃生门 /
//! 下载来的产物 = 新的信任面）与 **§8.4**（离线降级：**永不阻塞**、只是慢不是错）。
//! 本地那一半（§8.1/§8.2）的真进程判据在隔壁 `module_artifacts.rs`（`9de2bbbe` ✓）——
//! **本文件一个字都不改它** ✓，只补它够不着的那几格。
//!
//! ## 为什么这几格必须站在 CLI 边界上（既有守卫够不着）
//!
//! | 既有守卫 | 它量的是 | 够不着什么 |
//! |---|---|---|
//! | `crates/front/src/project/artifacts.rs` 单测 | `read`/`write`/`prune` 本身（含 `SOKO_T1B_NO_PRUNE` 那一臂 ✓） | **没人从命令行走一遍** —— "压回上界"≠"下一个冷进程还命中" ✗ |
//! | `crates/cli/tests/module_artifacts.rs` | 命中/篡改/失效/`clean`（`NO_MODULE_ARTIFACTS` 一臂 ✓） | **离线档** · **半对/半写凭据**（下载器被杀的形状）· **`NO_PROJECT_ARTIFACTS` 一并关** ✗ |
//! | `crates/cli/tests/artifacts.rs` | **项目**产物 `<模块根>/.sokonanoda/compiled/` | **模块**产物 `<模块根>/.sokonanoda/artifacts/` ✗ |
//!
//! ## ⚠ 下载那一半今天到哪一步（**如实写，不粉饰** ✓）
//!
//! * §8.3 的**传输**（`artifacts.lock.json` + 清单签名 + 按 tag 锁定 + 「用了就报一行
//!   来源」）**还没实现** ⇒ 本文件**不**假装验它：给没实现的东西写判据 = **咬不住的
//!   守卫** ✗（`AGENTS.md` 判据纪律①）。它落地后要补的判据写在队列条目
//!   `MODULE-ARTIFACTS-KNIFE2-DOWNLOAD` 里 ✓。
//! * 本文件验的是**已经存在、且下载器将来必须插在上面**的四格 —— 它们都是
//!   §8.3/§8.4 的**用户可见契约**，且每格都能被一处**具体突变**咬红（见各用例的
//!   「反向验证」段 ✓）：
//!   1. **§8.4 离线降级**：`SOKONANODA_OFFLINE=1` ⇒ **不阻塞**、**不改判定**、
//!      也**不许顺手关掉本地产物**（离线 = 跳过网络，不是跳过磁盘 ✓）；
//!   2. **§8.2「截断/损坏的下载当不存在」**：下载器被杀留下的**半对**（载荷孤零零）
//!      与**半写凭据**（JSON 截断）⇒ 当不存在 + 回落重编 + 重写一份完好的 ✓；
//!   3. **§8.1 有界**：超 64 对按 mtime 淘汰，且**刚写的那一对（`keep_key`）永不被
//!      自己淘汰** ⇒ 压回上界之后**下一个冷进程仍必须命中** ✓；
//!   4. **§8.3 两层逃生门**：`SOKONANODA_NO_MODULE_ARTIFACTS` **与**
//!      `SOKONANODA_NO_PROJECT_ARTIFACTS` **都**关掉模块层 ✓（后者是文档写死的
//!      契约，此前**没有一条判据站在 CLI 边界上钉它** ✗）。
//!
//! ## 读数为什么是 `by_calls`（结构计数）
//!
//! `AGENTS.md` 判据纪律②：**不许用绝对毫秒**（同一份代码在共享 runner 上量到过
//! 44ms ↔ 2431ms ✗）。夹具与隔壁 `module_artifacts.rs` **逐字相同**（库层恰好 1 个
//! `by` 证明、两个入口都是项风格）⇒ `SOKO_STAGE_STATS=1` 的 `by_calls` 就是
//! 「**库层被 elaborate 了几次**」的直接读数：冷 = 2（两个入口各一趟）、
//! 命中产物 = 1（只剩目录里那个没有 `import` 的库文件自己那趟 ✓）。
//!
//! ## ⚠ 每臂都必须先丢掉**报告缓存**（`compiled/`）
//!
//! 与隔壁同一条纪律（实测踩过 ✗）：`build <dir>` 先查**项目产物**
//! `<模块根>/.sokonanoda/compiled/` —— 它一命中就整趟跳过，模块产物
//! （`artifacts/`）**根本没被咨询** ⇒ 两臂读数都变成 `hit:3` ✗。⇒ 每臂跑之前
//! `rm -rf <root>/.sokonanoda/compiled`（**只丢报告层、留住产物层** ✓）。
//!
//! ## 逃生门必须自己控制
//!
//! 继承来的 `SOKONANODA_NO_MODULE_ARTIFACTS=1` / `SOKONANODA_NO_PROJECT_ARTIFACTS=1`
//! （本地为绕开受限环境而给 gate 开的 ✓）会把**被测功能本身**关掉 ⇒ 判据假红 ✗。
//! 与隔壁同一条纪律：显式 `env_remove`，再由用例自己设回来 ✓。
//!
//! ## 为什么 `SOKONANODA_OFFLINE` 那一臂**必须**有
//!
//! §8.4 只说"下载失败 / 超时 / 离线 ⇒ **静默退回本地编译**"。最像回归的误读是把它
//! 读成"离线 ⇒ 别碰产物"（把**网络**那道闸门当成**功能**开关 ✗）⇒ 那样一断网，
//! 连**自己刚编出来的**产物都不用了，用户看到的是"越断网越慢" ✗。判据 1 的臂 ③
//! 就是钉这一条：**离线时本地产物仍必须命中** ✓。

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

/// 入口：`import` 库 + **项风格**（0 个 `by`）⇒ 入口那两趟不贡献读数 ✓。
const ENTRY_SRC: &str = "\
import lib.Shared

theorem {name}_thm (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := shared_and P Q hp hq
";

/// 外来对的命名前缀（判据 3 靠它认"哪一对是**刚写出来的**" ✓）。
const FOREIGN: &str = "foreign";

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-knifeu-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).expect("create scratch");
    std::fs::write(dir.join("sokonanoda.toml"), "name = \"knifeu\"\n").unwrap();
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

/// 产物目录里的 `(<key>.bin, <key>.meta.json)` 两张清单（**同一个上界**管的是"对" ✓）。
fn pairs(root: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let all = files(&artifacts(root));
    let bins = all
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "bin"))
        .cloned()
        .collect();
    let metas = all
        .iter()
        .filter(|p| p.to_string_lossy().ends_with(".meta.json"))
        .cloned()
        .collect();
    (bins, metas)
}

/// 造 `count` 对外来产物（**内容随便**：判据只看"对数"与"活着的那一对" ✓），
/// mtime 一律钉到**远古** ⇒ 淘汰顺序确定，不靠文件系统的抖动 ✓。
fn plant_foreign_pairs(root: &Path, count: usize) {
    let dir = artifacts(root);
    std::fs::create_dir_all(&dir).expect("mkdir artifacts");
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(3600);
    for i in 0..count {
        let stem = format!("{FOREIGN}{i:04}");
        for name in [format!("{stem}.bin"), format!("{stem}.meta.json")] {
            let path = dir.join(&name);
            std::fs::write(
                &path,
                format!("外来产物 {name}（谁写的不重要，判据只看对数）\n"),
            )
            .expect("write foreign pair");
            let file = std::fs::File::options()
                .write(true)
                .open(&path)
                .expect("open foreign pair");
            file.set_modified(old).expect("set mtime");
        }
    }
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
///   （见文件头的「每臂都必须先丢掉报告缓存」✓）；**产物层（`artifacts/`）原样留着** ✓；
/// * 缓存目录每臂一份**新**的（`SOKONANODA_CACHE_DIR`）⇒ 全局缓存也不参与 ✓；
/// * `SOKO_STAGE_STATS=1` ⇒ `by_calls` 读数 ✓；
/// * 继承来的 `CARGO_TARGET_DIR`（受管环境的共享编译缓存）与被测行为无关 ⇒ 摘掉，
///   让这条判据在**任何**构建环境里读到同一件事 ✓（与队列纪律「对测试进程
///   `env -u CARGO_TARGET_DIR`」同一条 ✓）。
fn build(root: &Path, tag: &str, env: &[(&str, &str)]) -> Run {
    let _ = std::fs::remove_dir_all(root.join(".sokonanoda").join("compiled"));
    let cache = std::env::temp_dir().join(format!(
        "sokonanoda-knifeu-cache-{tag}-{}-{}",
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
        .env_remove("CARGO_TARGET_DIR")
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

/// 把一段文本**按字符边界**截一半（半写凭据的夹具；不许切出非法 UTF-8 ✓）。
fn halve(text: &str) -> String {
    let cut = (0..=text.len() / 2)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .expect("总有边界");
    text[..cut].to_string()
}

/// ⭐ **判据 1（§8.4 离线降级：永不阻塞 · 不改判定 · 也不许关掉本地产物）**。
///
/// 四臂（同一夹具、同一二进制、同一命令）：
///
/// | 臂 | 前置 | 期望 |
/// |---|---|---|
/// | ① 冷 | 模块根干净（无产物） | `by_calls = cold ≥ 1` + 落下一对 ✓ |
/// | ② 命中基线 | 只丢 `compiled/` | `by_calls < cold` + `--json` 逐字节相同 ✓ |
/// | ③ **离线** | 同 ②，另加 `SOKONANODA_OFFLINE=1` | **仍然命中**（`by_calls < cold`）✓ + `--json` 逐字节相同 ✓ |
/// | ④ **离线 + 无产物** | 产物目录整个删掉 | `by_calls == cold`（**回落本地基线** ✓）+ `--json` 逐字节相同 ✓ |
///
/// ③ 是**有牙的那一臂**：把"离线"误读成"别碰产物"（网络闸门当功能开关 ✗）就会判红。
/// ④ 钉 §8.4 的另一半：**离线只该慢，不该错** —— 读数回本地基线、判定逐字节不变 ✓。
///
/// **反向验证**（实测做在 `module_artifacts_enabled()`：让它在 `SOKONANODA_OFFLINE`
/// 存在时也返回 `false`）⇒ 臂 ③ 判红 ✓，跑完已还原（`grep` = 0 ✓）。
#[test]
fn offline_never_blocks_and_never_disables_local_artifacts() {
    let root = scratch("offline");

    // ── ① 冷：模块根干净 ⇒ 整条库层重编 + 写出产物 ──────────────────────
    let cold = build(&root, "cold", &[]);
    assert!(
        cold.by_calls >= 1,
        "夹具的库层必须真的跑过 `by`（否则判据没牙）：by_calls={} · passes={}",
        cold.by_calls,
        cold.passes
    );
    let (bins, metas) = pairs(&root);
    assert_eq!(
        (bins.len(), metas.len()),
        (1, 1),
        "冷编之后必须恰好落一对 `.bin` + `.meta.json`（设计 §8.1）：{bins:?} / {metas:?}"
    );

    // ── ② 命中基线（不设 OFFLINE）⇒ 先证明"这条路上本来就能命中" ────────
    let warm = build(&root, "warm", &[]);
    assert!(
        warm.by_calls < cold.by_calls,
        "前置：不设 `SOKONANODA_OFFLINE` 时必须先命中（冷 {} vs 命中 {}）—— \
         若这里就不命中，臂 ③ 的读数无从判起 ✗",
        cold.by_calls,
        warm.by_calls
    );
    assert_eq!(
        warm.stdout, cold.stdout,
        "命中产物那条路的 `--json` 必须与冷跑**逐字节相同**（判定红线，设计 §5 判据 3）"
    );

    // ── ③ ⭐ 离线 ⇒ **仍然命中**（离线 = 跳过网络，不是跳过磁盘 ✓）────────
    let offline = build(&root, "offline", &[("SOKONANODA_OFFLINE", "1")]);
    assert!(
        offline.by_calls < cold.by_calls,
        "`SOKONANODA_OFFLINE=1` **不许**把本地产物一并关掉（设计 §8.4：离线只是「跳过网络」）\
         —— 实测 冷 {} vs 离线 {} ⇒ 若相等，说明离线档被读成了功能开关 ✗",
        cold.by_calls,
        offline.by_calls
    );
    assert_eq!(
        offline.stdout, cold.stdout,
        "离线只该改**快慢**，绝不该改判定 ⇒ `--json` 必须逐字节相同"
    );

    // ── ④ 离线 + 一点产物都没有（下载失败/断网那趟）⇒ 回落本地、判定不变 ──
    let _ = std::fs::remove_dir_all(artifacts(&root));
    let bare = build(&root, "offline-bare", &[("SOKONANODA_OFFLINE", "1")]);
    assert_eq!(
        bare.by_calls, cold.by_calls,
        "离线且无产物 ⇒ 必须**静默回落本地重编**（设计 §8.4「只是慢，不是错」）\
         （冷 {} vs 离线裸跑 {}）—— 不许阻塞、不许报错、不许改判定",
        cold.by_calls, bare.by_calls
    );
    assert_eq!(
        bare.stdout, cold.stdout,
        "回落重编的 `--json` 必须与冷跑逐字节相同（降级只是慢，不是错 ✓）"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// ⭐ **判据 2（§8.2「截断/损坏的下载当不存在」）**：下载器被杀留下的两种残骸。
///
/// | 臂 | 盘上留下什么 | 若不拦会怎样 |
/// |---|---|---|
/// | ② **半对**（凭据没到） | `<key>.bin` 孤零零一份，`<key>.meta.json` 没了 | "看着像产物"就用 ⇒ **没有凭据的载荷进内核** ✗ |
/// | ③ **半写凭据** | `<key>.meta.json` 被截断成非法 JSON | 半份凭据被当凭据 ⇒ 摘要/版本那两道**形同虚设** ✗ |
///
/// 每臂期望：① `by_calls` 回到冷跑水平（当不存在 + 回落重编 ✓）；② `--json` 与冷跑
/// **逐字节相同**（拒绝使用**不该**改判定 ✓ —— 与 §8.4 同口径）；③ 回落后**重写出一份
/// 完好的**（残骸不许留在盘上 ✗）。
///
/// **反向验证**（实测做在 `artifacts::read`：凭据读不出来时**跳过凭据**直接用载荷 ——
/// 下载器最容易写出的"快路"）⇒ 臂 ② 判红 ✓，跑完已还原（`grep` = 0 ✓）。
///
/// ⚠ **为什么不补"载荷截断"那一臂**：截断通常**先把 NDJSON 弄坏** ⇒ 回落是"解析失败"
/// 挡下的，**证明不了 §8.2 第②道有牙** ✗（隔壁 `module_artifacts.rs` 实测过同形的坑：
/// 翻载荷那一臂单独存在时，把第②道整段拆掉它**仍然绿** ✗）。要咬第②道，载荷必须
/// **完好可解析**而只有凭据对不上 —— 那一臂已经由隔壁的 `bump_meta_bytes` 钉住了 ✓，
/// 本文件不重复（**咬不住的守卫等于没有** ✓）。
#[test]
fn an_incomplete_or_half_written_pair_is_treated_as_absent() {
    let root = scratch("incomplete");
    let cold = build(&root, "cold", &[]);
    assert!(
        cold.by_calls >= 1,
        "夹具的库层必须真的跑过 `by`（否则判据没牙）"
    );
    let (bins, metas) = pairs(&root);
    assert_eq!(
        (bins.len(), metas.len()),
        (1, 1),
        "夹具自检：冷编之后恰好一对（实得 {} / {}）",
        bins.len(),
        metas.len()
    );
    let good_bin = std::fs::read(&bins[0]).expect("read payload");
    let good_meta = std::fs::read(&metas[0]).expect("read meta");

    // ── ② 半对：凭据**没到**（下载器写完载荷就被杀）────────────────────────
    std::fs::remove_file(&metas[0]).expect("remove meta");
    let after = build(&root, "no-meta", &[]);
    assert_eq!(
        after.by_calls, cold.by_calls,
        "载荷孤零零在盘上**不算**一份产物（设计 §8.2 第①道：凭据先于载荷）\
         （冷 {} vs 半对 {}）—— 若仍命中，说明「看着像产物就用」✗",
        cold.by_calls, after.by_calls
    );
    assert_eq!(
        after.stdout, cold.stdout,
        "把残骸当不存在只该改**快慢**，不该改判定 ⇒ `--json` 必须逐字节相同"
    );
    assert_eq!(
        std::fs::read(&bins[0]).expect("read payload"),
        good_bin,
        "回落重编之后载荷必须是完好的那一份（坏字节不许留在盘上 ✗）"
    );
    assert_eq!(
        std::fs::read(&metas[0]).expect("read meta"),
        good_meta,
        "回落重编之后必须**重写出一份完好的凭据**（半对不许留在盘上 ✗）"
    );

    // ── ③ 半写凭据：JSON 被截断（下载器写到一半被杀）──────────────────────
    let text = String::from_utf8(good_meta.clone()).expect("meta 是文本");
    let cut = halve(&text);
    assert_ne!(cut, text, "夹具必须真的截短了凭据（否则判据空转 ✗）");
    std::fs::write(&metas[0], &cut).expect("truncate meta");

    let after = build(&root, "short-meta", &[]);
    assert_eq!(
        after.by_calls, cold.by_calls,
        "半写的凭据**不是**凭据（设计 §8.2 第①/②道）\
         （冷 {} vs 半写 {}）—— 若仍命中，说明非法 JSON 被宽容掉了 ✗",
        cold.by_calls, after.by_calls
    );
    assert_eq!(
        after.stdout, cold.stdout,
        "半写凭据也只该改**快慢** ⇒ `--json` 必须逐字节相同"
    );
    assert_eq!(
        std::fs::read(&metas[0]).expect("read meta"),
        good_meta,
        "回落重编之后必须**重写出一份完好的凭据**（半写的凭据不许留在盘上 ✗）"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// ⭐ **判据 3（§8.1 有界：超 `MAX_ARTIFACT_PAIRS` 对按 mtime 淘汰）** —— 站在 CLI 边界上。
///
/// 用户每改一次库层就多一个新键（键 = 库层闭包摘要 ✓）⇒ 老产物**再也没人读**却一直
/// 占盘 ✗ ⇒ 实现里有一条"压回上界、最旧先走、**刚写的那一对永不被自己淘汰**"。
/// 本用例从命令行验穿**三件事**（单测只能验前两件 ✓）：
///
/// 1. 冷编之后对数**回到上界以内**（不是"以后会回收"，是**这一趟就回收** ✓）；
/// 2. **刚写的那一对活着**（`keep_key` ⇒ 不许被自己淘汰 ✗），且**不留半对**垃圾 ✓；
/// 3. ⭐ **压回上界之后，下一个冷进程仍必须命中它** —— "写成功"与"读得到"不许打架 ✗。
///
/// **反向验证**（实测用实现里既有的逃生门 `SOKO_T1B_NO_PRUNE=1`，它正是为这条判据留的 ✓）：
/// 70 对外来产物 + 冷编 ⇒ 不淘汰 ⇒ **71 对 > 64** ⇒ 判红 ✓，跑完已还原（该变量只在
/// 显式设置时生效，本用例从不设它 ✓）。
#[test]
fn the_store_stays_bounded_at_the_cli_boundary_and_keeps_the_live_pair() {
    let root = scratch("bounded");
    let total = sokonanoda_front::project::artifacts::MAX_ARTIFACT_PAIRS + 6;
    plant_foreign_pairs(&root, total);
    let planted = pairs(&root).0.len();
    assert_eq!(
        planted, total,
        "夹具自检：{total} 对外来产物必须真的落盘（实得 {planted}）"
    );

    // ── 冷编一趟（库层 miss ⇒ 一定写出一对新产物 ⇒ 顺带走一遍淘汰）────────
    let cold = build(&root, "cold", &[]);
    assert!(
        cold.by_calls >= 1,
        "夹具的库层必须真的跑过 `by`（否则这一趟不会写产物、淘汰也不会被触发 ✗）"
    );

    let (bins, metas) = pairs(&root);
    assert!(
        bins.len() <= sokonanoda_front::project::artifacts::MAX_ARTIFACT_PAIRS,
        "产物对数必须被压回上界 {} 以内（实得 {}）—— 否则用户每改一次库层就多一份、\
         永远不回收 ✗",
        sokonanoda_front::project::artifacts::MAX_ARTIFACT_PAIRS,
        bins.len()
    );
    assert_eq!(
        bins.len(),
        metas.len(),
        "淘汰的单位是**一对**（`.bin` + `.meta.json` 一起走 ✓）—— 不许留半对垃圾：\
         {} 个载荷 vs {} 个凭据",
        bins.len(),
        metas.len()
    );
    let live: Vec<&PathBuf> = bins
        .iter()
        .filter(|path| {
            !path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(FOREIGN))
        })
        .collect();
    assert_eq!(
        live.len(),
        1,
        "**刚写的那一对永不被自己淘汰**（`keep_key` ✓）—— 实得 {live:?}（外来对剩 {}）",
        bins.len() - live.len()
    );

    // ── ⭐ 有牙的那一条：被压回上界之后，**活着的那一对**仍必须被用上 ──────
    let warm = build(&root, "warm", &[]);
    assert!(
        warm.by_calls < cold.by_calls,
        "压回上界之后，活着的那一对**仍必须命中**（冷 {} vs 之后 {}）—— \
         若相等 ⇒ 淘汰把「写成功」与「读得到」弄打架了 ✗",
        cold.by_calls,
        warm.by_calls
    );
    assert_eq!(
        warm.stdout, cold.stdout,
        "淘汰只该改**快慢**，绝不该改判定 ⇒ `--json` 必须逐字节相同"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// **判据 4（§8.3 两层逃生门）**：`SOKONANODA_NO_MODULE_ARTIFACTS` **与**
/// `SOKONANODA_NO_PROJECT_ARTIFACTS` **都**必须关掉模块层（`AGENTS.md` 写死的契约：
/// "设上面那条**也**会一并关掉 ✓"）。
///
/// * 臂 ① 是对照组（隔壁 `module_artifacts.rs` 的臂 ③ 已经钉过它 ✓）—— 放这里是为了让
///   臂 ② 的读数**可解释**：同样的夹具、同样的前置，只有逃生门的名字不同 ✓；
/// * 臂 ② 是**此前没有任何 CLI 判据钉过**的那一格 ✗：`NO_PROJECT_ARTIFACTS` 是
///   **项目**产物（`compiled/`）的逃生门，但它必须**连带**关掉**模块**产物 ——
///   两个层同生共死，不许只关一半 ✗。
///
/// 两臂期望：① `by_calls` 回到冷跑水平（整条库层重编 ✓）；② `--json` 与冷跑逐字节相同
/// （逃生门只该改**快慢** ✓）；③ **不许再写新的产物**（只读不写才叫"关" ✓）。
///
/// **反向验证**（实测做在 `module_artifacts_enabled()`：让它只认
/// `SOKONANODA_NO_MODULE_ARTIFACTS`、忽略 `SOKONANODA_NO_PROJECT_ARTIFACTS`）⇒ 臂 ② 判红 ✓，
/// 跑完已还原（`grep` = 0 ✓）。
#[test]
fn both_escape_hatches_close_the_module_artifacts() {
    let root = scratch("hatches");
    let cold = build(&root, "cold", &[]);
    assert!(
        cold.by_calls >= 1,
        "夹具的库层必须真的跑过 `by`（否则判据没牙）"
    );
    let (bins, _) = pairs(&root);
    assert_eq!(
        bins.len(),
        1,
        "夹具自检：冷编之后恰好一对产物（实得 {}）",
        bins.len()
    );

    for (tag, hatch) in [
        ("no-module", "SOKONANODA_NO_MODULE_ARTIFACTS"),
        ("no-project", "SOKONANODA_NO_PROJECT_ARTIFACTS"),
    ] {
        let escape = build(&root, tag, &[(hatch, "1")]);
        assert_eq!(
            escape.by_calls, cold.by_calls,
            "`{hatch}=1` ⇒ **回到今天**（整条库层重编）（冷 {} vs 逃生门 {}）—— \
             若它仍低于冷跑 ⇒ 这条逃生门没关掉模块产物 ✗",
            cold.by_calls, escape.by_calls
        );
        assert_eq!(
            escape.stdout, cold.stdout,
            "`{hatch}` 只该改**快慢**，绝不该改判定 ⇒ `--json` 必须逐字节相同"
        );
    }

    let (bins, metas) = pairs(&root);
    assert_eq!(
        (bins.len(), metas.len()),
        (1, 1),
        "两层逃生门关掉之后**不许再写新的产物**（只读不写才叫「关」✓）：\
         {} 个载荷 / {} 个凭据",
        bins.len(),
        metas.len()
    );

    let _ = std::fs::remove_dir_all(&root);
}
