//! **判据 ④（增量身份等价）+ 判据 ③（零退回原文）+ 判据 ②（不许 O(n²)）**
//! —— 2026-10-04 值守派单的复现件 ✓。
//!
//! ## 这一件钉的是什么
//!
//! `judge` 的缓存键用的是**前缀的环境身份**（`compile::canonical_prefix_id` ✓），
//! 而 walker 为了不 O(n²)（前缀逐命令变长 ⇒ 每命令重解析整份 ✗）会**边读边累加**
//! 出身份 ✓、用 `judge::seed_canonical_prefix` **预置**进表 ✓。**两条路必须逐位相等** ✗
//! —— 不等就是"键与真身份脱钩" = **错编的入口** ✗。
//!
//! ## 为什么必须**三条一起**断言（上一棒的教训 ✓）
//!
//! 上一棒只读"有没有 `MISMATCH`" ⇒ 读到 **0** ⇒ 判成"已证等价" ✗。
//! 实际是**整段降级**：那些条全部落进"**前缀解析不过 ⇒ 退回原文 ⇒ 这次不比**" ✗
//! （实测课程 `unit08`：1540 条里 **1505 条** ✗）。**会全被跳过的判据等于没判据** ✗。
//! ⇒ 所以这里同时钉：
//! * `probed > 0` —— 判据**真的在比** ✓（防空转 ✗）；
//! * `uncomparable == 0` —— 没有一条是"跳过"的 ✓；
//! * `mismatches == 0` —— 比过的**逐位相等** ✓；
//! * `fallbacks == 0` —— **一条都没退回原文** ✓（退回 = 特性对那个模块失效 ✗）。
//!
//! ## 复现件在册（前后翻转 ✓）
//!
//! * **改前**（`parse` 严格 + 片段重解析）：`fallbacks = 536`（unit08）、`mismatches = 1540`
//!   （其中 1505 条 `parse-namespace-unclosed`、27 条 `notation-unknown-symbol`、
//!   18 条判官合成声明 `def#`）⇒ 本文件**判红** ✗；
//! * **改后**：四条全绿 ✓（`probed > 0` ✓）。
//!
//! 反向验证：把 `canonical_prefix_id` 换回严格 `parse` ⇒ `fallbacks` 立刻 > 0 ⇒ 判红 ✓。
//!
//! ## 为什么放集成测试
//!
//! 这些计数器是**进程级**的 ✓（`judge::stats`）⇒ 放 lib 测试会被并行用例串味 ✗
//! （同 `session_reuse.rs` / `lsp_keystroke_structure.rs` 的理由 ✓）。

use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::judge::{
    identity_evictions, identity_parses, identity_probe, prefix_fallbacks_report,
};
use sokonanoda_front::project::compile_project;

/// **合成夹具**：一份覆盖**两条真实失效路**的闭包 —— 不依赖课程仓也能跑 ✓。
///
/// ① 依赖用 `namespace` 包起来 ⇒ **入口的前缀必然停在未闭合的 `namespace` 里** ✓
///    （G-05 §4.1）⇒ 严格 `parse` 必报 `parse-namespace-unclosed` ✗；
/// ② 依赖声明一条**记法**（`infix`）⇒ 入口**用了它** ⇒ 入口的**片段**单独 parse 必报
///    `notation-unknown-symbol` ✗（继承记法表不跟着走 ✓）。
///
/// 两条都命中 ⇒ 只有"`parse_fragment` + 从 AST 直取身份"这一版才可能全绿 ✓。
///
/// ⚠ **必须落盘 + 走 `compile_project`**（不是 `parse` + `compile_all_units` ✗）：
/// 记法的**继承表**是项目加载器（`project/graph.rs`）在解析入口时喂进去的 ✓ ——
/// 这正是"片段单独 parse 会失败"的成因本身 ✓，绕过它就**复现不出**那条路 ✗。
const DEP_SRC: &str = "\
namespace Lib

def twice (α : Type) (f : α → α) (a : α) : α := f (f a)

infix:50 \" ⊕ \" => Lib.twice

def dep_val (α : Type) (f : α → α) (a : α) : α := f ⊕ a

end Lib
";

const ENTRY_SRC: &str = "\
import Dep

def entry_uses_notation (α : Type) (f : α → α) (a : α) : α := f ⊕ a

def entry_uses_dep (α : Type) (f : α → α) (a : α) : α := Lib.twice α f a
";

/// 铺合成工程并编译 ⇒ 返回四个读数（**差量**，计数器是进程级的 ✓）。
fn probe_synthetic() -> (u64, u64, u64, u64, u64, u64) {
    let dir = std::env::temp_dir().join(format!("soko-identity-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建临时目录");
    std::fs::write(dir.join("Dep.sokonanoda"), DEP_SRC).expect("写依赖");
    let entry = dir.join("Entry.sokonanoda");
    std::fs::write(&entry, ENTRY_SRC).expect("写入口");

    let options = CompileOptions::default();
    let before_probe = identity_probe();
    let before_fb = prefix_fallbacks_report().0;
    let before_parses = identity_parses();
    let before_evict = identity_evictions();

    let report = compile_project(&entry, None, &options, Some(&dir));
    let errors: Vec<_> = report
        .modules
        .iter()
        .flat_map(|m| m.events.errors.iter().chain(m.report.errors.iter()))
        .collect();
    let failed: Vec<_> = report
        .modules
        .iter()
        .filter(|m| !matches!(m.status, sokonanoda_front::project::ModuleStatus::Compiled))
        .map(|m| m.name.clone())
        .collect();
    assert!(
        errors.is_empty() && failed.is_empty(),
        "夹具前提：合成工程必须编得过（否则量的不是身份那条路 ✗）：errors={errors:?} failed={failed:?}"
    );

    let after_probe = identity_probe();
    let after_fb = prefix_fallbacks_report().0;
    let after_parses = identity_parses();
    let after_evict = identity_evictions();
    let _ = std::fs::remove_dir_all(&dir);
    (
        after_probe.0 - before_probe.0,
        after_probe.1 - before_probe.1,
        after_probe.2 - before_probe.2,
        after_fb - before_fb,
        after_parses - before_parses,
        after_evict - before_evict,
    )
}

/// **真实课程**那一段（可选：课程仓可以分开检出 ✓）：合成夹具是"最小复现" ✓，
/// 真课程是"量级复现" ✓ —— 上一棒那 1505 条 `parse-namespace-unclosed` 只在
/// **真课程**上出现 ✗（合成夹具的 namespace 太浅 ✓）。
fn probe_course() -> (u64, u64, u64, u64, u64, u64) {
    let course = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    let entry = course.join("units/I.3/unit08-images-preimages.sokonanoda");
    if !course.join("sokonanoda.toml").is_file() || !entry.is_file() {
        return (0, 0, 0, 0, 0, 0);
    }
    let options = CompileOptions::default();
    let before_probe = identity_probe();
    let before_fb = prefix_fallbacks_report().0;
    let before_parses = identity_parses();
    let before_evict = identity_evictions();
    let _ = compile_project(&entry, None, &options, Some(&course));
    let after_probe = identity_probe();
    let after_fb = prefix_fallbacks_report().0;
    let after_parses = identity_parses();
    let after_evict = identity_evictions();
    (
        after_probe.0 - before_probe.0,
        after_probe.1 - before_probe.1,
        after_probe.2 - before_probe.2,
        after_fb - before_fb,
        after_parses - before_parses,
        after_evict - before_evict,
    )
}

fn probe_readings() -> (u64, u64, u64, u64, u64, u64) {
    let (p0, m0, u0, f0, r0, e0) = probe_synthetic();
    let (p1, m1, u1, f1, r1, e1) = probe_course();
    (p0 + p1, m0 + m1, u0 + u1, f0 + f1, r0 + r1, e0 + e1)
}

/// **判据 ④ + ③**：增量身份逐位相等、零退回原文。
///
/// ⚠ 这个用例必须在**独立进程**里跑（计数器是进程级的 ✓）——它是**集成测试** ✓。
#[test]
fn incremental_identity_matches_reparse_bit_for_bit() {
    // 探针是环境变量开的（默认关 ⇒ 零开销 ✓；它每次都要**重解析整份前缀** ✗，
    // 那正是被修掉的 O(n²) ⇒ 不能常开 ✓）。
    std::env::set_var("SOKO_PREFIX_ID_CHECK", "1");
    let (probed, mismatches, uncomparable, fallbacks, parses, _evictions) = probe_readings();
    println!(
        "PERF identity-probe: probed={probed} mismatches={mismatches} \
         uncomparable={uncomparable} fallbacks={fallbacks} reparse={parses}"
    );

    // ① **防空转** ✗：判据必须真的比过东西（上一棒就栽在"全被跳过"上 ✗）。
    assert!(
        probed > 0,
        "判据空转 ✗：探针一条都没比过 —— `SOKO_PREFIX_ID_CHECK` 没生效，\
         或者夹具压根没走到 walk（守卫会永远绿 ✗）"
    );
    // ② **不许有跳过**：前缀解析不过 ⇒ `canonical_prefix_id` 退回原文 ⇒ 那次不比 ✗。
    assert_eq!(
        uncomparable, 0,
        "有 {uncomparable} 条前缀**解析不过**（`canonical_prefix_id` 退回原文 ✗）\
         ⇒ 判据对它们**不可比** ✗。前缀是**文件片段**，本来就可能停在未闭合的 \
         `namespace` 里（G-05 §4.1）⇒ 必须用 `parse_fragment` ✓"
    );
    // ③ **逐位相等**。
    assert_eq!(
        mismatches, 0,
        "增量身份与 `canonical_prefix_id(整份前缀)` **不相等** ✗ —— \
         键与真身份脱钩 = **错编的入口** ✗（先把分歧点打出来：`SOKO_PREFIX_ID_CHECK=1`）"
    );
    // ④ **零退回原文**（判据 ③ ✓）：退回 = 「只改证明体 ⇒ 后面不重编」对那个模块失效 ✗。
    assert_eq!(
        fallbacks, 0,
        "有 {fallbacks} 次**身份退回原文** ✗（读 `STAGE_STATS … fallbacks=N` 与 \
         `judge::prefix_fallbacks_report()` 的第一份头 80 字节定位模块 ✓）—— \
         退回原文会让键退化成**原文哈希** ⇒ 改证明体照样全失效 ✗"
    );
    assert!(probed > 0);
}

/// **判据 ②（结构版）**：预置生效 ⇒ 身份**几乎不用重解析** ✓（O(n²) 的结构读数 ✓）。
///
/// ⚠ **为什么不用毫秒** ✗（`AGENTS.md` 2026-09-29 第三条，同一个病犯过三次 ✓）：
/// 共享 runner 上同一份代码量到过 **44ms ↔ 2431ms** ⇒ 判据一律用**结构计数** ✓。
/// 这里钉的是 O(n²) 的**源头**：`canonical_prefix_cached` 未命中 ⇒ 真的 parse 一整份前缀 ✗。
/// 预置生效时它应当**远小于命令数** ✓（实测：合成夹具 + 真课程一起 ≈ 个位数 ✓）；
/// 失效时会涨到 **≈ 命令数** ✗（那就是 O(n²) 回来了 ✗）。
#[test]
fn seeded_identity_avoids_reparsing_the_prefix() {
    std::env::set_var("SOKO_PREFIX_ID_CHECK", "1");
    let (probed, mismatches, uncomparable, fallbacks, parses, evictions) = probe_readings();
    println!(
        "PERF identity-reparse: probed={probed} mismatches={mismatches} \
         uncomparable={uncomparable} fallbacks={fallbacks} reparse={parses} evictions={evictions}"
    );
    // **记忆表淘汰必须为 0** ✓（G-91 的闸类计数出口 ✓）：淘汰**不改答案** ✓（重解析
    // 给出同一个身份 ✓），但它会让预置**白做** ✗ —— 实测 `CAP=4096` 时整本课程
    // `identity_parses=3062` ✗（表比工作集小 ⇒ **抖动** ✗）⇒ 这条钉住"别再把它调小" ✓。
    assert_eq!(
        evictions, 0,
        "记忆表淘汰了 {evictions} 条 ✗ —— `canonical_prefix_table` 的容量比工作集小 ⇒          **刚种进去就被挤掉** ⇒ 预置白做、O(n²) 回来 ✗（实测 `CAP=4096` 时整本课程          重解析 3062 趟 ✗；65536 ⇒ 0 ✓）"
    );
    // 夹具 + 真课程的命令总数是**几百**量级 ✓；预置生效时重解析应当是个位数 ✓。
    // **实测**（2026-10-04）：开预置 **0** ✓ / `SOKO_NO_SEED=1` **54** ✗
    // （合成夹具 + 真课程 `unit08` 一起 ✓）⇒ 阈值 20 两边都有余量 ✓。
    assert!(
        parses < 20,
        "身份**重解析**了 {parses} 趟 ✗ —— 预置（`seed_canonical_prefix` ✓）没生效 ⇒ \
         前缀逐命令变长 ⇒ 每命令重解析整份 = **O(n²)** ✗（这正是 unit12 冷开 10.5s 的根因 ✓）。\
         查 `SOKO_NO_SEED` 是不是被设了 ✓"
    );
}
