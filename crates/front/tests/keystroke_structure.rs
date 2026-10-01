//! **G-29 按键结构计数**：`QueryDoc`（LSP 用的**会话式**路径）上「改一条声明」
//! 到底重查了多少条命令 —— 判据是**结构计数**，不是墙钟。
//!
//! 判据对齐用户 2026-10-01 拍板的**目标模型**（`docs/design/declaration-incremental.md` §5.1，
//! 「对齐 Lean4 的依赖图脏传播」），三条验收读数就在本文件里：
//!
//! | 场景 | 验收（结构计数） |
//! | --- | --- |
//! | 改第 k 条，**无人依赖**它 | `entry_kernel_checks` = **1** |
//! | 改第 k 条，被 m 条依赖 | `entry_kernel_checks` = **1 + m**（恰好那 m 条，不是"它后面全部"） |
//! | 脏闭包之外 | **逐字节不变**（`--json` + 课程门禁 43/377/99/0） |
//!
//! `entry_kernel_checks` = `CompileOutput.stats.kernel_checks`，它的定义就是
//! 「`try_check_declar` 的实际调用次数（**受信任前缀不计入**）」
//! （`crates/front/src/compile/event.rs`）⇒ **它就是"重查命令数"** ✓。
//!
//! 为什么放在这一层（而不是 CLI 或 LSP）：
//! * CLI 是**一次性进程** ⇒ 量不到"按键"（会话复用只活在同一个 `QueryDoc` 里）；
//! * LSP 的结构计数只在**进程退出**时打（`atexit`），而 LSP **不响应 `exit`**
//!   （实测：发 `shutdown`/`exit` 后进程不退出）⇒ 外面读不到 ✗；
//! * `QueryDoc::set_text` 就是 LSP 每次 `didChange` 走的那一步 ⇒ **这里量的就是
//!   按键的真实成本**，而所有计数都能**前后取差** ✓。
//!
//! 夹具是**合成项目**（`Lib ← Main`），不依赖课程目录 ⇒ 与 `perf_project.rs` 同纪律。
//!
//! ⚠ **口径**：`entry_kernel_checks` 是**本次编译**的读数（不受顺序影响 ✓）；
//! 其余计数是**进程级累计**取差 ⇒ 同一进程里后量的那次会看见更暖的判定缓存
//! （`by`/`infer_hits` 因此偏小）—— 判据只用 `entry_kernel_checks`，其余是诊断。

use std::path::PathBuf;

use sokonanoda_front::compile::{by_calls_total, module_compiles_total};
use sokonanoda_front::depgraph::DepGraph;
use sokonanoda_front::judge::infer_totals;
use sokonanoda_front::query::QueryDoc;

/// 一次按键的**结构读数**（全部是计数，没有墙钟）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Reading {
    /// **重查命令数**（`try_check_declar` 调用次数；受信任前缀不计入）—— 验收主读数。
    entry_kernel_checks: usize,
    /// 模块编译次数（库层每次按键被重编几次）。
    modules: u64,
    /// `by` 引擎调用次数。
    by: u64,
    /// 类型推断：调用 / 命中 / **未命中**。
    infer_calls: u64,
    infer_hits: u64,
    infer_miss: u64,
    /// 类型推断**重跑整份前缀**的趟数与字节数。
    prefix_runs: u64,
    prefix_bytes: u64,
}

impl Reading {
    /// 取一次快照（`doc` 给**本次编译**的 `kernel_checks`，其余是进程级累计）。
    fn snapshot(doc: &QueryDoc, base: Counters) -> Self {
        let now = Counters::now();
        Self {
            entry_kernel_checks: doc.compiled_output().stats.kernel_checks,
            modules: now.modules - base.modules,
            by: now.by - base.by,
            infer_calls: now.infer_calls - base.infer_calls,
            infer_hits: now.infer_hits - base.infer_hits,
            infer_miss: now.infer_miss - base.infer_miss,
            prefix_runs: now.prefix_runs - base.prefix_runs,
            prefix_bytes: now.prefix_bytes - base.prefix_bytes,
        }
    }

    fn json(self) -> String {
        format!(
            "{{\"entry_kernel_checks\":{},\"modules\":{},\"by\":{},\"infer_calls\":{},\
             \"infer_hits\":{},\"infer_miss\":{},\"infer_prefix_runs\":{},\"infer_prefix_bytes\":{}}}",
            self.entry_kernel_checks,
            self.modules,
            self.by,
            self.infer_calls,
            self.infer_hits,
            self.infer_miss,
            self.prefix_runs,
            self.prefix_bytes,
        )
    }
}

/// 进程级计数器的快照（取差用）。
#[derive(Debug, Clone, Copy)]
struct Counters {
    modules: u64,
    by: u64,
    infer_calls: u64,
    infer_hits: u64,
    infer_miss: u64,
    prefix_runs: u64,
    prefix_bytes: u64,
}

impl Counters {
    fn now() -> Self {
        let (calls, hits, miss, runs, bytes) = infer_totals();
        Self {
            modules: module_compiles_total(),
            by: by_calls_total(),
            infer_calls: calls,
            infer_hits: hits,
            infer_miss: miss,
            prefix_runs: runs,
            prefix_bytes: bytes,
        }
    }
}

/// 夹具的**依赖形状**（决定了验收该期望几条）：
///
/// ```text
/// t00 ─┬─► d00 ──► d01        （d01 **间接**依赖 t00：脏集必须**传递**）
///      └─► d02
/// t01 … t06                   （互相独立：无人依赖）
/// t07 = 最后一条              （无人依赖）
/// ```
///
/// ⇒ 改 `t00` 的脏集 = `{t00, d00, d01, d02}` = **1 + 3**（**传递闭包**，不是"直接依赖 2 条"）。
/// `LEAF` = 无人依赖的一条 · `ROOT` = 被 3 条（含 1 条间接）依赖的一条。
const DECLS: usize = 8;
const ROOT: usize = 0;
const LEAF: usize = 7;
/// **传递**依赖 `t00` 的条数：`d00` · `d01`（经 `d00`）· `d02`。
const ROOT_DEPENDENTS: usize = 3;

/// 生成合成闭包夹具：`Lib.sokonanoda`（一条自定义记法 + 三条公理）←
/// `Main.sokonanoda`（`DECLS` 条已证 theorem，其中 3 条依赖 `t00`）。
///
/// 记法与 `by` 都要有：前者让**类型推断**那条路真的被走到（`judge_infer`），
/// 后者让 `by` 引擎被走到 —— 两者是"一次按键重跑前缀"的两个来源。
fn gen_project(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("soko-keystroke-{tag}-{}", std::process::id()));
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
    // `t00` 的下游：3 条（含 1 条**间接**）真的引用 `t00` 的声明
    // ⇒ 目标模型下改 `t00` 要重查 **1+3**（脏集必须走**传递闭包**）。
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

fn open_doc(entry: &PathBuf, text: &str) -> QueryDoc {
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());
    doc.set_text(text, 1, None);
    doc
}

/// 改第 `index` 条 theorem 的**名字**（不动别的字节）——最干净的一次按键。
fn rename_decl(text: &str, index: usize) -> String {
    let needle = format!("theorem t{index:02} ");
    let replacement = format!("theorem t{index:02}x ");
    text.replacen(&needle, &replacement, 1)
}

/// 量三次按键：**无人依赖**的一条 · **被 2 条依赖**的一条 · **最后一条**。
fn measure(tag: &str) -> (Reading, Reading, Reading) {
    let (entry, text) = gen_project(tag);
    let mut doc = open_doc(&entry, &text);

    let press = |doc: &mut QueryDoc, index: usize, version: u64| -> Reading {
        let base = Counters::now();
        let edited = rename_decl(&text, index);
        doc.set_text(&edited, version, None);
        let reading = Reading::snapshot(doc, base);
        // **正确性不变量**（与"改哪一条"无关）：没被改的声明不许掉出 `checked`。
        let report = doc.report.as_ref().expect("改完必须有报告");
        for i in 0..DECLS {
            if i == index {
                continue;
            }
            let name = format!("t{i:02}");
            let decl = report
                .decls
                .iter()
                .find(|d| d.name.as_deref() == Some(name.as_str()))
                .unwrap_or_else(|| panic!("`{name}` 必须在报告里（改了 t{index:02} 之后）"));
            assert_eq!(
                format!("{:?}", decl.status),
                "Checked",
                "改了 t{index:02} ⇒ 没改过的 `{name}` 不许掉出 checked"
            );
        }
        doc.set_text(&text, version + 100, None); // 回到原样（缓存命中，清干净状态）
        reading
    };

    let leaf = press(&mut doc, LEAF, 2);
    let root = press(&mut doc, ROOT, 4);
    let mid = press(&mut doc, DECLS / 2, 6);
    (leaf, root, mid)
}

#[test]
fn keystroke_structure_is_measured() {
    let (leaf, root, mid) = measure("measure");
    println!(
        "PERF keystroke leaf(无人依赖)={:?}\n               root(被{ROOT_DEPENDENTS}条依赖)={:?}\n               mid={:?}",
        leaf, root, mid
    );
    println!(
        "PERFJSON {{\"case\":\"keystroke_structure\",\"leaf\":{},\"root\":{},\"mid\":{}}}",
        leaf.json(),
        root.json(),
        mid.json()
    );
    // 三次按键都必须**真的编了东西**（否则判据会被"什么都没编"骗过去）。
    //
    // ⚠ **不断言 `entry_kernel_checks > 0`**：**今天它恒为 0** —— 项目那条路
    // （`compile_project_with_overlay` → `compile_plan`）**没有把 `pass.checks`
    // 搬进 `out.stats.kernel_checks`**（只有 `project/session.rs` 那条搬了）。
    // 用户 2026-10-01 的验收读数正是"重查命令数"，所以这条**必须先修**才能判
    // （已记进台账 G-29）。在那之前：主读数打出来、用能用的计数做下限断言。
    assert!(leaf.modules > 0 && root.modules > 0 && mid.modules > 0, "改一条必须至少编一个模块");
    assert!(leaf.by > 0 && root.by > 0 && mid.by > 0, "夹具必须真的走到 `by` 引擎");
}

/// **目标用例**（`#[ignore]`，目标模型落地后应当翻绿）—— 用户 2026-10-01 的验收标准
/// （`docs/design/declaration-incremental.md` §5.1）：
///
/// * 改**无人依赖**的一条 ⇒ 重查命令数 = **1**；
/// * 改**被 3 条（含 1 条间接）依赖**的一条 ⇒ 重查命令数 = **1 + 3 = 4**
///   （**传递闭包**，恰好那几条，不是"后面全部"）。
///
/// 为什么现在 `#[ignore]`：今天是"整份重查"（前缀复用还没接到会话上），跑它必红；
/// 必红的用例不能进 `cargo test` 的默认集合。它的价值是把验收标准写成**可执行的一句话**：
/// `cargo test -p sokonanoda-front --test keystroke_structure -- --ignored --nocapture`
#[test]
#[ignore = "G-29 目标模型：依赖图脏传播落地前必红"]
fn dirty_propagation_target() {
    let (leaf, root, mid) = measure("target");
    println!("PERF keystroke leaf={:?} root={:?} mid={:?}", leaf, root, mid);
    assert_eq!(
        leaf.entry_kernel_checks, 1,
        "改**无人依赖**的一条 ⇒ 重查命令数必须是 1（今天 {}）",
        leaf.entry_kernel_checks
    );
    assert_eq!(
        mid.entry_kernel_checks, 1,
        "改**无人依赖**的中间一条 ⇒ 重查命令数必须是 1（今天 {}）",
        mid.entry_kernel_checks
    );
    assert_eq!(
        root.entry_kernel_checks,
        1 + ROOT_DEPENDENTS,
        "改**被 {ROOT_DEPENDENTS} 条依赖**的一条 ⇒ 重查命令数必须是 1+{ROOT_DEPENDENTS}（今天 {}）",
        root.entry_kernel_checks
    );
}

/// **依赖边必须在真实报告上成立**（S5 的材料 ①②）：`DepGraph` 靠
/// `DocumentReport.hover_cmds` + `ResolvedTarget::Declaration` 建边 ——
/// 这条用例在**真编译出来的报告**上验证那个假设（合成夹具的单元测试只能验算法）。
///
/// 夹具的依赖形状见 [`gen_project`]：`t00 ← d00 ← d01` 且 `t00 ← d02`
/// ⇒ 改 `t00` 的**脏集**必须是 `{t00, d00, d01, d02}`（**传递**闭包）；
/// 改最后一条 `t07`（无人依赖）⇒ 只有它自己。
#[test]
fn dependency_edges_come_from_the_real_report() {
    let (entry, text) = gen_project("depgraph");
    let doc = open_doc(&entry, &text);
    let report = doc.report.as_ref().expect("必须有报告");
    let graph = DepGraph::from_report(report);

    let cmd_of = |name: &str| {
        graph
            .declaration_command(name)
            .unwrap_or_else(|| panic!("`{name}` 必须在声明表里"))
    };
    let (t00, d00, d01, d02, t07) = (
        cmd_of("t00"),
        cmd_of("d00"),
        cmd_of("d01"),
        cmd_of("d02"),
        cmd_of("t07"),
    );
    assert!(
        graph.uses(d00).contains(&t00),
        "d00 引用 t00 ⇒ 必须建出边（uses(d00)={:?}）",
        graph.uses(d00)
    );
    assert!(
        graph.uses(d01).contains(&d00),
        "d01 引用 d00 ⇒ 必须建出边（uses(d01)={:?}）",
        graph.uses(d01)
    );
    let mut dirty = graph.dirty_commands(t00);
    dirty.sort_unstable();
    let mut want = vec![t00, d00, d01, d02];
    want.sort_unstable();
    assert_eq!(
        dirty, want,
        "改 t00 的脏集必须是传递闭包 {{t00,d00,d01,d02}}（只做直接依赖会漏 d01）"
    );
    assert_eq!(
        graph.dirty_commands(t07),
        vec![t07],
        "改无人依赖的最后一条 ⇒ 脏集只有它自己"
    );
    println!(
        "PERF depgraph t00_dirty={} leaf_dirty={} commands={} unknown={}",
        graph.dirty_commands(t00).len(),
        graph.dirty_commands(t07).len(),
        graph.commands(),
        graph.unknown_references().len(),
    );
}
