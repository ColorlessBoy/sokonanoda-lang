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

/// **P2 心跳周期**：机器消费者显式要求时，这么久没有任何其它输出 ⇒ 发一条 `build.tick`。
///
/// ⚠ 2026-09-30 从 **1000ms 放宽到 5000ms**，并且**默认根本不发**（见 [`tick_period_ms`]）。
/// 依据是**实测**，不是口味：冷编 `courses/set-theory` 的 `--json` 流里
/// **159 条 tick 的 `file` 全是空串**，而且**全部 159 条都排在第一条 `build.decl` 之前**
/// —— 文件级进度要等**全部**文件编译完才按 `files` 顺序重放（本文件下面那段），
/// 所以 `set_file` 在整个编译期间恒为空 ⇒ 这些 tick 既不是文件级进度、
/// 也没被任何仓库内消费者用来渲染百分比（`extension.js` 的 `percent` 始终是文件级）
/// ⇒ 对"用户可见面在不在动"贡献为零，纯粹是刷屏 ✗。
/// 5s 仍足以兜住消费者侧的"长时间静默"。
const DEFAULT_TICK_MS: u64 = 5000;

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
/// 开关：
/// * `SOKO_BUILD_TICK_MS=<毫秒>` ⇒ **发**，用这个周期（给终端里想看心跳的人/脚本）；
///   **不写**就是 [`DEFAULT_TICK_MS`]（5s）—— 显式要了心跳的人要的是"别太久没动静"，
///   不是"每秒一条" ✓；
/// * `SOKO_BUILD_TICK_MS=0` 或 `SOKO_BUILD_NO_TICK=1` ⇒ **不发**（显式关，也是默认）。
fn tick_period_ms() -> Option<u64> {
    if std::env::var_os("SOKO_BUILD_NO_TICK").is_some() {
        return None;
    }
    match std::env::var("SOKO_BUILD_TICK_MS") {
        Err(_) => None,
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
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Heartbeat {
    /// `period_ms = None` ⇒ **不发心跳**（默认路径：零线程、零输出 ✓）。
    fn start(period_ms: Option<u64>) -> Self {
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use std::sync::{Arc, Mutex};
        let t0 = std::time::Instant::now();
        let last_ms = Arc::new(AtomicU64::new(0));
        let current = Arc::new(Mutex::new(String::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let join = period_ms.map(|period| {
            let (last, cur, st) = (
                Arc::clone(&last_ms),
                Arc::clone(&current),
                Arc::clone(&stop),
            );
            std::thread::spawn(move || {
                let base = std::time::Instant::now();
                while !st.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(period));
                    if st.load(Ordering::Relaxed) {
                        break;
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
            stop,
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
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
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
                                    build_one(file, &src, root, no_project, progress, None)
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
                .and_then(|src| build_one(file, &src, root, no_project, progress, None));
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
