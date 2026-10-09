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

use std::path::{Path, PathBuf};

use sokonanoda_front::compile::DeclStatus;
use sokonanoda_front::compile::{
    by_calls_total, closure_module_compiles_total, elaborated_commands_total, module_compiles_total,
};
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
    /// **闭包模块编译次数**（G-29 的精确读数，2026-10-07）：只数**真模块**
    /// （`path: Some(..)`），**不数** judge 合成的判定文档。
    ///
    /// 与 `modules` 的分工（实测归因，设计 §33）：`modules` 只在 `check::run` 里累加
    /// ⇒ 会话那条路（`run_pass_with`）**一次都不计**，而 `judge` 的合成编译
    /// （`compile_fol_with`，`units=1 names=[""]`）**照计** ⇒ 它在"改一行"那一刀上
    /// 读到的是**合成编译次数**，不是"闭包被编了几个模块" ✗。这个才是后者。
    closure_modules: u64,
    /// **本次编译真的产出了事件/错误的命令数**（去重）——「重查命令数」的
    /// 旁证：被复用（信任前缀/缓存）的命令**不产出事件** ⇒ 这个数会跟着脏集走。
    ///
    /// ⚠ **它是旁证不是判据**：不产出事件的命令（`import`、纯 `#check` 之类）
    /// 本来就不进这个数 ⇒ 它**只会偏小**，不能拿来判"重查命令数 = 1" ✗。
    /// 判据用 `entry_kernel_checks`（语义精确：`try_check_declar` 调用次数）。
    recomputed_commands: usize,
    /// `by` 引擎调用次数。
    by: u64,
    /// 类型推断：调用 / 命中 / **未命中**。
    infer_calls: u64,
    infer_hits: u64,
    infer_miss: u64,
    /// 类型推断**重跑整份前缀**的趟数与字节数。
    prefix_runs: u64,
    prefix_bytes: u64,
    /// **T2-B 的判据读数**：这一刀**真的 elaborate 了几条命令**（进程级累计取差）。
    ///
    /// ⚠ **今天它 = 入口命令数（+ 库层那几条）** —— 命令级快照还没做 ⇒ 每刀从第 0 条
    /// 重走 ✗。**先建先红**：下面的断言**写死今天的行为**（防漂移 ✓）；
    /// **T2-B 落地后改判成"改最后一条 ⇒ 1"**（**不许放宽** ✗，同 T2-B0 的先例）。
    elaborated_commands: u64,
}

impl Reading {
    /// 取一次快照（`doc` 给**本次编译**的 `kernel_checks`，其余是进程级累计）。
    fn snapshot(doc: &QueryDoc, base: Counters) -> Self {
        let now = Counters::now();
        let out = doc.compiled_output();
        let mut cmds: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        cmds.extend(out.event_cmds.iter().copied());
        cmds.extend(out.error_cmds.iter().copied());
        cmds.extend(out.warning_cmds.iter().copied());
        // **重查命令数**：项目编译从 `ProjectReport` 读（`669b6f2a` 起它带上
        // `kernel_checks`；入口那份 `CompiledOutput` 在闭包路径上是组装出来的，
        // 它的 `stats` 不含这一项 ✗）；单文件路径退回 `CompiledOutput.stats`。
        let entry_kernel_checks = doc
            .project_report_ref()
            .map_or(out.stats.kernel_checks, |p| p.kernel_checks);
        Self {
            entry_kernel_checks,
            modules: now.modules - base.modules,
            closure_modules: now.closure_modules - base.closure_modules,
            recomputed_commands: cmds.len(),
            by: now.by - base.by,
            infer_calls: now.infer_calls - base.infer_calls,
            infer_hits: now.infer_hits - base.infer_hits,
            infer_miss: now.infer_miss - base.infer_miss,
            prefix_runs: now.prefix_runs - base.prefix_runs,
            prefix_bytes: now.prefix_bytes - base.prefix_bytes,
            elaborated_commands: now.elaborated_commands - base.elaborated_commands,
        }
    }

    fn json(self) -> String {
        format!(
            "{{\"entry_kernel_checks\":{},\"recomputed_commands\":{},\"modules\":{},\
             \"closure_modules\":{},\"by\":{},\
             \"infer_calls\":{},\"infer_hits\":{},\"infer_miss\":{},\"infer_prefix_runs\":{},\
             \"infer_prefix_bytes\":{},\"elaborated_commands\":{}}}",
            self.entry_kernel_checks,
            self.recomputed_commands,
            self.modules,
            self.closure_modules,
            self.by,
            self.infer_calls,
            self.infer_hits,
            self.infer_miss,
            self.prefix_runs,
            self.prefix_bytes,
            self.elaborated_commands,
        )
    }
}

/// 进程级计数器的快照（取差用）。
#[derive(Debug, Clone, Copy)]
struct Counters {
    modules: u64,
    closure_modules: u64,
    by: u64,
    infer_calls: u64,
    infer_hits: u64,
    infer_miss: u64,
    prefix_runs: u64,
    prefix_bytes: u64,
    /// **真的 elaborate 过的命令数**（T2-B 的判据读数）。
    elaborated_commands: u64,
}

impl Counters {
    fn now() -> Self {
        let (calls, hits, miss, runs, bytes) = infer_totals();
        Self {
            modules: module_compiles_total(),
            closure_modules: closure_module_compiles_total(),
            by: by_calls_total(),
            infer_calls: calls,
            infer_hits: hits,
            infer_miss: miss,
            prefix_runs: runs,
            prefix_bytes: bytes,
            elaborated_commands: elaborated_commands_total(),
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

fn open_doc(entry: &Path, text: &str) -> QueryDoc {
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.to_path_buf());
    doc.set_text(text, 1, None);
    doc
}

/// 一次**真实且长度不变**的按键：把第 `index` 条 theorem 的**局部 binder 改名**
/// （`(h : …) … := by exact h` → `(k : …) … := by exact k`）。
///
/// ⚠ **必须长度不变**：改名只要改了字节数，后面每条命令的 `starts` 都会平移 ⇒
/// 缓存里的 span 不能再当新的用 ⇒ 它们**必须**进脏集（否则报告里的 span 是错的 ✗）
/// ⇒ 量出来的就不是"依赖脏集"而是"后缀 + 平移"✗。这一条是实测踩出来的。
/// ⚠ 也**不能**改声明名（`t00` → `u00`）：那会让引用它的下游**解析不到** ⇒ 它们
/// 真的坏了（判定翻转），量到的就不是用户模型里的"1+m" ✗。
fn edit_decl(text: &str, index: usize) -> String {
    let needle = format!("theorem t{index:02} ");
    let at = text
        .find(&needle)
        .unwrap_or_else(|| panic!("找不到 `{needle}`"));
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    let line = &text[at..end];
    let edited = line
        .replacen("(h :", "(k :", 1)
        .replacen("exact h", "exact k", 1);
    assert_eq!(
        edited.len(),
        line.len(),
        "这次按键必须**长度不变**（否则下游命令的起点平移 ⇒ 它们进脏集，量到的不是依赖脏集）"
    );
    format!("{}{}{}", &text[..at], edited, &text[end..])
}

/// 入口文件的**命令数**（`import Lib` + `DECLS` 条 `t…` + 3 条 `d…`）——
/// T2-B 判据里的那个 N ✓。
const ENTRY_COMMANDS: u64 = DECLS as u64 + 1 + 3;

/// 一次**改最后一条命令**的按键（长度不变 ✓）：`d02` 的局部 binder 改名
/// （`(h :` → `(k :`，`exact t00 a b h` → `exact t00 a b k`）。
///
/// 为什么专门有它：T2-B 的判据正是「改**最后一条** ⇒ elaborate 命令数 = **1**」✓。
fn edit_last_decl(text: &str) -> String {
    let at = text.find("theorem d02 ").expect("找不到 `theorem d02`");
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    let line = &text[at..end];
    let edited = line
        .replacen("(h :", "(k :", 1)
        .replacen("exact t00 a b h", "exact t00 a b k", 1);
    assert_eq!(
        edited.len(),
        line.len(),
        "这次按键必须**长度不变**（否则下游起点平移，量到的不是「最后一条」）"
    );
    format!("{}{}{}", &text[..at], edited, &text[end..])
}

/// **T2-B 的"先建先红"**（2026-10-09）：今天，**改最后一条命令**也要把入口趟的
/// **每一条命令**从头重走一遍 ✗（命令级环境快照未做）。
///
/// ## 判据（`PLAN-align-lean4` §3.3 T2-B）
///
/// 改**最后一条** ⇒ [`elaborated_commands_total`] 的增量 = **1**（今天 = 入口命令数
/// N = [`ENTRY_COMMANDS`]，另加库层那几条）；改第 k 条 ⇒ N−k+1。
/// **反向验证**：改依赖文件 ⇒ 回到 N。
///
/// ## 为什么这条现在断言的是"今天的行为"
///
/// T2-B 是多环节件（命令级快照要连 **walk 的累加器**一起留：`known`/`inductives`/
/// `defs`/`out`/`ops`…，且跨按键要留住 arena）⇒ 本轮先把**读数**立起来 ✓。
/// **先建先红**的纪律照 T2-B0 的先例：**T2-B 落地后把这里改判成 `== 1`，
/// 不许放宽** ✗。
#[test]
fn t2b_last_command_edit_still_reelaborates_every_entry_command() {
    let (entry, text) = gen_project("t2b");
    let mut doc = open_doc(&entry, &text);
    let base = Counters::now();
    let edited = edit_last_decl(&text);
    doc.set_text(&edited, 2, None);
    let reading = Reading::snapshot(&doc, base);
    println!(
        "PERF t2b 改最后一条：elaborated_commands={}（入口命令数 N={ENTRY_COMMANDS}）",
        reading.elaborated_commands
    );
    // **有牙**：读数不许是 0（那说明量具坏了、或什么都没编 ✗）。
    assert!(
        reading.elaborated_commands > 0,
        "改最后一条必须真的 elaborate 了东西（读到 0 ⇒ 量具坏了 ✗）"
    );
    // **T2-B 的判据**：入口趟只 elaborate **最后那一条** ✓
    // （检查点边界 = 倒数第二条之后 ⇒ 前缀整段跳过 ✓）。
    assert_eq!(
        reading.elaborated_commands, 1,
        "改**最后一条**命令 ⇒ 入口趟只许 elaborate **1** 条；实测 {} ⇒ 检查点没命中 \
         （或判据被人放宽了 ✗）",
        reading.elaborated_commands
    );
}

/// **T2-B 的正确性守卫**：续编出来的报告里，入口的**每条声明恰好出现一次** ✓，
/// 且被改的那条**真的重判过**（`Checked`）✓。
///
/// 为什么是这条（而不是"两臂逐字段对拍"）：`QueryDoc` 的信任前缀缓存
/// （`EntryCache`）是**线程局部且按内容键**的 ⇒ 同一进程里"续编臂"会喂"重编臂"
/// ⇒ 两臂**不可比** ✗（实测：重编那臂只报 1 条声明 —— 量具坏了，不是结果坏了）。
/// **干净的 A/B 要进程隔离**（真 LSP 子进程 + 每臂全新缓存，同
/// `crates/lsp/tests/perf_keystroke_wallclock.rs`）⇒ 记为**已知验证缺口**（设计档 §6.4）。
///
/// 这条**真咬过东西** ✓：快照装回报告侧累加器（`decl_states`）之后，`EntryCache`
/// 的前缀拼接又给一份 ⇒ 报告里每条声明**出现两次**（22 vs 13）⇒ 正是它逮到的 ✓。
#[test]
fn t2b_resumed_report_has_no_duplicate_declarations() {
    let (entry, text) = gen_project("t2b-dup");
    let mut doc = open_doc(&entry, &text);
    let edited = edit_last_decl(&text);
    doc.set_text(&edited, 2, None); // ← 这一刀命中续编（只 elaborate 1 条命令）
    let report = doc.report.as_ref().expect("报告");

    // ① **不许重复**：入口自己的每条声明（`t…` / `d…`）恰好一次 ✓。
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for decl in &report.decls {
        if let Some(name) = &decl.name {
            if name.starts_with('t') || name.starts_with('d') {
                *counts.entry(name.clone()).or_default() += 1;
            }
        }
    }
    let dup: Vec<_> = counts.iter().filter(|(_, n)| **n != 1).collect();
    assert!(
        dup.is_empty(),
        "续编之后报告里出现**重复声明** ✗（{dup:?}）⇒ 快照装回的那一份与 \
         `EntryCache` 的前缀拼接**各算了一次**（历史上正是这个 bug ✓）"
    );
    // ② **被改的那条真的重判过** ✓（续编不是"跳过一切" ✗）。
    let d02 = report
        .decls
        .iter()
        .find(|d| d.name.as_deref() == Some("d02"))
        .expect("d02 必须在报告里");
    assert!(
        matches!(d02.status, DeclStatus::Checked),
        "被改的那条 `d02` 必须是 `Checked`（续编只跳过**没变的前缀** ✗）—— 实测 {:?} · \
         诊断 {:?}",
        d02.status,
        d02.error.as_ref().map(|e| e.message.clone())
    );
}

/// 量三次按键：**无人依赖**的一条 · **被 2 条依赖**的一条 · **最后一条**。
fn measure(tag: &str) -> (Reading, Reading, Reading) {
    let (entry, text) = gen_project(tag);
    let mut doc = open_doc(&entry, &text);

    let press = |doc: &mut QueryDoc, index: usize, version: u64| -> Reading {
        let base = Counters::now();
        let edited = edit_decl(&text, index);
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
    //
    // ⚠ **2026-10-07 第 3 棒：下限改用 `closure_modules`**（`modules` 现在如实读出 **0** ✗）——
    // `modules` = `MODULE_COMPILES`，只在 `check::run` 里累加、**会话那条路一次都不计**
    // ⇒ 它此前 > 0 **只是因为** judge 的合成编译回退调了 `compile_fol_with`（`run(units=1)` ✗）。
    // 第 3 棒把那些合成编译**导回增量路**（`run_incremental` ⇒ 不经 `run`）之后它**如实变成 0**
    // —— 那是量具口径，不是"什么都没编" ✗。这条断言要说的事由 `closure_modules` 承担
    // （**真模块**被编了几个 ✓，老路/会话路**同口径** ✓）—— 读数仍是 5/4/4 > 0 ⇒ **守卫照咬** ✓。
    assert!(
        leaf.closure_modules > 0 && root.closure_modules > 0 && mid.closure_modules > 0,
        "改一条必须至少编一个模块（**闭包**模块数；`modules` 那条口径见上面的 2026-10-07 注 ✗）"
    );
    assert!(
        leaf.by > 0 && root.by > 0 && mid.by > 0,
        "夹具必须真的走到 `by` 引擎"
    );
}

/// **验收判据**（用户 2026-10-01 的 §5.1，**常驻**）
/// （`docs/design/declaration-incremental.md` §5.1），**按声明数**：
///
/// * 改**无人依赖**的一条 ⇒ 重查**声明数 = 1**；
/// * 改**被 3 条（含 1 条间接）依赖**的一条 ⇒ **1 + 3 = 4**（**传递闭包**，恰好那几条）。
///
/// 读数是 `recomputed_commands`（本次编译**真的产出事件**的命令数）——它就是
/// "重查了几条声明"：被复用（信任前缀/缓存）的命令**不产出事件** ⇒ 当前实测
/// leaf/mid/root = **4/7/8**（= **后缀**，S2 的语义）✗，目标 **1/1/4**（= **脏集**，S6）。
///
/// ⚠ 不用 `entry_kernel_checks` 判：那个数的是 `try_check_declar` **调用次数**，
/// 实测**一条声明 ≈ 2 次**（签名 + 值）⇒ "1" 那个目标在它上面根本不可达 ✗
/// （它的用途是**同一形状下的前后对比**，见 `keystroke_structure_is_measured` 的打印）。
///
/// **S6 落地后它已经翻绿**（2026-10-01）⇒ 从"目标"升格成**常驻回归守卫**（不再 `#[ignore]`）：
/// 谁把脏集改回"后缀"，这里当场判红 ✓。
#[test]
fn dirty_propagation_is_the_acceptance_criterion() {
    let (leaf, root, mid) = measure("target");
    println!(
        "PERF keystroke leaf={:?}\n               root={:?}\n               mid={:?}",
        leaf, root, mid
    );
    assert_eq!(
        leaf.recomputed_commands, 1,
        "改**无人依赖**的一条 ⇒ 重查声明数必须是 1（今天 {}）",
        leaf.recomputed_commands
    );
    assert_eq!(
        mid.recomputed_commands, 1,
        "改**无人依赖**的中间一条 ⇒ 重查声明数必须是 1（今天 {}）",
        mid.recomputed_commands
    );
    assert_eq!(
        root.recomputed_commands,
        1 + ROOT_DEPENDENTS,
        "改**被 {ROOT_DEPENDENTS} 条（含间接）依赖**的一条 ⇒ 重查声明数必须是 1+{ROOT_DEPENDENTS}（今天 {}）",
        root.recomputed_commands
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

/// **开档命中产物缓存之后，第一刀也要能信任前缀**（2026-10-01 修的那个用户可见毛刺）。
///
/// 场景：编辑器打开一个带 `import` 的单元，产物缓存命中（真 LSP 实测开档 11ms）——
/// 若回放时不填 `entry_cache`，**下一刀没有可信任的前缀 ⇒ 恒为整闭包**
/// （实测第一刀 **2977ms**，而第二刀只要 **314ms**）。这条判据咬的就是它：
/// 用 `set_cached_entry` 模拟"缓存回放"，再改最后一条，`trusted_prefix_len()` 必须 > 0。
///
/// **反向验证**：去掉 `set_cached_entry` 里那次 `self.entry_cache = self.cached_entry_for(text)`
/// ⇒ 这条当场判红（实测过）。
#[test]
fn a_cached_open_still_leaves_a_usable_entry_cache() {
    let (entry, text) = gen_project("cachedopen");
    // 先真编一次，拿到"缓存里会存的那三样"。
    let doc = open_doc(&entry, &text);
    let report = doc.report.clone().expect("第一次编译必须有报告");
    let output = doc.compiled_output().clone();
    let project = doc.project_report_ref().cloned();

    // 模拟**开档命中产物缓存**：全新的 doc + `set_cached_entry` 回放。
    let mut fresh = QueryDoc::new();
    fresh.path = Some(entry.clone());
    fresh.set_cached_entry(&text, 1, report, output, project);

    // 第一刀：改最后一条（无人依赖）。
    let edited = edit_decl(&text, LEAF);
    fresh.set_text(&edited, 2, None);
    assert!(
        fresh.trusted_prefix_len() > 0,
        "缓存回放之后的第一刀必须能信任前缀（否则它恒为整闭包：实测 2977ms vs 314ms）"
    );
}

/// **prelude 模式变了 ⇒ 信任必须失效**（2026-10-01 补的静默错编口子）。
///
/// 模式是**文件注释指令**（`-- soko:prelude bare`）决定的，而注释**不是命令** ⇒
/// 改指令**不改任何命令的文本** ⇒ 只看命令文本的 `trusted_prefix` 会认**整份**前缀
/// ⇒ 拿另一个 prelude 下的结论当这一份的 ✗。判据：模式一变，`trusted_prefix_len()` 必须归 0。
///
/// **反向验证**：把 `dependency_fingerprint` 里那两行 `prelude` 的 `mix` 去掉 ⇒ 这条判红。
#[test]
fn a_prelude_mode_change_invalidates_the_entry_trust() {
    let (entry, text) = gen_project("prelude");
    let mut doc = open_doc(&entry, &text);
    // 同一份文本、显式换模式（`Bare` 不装 prelude）——命令文本一个字都没变。
    doc.set_text(&text, 2, Some(sokonanoda_front::compile::PreludeMode::Bare));
    assert_eq!(
        doc.trusted_prefix_len(),
        0,
        "prelude 模式变了 ⇒ 一条都不许信任（注释不是命令，文本比对看不见这个变化）"
    );
}

/// **改了记法声明 ⇒ 后缀一条都不许信任**（2026-10-01 自查发现的第三个信任边界口子）。
///
/// 记法用法的 hover 是 `ResolvedTarget::Notation { span, module }`：`span` 是**使用处**、
/// `module` 只在跨模块声明时有值 ⇒ **建不出"用到它"这条边** ✗ ⇒ 改了 `infix:` 的目标
/// 而使用处文本没变时，使用处会被当成"干净"⇒ 拿旧含义的结论 ⇒ **静默错编** ✗。
/// 修法：**记法声明一变就整份重查**（保守；记法声明极少改 ✓）。
///
/// 判据用**重查命令数**（`recomputed_commands`），**实测标定过**：
/// **不带**这条规则 ⇒ **0** ✗✗（`infix` 行本身不产出事件，而用到 `⊕` 的 theorem
/// 被当成"干净" ⇒ 一条都不重查 ⇒ 拿旧含义的结论）；**带**规则 ⇒ **2** ✓。
/// **反向验证**：把 `dirty_commands` 里那段 `notation_decl_changed` 去掉 ⇒ 这条判红
/// （实测 `left: 0`）✓。
#[test]
fn a_notation_declaration_change_invalidates_the_whole_suffix() {
    let dir = std::env::temp_dir().join(format!("soko-notation-trust-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("Lib.sokonanoda"), "axiom Point : Type\n").expect("write Lib");
    let text = "import Lib\n\ninfix:50 \" \u{2295} \" => EqP\n\naxiom EqP : Point -> Point -> Prop\n\naxiom Other : Point -> Prop\n\ntheorem t (a b : Point) (h : a \u{2295} b) : a \u{2295} b := by exact h\n";
    let entry = dir.join("Main.sokonanoda");
    std::fs::write(&entry, text).expect("write Main");
    let mut doc = QueryDoc::new();
    doc.path = Some(entry.clone());
    doc.set_text(text, 1, None);

    let base = Counters::now();
    let edited = text.replace("=> EqP", "=> EqQ");
    assert_eq!(edited.len(), text.len(), "这一刀也保持长度不变");
    doc.set_text(&edited, 2, None);
    let after = Reading::snapshot(&doc, base);
    assert_eq!(
        after.recomputed_commands, 2,
        "记法声明变了 ⇒ 用到它的命令必须重查（**不带这条规则实测是 0**：一条都不重查 ✗）"
    );
}
