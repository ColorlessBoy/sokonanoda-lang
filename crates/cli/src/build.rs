//! `sokonanoda build [--clean] [<path> ...]`: warm (and inspect) the shared
//! persistent compile cache so later `check`/`course`/LSP runs are hits
//! (docs/design/compile-cache.md; event contract in docs/protocol.md).
//!
//! A positional may be a `.sokonanoda` file or a directory (walked
//! recursively, sorted). No positional means the current directory. Exit is 0
//! when at least one file resolved; a path that resolves to nothing is usage
//! (FAILURE). Parse/read failures are counted, not fatal.

use sokonanoda_front::compile::cache::{self, CachedCompile};
use sokonanoda_front::compile::{compile_all_with, prelude_mode_from_source, CompileOptions};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// **P2 心跳周期**（**1000ms，不变**）：这么久没有任何其它输出 ⇒ 发一条 `build.tick`。
///
/// ⚠ 2026-09-30 一度把它放宽到 5000ms 并"默认一律不发"，**实测证明那是错的** ✗
/// （CI 当场判红：`scripts/check-progress-gap.py` 报 `最长无输出间隔 4.99s > 2.5s`）：
/// 那条判据要的是"**能力上限** —— 两条通道都在时能做到多好"，而 5s 的周期
/// **本身就达不到 2.5s** ⇒ 放宽周期等于把契约悄悄改掉 ✗。
///
/// **正确的口径是工作单三选一里的第一项「非管道不发」**（见 [`tick_period_ms`]）：
/// 周期**不动**（机器消费者拿到的仍是 1s 心跳，契约照旧 ✓），
/// 只是**人看的终端**不再发 ✓ —— 那才是用户抱怨的东西。
///
/// 用户实测（冷编 `courses/set-theory`，159 条 tick）：`file` **全是空串**、
/// 且**全排在第一条 `build.decl` 之前** —— 文件级进度要等**全部**文件编译完才按
/// `files` 顺序重放，所以 `set_file` 在整个编译期间恒为空。这说明**心跳不是进度**
/// （扩展的 `percent` 一直是文件级，且由 `build.progress` 驱动），
/// 但它**仍是"可见面在动"的兜底**，对**管道消费者**保留 ✓。
const DEFAULT_TICK_MS: u64 = 1000;

/// **心跳要不要发、按什么周期**（`None` = 不发；**默认不发** ✓）。
///
/// 用户 2026-09-29 实测原话：终端每秒刷 `{"elapsed_ms":1001,"file":"","type":"build.tick"}`。
/// 根因不是周期，是**发错了地方**：心跳写的是 stdout，而**人看的终端与管道消费者
/// 共用同一个 stdout** ✗（工作单给的三个修法里，「周期 5s」只让它慢一点、
/// 「`file` 空不发」实测会把心跳**整个删掉**（159/159 空））。
///
/// ⇒ 口径：**默认不发**，谁要谁显式要 ✓。仓库内的消费者（VS Code 扩展）
/// **自己 spawn 子进程**（`runBuildProcess`）⇒ 它默认拿不到 tick 是**有意的**：
/// 它把每一行原样 `appendLine` 进「sokonanoda build」输出面板 ⇒ 159 行 JSON 刷屏
/// 对用户是噪声，而它渲染的百分比本来就是文件级（`build.file`）✓。
///
/// 开关（**三档，默认那档就是工作单的「非管道不发」**）：
///
/// 1. **默认**：`stdout` **是管道**（机器消费者：扩展 spawn 的子进程、`| jq`、
///    CI 的重定向）⇒ **发**，周期 [`DEFAULT_TICK_MS`]；**是终端**（人在看）
///    ⇒ **不发** ✓ —— 这正是"人看的终端不再刷 + 机器消费者仍能拿到心跳"两句话的
///    合取，也是工作单三选一里的第一项；
/// 2. `SOKO_BUILD_TICK_MS=<毫秒>` ⇒ **强制发**，用这个周期（终端里想看心跳的人/脚本）；
///    **空串** = 要心跳但用默认周期；
/// 3. `SOKO_BUILD_TICK_MS=0` 或 `SOKO_BUILD_NO_TICK=1` ⇒ **强制不发**（逃生门）。
///
/// ⚠ **第一版把默认写成"一律不发"是错的** ✗（CI 当场判红）：那样**机器消费者也拿不到
/// 心跳**，`scripts/check-progress-gap.py` 的「最长无输出间隔 ≤ 2.5s」立刻假红
/// （实测 `4.99s > 2.5s`）—— 而那条判据要的正是"**能力上限**：两条通道都在时能做到
/// 多好"。用户抱怨的是**终端**刷屏，不是管道里有心跳 ✓。
fn tick_period_ms() -> Option<u64> {
    if std::env::var_os("SOKO_BUILD_NO_TICK").is_some() {
        return None;
    }
    match std::env::var("SOKO_BUILD_TICK_MS") {
        Ok(raw) => match raw.trim() {
            // 空串 = "要心跳，但没指定周期" ⇒ 用默认周期
            "" => Some(DEFAULT_TICK_MS),
            _ => match raw.trim().parse::<u64>() {
                Ok(0) => None,
                Ok(ms) => Some(ms),
                // 非法值 ⇒ **不发**（绝不因为一个坏环境变量去刷用户的屏 ✗）
                Err(_) => None,
            },
        },
        // 没显式设置 ⇒ **看 stdout 是不是管道**：管道 = 机器消费者 ⇒ 发；
        // 终端 = 人在看 ⇒ 不发 ✓。
        Err(_) => {
            use std::io::IsTerminal;
            (!std::io::stdout().is_terminal()).then_some(DEFAULT_TICK_MS)
        }
    }
}
/// 所有 `--json` 输出走同一把锁：心跳线程与编译线程都会写 stdout，
/// 不加锁会**串行交错**（两条 JSON 拼在一行 ⇒ 消费者解析失败）✗。
static PRINT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn emit_json(value: serde_json::Value) {
    let _guard = PRINT_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    println!("{value}");
}

/// **② 文件级进度**（2026-09-30）：一个文件**编译完**就报一条 `build.progress`。
///
/// 为什么非加不可（用户 2026-09-29 实测「Rebuild 长时间 0%、最后突跳」）：
/// `build.file` 要等**全部**文件编译完才按 `files` 顺序重放 ⇒ 编译期间**一条
/// 进度都没有**（实测冷编课程：首条 `build.decl` 在 **160.8s** 之后），
/// 于是前端只能把 `0%` 一直挂着、最后几十毫秒里从 0 跳到 100 ✗。
///
/// **为什么是新事件而不是"提前发 `build.file`"**：`build.file` 的顺序与内容是
/// `--json` 的**确定性红线**（并行只并行编译，输出仍按 `files` 顺序重放 ⇒ 与串行
/// 逐字节相同）。提前发就会让顺序随线程完成次序变 ⇒ 破红线 ✗。
/// ⇒ **additive 的新 `type`**（老消费者忽略未知 `type` ✓，`docs/protocol.md` 已同步）：
/// 它**只带"已完成几个"**（`done`/`total`/`file`），是**进度**不是结果，
/// 消费者按"最后一条为准"渲染 ⇒ 天然并发安全（少一条只是少一次刷新）✓。
///
/// **它替代了心跳的位置**：有了它，可见面**只在真有文件编完时才动**
/// （不假装百分比 ✓）；而 `build.tick` 是**按墙钟**的兜底（默认关，见
/// [`tick_period_ms`]）—— 两者互补，不是二选一。
struct ProgressCounter {
    done: std::sync::atomic::AtomicUsize,
    total: usize,
    enabled: bool,
}

impl ProgressCounter {
    fn new(total: usize, enabled: bool) -> Self {
        Self {
            done: std::sync::atomic::AtomicUsize::new(0),
            total,
            enabled,
        }
    }

    /// 一个文件编完了 ⇒ 报一条。**只报"已完成几个"**（不报是谁：结果按顺序重放）。
    fn note_file(&self, file: &Path) {
        use std::sync::atomic::Ordering;
        if !self.enabled {
            return;
        }
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        emit_json(serde_json::json!({
            "type": "build.progress",
            "done": done,
            "total": self.total,
            "file": file.display().to_string(),
        }));
    }
}

/// **P2 心跳**：声明级事件之间的间隔仍可能很长（实测 `unit08-solution` 里**单条声明**
/// 最贵 ~14s —— 那是一条 `by` 证明内部的判定，前端没有更细的回调点）⇒ 光有声明级
/// 事件，UI 还是会"长时间不动"✗。心跳**只报"已用时 + 当前文件"**（**不假装百分比** ✓），
/// 保证用户可见面持续在动 ⇒ 判据「最长无输出间隔 ≤ N 秒」的 N 由它兜底。
struct Heartbeat {
    t0: std::time::Instant,
    last_ms: std::sync::Arc<std::sync::atomic::AtomicU64>,
    current: std::sync::Arc<std::sync::Mutex<String>>,
    /// **停**（`Condvar` 唤醒，见 [`Heartbeat::stop`]）—— 不用 `sleep` 干等。
    gate: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Heartbeat {
    /// `period_ms = None` ⇒ **不发心跳**（默认路径：零线程、零输出 ✓）。
    fn start(period_ms: Option<u64>) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::{Arc, Condvar, Mutex};
        let t0 = std::time::Instant::now();
        let last_ms = Arc::new(AtomicU64::new(0));
        let current = Arc::new(Mutex::new(String::new()));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let join = period_ms.map(|period| {
            let (last, cur) = (Arc::clone(&last_ms), Arc::clone(&current));
            let g = Arc::clone(&gate);
            std::thread::spawn(move || {
                let base = std::time::Instant::now();
                loop {
                    // ⚠ **不许用 `sleep(period)` 干等** ✗（2026-09-30 CI 实测）：
                    // `stop()` 会 `join()` 这个线程，而 `sleep` 中的线程要**睡满
                    // 一整个周期**才醒来检查停止位 ⇒ **每一次 build 都白等 up to
                    // `period`**（实测：`perf_project` 的冷/热两趟从 60ms/4.5ms
                    // 一起变成 **~1025ms**，因为两边都多付了 1s ✗✗）。
                    // `wait_timeout` 既能被 `stop()` 立刻唤醒、又保留周期语义 ✓。
                    let (lock, cvar) = &*g;
                    let stopped = {
                        let guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                        let (guard, _timeout) = cvar
                            .wait_timeout(guard, std::time::Duration::from_millis(period))
                            .unwrap_or_else(|p| p.into_inner());
                        *guard
                    };
                    if stopped {
                        return;
                    }
                    let now = base.elapsed().as_millis() as u64;
                    if now.saturating_sub(last.load(Ordering::Relaxed)) >= period {
                        last.store(now, Ordering::Relaxed);
                        let file = cur.lock().map(|g| g.clone()).unwrap_or_default();
                        emit_json(serde_json::json!({
                            "type": "build.tick",
                            "elapsed_ms": now,
                            "file": file,
                        }));
                    }
                }
            })
        });
        Self {
            t0,
            last_ms,
            current,
            gate,
            join,
        }
    }

    /// 刚发过一条**真**事件 ⇒ 心跳让位（避免"真事件 + 心跳"刷屏）。
    fn note(&self) {
        self.last_ms.store(
            self.t0.elapsed().as_millis() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    fn set_file(&self, file: &str) {
        if let Ok(mut guard) = self.current.lock() {
            *guard = file.to_string();
        }
    }

    fn stop(&mut self) {
        // **置位 + 立刻唤醒**（`Condvar`）⇒ 不用等满一个周期 ✓。
        {
            let (lock, cvar) = &*self.gate;
            let mut stopped = lock.lock().unwrap_or_else(|p| p.into_inner());
            *stopped = true;
            cvar.notify_all();
        }
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

pub(crate) fn build(
    args: &[String],
    json: bool,
    clean: bool,
    root: Option<&str>,
    no_project: bool,
) -> ExitCode {
    if clean {
        // R-3（T-B5）：项目条目现在落在**模块根**的 `.sokonanoda/compiled/`，所以
        // `--clean` 必须**两处都清** —— 只清全局的话 `rebuild`（= `build --clean`）
        // 会命中项目条目 ⇒ 表面"清空了"，实际什么都没重编（用户可见的假动作 ✗）。
        let global = cache::clean();
        let mut project = 0usize;
        for root in project_roots(args) {
            project += sokonanoda_front::project::cache::clean_at(&root);
        }
        let removed = global + project;
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "type": "build.clean",
                    "removed": removed,
                    // additive：老消费者读 `removed`（= 两处之和）语义不变 ✓
                    "global": global,
                    "project": project,
                })
            );
        } else {
            println!("removed {removed} cached file(s) ({global} global, {project} project)");
        }
        return ExitCode::SUCCESS;
    }

    let roots: Vec<PathBuf> = if args.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!("usage: sokonanoda build [--json] [--clean] [<file.sokonanoda> | <dir> ...]");
        return ExitCode::FAILURE;
    }

    let mut hit = 0usize;
    let mut compiled = 0usize;
    let mut failed = 0usize;
    // **E23**：先把**总数**说出去 —— 进度条要报「3/13 文件」，而总数只在
    // `build.summary` 里、那已经是结束之后了 ✗。additive：老消费者忽略未知
    // `type` ✓（`docs/protocol.md` 的 build 事件契约已同步）。
    if json {
        println!(
            "{}",
            serde_json::json!({"type": "build.begin", "files": files.len()})
        );
    }
    // **P2 心跳**：**默认不发**（见 [`tick_period_ms`] 的实测依据）；要就显式开。
    let mut heartbeat = Heartbeat::start(tick_period_ms());

    // ═══ **入口级并行编译**（2026-09-29，用户 09:43「那就并行编译」）═══
    //
    // **为什么入口级**：基线本来就是"**每个入口独立编自己的闭包**"（42 个入口互不共享
    // 环境）⇒ 并行 = 把这个 for 循环并行跑，**架构上现成**，不需要任何产物复用 ✓。
    // （模块级并行要按拓扑序排依赖、还要处理共享环境 —— 那是另一个形状，
    //  等"模块级产物"落地后再做。）
    //
    // **线程安全**（逐项核对过，2026-09-29）：
    //   * arena：`run_pass_in` 起就是 **per-call**（切片 1a 把 arena 提到调用方）✓
    //   * judge 的两张缓存：`OnceLock<Mutex<HashMap>>` ✓（锁竞争是唯一代价）
    //   * `quiet_catch` / prelude 安装深度 / judge 批次：**thread_local** ✓
    //   * 统计计数器：`AtomicU64` ✓ · `install_printer`/`install_quiet_hook`：`Once` ✓
    //   * LSP 侧全局只有 `OnceLock` ✓（`build` 路径不经它）
    //
    // **确定性红线**：并行**只**并行"编译"，**输出仍在主线程按 `files` 顺序重放** ✓
    // ⇒ `--json` 与串行**逐字节相同**（含 `build.decl` 逐条事件 —— 事件内容与顺序
    //    都由被编译文件自己决定，与谁先跑完无关）✓
    //
    // **并发度**：`SOKONANODA_BUILD_JOBS` 可配；默认 = 可用核数；`1` ⇒ 走**串行原路**
    // （逐字节等价的最强保证，也方便 A/B）✓
    let jobs = build_jobs(files.len());
    // **② 文件级进度**：并行阶段每编完一个就报一条（additive，见 [`ProgressCounter`]）。
    let progress_counter = ProgressCounter::new(files.len(), json);
    let mut per_file: Vec<Option<Result<&'static str, String>>> =
        (0..files.len()).map(|_| None).collect();
    let mut per_file_ticks: Vec<Vec<(String, usize, usize)>> =
        (0..files.len()).map(|_| Vec::new()).collect();
    // ═══ **G-68（2026-10-03 ✓）**：先跑一次**项目会话**，把每个入口的报告算好 ═══
    // 病灶：以前**每个文件各跑一次** `compile_plan`（`build_one` 内部 ✓）⇒ 共享库层被逐文件重编 ✗
    //（台账 G-68：`by_calls=3` ✓）。修法：`units_for_modules` 分开"共享库层 / 各入口自己" ✓
    // ⇒ 共享层只编一次 ✓（`with_project_session` ✓），报告用 `assemble_report` ✓ 造好
    // ⇒ `build_one` 走它预留的 `precomputed` 分支 ✓（切片 1b ✓）。
    // ⚠ **reports 必须按"闭包顺序"对齐** ✓：`assemble_report` 用闭包下标取 `reports[slot]` ✗
    // ⇒ `on_entry` 给的两份（库层 `&[DocumentReport]` + 入口 `Vec<DocumentReport>` ✓）要**按模块名**
    // 铺回闭包顺序 ✓（上一版直接丢掉库层那份 ✗ ⇒ `reports[slot]` 越界 panic ✗，已实测 ✓）。
    // ⚠ 语义安全 ✓：`prelude` 只依赖文件自身 ✓ ⇒ 按模式**分组**跑会话 ✓。确定性红线 ✓：输出仍在
    // 主线程按 `files` 顺序重放（下面那段**一字未改** ✓）。
    let precomputed: Vec<Option<sokonanoda_front::project::ProjectReport>> = {
        use sokonanoda_front::project as proj;
        let mut plans: Vec<Option<proj::ProjectPlan>> = Vec::with_capacity(files.len());
        let mut srcs: Vec<String> = Vec::with_capacity(files.len());
        for file in files.iter() {
            let src = std::fs::read_to_string(file).unwrap_or_default();
            let root_override = if no_project {
                file.parent().map(Path::to_path_buf)
            } else {
                root.map(PathBuf::from)
            };
            let plan = if proj::is_project_source(&src) {
                Some(proj::plan_project(
                    file,
                    Some(&src),
                    root_override.as_deref(),
                ))
            } else {
                None
            };
            plans.push(plan);
            srcs.push(src);
        }
        let mut shared: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for plan in plans.iter().flatten() {
            for module in plan.modules() {
                *shared.entry(module.name.clone()).or_insert(0) += 1;
            }
        }
        let mut groups: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, _file) in files.iter().enumerate() {
            if plans[i].is_none() {
                continue;
            }
            let mode = format!(
                "{:?}",
                sokonanoda_front::compile::prelude_mode_from_source(&srcs[i])
            );
            groups.entry(mode).or_default().push(i);
        }
        let mut table: Vec<Option<proj::ProjectReport>> = (0..files.len()).map(|_| None).collect();
        for idxs in groups.into_values() {
            let first = idxs[0];
            let options = sokonanoda_front::compile::CompileOptions {
                prelude: sokonanoda_front::compile::prelude_mode_from_source(&srcs[first]),
            };
            let plan0 = plans[first].as_ref().expect("plan for grouped entry");
            let lib_units = proj::units_for_modules(plan0, |module| {
                shared.get(module.name.as_str()).copied().unwrap_or(0) >= 2
            });
            let lib_names: Vec<String> = lib_units.iter().map(|u| u.name.to_string()).collect();
            let entries: Vec<Vec<_>> = idxs
                .iter()
                .map(|&i| {
                    let plan = plans[i].as_ref().expect("plan for grouped entry");
                    proj::units_for_modules(plan, |module| {
                        shared.get(module.name.as_str()).copied().unwrap_or(0) < 2
                    })
                })
                .collect();
            let produced: Vec<(usize, proj::ProjectReport)> = proj::session::with_project_session(
                &lib_units,
                &entries,
                &options,
                |k, flat_out, entry_reports, lib_reports, _lib_ranges, _entry_range| {
                    let i = idxs[k];
                    let plan = plans[i].as_ref().expect("plan for grouped entry");
                    // **按模块名铺回闭包顺序** ✓（`assemble_report` 用闭包下标取 `reports[slot]` ✗）
                    let mut by_name: std::collections::HashMap<
                        &str,
                        &sokonanoda_front::compile::DocumentReport,
                    > = std::collections::HashMap::new();
                    for (n, report) in lib_names.iter().zip(lib_reports.iter()) {
                        by_name.insert(n.as_str(), report);
                    }
                    let entry_names: Vec<String> =
                        entries[k].iter().map(|u| u.name.to_string()).collect();
                    for (n, report) in entry_names.iter().zip(entry_reports.iter()) {
                        by_name.insert(n.as_str(), report);
                    }
                    let reports: Vec<sokonanoda_front::compile::DocumentReport> = plan
                        .modules()
                        .iter()
                        .filter_map(|m| by_name.get(m.name.as_str()).map(|r| (*r).clone()))
                        .collect();
                    let report = proj::assemble_report(proj::PlanCompiled {
                        compilable: plan.compilable(),
                        flat_out,
                        reports,
                        closure: plan.closure(),
                        diagnostics: plan.diagnostics().to_vec(),
                        entry_path: files[i].clone(),
                        root: plan.root.clone(),
                        manifest_path: plan.manifest.clone(),
                        requires_warning: plan.requires_warning.clone(),
                    });
                    (i, report)
                },
            );
            for (i, report) in produced {
                table[i] = Some(report);
            }
        }
        table
    };
    let precomputed = std::sync::Mutex::new(precomputed);
    let precomputed = &precomputed;

    if jobs > 1 {
        use std::sync::atomic::{AtomicUsize, Ordering};
        // **每个 worker 自己攒结果**（`Vec` 各归各的 ⇒ 不用锁 ✓），
        // `scope` 结束后由主线程按 `files` 顺序归位 —— 这样连"写结果"都不需要同步 ✓。
        type Slot = (
            usize,
            Result<&'static str, String>,
            Vec<(String, usize, usize)>,
        );
        let next = AtomicUsize::new(0);
        let next = &next;
        let files = &files;
        let counter = &progress_counter;
        let collected: Vec<Vec<Slot>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..jobs)
                .map(|_| {
                    scope.spawn(move || {
                        let mut mine: Vec<Slot> = Vec::new();
                        loop {
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            if index >= files.len() {
                                break;
                            }
                            let file = &files[index];
                            let mut ticks: Vec<(String, usize, usize)> = Vec::new();
                            let mut sink = |tick: sokonanoda_front::compile::ProgressTick<'_>| {
                                ticks.push((tick.module.to_string(), tick.index, tick.total));
                            };
                            let progress: Option<&mut dyn sokonanoda_front::compile::ProgressSink> =
                                if json { Some(&mut sink) } else { None };
                            let status = std::fs::read_to_string(file)
                                .map_err(|e| format!("cannot read: {e}"))
                                .and_then(|src| {
                                    let report = precomputed
                                        .lock()
                                        .expect("precomputed lock")
                                        .get_mut(index)
                                        .and_then(Option::take);
                                    build_one(file, &src, root, no_project, progress, report)
                                });
                            counter.note_file(file);
                            mine.push((index, status, ticks));
                        }
                        mine
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap_or_default())
                .collect()
        });
        for slot in collected.into_iter().flatten() {
            per_file[slot.0] = Some(slot.1);
            per_file_ticks[slot.0] = slot.2;
        }
    } else {
        for (index, file) in files.iter().enumerate() {
            let mut ticks: Vec<(String, usize, usize)> = Vec::new();
            let mut sink = |tick: sokonanoda_front::compile::ProgressTick<'_>| {
                ticks.push((tick.module.to_string(), tick.index, tick.total));
            };
            let progress: Option<&mut dyn sokonanoda_front::compile::ProgressSink> =
                if json { Some(&mut sink) } else { None };
            let status = std::fs::read_to_string(file)
                .map_err(|e| format!("cannot read: {e}"))
                .and_then(|src| {
                    let report = precomputed
                        .lock()
                        .expect("precomputed lock")
                        .get_mut(index)
                        .and_then(Option::take);
                    build_one(file, &src, root, no_project, progress, report)
                });
            progress_counter.note_file(file);
            per_file_ticks[index] = ticks;
            per_file[index] = Some(status);
        }
    }

    // **按 `files` 顺序重放**（输出确定性 = 与串行逐字节相同的前提）。
    for (index, file) in files.iter().enumerate() {
        let file_text = file.display().to_string();
        heartbeat.set_file(&file_text);
        heartbeat.note();
        if json {
            for (module, tick_index, total) in &per_file_ticks[index] {
                emit_json(serde_json::json!({
                    "type": "build.decl",
                    "file": file_text,
                    "module": module,
                    "index": tick_index,
                    "total": total,
                }));
                heartbeat.note();
            }
        }
        let status = match per_file[index].take() {
            Some(Ok(status)) => status,
            Some(Err(message)) => {
                eprintln!("error: {}: {message}", file.display());
                "failed"
            }
            None => "failed",
        };
        match status {
            "hit" => hit += 1,
            "compiled" => compiled += 1,
            _ => failed += 1,
        }
        if json {
            emit_json(serde_json::json!({
                "type": "build.file",
                "file": file_text,
                "status": status,
            }));
            heartbeat.note();
        }
    }
    heartbeat.stop();

    let total = hit + compiled + failed;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "type": "build.summary",
                "files": total,
                "hit": hit,
                "compiled": compiled,
                "failed": failed,
            })
        );
    } else {
        println!("built {total} file(s) — {hit} hit, {compiled} compiled, {failed} failed");
    }
    ExitCode::SUCCESS
}

/// **并行度**（入口级并行）：`SOKONANODA_BUILD_JOBS` 可配，默认 = 可用核数。
///
/// `1` ⇒ 走串行原路（A/B 与"逐字节等价"的最强保证）；上限 = 文件数（开更多线程没意义）。
/// 出处（业界先例）：官方建议**量单文件时逐文件单独跑**以免并行开销污染
/// ⇒ 所以本函数可退化成 1 ✓
/// <https://leanprover-community.github.io/archive/stream/270676-lean4/topic/profiling.20a.20project.html#500637969>
fn build_jobs(files: usize) -> usize {
    let requested = std::env::var("SOKONANODA_BUILD_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        });
    requested.clamp(1, files.max(1))
}

/// `--clean` 用：从位置参数解析出**模块根**（项目入口的 `plan.root`）并去重。
///
/// 无参数（`build --clean`）⇒ 返回空 ⇒ 只清全局缓存，并在输出里如实报告
/// `project=0`（设计 §3.7：解不出模块根时不清项目产物，但不假装清了）。
fn project_roots(args: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for arg in args {
        collect_files(Path::new(arg), &mut files);
    }
    let mut roots: Vec<PathBuf> = Vec::new();
    for file in files {
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        if !sokonanoda_front::project::is_project_source(&src) {
            continue;
        }
        let root = sokonanoda_front::project::plan_project(&file, Some(&src), None).root;
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    roots
}

/// Compile one source through the cache, returning `"hit"`, `"compiled"` or
/// `"failed"`. Options mirror the batch checker (file-directive prelude mode)
/// so a `build` warms exactly the entries `check`/`course` later load.
fn build_one(
    path: &Path,
    src: &str,
    root: Option<&str>,
    no_project: bool,
    progress: Option<&mut dyn sokonanoda_front::compile::ProgressSink>,
    // **切片 1b**：由 `with_project_session` 预先算好的结果（`None` = 老路径，自己编）。
    precomputed: Option<sokonanoda_front::project::ProjectReport>,
) -> Result<&'static str, String> {
    let options = CompileOptions {
        prelude: prelude_mode_from_source(src),
    };
    // 有 import ⇒ 项目闭包（v1 不进缓存：闭包键在 P4 落地；现在宁可重编译，
    // 也不拿单文件键去缓存一个依赖别人环境的报告）。
    //
    // 分发用 `is_project_source`：入口**单独 parse 失败**但写了 `import` 时也走
    // 闭包——入口可能用了依赖声明的记法（G-04 第二刀），闭包路径能编。
    if sokonanoda_front::project::is_project_source(src) {
        let root_override = if no_project {
            path.parent().map(Path::to_path_buf)
        } else {
            root.map(PathBuf::from)
        };
        let plan =
            sokonanoda_front::project::plan_project(path, Some(src), root_override.as_deref());
        let digest = plan.digest(&options);
        // R-3（T-B5）：项目条目落**模块根**的 `.sokonanoda/compiled/`
        // （`plan.root` 是发现规则算出来的模块根，这里现成）。
        let artifacts_root = plan.root.clone();
        if let Some(entry) =
            sokonanoda_front::project::cache::load_at(&artifacts_root, &digest, &options)
        {
            if entry.output.is_some() {
                return Ok("hit");
            }
        }
        let project = match precomputed {
            Some(project) => project,
            None => sokonanoda_front::project::compile_plan_with_progress(plan, &options, progress),
        };
        let ok = project
            .entry_module()
            .is_none_or(|module| module.events.errors.is_empty())
            && !project.has_errors();
        if ok && project.is_clean() {
            sokonanoda_front::project::cache::store_at(
                &artifacts_root,
                &digest,
                &options,
                &project,
            );
        }
        return Ok(if ok { "compiled" } else { "failed" });
    }
    let parsed = match sokonanoda_front::parse(src) {
        Ok(parsed) => parsed,
        Err(diag) => {
            return Err(format!(
                "{}:{}: error[{}]: {}",
                diag.span.start.line,
                diag.span.start.column,
                diag.stage_code(),
                diag.message
            ));
        }
    };
    if let Some(entry) = cache::load(src, &options) {
        if entry.output.is_some() {
            return Ok("hit");
        }
    }
    let (output, report) = compile_all_with(&parsed, &options);
    cache::store(
        src,
        &options,
        &CachedCompile {
            report,
            output: Some(output),
            // 单文件条目（无项目报告）。
            project: None,
        },
    );
    Ok("compiled")
}

/// Recursively collect `*.sokonanoda` files under `path` (sorted per level).
fn collect_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = match std::fs::read_dir(path) {
            Ok(read) => read.filter_map(|e| e.ok().map(|e| e.path())).collect(),
            Err(e) => {
                eprintln!("error: cannot read directory {}: {e}", path.display());
                return;
            }
        };
        entries.sort();
        for entry in entries {
            collect_files(&entry, out);
        }
    } else if path
        .extension()
        .map(|ext| ext == "sokonanoda")
        .unwrap_or(false)
    {
        out.push(path.to_path_buf());
    }
}

#[cfg(test)]
mod heartbeat_tests {
    //! **心跳判据的"真相层"版本**（2026-10-03 ✓，AGENTS.md 判据纪律② ✓）。
    //!
    //! 原来的判据在 `tests/cli.rs` 里量**两个进程的墙钟差**（`slack < 0.5` ✗，后抬到 0.8 ✗）——
    //! 那里面**混着进程启动/调度噪声** ✗（CI 实测 `slack = 0.55s` ✗，而真信号 ≈1.0s ✓
    //! ⇒ 信噪比不到 2× ✗ ⇒ 必然假红 ✓）。**结构性判据 = 直接量 `stop()` 自己的 `join` 时长** ✓
    //! —— 它**不含进程启动开销** ✓，且正是设计契约的原话（"`stop()` 的 `join()` 不许等满一个周期" ✓）。
    //!
    //! 本模块在 `build.rs` 内部 ⇒ 能看见**私有的** `Heartbeat` ✓（子模块可见父模块私有项 ✓）。
    use super::Heartbeat;
    use std::time::{Duration, Instant};

    #[test]
    fn stop_does_not_wait_a_full_period() {
        // 周期取 1s（= `DEFAULT_TICK_MS` ✓）：若 `stop()` 用 `sleep` 干等 ✗，
        // 它会等到**下一个整周期**（最坏 ≈1s ✗）；用 `Condvar` 唤醒 ✓ 则是**立即** ✓。
        let mut hb = Heartbeat::start(Some(1000));
        std::thread::sleep(Duration::from_millis(50)); // 让它真的跑起来 ✓
        let t = Instant::now();
        hb.stop();
        let waited = t.elapsed();
        assert!(
            waited < Duration::from_millis(300),
            "`stop()` 等了 {waited:?} —— 不许等满一个周期（用 `Condvar` 唤醒，别用 `sleep`）"
        );
    }

    #[test]
    fn stop_is_immediate_even_when_nothing_was_started() {
        // 没开心跳（`period_ms = None` ✓）⇒ `stop()` 必须**立刻**返回 ✓（幂等 ✓）。
        let mut hb = Heartbeat::start(None);
        let t = Instant::now();
        hb.stop();
        assert!(
            t.elapsed() < Duration::from_millis(100),
            "空心跳的 `stop()` 不该花时间"
        );
    }
}
