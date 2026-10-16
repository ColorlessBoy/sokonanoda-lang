//! **`LEAKED-LIB-ARENAS-CLIFF` 的判据**（队列 id 同名 · `medium` · 产品面）。
//!
//! ## 病（修前实测）
//!
//! `crates/front/src/project/session.rs` 的库层检查点要存进线程局部就得 `'static`
//! ⇒ 每建一份就 `Box::leak` 一份 arena，而泄漏数**只增不减**（进程级
//! `LEAKED_LIB_ARENAS`，上界 `MAX_LEAKED_LIB_ARENAS = 8`，设计 §33.1 上界 ①）。
//! 上界 ②（`MAX_REUSES_PER_CHECKPOINT = 64`）到点**轮换** —— 而**轮换的代价就是
//! 再泄漏一份 arena** ⇒ **8 × 64 = 512 刀**之后上界 ① 用尽。用尽之后会话落进回退路
//! （栈上 arena）—— 回退路本身是**设计如此** ✓，但它做了两件**纯亏**的事 ✗：
//!
//! 1. `slots.clear()` 把 LRU 里那些**已经泄漏、已经付过钱、键仍然有效**的检查点
//!    一起抹掉 ⇒ 本进程内后续每一次编译都整条重走，而且**再也填不回来**
//!    （下一次 miss 时 `can_leak` 仍是假 ⇒ 又 `clear()` 一次）✗；
//! 2. 轮换闸**不看**泄漏预算 ⇒ 上界 ① 用尽之后那一份检查点**被永久停用** ✗。
//!
//! ## 两个阶段（同 `g29_closure_recompile.rs` 的纪律：**一个文件一个 `#[test]`**）
//!
//! `LEAKED_LIB_ARENAS` 是**进程级** `AtomicUsize` ⇒ 同一二进制里并排跑两个用例会
//! 互相污染取差 ✗ ⇒ 两段判据放在**同一个** `#[test]` 里（同 g29 的先例 ✓）。
//!
//! | 阶段 | 场景 | 判据 |
//! | --- | --- | --- |
//! | ① **长命 LSP** | 单项目连敲 **700** 刀（> 512） | 每一刀只 elaborate **1** 条（修前：第 **521** 刀起恒 **12** = N ✗） |
//! | ② **上界用尽 + 新键** | 8 个不同 `lib_key` 的项目 + 第 9 个新键 ⇒ 回头改第 1 个 | `closure_module_compiles_total` = **1**（修前 = 2）· elaborate = **1**（修前 = 12） |
//!
//! 判据只断言"已经泄漏的检查点**继续命中**"：判定路径一个字节没动
//! （报告/事件的逐字节等价由既有的 `t2b_*` / `g29_*` / `session_reuse` 守卫管 ✓）。
//!
//! ## 反向验证（`SOKO_NO_LRU_KEEP=1`）
//!
//! `project/session.rs` 的逃生门把**修前的策略**原样装回来（回退路 `slots.clear()`
//! 加上"轮换闸不看预算"）⇒ 阶段①读到 `worst = 12`（第 521 刀）⇒ 本文件判红 ✓
//! （`docs/guard-bite.jsonl`）。

use std::path::{Path, PathBuf};

use sokonanoda_front::compile::{closure_module_compiles_total, elaborated_commands_total};
use sokonanoda_front::project::session::{
    lib_checkpoint_arenas_leaked, lib_checkpoint_is_live, lib_checkpoint_reset,
    lib_checkpoint_reuses,
};
use sokonanoda_front::query::QueryDoc;

/// 入口文件里 `t…` 的条数（`keystroke_structure.rs` 的 `DECLS` 同值）。
const DECLS: usize = 8;

/// 入口文件的**命令数**（`import Lib` + `DECLS` 条 `t…` + 3 条 `d…`）= 12 = 回退路的读数 N ✓。
const ENTRY_COMMANDS: u64 = DECLS as u64 + 1 + 3;

/// `MAX_LEAKED_LIB_ARENAS`（`project/session.rs` 的上界 ①，**本判据不许抬它** ✗）。
const MAX_LEAKED: usize = 8;

/// `MAX_REUSES_PER_CHECKPOINT`（上界 ②）—— 阶段①要**越过**它（证明"软上界"生效 ✓）。
const MAX_REUSES: usize = 64;

/// 阶段①的刀数：`MAX_LEAKED × MAX_REUSES` = 512 是修前的悬崖点 ⇒ 取 700 留余量 ✓。
const LONG_PRESSES: u64 = 700;

/// 合成项目（`Lib ← Main`），与 `keystroke_structure.rs::gen_project` **同纪律**
/// （合成、不依赖课程目录 ✓）；`tag` 只用来隔离临时目录 ⇒ 也决定了 `lib_key`
/// （键里有**绝对路径** ⇒ 不同 tag = 不同键 = 不同检查点 ✓）。
fn gen_project(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("soko-arena-cliff-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let lib = "axiom Point : Type\n\
               axiom EqP : Point -> Point -> Prop\n\
               infix:50 \" \u{2248} \" => EqP\n\
               axiom refl : (a : Point) -> a \u{2248} a\n\
               axiom trans : (a b c : Point) -> a \u{2248} b -> b \u{2248} c -> a \u{2248} c\n";
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

/// 一次 `set_text` 的**两份结构读数**（进程级累计取差；同线程 ✓）。
#[derive(Debug, Clone, Copy)]
struct Reading {
    /// 这一刀编了几个**闭包模块**（命中检查点 ⇒ 只入口 = 1；回退路 ⇒ 库层 + 入口 = 2）。
    modules: u64,
    /// 这一刀真的 elaborate 了几条命令（命中尾快照 ⇒ 1；整条重走 ⇒ N = 12）。
    elaborated: u64,
}

fn read_by<F: FnOnce()>(f: F) -> Reading {
    let m0 = closure_module_compiles_total();
    let e0 = elaborated_commands_total();
    f();
    Reading {
        modules: closure_module_compiles_total() - m0,
        elaborated: elaborated_commands_total() - e0,
    }
}

/// ⭐ **两段判据**（阶段① = 长命 LSP 的悬崖；阶段② = 上界用尽之后**新键**不许抹掉 LRU）。
#[test]
fn arena_budget_exhaustion_stays_incremental() {
    // ══ 阶段①：长命 LSP 的形状 —— 单项目连敲 700 刀（> 8 × 64 = 512）══
    //
    // 修前：每 64 刀轮换一次（每次再泄漏一份 arena）⇒ 第 8 次轮换之后上界 ① 用尽 ⇒
    // **第 521 刀起每一刀整条重走**（实测 `worst = 12`、700 刀合计 elaborate **2679**）✗。
    lib_checkpoint_reset();
    let (entry, text) = gen_project("long");
    let mut doc = open_doc(&entry, &text);
    let mut current = text.clone();
    let mut worst = 0u64;
    let mut worst_at = 0u64;
    let mut total = 0u64;
    for version in 2..=LONG_PRESSES {
        current = toggle_last_decl(&current, version % 2 == 0);
        let r = read_by(|| doc.set_text(&current, version, None));
        if r.elaborated > worst {
            worst = r.elaborated;
            worst_at = version;
        }
        total += r.elaborated;
    }
    println!(
        "PERF arena-cliff 长跑：{LONG_PRESSES} 刀 · worst={worst}（第 {worst_at} 刀）· \
         合计 elaborate={total} · leaked={} · reuses={}",
        lib_checkpoint_arenas_leaked(),
        lib_checkpoint_reuses()
    );
    assert_eq!(
        worst, 1,
        "长命 LSP：**每一刀**都只许 elaborate 最后那一条（实测最坏 {worst}，在第 {worst_at} 刀；\
         回退路的 N = {ENTRY_COMMANDS}）—— 上界 ① 用尽之后**不许**把那一份检查点停用 ✗ \
         （修前正是第 521 刀起恒 {ENTRY_COMMANDS}）"
    );
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        MAX_LEAKED,
        "700 刀（> 8 × 64）必须把上界 ① **恰好**走到顶 —— 否则本阶段没量到悬崖点（量具坏了 ✗）"
    );
    assert!(
        lib_checkpoint_reuses() > MAX_REUSES,
        "上界 ① 用尽之后必须**越过**上界 ② 继续复用那一份（实测 reuses={}，上界 ② = {MAX_REUSES}）\
         —— 读到 ≤ {MAX_REUSES} ⇒ 本阶段没走到「软上界」那条路（量具坏了 ✗）",
        lib_checkpoint_reuses()
    );

    // ══ 阶段②：上界用尽之后再来一个**新键**，不许把 LRU 抹掉 ══
    lib_checkpoint_reset();
    // 开 8 个不同 `lib_key` 的项目：每个泄漏一份 arena ⇒ 上界恰好用尽。
    let mut docs: Vec<QueryDoc> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    for i in 0..MAX_LEAKED {
        let (entry_i, text_i) = gen_project(&format!("budget{i}"));
        let doc_i = open_doc(&entry_i, &text_i);
        assert_eq!(
            lib_checkpoint_arenas_leaked(),
            i + 1,
            "第 {} 个项目冷开必须泄漏第 {} 份 arena",
            i + 1,
            i + 1
        );
        docs.push(doc_i);
        texts.push(text_i);
    }
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        MAX_LEAKED,
        "上界 ① 恰好用尽（= {MAX_LEAKED} 份）"
    );
    assert!(
        lib_checkpoint_is_live(),
        "用尽之前 LRU 里必须有活着的检查点"
    );

    // 第 9 个**新键**：`can_leak == false` ⇒ 修前在这一刀 `slots.clear()` ✗
    let (entry9, text9) = gen_project("freshkey");
    let mut doc9 = open_doc(&entry9, &text9);
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        MAX_LEAKED,
        "上界用尽之后**不许**再泄漏 arena（上界 ① 一个字不动）"
    );
    assert!(
        lib_checkpoint_is_live(),
        "上界用尽之后**不许**把已经泄漏的检查点一起抹掉（修前的 `slots.clear()` ✗）—— \
         那 8 份 arena 已经泄漏、键也仍然有效，抹掉是**纯亏** ✗"
    );
    // ⚠ **诚实边界**（**不是**本段改动引入的，也**不许**假装它没有 ✗）：新键**没有**
    // 可复用的检查点（8 份 arena 都被别的键占着，而回收要 `unsafe` ✗）⇒ 它**必然**
    // 走回退路。这就是上界 ① 的**诚实代价**（设计 §33.1 ④）。
    //
    // ⭐ **2026-10-16 `ARENA-BUDGET-SHARED-BY-TWO-DRIVERS`（本棒的修）**：这条断言的
    // **两格各修了一半**（同一份构建、同一操作序列）：
    //   * `modules` **2 → 1** ✓ —— 修后新增 **②.5**：上界 ① 用尽之后产物层**照旧服务**
    //     （装进**栈上** arena、零泄漏），而回退路**照旧把产物写出去** ⇒ 冷开那一刀
    //     建好的产物，**下一刀**就把整趟库层接住 ✓（库层的 `by` 不再重跑）；
    //   * `elaborated` **仍是 N** ✗ —— 栈上 arena 装不出 `'static` ⇒ 入口尾快照只能
    //     `None` ⇒ 入口趟整条重走。要消掉它得让入口尾快照的 arena **可回收**
    //     （`unsafe` 或拆 `EnvBuilder` ⇒ **内核改动**，见队列条目的设计 ✗）。
    // ⇒ 断言从 (2, N) 收紧成 **(1, N)**（方向是**变快** ✓，不是放宽 ✗）；本棒另立
    // 判据 `crates/front/tests/arena_budget_two_drivers.rs` 把两格都钉住 ✓。
    let fresh = read_by(|| doc9.set_text(&toggle_last_decl(&text9, true), 2, None));
    println!(
        "PERF arena-cliff 上界用尽后的**新键**：modules={} elaborated={}（回退路 = 2 / {ENTRY_COMMANDS}）",
        fresh.modules, fresh.elaborated
    );
    assert_eq!(
        (fresh.modules, fresh.elaborated),
        (1, ENTRY_COMMANDS),
        "上界用尽之后**新键**没有可复用的检查点 ⇒ 它必然走回退路**建**产物（上界 ① 的诚实代价 ✓），\
         但**下一刀**必须由产物层（②.5，零泄漏）接住 ⇒ `modules` 只许是 **1**（只编入口）。\
         实测 ({}, {}) —— 读到 2 ⇒ 产物层在上界用尽之后被一起停掉了 ✗；\
         `elaborated` 仍是 N = {ENTRY_COMMANDS}（栈上 arena 装不出 `'static` ⇒ 入口尾快照 \
         只能 `None`），**不是**回归、是**剩下的那半**诚实边界 ✗（见上）",
        fresh.modules,
        fresh.elaborated
    );
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        MAX_LEAKED,
        "新键走回退路也不许泄漏 arena（上界 ①）"
    );

    // ⭐ 回头改**第 1 个项目**：它的检查点已经泄漏、键仍然有效 ⇒ 必须命中
    let reading = read_by(|| docs[0].set_text(&toggle_last_decl(&texts[0], true), 2, None));
    assert_eq!(
        reading.modules, 1,
        "上界用尽之后回头改项目 ⇒ 只许编**入口**那一个模块（实测 {}；回退路 = 2 = 库层 + \
         入口）⇒ 检查点被抹掉了 ✗",
        reading.modules
    );
    assert_eq!(
        reading.elaborated, 1,
        "上界用尽之后回头改项目 ⇒ 只许 elaborate **1** 条（实测 {}，回退路的 N = \
         {ENTRY_COMMANDS}）",
        reading.elaborated
    );
    assert_eq!(
        lib_checkpoint_arenas_leaked(),
        MAX_LEAKED,
        "命中复用路不许泄漏新 arena"
    );

    // 判定的正确性不变量（性能改动不许改变判定）
    let report = docs[0].report.as_ref().expect("必须有报告");
    for i in 0..DECLS {
        let name = format!("t{i:02}");
        let decl = report
            .decls
            .iter()
            .find(|d| d.name.as_deref() == Some(name.as_str()))
            .unwrap_or_else(|| panic!("`{name}` 必须在报告里"));
        assert_eq!(
            format!("{:?}", decl.status),
            "Checked",
            "改最后一条不许让没改过的 `{name}` 掉出 checked"
        );
    }
}
