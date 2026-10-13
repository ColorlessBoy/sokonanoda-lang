//! **§8.3 闸门 + §3 内容寻址产物键 —— CLI 边界的判据**（键拆分与信任面）。
//!
//! 契约 = `docs/design/module-artifacts.md` **§3**（per-module Merkle 键：`format · 版本 ·
//! build stamp · prelude 模式 · 源文本 · 依赖的 (name,key)`）· **§8.1**（文件名即内容寻址
//! ⇒ 天然可共享）· **§8.3**（安全模型：**默认关** · 清单 `artifacts.lock.json` · 不静默）。
//!
//! ## 本文件存在的理由：**上一轮实测出来的硬阻塞**
//!
//! 上一轮（`1f054f9b`/`bfa153ec`）实测：产物键折进**每个模块的绝对路径**
//! （`session.rs` 的 `lib_key` → `push_lib_key_text`，T-A06 **故意**的 ✓）⇒ 两份**逐字节相同**
//! 的项目放在两个绝对路径下拿到**两把不同的键** ⇒ **下载来的产物永远过不了 §8.2 第三道**
//! ⇒ 下载路径**按构造命中不了** ✗。
//!
//! ⚠ 而**只把路径从键里拿掉**会开出 §8.3 明令要关的信任口子 ✗：内容寻址**挡不住伪造**
//! （键由**输入**算出，不校验产物**内容**是否真由这份输入编出来）⇒ 任何放进制品目录的
//! 外来产物都会被吃 ✗。⇒ **键拆分与闸门必须同一轮落，缺一不可** ✓。
//!
//! ## 落地形状（一句话）
//!
//! | 闸门 `SOKONANODA_SHARED_ARTIFACTS` | 产物层的键 | 能吃到谁 |
//! |---|---|---|
//! | **关（默认 ✓）** | `lib_key`（**含绝对路径 + `SOKO_*`**，T-A06 的语义） | 只可能是**这个根、这个环境**写出来的 ✓ |
//! | 开（显式同意） | `lib_key_content`（**无路径、无环境**，§3） | **清单允许**的那几份（§8.3 ✓） |
//!
//! 检查点那一侧的键（线程局部 LRU 的身份）**恒为 `lib_key`** —— 本环节**一个字没动**它 ✓。
//!
//! ## 读数为什么是 `by_calls`（结构计数）
//!
//! `AGENTS.md` 判据纪律②：**不许用绝对毫秒**（同一份代码在共享 runner 上量到过
//! 44ms ↔ 2431ms ✗）。夹具与隔壁 `module_artifacts.rs` **逐字相同**（库层恰好 1 个 `by`
//! 证明、两个入口都是项风格）⇒ `SOKO_STAGE_STATS=1` 的 `by_calls` 就是"**库层被 elaborate
//! 了几次**"的直接读数：冷 / 未命中 = **2**，命中产物 = **1** ✓。
//!
//! ## 每臂都必须先丢掉**报告缓存**（`compiled/`）
//!
//! 与两个隔壁文件同一条纪律（实测踩过 ✗）：`build <dir>` 先查**项目产物**
//! `<模块根>/.sokonanoda/compiled/` —— 它一命中就整趟跳过，模块产物**根本没被咨询**
//! ⇒ `by_calls` 恒 0、两臂读数分不开 ✗。⇒ 每臂跑之前 `rm -rf <root>/.sokonanoda/compiled`
//! （**只丢报告层、留住产物层** ✓）。
//!
//! ## 逃生门必须自己控制
//!
//! 继承来的 `SOKONANODA_NO_MODULE_ARTIFACTS=1` / `SOKONANODA_NO_PROJECT_ARTIFACTS=1`
//! （本地为绕开受限环境给 gate 开的 ✓）会把**被测功能本身**关掉 ⇒ 判据假红 ✗；
//! 同理**必须** `env_remove("SOKONANODA_SHARED_ARTIFACTS")` —— 否则"默认关"那一臂
//! 会因为外面设了它而**假红**（或反之**假绿** ✗✗）。每条判据自己把它设回来 ✓。

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

/// 闸门环境变量（设计 §8.3 的"默认关"开关）。
const GATE: &str = "SOKONANODA_SHARED_ARTIFACTS";

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sokonanoda-shared-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

/// 一份模块根：`sokonanoda.toml` + `lib/Shared` + 两个入口（与隔壁夹具**逐字相同** ✓）。
///
/// ⚠ **内容里不含任何路径**（模块名/定理名都是固定的 ✓）⇒ 两次调用得到的两份是
/// **逐字节相同**的项目，只有**绝对路径**不同 —— 这正是本文件要的那个自变量 ✓。
fn scratch(tag: &str) -> PathBuf {
    let dir = unique(tag);
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

/// 产物目录里的**载荷**文件名（`.bin` 的**词干** = 产物键 ✓），排序后返回 ✓。
fn keys(root: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(artifacts(root))
        .map(|read| {
            read.flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "bin") {
                        path.file_stem()
                            .map(|stem| stem.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// 目录里**恰好一把**产物键（"冷跑一次之后应有的形状" ✓）。
fn only_key(root: &Path) -> String {
    let found = keys(root);
    assert_eq!(found.len(), 1, "应当恰好一把产物键：{found:?}");
    found[0].clone()
}

/// 凭据里的 `(digest, bytes)`（清单要记的就是这两个 ✓）。
fn credential(root: &Path, key: &str) -> (String, usize) {
    let text = std::fs::read_to_string(artifacts(root).join(format!("{key}.meta.json")))
        .expect("凭据必须存在");
    let digest = text
        .split("\"digest\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("凭据里必须有 digest")
        .to_string();
    let bytes: usize = text
        .split("\"bytes\":")
        .nth(1)
        .and_then(|rest| {
            rest.split(|c: char| !c.is_ascii_digit())
                .next()
                .map(str::to_string)
        })
        .and_then(|value| value.parse().ok())
        .expect("凭据里必须有 bytes");
    (digest, bytes)
}

/// 写一份清单（§8.3：`{version, artifacts:[{key,digest,bytes}]}` ✓）。
fn write_lock(root: &Path, version: u32, body: &str) {
    std::fs::write(
        root.join("artifacts.lock.json"),
        format!("{{\"version\":{version},\"artifacts\":[{body}]}}\n"),
    )
    .unwrap();
}

/// 一条**合法**的清单条目。
fn lock_entry(key: &str, digest: &str, bytes: usize) -> String {
    format!("{{\"key\":\"{key}\",\"digest\":\"{digest}\",\"bytes\":{bytes}}}")
}

/// 把 `from` 的产物对**搬进** `to`（= 模拟"下载来的产物躺在制品目录里" ✓）。
fn plant(from: &Path, to: &Path, key: &str) {
    let _ = std::fs::remove_dir_all(artifacts(to));
    std::fs::create_dir_all(artifacts(to)).unwrap();
    for suffix in ["bin", "meta.json"] {
        std::fs::copy(
            artifacts(from).join(format!("{key}.{suffix}")),
            artifacts(to).join(format!("{key}.{suffix}")),
        )
        .unwrap();
    }
}

/// 把载荷中间的一个字节翻掉（§8.2 第②道的前件 ✓）。
fn flip_payload(root: &Path, key: &str) {
    let path = artifacts(root).join(format!("{key}.bin"));
    let mut bytes = std::fs::read(&path).unwrap();
    let at = bytes.len() / 2;
    bytes[at] ^= 0x01;
    std::fs::write(&path, bytes).unwrap();
}

struct Run {
    stdout: String,
    /// `SOKO_STAGE_STATS=1` 的 `STAGE_STATS` 行里 `by_calls` = 库层被 elaborate 的次数 ✓。
    by_calls: u64,
    /// `artifacts:` 来源行的条数（§8.3「不静默」✓）。
    source_lines: usize,
}

/// **一个真·冷进程**跑一次 `build --json <模块根>`（与隔壁同形 ✓）。
fn build(root: &Path, tag: &str, env: &[(&str, &str)]) -> Run {
    let _ = std::fs::remove_dir_all(root.join(".sokonanoda").join("compiled"));
    let cache = unique(&format!("cache-{tag}"));
    let _ = std::fs::remove_dir_all(&cache);
    let mut command = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    command
        .args(["build", "--json"])
        .arg(root)
        .env("SOKONANODA_CACHE_DIR", &cache)
        .env("SOKO_STAGE_STATS", "1")
        // 被测功能**就是**产物/闸门 ⇒ 继承来的开关必须全部摘掉（再由 `env` 设回来 ✓）。
        .env_remove("SOKONANODA_NO_MODULE_ARTIFACTS")
        .env_remove("SOKONANODA_NO_PROJECT_ARTIFACTS")
        .env_remove("SOKONANODA_NO_CACHE")
        .env_remove(GATE)
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
        .unwrap();
    assert!(
        out.status.success(),
        "`build <dir>` 必须 exit 0（{tag}）：{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let line = stderr
        .lines()
        .find(|line| line.starts_with("STAGE_STATS"))
        .unwrap_or_else(|| panic!("`SOKO_STAGE_STATS=1` 必须打一行 STAGE_STATS：{stderr}"));
    let by_calls = line
        .split_whitespace()
        .find_map(|field| field.strip_prefix("by_calls="))
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("STAGE_STATS 行里没有可解析的 `by_calls=`：{line}"));
    Run {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        by_calls,
        source_lines: stderr
            .lines()
            .filter(|line| line.starts_with("artifacts: "))
            .count(),
    }
}

/// **整份夹具**：`alpha` 冷跑一次（闸门**开** ⇒ 写出**内容寻址**的那一份）·
/// 把那一对**原样搬进** `beta`（`beta` 自己**从没**编过）⇒ 返回三元组。
///
/// `beta` 全程只当"**下载方**"：它手上的产物**只可能**来自 alpha ✓。
fn fixture() -> (PathBuf, PathBuf, String, String, usize) {
    let alpha = scratch("alpha");
    // 闸门开 ⇒ 产物层的键 = §3 的内容寻址键 ✓（读数：这一步 alpha 自己必然 cold ✓）。
    let run = build(&alpha, "alpha", &[(GATE, "1")]);
    assert_eq!(run.by_calls, 2, "alpha 自己是冷的（两个入口各一趟库层）");
    let key = only_key(&alpha);
    let (digest, bytes) = credential(&alpha, &key);
    let beta = scratch("beta");
    plant(&alpha, &beta, &key);
    (alpha, beta, key, digest, bytes)
}

// ───────────────────────────────────────────────────────────────────────────
// 判据 ①：**闸门默认关** ⇒ 外来产物**默认不被吃**（§8.3 的红线 ✓）
// ───────────────────────────────────────────────────────────────────────────

/// **反向验证**（实测做在 `shared_artifacts_enabled()`：把 `env_flag_on(GATE)` 改成恒 `true`
/// ⇒ 本判据**当场判红**：`左 2 · 右 1` ⇒ 它咬住的正是"默认关"这一个事实 ✓）。
#[test]
fn a_foreign_artifact_is_not_eaten_while_the_gate_is_off() {
    let (_alpha, beta, key, digest, bytes) = fixture();
    // 先记一条**对照**：清单**已经**允许它（唯一变量就只剩闸门 ✓）。
    write_lock(&beta, 1, &lock_entry(&key, &digest, bytes));
    let off = build(&beta, "off", &[]);
    assert_eq!(
        off.by_calls, 2,
        "闸门默认关 ⇒ 内容寻址的那份产物**根本不在候选里**（§8.3）"
    );
    assert_eq!(off.source_lines, 0, "没命中就不该报来源行");
    // 把闸门打开（**同一份夹具、同一个清单**）⇒ 必须命中 ⇒ 证明上面那臂不是空转 ✓。
    let on = build(&beta, "on", &[(GATE, "1")]);
    assert_eq!(
        on.by_calls, 1,
        "同一份产物：开了闸门就命中（否则本判据没牙）"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// 判据 ②：**跨 root 命中**（键拆分的收益）· 清单是必需的 · `--json` 逐字节不变
// ───────────────────────────────────────────────────────────────────────────

/// **反向验证**（实测做在 `read_lib_artifact()`：把 `read_shared` 换回 `read`
/// —— 即"**只看 §8.2、不看清单**" ⇒ 本判据的**第 2 臂当场判红**：`左 2 · 右 1`
/// ⇒ 它咬住的正是"清单必须真的拦"这一格 ✓）。
#[test]
fn a_manifest_backed_artifact_hits_across_roots() {
    let (_alpha, beta, key, digest, bytes) = fixture();

    // ① **基线**：没有清单 ⇒ **fail-closed**（不是"打开就什么都吃" ✗）。
    let no_lock = build(&beta, "no-lock", &[(GATE, "1")]);
    assert_eq!(
        no_lock.by_calls, 2,
        "闸门开了但没有清单 ⇒ 一份都不许吃（§8.3 的清单是**必需**的）"
    );
    assert_eq!(no_lock.source_lines, 0);

    // ② 清单允许它 ⇒ **跨 root 命中**（beta 的键 == alpha 写出来的那个文件名 ✓）。
    write_lock(&beta, 1, &lock_entry(&key, &digest, bytes));
    let hit = build(&beta, "hit", &[(GATE, "1")]);
    assert_eq!(
        hit.by_calls, 1,
        "清单允许 ⇒ 库层**只 elaborate 一次**（跨 root 复用的直接读数）"
    );
    // ③ §8.3「不静默」：**用了**就报一行来源（谁给的、哪个 key ✓）。
    assert_eq!(hit.source_lines, 1, "命中必须报**恰好一行**来源（§8.3）");

    // ④ 错编红线：命中那趟的 `--json` 与"本地老老实实重编"那趟**逐字节相同** ✓。
    //
    // 这条不是"锦上添花"：产物 = **直接注入内核的声明** ⇒ 命中路径与冷路径只要差一个
    // 字节，就是"同一份源码、两种判定"✗。两侧都先丢了 `compiled/`（上面 `build` 里 ✓）
    // ⇒ `build.summary` 都是 `hit:0` ⇒ 两份 stdout 才逐字节可比 ✓。
    let cold = build(&beta, "cold", &[]);
    assert_eq!(cold.by_calls, 2);
    assert_eq!(
        hit.stdout, cold.stdout,
        "命中外来产物那趟的 `--json` 必须与本地重编**逐字节相同**"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// 判据 ③：清单**被篡改 ⇒ 拒绝**（§8.3 第 3/5 条：清单里记每个 key 的产物摘要 ✓）
// ───────────────────────────────────────────────────────────────────────────

/// **反向验证**（实测做在 `artifacts::lock_allows()`：把 `||` 那三条件里**任一**去掉
/// ⇒ 对应的那一臂**当场判红**（`左 2 · 右 1`）✓ —— 五臂各咬住清单的一格 ✓）。
#[test]
fn a_tampered_manifest_is_refused() {
    let (_alpha, beta, key, digest, bytes) = fixture();

    // **对照臂**（必须先绿）：这份清单是好的 ⇒ 命中 ✓ ⇒ 下面五臂才不是空转 ✓。
    write_lock(&beta, 1, &lock_entry(&key, &digest, bytes));
    assert_eq!(build(&beta, "control", &[(GATE, "1")]).by_calls, 1);

    let mut tampered = vec![
        // ① 摘要改一位（"清单里的摘要"就是防伪造的那一层 ✓）。
        (
            "digest-one-nibble",
            lock_entry(&key, "0000000000000000", bytes),
        ),
        // ② 字节数不符。
        ("bytes-off-by-one", lock_entry(&key, &digest, bytes + 1)),
        // ③ 清单说的是**别的** key（"允许 A"不等于"允许 B" ✓）。
        ("other-key", lock_entry("deadbeefdeadbeef", &digest, bytes)),
    ]
    .into_iter()
    .map(|(tag, body)| (tag, 1, body))
    // ④ 不认的清单格式号（与产物 `format` 同一条纪律：不认就当**空** ✓）。
    .chain(std::iter::once((
        "unknown-version",
        2,
        lock_entry(&key, &digest, bytes),
    )))
    .collect::<Vec<_>>();

    for (tag, version, body) in tampered.drain(..) {
        let fresh = scratch(&format!("tamper-{tag}"));
        plant(&beta, &fresh, &key);
        write_lock(&fresh, version, &body);
        let run = build(&fresh, tag, &[(GATE, "1")]);
        assert_eq!(
            run.by_calls, 2,
            "篡改后的清单必须**拒绝**（{tag}）：不许命中"
        );
        assert_eq!(run.source_lines, 0, "被拒就不该报来源行（{tag}）");
    }

    // ⑤ 清单**根本读不出来**（JSON 截断 —— 下载器被杀的形状 ✓）⇒ 同样 fail-closed ✓。
    let broken = scratch("tamper-broken-json");
    plant(&beta, &broken, &key);
    std::fs::write(
        broken.join("artifacts.lock.json"),
        "{\"version\":1,\"artifacts\":[",
    )
    .unwrap();
    let run = build(&broken, "broken-json", &[(GATE, "1")]);
    assert_eq!(run.by_calls, 2, "清单解析不了 ⇒ 一份都不许吃");
}

// ───────────────────────────────────────────────────────────────────────────
// 判据 ④：产物**对不上自己的凭据 ⇒ 拒绝**（§8.2 第②道在共享档下**仍然**有牙 ✓）
// ───────────────────────────────────────────────────────────────────────────

/// 把凭据的 `bytes` 字段**加一**（载荷**完好可解析**、清单**完全合法**）。
///
/// **为什么必须用这一手而不是"翻载荷一个字节"**（隔壁 `module_artifacts.rs` 踩过并写下了 ✓）：
/// 翻载荷字节**通常**会先把 NDJSON 解析弄坏 ⇒ 回落是"解析失败"挡下的，
/// **证明不了第②道有牙** ✗（实测：把第②道整段拆掉，翻字节那一臂**仍然绿** ✗）。
/// 改凭据则载荷一字未动 ⇒ 若不命中，**只可能**是第②道拦下的 ✓。
fn bump_credential_bytes(root: &Path, key: &str) -> usize {
    let path = artifacts(root).join(format!("{key}.meta.json"));
    let text = std::fs::read_to_string(&path).unwrap();
    const FIELD: &str = "\"bytes\":";
    let at = text.find(FIELD).expect("凭据里必须有 `bytes` 字段") + FIELD.len();
    let end = text[at..]
        .find(|c: char| !c.is_ascii_digit())
        .map(|i| at + i)
        .expect("`bytes` 后面必须跟数字");
    let value: usize = text[at..end].parse().expect("`bytes` 必须是数字");
    std::fs::write(
        &path,
        format!("{}{}{}", &text[..at], value + 1, &text[end..]),
    )
    .unwrap();
    value + 1
}

/// 把凭据的 `digest` 换成另一串**形状合法**的十六进制（载荷一字未动 ✓）。
fn swap_credential_digest(root: &Path, key: &str, digest: &str) {
    let path = artifacts(root).join(format!("{key}.meta.json"));
    let text = std::fs::read_to_string(&path).unwrap();
    let swapped = text.replace(
        &format!("\"digest\":\"{digest}\""),
        "\"digest\":\"0000000000000000\"",
    );
    assert_ne!(swapped, text, "凭据里的 digest 必须真的被换掉了");
    std::fs::write(&path, swapped).unwrap();
}

/// **反向验证**（实测 · M4）：把 `read_pair()` 第②道那半段**整段失效**（用
/// `if digest_hex(&bytes) == "\u{0}never" {` —— ⚠ **必须**仍然"用到" `digest_hex`，
/// 否则本 crate 的 `-D warnings` 会先把"未用导入"判成**编译错误** ✗ ⇒ 那是**假红**，
/// 证明不了判据有牙 —— **实测踩过**）⇒ 本判据**当场判红**（`左 2 · 右 1`）✓。
///
/// ⚠ **两臂的清单必须"同意"被篡改后的凭据**（`bytes+1` / `"0000…"`）—— 否则拦下它们的
/// 可能是 §8.3 的清单对账（那是判据 ③ 的地方 ✗），**这一格就白测了**（第②道有没有牙分不
/// 出来 ✗）。清单同意之后，**唯一**还能拦的就是 §8.2 第②道 ✓。
#[test]
fn an_artifact_that_does_not_match_its_credential_is_refused() {
    let (_alpha, beta, key, digest, bytes) = fixture();
    // **对照臂**（必须先绿）：凭据与载荷一致 + 清单允许 ⇒ 命中 ✓ ⇒ 下面三臂不是空转 ✓。
    let control = scratch("credential-control");
    plant(&beta, &control, &key);
    write_lock(&control, 1, &lock_entry(&key, &digest, bytes));
    assert_eq!(
        build(&control, "credential-control", &[(GATE, "1")]).by_calls,
        1
    );

    // ① 凭据的 `bytes` 加一（载荷**完好** ⇒ 拦下它的只可能是第②道 ✓）；
    //    清单**同步**改成加一后的值 ⇒ 清单那一层同意 ⇒ 只剩第②道能拦 ✓。
    let bumped = scratch("credential-bytes");
    plant(&beta, &bumped, &key);
    write_lock(&bumped, 1, &lock_entry(&key, &digest, bytes + 1));
    bump_credential_bytes(&bumped, &key);
    let run = build(&bumped, "credential-bytes", &[(GATE, "1")]);
    assert_eq!(
        run.by_calls, 2,
        "载荷字节数对不上凭据 ⇒ 当**不存在**（§8.2 ②）"
    );
    assert_eq!(run.source_lines, 0);

    // ② 凭据的 `digest` 换成别的（同上：载荷没动 ✓、清单同步同意 ✓）。
    let swapped = scratch("credential-digest");
    plant(&beta, &swapped, &key);
    write_lock(&swapped, 1, &lock_entry(&key, "0000000000000000", bytes));
    swap_credential_digest(&swapped, &key, &digest);
    let run = build(&swapped, "credential-digest", &[(GATE, "1")]);
    assert_eq!(
        run.by_calls, 2,
        "载荷摘要对不上凭据 ⇒ 当**不存在**（§8.2 ②）"
    );
    assert_eq!(run.source_lines, 0);

    // ③ 载荷被翻了一个字节（清单与凭据都合法 ⇒ 拦它的是第②道**或**解析 —— 兜底臂 ✓）。
    let flipped = scratch("credential-flipped");
    plant(&beta, &flipped, &key);
    write_lock(&flipped, 1, &lock_entry(&key, &digest, bytes));
    flip_payload(&flipped, &key);
    let run = build(&flipped, "credential-flipped", &[(GATE, "1")]);
    assert_eq!(run.by_calls, 2, "被改过的载荷 ⇒ 当**不存在**");
    assert_eq!(run.source_lines, 0);
}

// ───────────────────────────────────────────────────────────────────────────
// 判据 ⑤：**键拆分**本身 —— 产物键与 root 无关，检查点键与 root 有关（T-A06 不动 ✓）
// ───────────────────────────────────────────────────────────────────────────

/// **反向验证**（实测做在 `artifact_layer_key()`：把闸门那一支**忽略**（恒走 `lib_key`）
/// ⇒ 本判据的第 ① 臂**当场判红**（`alpha ≠ beta`）；反过来让 `lib_key` 也走内容键
/// ⇒ 第 ② 臂判红 ⇒ 两臂分别咬住"产物键去路径"与"检查点键守 T-A06" ✓）。
#[test]
fn the_content_key_is_root_independent_while_the_checkpoint_key_is_not() {
    // ① 闸门开：两份**逐字节相同**的项目 ⇒ **同一把产物键**（= 可共享/可下载 ✓）。
    let shared_a = scratch("share-a");
    let shared_b = scratch("share-b");
    build(&shared_a, "share-a", &[(GATE, "1")]);
    build(&shared_b, "share-b", &[(GATE, "1")]);
    assert_eq!(
        only_key(&shared_a),
        only_key(&shared_b),
        "§3 的产物键**无路径** ⇒ 两个 root 下逐字相同的项目必须落到**同一个名字**"
    );

    // ② 闸门关（默认）：同样两份 ⇒ **两把不同的键** —— T-A06 的语义原样保留 ✓
    //    （"内容相同不代表位置相同"：两个目录下逐字相同的项目**不许**共用 ✓）。
    let local_a = scratch("local-a");
    let local_b = scratch("local-b");
    build(&local_a, "local-a", &[]);
    build(&local_b, "local-b", &[]);
    assert_ne!(
        only_key(&local_a),
        only_key(&local_b),
        "闸门关 ⇒ 产物层仍用**含绝对路径**的检查点键（T-A06），两把键必须不同"
    );
}
