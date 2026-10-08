//! **A5 的判据**：**产物命中**的开档 ⇒ 诊断发出之后补一趟**后台库层预热** ⇒
//! 开档后的**第一次编辑**不再重编整条库闭包。
//!
//! ## 为什么需要这条守卫（开工 profiling 实测 · `PLAN-cli-editor-perf.md` §8）
//!
//! `set_cached_entry`（产物命中那条路）**一趟 pass 都不跑** ⇒ front 的线程局部
//! 库层检查点（G-29）是冷的 ⇒ **第一次编辑**把整条库闭包重编一遍：真课程 unit08
//! 实测同一刀 `modules=5 by=87` · **1233ms**，而检查点热的同一刀只要
//! `modules=1 by=63` · **321ms** ✗。用户看到的就是"打开很快、敲第一个字符卡一秒"。
//!
//! ## 判据（**结构计数**，与 debug/release 无关 ✓）
//!
//! 合成夹具（自足、不依赖 `courses/`）：
//!
//! ```text
//! lib/Shared.sokonanoda  恰好 1 个 `by` 证明（库层那趟的分度）
//! Main.sokonanoda        `import lib.Shared` + 自己一个 `by` 证明
//! ```
//!
//! | 步骤 | 期望 |
//! |---|---|
//! | ① 先冷编一次（**写产物**） | `modules >= 2`（库 + 入口） |
//! | ② 新进程开档（**产物命中**） | `modules == 0`（一个模块都没编 ⇒ 第一屏没被推迟 ✓） |
//! | ② 预热那行 | 出现且 `built=true` |
//! | ② 改一刀 | **`modules == 1`**（只剩入口；库层复用检查点 ✓） |
//! | ③ 反向验证（`SOKO_NO_LIB_WARMUP=1`） | 改同一刀 ⇒ **`modules == 2`**（库层又被重编 ✗） |
//! | ③ 两臂的**诊断逐字节相同** | 预热不改判定 ✓ |
//!
//! ⚠ **为什么不用墙钟**：`AGENTS.md` 判据纪律②（共享机器上同一份代码量到过
//! 44ms ↔ 2431ms）⇒ 这里一律读 `LSP_TRACE` 的**差量结构计数** ✓。
//!
//! ⚠ **为什么必须等预热那行再改**：真实用户的"打开 → 读 → 敲"之间有几秒；
//! 而**敲得太早**时服务端会**主动跳过**预热（`pending_version` 非空 ⇒ 那一趟
//! 由编辑自己的编译做）—— 那是"不更坏"的分支，不是这条用例要量的东西 ✓。

mod common;

use common::Client;
use std::path::PathBuf;

/// 共享库：**恰好一个 `by` 证明** —— 库层那趟的分度就是它。
const DEP_SRC: &str = "\
theorem shared_and (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
";

/// 入口：`import` 库 + 自己一个 `by` 证明（入口趟也有 `by` 工作）。
const ENTRY_SRC: &str = "\
import lib.Shared
theorem main_thm (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
  exact hq
";

/// 被量的那一刀：**只改入口的证明体**（等长、语义相同 ⇒ 诊断不变 ✓）。
const ENTRY_EDITED: &str = "\
import lib.Shared
theorem main_thm (P Q : Prop) (hp : P) (hq : Q) : P ∧ Q := by
  apply And.intro
  exact hp
   exact hq
";

/// 写夹具，返回 (模块根, 入口路径, 入口文本)。
fn fixture(name: &str) -> (PathBuf, PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-a5-warmup-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("lib")).expect("mkdir lib");
    std::fs::write(root.join("lib/Shared.sokonanoda"), DEP_SRC).expect("write lib");
    let entry = root.join("Main.sokonanoda");
    std::fs::write(&entry, ENTRY_SRC).expect("write entry");
    (root, entry, ENTRY_SRC.to_string())
}

/// 一份进程私有（但**共享模块根产物**）的缓存目录。
fn cache_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-a5-cache-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn trace_field_u64(line: &str, key: &str) -> u64 {
    Client::trace_field(line, key)
}

/// **判据主体**：产物命中的开档预热了库层检查点 ⇒ 第一次编辑只编入口。
#[test]
fn artifact_hit_open_warms_the_library_checkpoint() {
    let (root, entry, text) = fixture("main");
    let uri = Client::file_uri(&entry);

    // ① 冷编一次：把**产物**写进模块根（`.sokonanoda/compiled/`）。
    //    ⚠ 这一步不是被测对象 —— 它只是把"开档就命中产物"这个前提造出来 ✓。
    {
        let mut warmup = Client::start_traced(&cache_dir("seed"));
        let _ = warmup.open(&root, &uri, &text);
        let _ = warmup.wait_for_trace_after(0);
        let line = warmup.last_trace();
        assert!(
            trace_field_u64(&line, "modules") >= 2,
            "前提：冷编至少要编库层 + 入口两个模块（夹具写坏了？）\n  {line}"
        );
    }

    // ② 产物命中那一臂：开档 **modules=0** ⇒ 预热 ⇒ 改一刀只编入口。
    let mut hit = Client::start_traced(&cache_dir("hit"));
    let _ = hit.open(&root, &uri, &text);
    let _ = hit.wait_for_trace_after(0);
    let opened = hit.last_trace();
    assert_eq!(
        trace_field_u64(&opened, "modules"),
        0,
        "前提：这一步必须是**产物命中**（一个模块都不编）—— 产物没写成就量不到预热 ✗\n  {opened}"
    );

    let before = hit.warm_trace_len();
    let _ = hit.wait_for_warm_after(before);
    let warm = hit.last_warm_trace();
    assert!(
        warm.contains("built=true"),
        "产物命中的开档**必须**建一份新检查点（`built=true`）—— 预热没跑起来 ✗\n  {warm}"
    );

    let hit_diagnostics = hit.did_change(&uri, 2, ENTRY_EDITED);
    let _ = hit.wait_for_trace_after(1);
    let edited = hit.last_trace();
    assert_eq!(
        trace_field_u64(&edited, "modules"),
        1,
        "**A5 的正身**：检查点热了 ⇒ 这一刀只该编**入口**那一个模块（库层复用 ✓）；\
         现在是 2 ⇒ 库层又被整条重编了 ✗\n  {edited}"
    );

    // ③ 反向验证：撤掉预热 ⇒ 同一刀必须**回到 2 个模块**（咬得住的守卫 ✓）。
    //
    // ⚠ **先为这一档冷编一次**：`SOKO_*` 开关**整体进缓存键**（`compile::cache::flags_state`，
    // 既有设计 ✓）⇒ 带 `SOKO_NO_LIB_WARMUP=1` 的进程**不认**默认档写下的产物
    // （这是设计，不是本用例的漏洞）。要量"关掉预热后那一刀"就必须先让**这一档**
    // 有产物可命中 —— 否则量的其实是"冷开" ✗。
    {
        let mut seed_off =
            Client::start_traced_with_env(&cache_dir("seed-off"), &[("SOKO_NO_LIB_WARMUP", "1")]);
        let _ = seed_off.open(&root, &uri, &text);
        let _ = seed_off.wait_for_trace_after(0);
    }
    let mut off = Client::start_traced_with_env(&cache_dir("off"), &[("SOKO_NO_LIB_WARMUP", "1")]);
    let _ = off.open(&root, &uri, &text);
    let _ = off.wait_for_trace_after(0);
    assert_eq!(
        trace_field_u64(&off.last_trace(), "modules"),
        0,
        "前提：关掉预热的那一臂也必须是产物命中\n  {}",
        off.last_trace()
    );
    let off_diagnostics = off.did_change(&uri, 2, ENTRY_EDITED);
    let _ = off.wait_for_trace_after(1);
    let off_edited = off.last_trace();
    assert_eq!(
        trace_field_u64(&off_edited, "modules"),
        2,
        "**反向验证**：`SOKO_NO_LIB_WARMUP=1` 时这一刀必须回到 2 个模块（库层重编 ✗）—— \
         回到 1 说明预热**没被真的关掉**（逃生门是空转的 ✗）\n  {off_edited}"
    );
    assert_eq!(
        off.warm_trace_len(),
        0,
        "逃生门关掉时**不许**出现 warm-library 行：{}",
        off.last_warm_trace()
    );

    // ④ 预热**不改判定**：两臂同一刀的**诊断正文逐字节相同** ✓（预热只搬运功，
    //    不改答案 —— 这是判定红线在本用例上的落点）。
    assert_eq!(
        hit_diagnostics, off_diagnostics,
        "预热不许改变诊断（逐字节）\n  warm={hit_diagnostics}\n  off={off_diagnostics}"
    );
    // ⑤ **功只减不增**：预热把库层那趟搬到了开档之后 ⇒ 这一刀的 `by` 必须**严格更小**
    //    （warm=2 vs off=3 = 库层那个 `by` 证明）✓；`modules` 同理（1 vs 2，见上）。
    assert!(
        trace_field_u64(&edited, "by") < trace_field_u64(&off_edited, "by"),
        "预热**必须**把库层那趟的 `by` 功从这一刀里搬走（严格更小 ✗）\n  \
         warm={edited}\n  off={off_edited}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// **不更坏**：开档后**立刻**改（预热还没跑完）⇒ 服务端**跳过**预热，
/// 结构计数回到"今天"那条路（库层由编辑自己的编译建检查点）。
///
/// 这条钉的是 A5 的**安全性论证**（`PLAN-cli-editor-perf.md` §8.2）：
/// 库层趟是首次编辑本来就要做的功的**子集** ⇒ 编辑先到就跳过，功不增 ✓。
#[test]
fn typing_immediately_skips_the_warmup_instead_of_doubling_work() {
    let (root, entry, text) = fixture("immediate");
    let uri = Client::file_uri(&entry);
    {
        let mut seed = Client::start_traced(&cache_dir("seed-immediate"));
        let _ = seed.open(&root, &uri, &text);
        let _ = seed.wait_for_trace_after(0);
    }
    let mut fast = Client::start_traced(&cache_dir("immediate"));
    let _ = fast.open(&root, &uri, &text);
    let _ = fast.wait_for_trace_after(0);
    assert_eq!(
        trace_field_u64(&fast.last_trace(), "modules"),
        0,
        "前提：产物命中\n  {}",
        fast.last_trace()
    );
    // **不等预热那行**：立刻改（模拟"打开就敲"）。
    let _ = fast.did_change(&uri, 2, ENTRY_EDITED);
    let _ = fast.wait_for_trace_after(1);
    let line = fast.last_trace();
    let modules = trace_field_u64(&line, "modules");
    assert!(
        modules <= 2,
        "立刻改的那一刀**不许比今天更贵**（今天 = 库层 + 入口 = 2）✗\n  {line}"
    );
    assert!(
        fast.warm_trace_len() <= 1,
        "立刻改 ⇒ 预热**最多**跑一次（要么被跳过、要么刚好已经起了）✗\n  {}",
        fast.last_warm_trace()
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 单文件（**没有 `import`**）不许预热：没有库层可喂（预热必须自己判空 ✓）。
#[test]
fn single_file_documents_have_no_library_to_warm() {
    let root =
        std::env::temp_dir().join(format!("sokonanoda-lsp-a5-single-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("mkdir");
    let entry: PathBuf = root.join("Solo.sokonanoda");
    std::fs::write(&entry, "theorem solo : True := True.intro\n").expect("write");
    let uri = Client::file_uri(&entry);
    let mut client = Client::start_traced(&cache_dir("single"));
    let _ = client.open(&root, &uri, "theorem solo : True := True.intro\n");
    let _ = client.wait_for_trace_after(0);
    // 单文件：产物走**全局缓存**（不是项目产物）⇒ 这一趟本来就编了，没有预热可言。
    assert!(
        client.warm_trace_len() == 0,
        "单文件文档没有库层 ⇒ 不许出现 warm-library 行：{}",
        client.last_warm_trace()
    );
    let _ = std::fs::remove_dir_all(&root);
}
