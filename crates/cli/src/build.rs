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

/// **P2 心跳周期**：这么久没有任何其它输出 ⇒ 发一条 `build.tick`。
const TICK_MS: u64 = 1000;

/// 所有 `--json` 输出走同一把锁：心跳线程与编译线程都会写 stdout，
/// 不加锁会**串行交错**（两条 JSON 拼在一行 ⇒ 消费者解析失败）✗。
static PRINT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn emit_json(value: serde_json::Value) {
    let _guard = PRINT_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    println!("{value}");
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
    fn start(enabled: bool) -> Self {
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use std::sync::{Arc, Mutex};
        let t0 = std::time::Instant::now();
        let last_ms = Arc::new(AtomicU64::new(0));
        let current = Arc::new(Mutex::new(String::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let join = enabled.then(|| {
            let (last, cur, st) = (
                Arc::clone(&last_ms),
                Arc::clone(&current),
                Arc::clone(&stop),
            );
            std::thread::spawn(move || {
                let base = std::time::Instant::now();
                while !st.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(TICK_MS));
                    if st.load(Ordering::Relaxed) {
                        break;
                    }
                    let now = base.elapsed().as_millis() as u64;
                    if now.saturating_sub(last.load(Ordering::Relaxed)) >= TICK_MS {
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
    // **P2 心跳**：`--json` 时启动（人类可读模式零线程、输出逐字节不变）。
    let mut heartbeat = Heartbeat::start(json);

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
