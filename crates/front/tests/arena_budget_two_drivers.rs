//! **`ARENA-BUDGET-SHARED-BY-TWO-DRIVERS` 的判据**（队列 id 同名 · medium · 产品面 · 2026-10-16）。
//!
//! ## 病（上一棒 `arena1` 定位 ✓ · 本棒复现 ✓）
//!
//! `crates/front/src/project/session.rs` 的**两条上界共用一份预算**：
//!
//! * **上界 ①** `MAX_LEAKED_LIB_ARENAS = 8`（**进程级、只增不减**，唯一减法是测试钩子
//!   `lib_checkpoint_reset()`）—— 检查点要进线程局部就得 `'static`，而 `'static`
//!   只能 `Box::leak`（零 `unsafe`）⇒ 泄漏的 arena **永远收不回**；
//! * **上界 ②** `MAX_REUSES_PER_CHECKPOINT = 64` 到点**轮换** —— 而**轮换的代价
//!   就是再泄漏一份 arena**（走 ③ 重建那条路）✗。
//!
//! ⇒ **两个驱动源**（① 冷建新键 · ② 轮换）**烧同一份 8 格预算** ⇒
//! `8 × 64 = 512 刀`之后用尽。
//!
//! ## 上一棒修掉的那一半（`e4019b19` · 本文件**不动它** ✓）
//!
//! 轮换闸改成 `reuses < 64 || (!can_leak && !legacy)`（**软上界**）+ 回退路不再
//! `slots.clear()` ⇒ **同一个键**的长跑不再退化（`worst_elaborated = 1` ✓）。
//! 判据在 `crates/front/tests/leaked_lib_arenas_cliff.rs` ✓（本文件**不重复**它）。
//!
//! ## 本棒修的那一半：**上界用尽之后，产物层照旧服务**（②.5）
//!
//! 修前 `if can_leak { …①.5 产物… }` 把**整块**产物层包住 ⇒ 上界用尽之后：
//! ①.5 **不读**、回退路**也不写** ⇒ 新键**每一刀**都是"库层 + 入口"两趟，
//! 而且**本进程内再也填不回来**（实测：产物目录条目数 = **0** ✗）。
//!
//! 而产物是**跨进程**加速件、装载它**零泄漏**（`load_lib_checkpoint_in` 收调用方的
//! arena ✓）⇒ 没有任何理由把它一起停掉 ✗。修后新增 **②.5**：`!can_leak` 时把产物装进
//! **栈上** arena、只服务本次调用（不进 LRU、不建尾快照）；回退路**照旧把产物写出去**。
//!
//! ## 三段判据（**一个文件一个 `#[test]`** —— `LEAKED_LIB_ARENAS` 是进程级计数器，
//! 同二进制里并排跑两个用例会互相污染取差 ✗，同 `g29_closure_recompile.rs` 的纪律 ✓）
//!
//! | 段 | 场景 | 判据 |
//! | --- | --- | --- |
//! | ① | 单项目连敲 **700** 刀 | 预算用尽点 = **456**（= `(64+1)×8 − 64`）· `leaked` = 8 · 软上界活着 |
//! | ② | 上界用尽之后**新键** | 每一刀 `modules` = **1**（修前 2）· `by` 只跑入口那几条 · 读数 = `artifact` |
//! | ③ | 判定路径未变 | 产物那支的报告与回退那支**逐字节相同**（`serde_json` 文本 ✓） |
//!
//! ⚠ **判据只用结构计数**（`closure_module_compiles_total` / `by_calls_total` /
//! `elaborated_commands_total`）—— 绝对毫秒在共享 runner 上不可转移 ✗（`AGENTS.md` 判据纪律②）。
//!
//! ## 反向验证（**必须跟着走，不许恒真** ✗）
//!
//! 把 `MAX_LEAKED_LIB_ARENAS` 临时压到 **2** ⇒ ① 段的预算用尽点必须从 **456 → 66**
//! （= `(64+1)×2 − 64`）⇒ 本文件在那一份构建上**判红**且读数可归属 ✓（读数见
//! `docs/perf/ledger.jsonl` 的 `arena_budget_two_drivers` 行）。跑完还原（`git show`）✓。
//!
//! ## 咬住证明
//!
//! `docs/guard-bite.jsonl`：把 `crates/front/src/project/session.rs` 换成**钉死的旧 rev
//! `b256cc2b`**（修前 main 的 tip，**不是 `HEAD`** ✗）⇒ 本文件 **rc=101**（② 段读到
//! `modules=2`）；空转闸：换进去的文件与工作区**逐字节相同** ⇒ `exit 3`（绝不冒充咬住 ✓）。

use std::path::{Path, PathBuf};

use sokonanoda_front::compile::{
    by_calls_total, closure_module_compiles_total, elaborated_commands_total,
};
use sokonanoda_front::project::session::{
    last_lib_source, lib_checkpoint_arenas_leaked, lib_checkpoint_reset, lib_checkpoint_reuses,
};
use sokonanoda_front::query::QueryDoc;

/// 入口文件里 `t…` 的条数（`leaked_lib_arenas_cliff.rs` 的 `DECLS` 同值）。
const DECLS: usize = 8;

/// 入口文件的**命令数**（`import Lib` + `DECLS` 条 `t…` + 3 条 `d…`）= 12。
const ENTRY_COMMANDS: u64 = DECLS as u64 + 1 + 3;

/// 入口文件里 `by` 的条数（每条 `t…`/`d…` 各一个 `by exact …`）= 11。
const ENTRY_BY: u64 = DECLS as u64 + 3;

/// **库层**里 `by` 的条数（夹具的三个 `theorem … := by …`）—— 它就是
/// "库层趟有没有重跑"的**结构判据**（`t1b_module_artifacts.rs` 同一手法 ✓）。
const LIB_BY: u64 = 3;

/// `MAX_LEAKED_LIB_ARENAS`（上界 ①，**本判据不许抬它** ✗）。
const MAX_LEAKED: usize = 8;

/// `MAX_REUSES_PER_CHECKPOINT`（上界 ②）—— ① 段要**越过**它（证明软上界活着 ✓）。
const MAX_REUSES: usize = 64;

/// ① 段的刀数：`MAX_LEAKED × MAX_REUSES` = 512 是修前的悬崖点 ⇒ 取 700 留余量 ✓。
const LONG_PRESSES: u64 = 700;

/// **预算用尽点**（刀数，**冷开那一刀算第 1 刀**）。
///
/// 推导（机制，不是魔数 ✓）：一个键占 1 份 arena，之后**每 `MAX_REUSES + 1` 刀**
/// 轮换一次、再泄漏一份 ⇒ 第 `N` 份泄漏落在第 `1 + (N−1)(MAX_REUSES + 1)` 刀
/// ⇒ 用尽点 = `(MAX_REUSES + 1) × MAX_LEAKED − MAX_REUSES` = **456** ✓。
///
/// ⚠ 这条**故意绑死上界**：把 `MAX_LEAKED_LIB_ARENAS` 压到 2 ⇒ 期望值必须变成
/// **66** ⇒ 本判据在那一份构建上判红（**反向验证的判别力就在这一条** ✓，不许恒真 ✗）。
const EXHAUST_AT_PRESS: u64 = (MAX_REUSES as u64 + 1) * MAX_LEAKED as u64 - MAX_REUSES as u64;

/// 合成项目（`Lib ← Main`），与 `leaked_lib_arenas_cliff.rs::gen_project` **同纪律**
/// （合成、不依赖课程目录 ✓）；`tag` 只用来隔离临时目录 ⇒ 也决定了 `lib_key`
/// （键里有**绝对路径** ⇒ 不同 tag = 不同键 = 不同检查点 ✓）。
///
/// ⚠ **与那份夹具的差别只有一处**：库层多了 **3 条 `by` 证明** ⇒ 让"库层趟有没有重跑"
/// 有一个**结构读数**（`by_calls_total`），不必靠墙钟 ✗（`AGENTS.md` 判据纪律②）。
fn gen_project(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("soko-arena-drivers-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let lib = "axiom Point : Type\n\
               axiom EqP : Point -> Point -> Prop\n\
               infix:50 \" \u{2248} \" => EqP\n\
               axiom refl : (a : Point) -> a \u{2248} a\n\
               axiom trans : (a b c : Point) -> a \u{2248} b -> b \u{2248} c -> a \u{2248} c\n\
               theorem l1 (a : Point) : a \u{2248} a := by exact refl a\n\
               theorem l2 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact h\n\
               theorem l3 (a b c : Point) (h : a \u{2248} b) (k : b \u{2248} c) : a \u{2248} c :=\n\
               \x20 by exact trans a b c h k\n";
    std::fs::write(dir.join("Lib.sokonanoda"), lib).expect("write Lib");
    let mut main = String::from("import Lib\n\n");
    for i in 0..DECLS {
        main.push_str(&format!(
            "theorem t{i:02} (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact h\n\n"
        ));
    }
    main.push_str(
        "theorem d00 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact t00 a b h\n\n",
    );
    main.push_str(
        "theorem d01 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact d00 a b h\n\n",
    );
    main.push_str(
        "theorem d02 (a b : Point) (h : a \u{2248} b) : a \u{2248} b := by exact t00 a b h\n\n",
    );
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, &main).expect("write Main");
    (entry, main)
}

fn open_doc(entry: &Path, text: &str) -> QueryDoc {
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.to_path_buf());
    doc.set_text(text, 1, None);
    doc
}

/// 交替改最后一条命令（长度不变 ✓）—— 长跑要**每一刀都真的改文本**（同文本会命中
/// T-A21 的"文本没变就短路" ⇒ 读到 0，量到的不是按键 ✗）。
fn toggle_last_decl(text: &str, to_k: bool) -> String {
    let (fb, tb) = if to_k {
        ("(h :", "(k :")
    } else {
        ("(k :", "(h :")
    };
    let (fe, te) = if to_k {
        ("exact t00 a b h", "exact t00 a b k")
    } else {
        ("exact t00 a b k", "exact t00 a b h")
    };
    let at = text.find("theorem d02 ").expect("找不到 `theorem d02`");
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    let line = &text[at..end];
    let edited = line.replacen(fb, tb, 1).replacen(fe, te, 1);
    assert_ne!(edited, line, "这一刀必须真的改变文本（否则量到的是短路 ✗）");
    assert_eq!(
        edited.len(),
        line.len(),
        "这次按键必须**长度不变**（否则下游起点平移，量到的不是「最后一条」）"
    );
    format!("{}{}{}", &text[..at], edited, &text[end..])
}

/// 一次 `set_text` 的**三份结构读数**（进程级累计取差；同线程 ✓）。
#[derive(Debug, Clone, Copy)]
struct Reading {
    /// 这一刀编了几个**闭包模块**（产物/检查点命中 ⇒ 只入口 = 1；回退路 ⇒ 库层 + 入口 = 2）。
    modules: u64,
    /// 这一刀真的 elaborate 了几条命令（命中尾快照 ⇒ 1；整条重走 ⇒ N = 12）。
    elaborated: u64,
    /// 这一刀跑了几个 `by`（库层重跑 ⇒ 多 `LIB_BY` 个）。
    by: u64,
}

fn read_by<F: FnOnce()>(f: F) -> Reading {
    let m0 = closure_module_compiles_total();
    let e0 = elaborated_commands_total();
    let b0 = by_calls_total();
    f();
    Reading {
        modules: closure_module_compiles_total() - m0,
        elaborated: elaborated_commands_total() - e0,
        by: by_calls_total() - b0,
    }
}

/// 产物目录里的条目数（`.bin` + `.meta.json` 成对 ⇒ 修后应为偶数且 ≥ 2）。
fn artifact_entries(entry: &Path) -> usize {
    let dir = entry
        .parent()
        .expect("入口必有父目录")
        .join(".sokonanoda")
        .join("artifacts");
    std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
}

/// ⭐ **三段判据**：① 两个驱动源烧同一份预算 · ② 用尽之后产物层照旧服务（本轮的修）·
/// ③ 判定路径未变。
#[test]
fn arena_budget_two_drivers_and_the_artifact_layer_survives_exhaustion() {
    // ══ ① 单项目长跑：**一个**驱动源就能烧完整份预算 ══
    lib_checkpoint_reset();
    let (entry, text) = gen_project("long");
    let mut doc = open_doc(&entry, &text); // 冷开 = **第 1 刀**（泄漏第 1 份 arena ✓）
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        1,
        "冷开必须恰好泄漏 1 份 arena（否则本段的刀数口径就错了 ✗）"
    );
    let mut current = text.clone();
    let mut worst = 0u64;
    let mut worst_at = 0u64;
    let mut exhaust_at = 0u64;
    for press in 2..=LONG_PRESSES {
        current = toggle_last_decl(&current, press % 2 == 0);
        let r = read_by(|| doc.set_text(&current, press, None));
        if r.elaborated > worst {
            worst = r.elaborated;
            worst_at = press;
        }
        if exhaust_at == 0 && lib_checkpoint_arenas_leaked() == MAX_LEAKED {
            exhaust_at = press;
        }
    }
    println!(
        "PERF arena-drivers 长跑：{LONG_PRESSES} 刀 · 预算用尽点={exhaust_at}（期望 {EXHAUST_AT_PRESS}）· \
         worst elaborate={worst}（第 {worst_at} 刀）· leaked={} · reuses={}",
        lib_checkpoint_arenas_leaked(),
        lib_checkpoint_reuses()
    );
    assert_eq!(
        exhaust_at, EXHAUST_AT_PRESS,
        "**预算用尽点必须跟着上界走**（不许恒真 ✗）：实测第 {exhaust_at} 刀用尽，\
         期望第 {EXHAUST_AT_PRESS} 刀（= (MAX_REUSES + 1) × MAX_LEAKED − MAX_REUSES \
         = ({MAX_REUSES} + 1) × {MAX_LEAKED} − {MAX_REUSES}）。\
         读到更早 ⇒ 上界 ① 被别的驱动源提前烧掉了；读到更晚 ⇒ 轮换没有真的泄漏 arena。\
         ⚠ **反向验证**：把 `MAX_LEAKED_LIB_ARENAS` 压到 2 ⇒ 这里必须读到 **66** ✓。"
    );
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        MAX_LEAKED,
        "单项目 700 刀（> 8 × 64）必须把上界 ① **恰好**走到顶 —— 这就是「两条上界共用一份预算」\
         的形状：**轮换**（上界 ②）烧掉了 7 格，只留 1 格给冷建 ✓"
    );
    assert_eq!(
        worst, 1,
        "同一份检查点**越过**上界 ② 之后必须继续命中（软上界 ✓，上一棒 `e4019b19` 的修）—— \
         实测最坏 elaborate = {worst}（第 {worst_at} 刀）；回退路的 N = {ENTRY_COMMANDS}"
    );
    assert!(
        lib_checkpoint_reuses() > MAX_REUSES,
        "① 段必须真的走到「软上界」那条路（实测 reuses={}，上界 ② = {MAX_REUSES}）\
         —— 读到 ≤ {MAX_REUSES} ⇒ 量具坏了 ✗",
        lib_checkpoint_reuses()
    );

    // ══ ② 上界用尽之后**新键**：产物层照旧服务（本轮的修）══
    //
    // 修前：①.5 **不读**、回退路**也不写** ⇒ 新键每一刀 = 库层 + 入口两趟
    // （实测 `modules` = 2 · `by` = LIB_BY + ENTRY_BY = 14 · 产物目录 = **0** 条 ✗）。
    let (entry9, text9) = gen_project("freshkey");
    assert_eq!(
        artifact_entries(&entry9),
        0,
        "夹具自检：新键的产物目录一开始必须是空的（否则量不到「回退路有没有写出去」✗）"
    );
    let mut doc9 = open_doc(&entry9, &text9);
    println!(
        "PERF arena-drivers 新键冷开：source={} leaked={} 产物条目={}",
        last_lib_source(),
        lib_checkpoint_arenas_leaked(),
        artifact_entries(&entry9)
    );
    assert_eq!(
        last_lib_source(),
        "fallback",
        "上界用尽之后新键的冷开**没有**可复用的检查点 ⇒ 必然走回退路 \
         （读数必须**可归属** ✓ —— 修前这一格读到的还是上一次的值 ✗）"
    );
    assert!(
        artifact_entries(&entry9) >= 2,
        "⭐ **本轮的修**：回退路**照旧把产物写出去**（写产物零泄漏 ✓）—— 实测条目 {} \
         （修前 = **0** ⇒ 新键永远没有产物可命中，本进程内再也填不回来 ✗）",
        artifact_entries(&entry9)
    );
    let mut t9 = text9.clone();
    for press in 2..=4u64 {
        t9 = toggle_last_decl(&t9, press % 2 == 0);
        let r = read_by(|| doc9.set_text(&t9, press, None));
        println!(
            "PERF arena-drivers 新键第 {press} 刀：modules={} elaborated={} by={} source={}",
            r.modules,
            r.elaborated,
            r.by,
            last_lib_source()
        );
        assert_eq!(
            last_lib_source(),
            "artifact",
            "第 {press} 刀必须由**产物层**服务（②.5：栈上 arena、零泄漏）—— 实测读数不是 \
             `artifact` ⇒ 本轮的修没生效 ✗"
        );
        assert_eq!(
            r.modules, 1,
            "第 {press} 刀只许编**入口**那一个模块（实测 {}；回退路 = 2 = 库层 + 入口）\
             ⇒ 上界用尽之后**库层趟必须被产物层接住** ✓（修前恒 2 ✗）",
            r.modules
        );
        assert_eq!(
            r.by,
            ENTRY_BY,
            "第 {press} 刀只许跑**入口**那 {ENTRY_BY} 条 `by`（实测 {}；回退路 = \
             {LIB_BY} + {ENTRY_BY} = {}）⇒ 库层的 {LIB_BY} 条 `by` **没有重跑** ✓",
            r.by,
            LIB_BY + ENTRY_BY
        );
        assert_eq!(
            r.elaborated, ENTRY_COMMANDS,
            "⚠ **诚实边界**（如实记 ✗，**不是**本轮的修）：栈上 arena 装不出 `'static` \
             ⇒ 入口尾快照只能 `None` ⇒ 入口趟**整条重走**（实测 {ENTRY_COMMANDS} = N）—— \
             这是上界 ① 的诚实代价里**剩下那一半**；要消掉它得让入口尾快照的 arena 可回收 \
             （内核改动 / `unsafe` ✗，见队列条目的设计）"
        );
        assert_eq!(
            lib_checkpoint_arenas_leaked(),
            MAX_LEAKED,
            "②.5 那条路**零泄漏** ⇒ 上界 ① 一个字不动 ✓"
        );
    }

    // ══ ③ 判定路径未变：产物那支 vs 回退那支**逐字节相同** ══
    //
    // 同一个项目、同一份文本 T0，两条路各编一次：
    //   * 回退路：冷开（那一刻产物还没写出来 ⇒ 走 ③ 的 `else`）✓；
    //   * 产物路：编辑离开 T0 再编辑回来（②.5）✓。
    // 两份报告按 `serde_json` 文本比 —— 与 `t1b_module_artifacts.rs` 同一手法 ✓。
    let (entry_p, text_p) = gen_project("parity");
    let doc_p = open_doc(&entry_p, &text_p);
    assert_eq!(
        last_lib_source(),
        "fallback",
        "③ 段的基准臂必须是**回退路**（否则比的是同一条路，判据空转 ✗）"
    );
    let report_fallback = serde_json::to_string(doc_p.report.as_ref().expect("基准臂必须有报告"))
        .expect("报告可序列化");
    let mut doc_p = doc_p;
    let t1 = toggle_last_decl(&text_p, true);
    let r1 = read_by(|| doc_p.set_text(&t1, 2, None));
    assert_eq!(
        (r1.modules, last_lib_source()),
        (1, "artifact"),
        "③ 段第一步必须走产物路（否则下面比的是两条同路 ✗）"
    );
    let r2 = read_by(|| doc_p.set_text(&text_p, 3, None));
    assert_eq!(
        (r2.modules, last_lib_source()),
        (1, "artifact"),
        "③ 段第二步必须走产物路"
    );
    let report_artifact = serde_json::to_string(doc_p.report.as_ref().expect("产物臂必须有报告"))
        .expect("报告可序列化");
    assert_eq!(
        report_artifact.len(),
        report_fallback.len(),
        "**判定路径未变**：同一份文本 T0 在「产物那支」与「回退那支」下的报告必须**逐字节相同** \
         —— 长度先不同（{} vs {}）⇒ 产物层改了判定 ✗",
        report_artifact.len(),
        report_fallback.len()
    );
    assert_eq!(
        report_artifact, report_fallback,
        "**判定路径未变**：同一份文本 T0 在「产物那支」（②.5）与「回退那支」（③ `else`）下的 \
         报告必须**逐字节相同** —— 报告是 `serde_json` 文本，逐字节比 ✓（与 `t1b_module_artifacts.rs` \
         同一手法）。不同 ⇒ 本轮的修**改了判定** ✗（那是红线，必须回退）"
    );
}
