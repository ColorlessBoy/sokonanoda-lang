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

/// **G-68 预跑（2026-10-06）**：每个文件"该怎么编"在**进并行循环之前**就定下来。
///
/// **为什么要预跑**：项目入口的共享 `lib/*` 今天被**每个入口各编一遍** ✗
/// （台账 G-68：真课程 Σ闭包 **1614** 次模块编译 vs 去重 **249** = **6.5×** ✗）。
/// 会话（[`sokonanoda_front::project::compile_entries_shared`]）能把**同一份库闭包**的
/// 入口合到一次编译里 ✓ —— 但它要求"先知道每个入口的闭包"，而那要 `plan_project`
/// （解析闭包）⇒ 必须在并行循环之前跑一次 ✓。
///
/// ⚠ **`Legacy` 是绝大多数**（无 `import` 的单文件语料）—— 它们**不进**预跑，
/// 行为与今天**逐字节相同** ✓。
enum Prep {
    /// 非项目源 / 读不出来 ⇒ 走今天的 [`build_one`]（它自己读、自己 parse）。
    Legacy,
    /// 项目入口且**缓存命中** ⇒ 直接 `"hit"`（**不重编** —— `hit` 语义与今天一致 ✓）。
    Hit,
    /// 会话已经编好（≥2 个入口共享同一份库闭包）⇒ 存缓存 + 报状态 + 重放 tick。
    Shared {
        report: sokonanoda_front::project::ProjectReport,
        options: CompileOptions,
        digest: String,
        root: PathBuf,
        ticks: Vec<(String, usize, usize)>,
    },
    /// 计划已算好、`precheck_plan` 已跑过（单入口组 / 入口被阻断）⇒ 直接编
    /// （**不再重复检查** ✗ —— 诊断会加两遍 ⇒ 报告进缓存 ⇒ `query` 侧可见 ✗）。
    Planned {
        plan: sokonanoda_front::project::ProjectPlan,
        options: CompileOptions,
        digest: String,
        root: PathBuf,
    },
}

/// 把 `(模块名, 命令数)` 的 tick 计划展开成 `build.decl` 的 `(module, index, total)` 流。
///
/// **为什么是"重放"而不是"新造"**：逐入口路径里，每个入口都会为**整条闭包**发一遍
/// tick（`walk.rs` 的 `unit_seen`/`unit_totals` ✓，与信任前缀无关 ✓）；会话把库层
/// **只编一次** ⇒ 库层那一段 tick 只发一遍 ✗ ⇒ 接线方必须按各入口自己的闭包补回来 ✓。
fn expand_ticks(plan: &[(String, usize)]) -> Vec<(String, usize, usize)> {
    plan.iter()
        .flat_map(|(name, total)| (0..*total).map(move |index| (name.clone(), index, *total)))
        .collect()
}

/// **G-68 预跑**：为每个文件定下 [`Prep`]（含"分组会话"这一次共享编译）。
///
/// 顺序（每一步都不能换 ✗）：
/// ① 读源 + `is_project_source` 判定（非项目源 ⇒ `Legacy`，**零额外开销** ✓）；
/// ② `plan_project` + `digest` + **缓存命中判定**（命中 ⇒ `Hit`，**不参与会话** ✓
///    —— 否则"命中"会被改写成"重编" ✗）；
/// ③ [`sokonanoda_front::project::compile_entries_shared`]（按库闭包签名分组 + 组间并行）；
/// ④ 会话没接的 ⇒ `Planned`（计划**留用**，避免 Phase C 再 plan 一次 + 再 precheck 一次 ✓）。
fn prepare(
    files: &[PathBuf],
    root: Option<&str>,
    no_project: bool,
    jobs: usize,
    on_entry_done: &(dyn Fn(usize) + Sync),
) -> Vec<Prep> {
    let mut prep: Vec<Prep> = (0..files.len()).map(|_| Prep::Legacy).collect();
    struct Pending {
        index: usize,
        plan: sokonanoda_front::project::ProjectPlan,
        options: CompileOptions,
        digest: String,
        root: PathBuf,
    }
    let mut pending: Vec<Pending> = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let Ok(src) = std::fs::read_to_string(file) else {
            continue; // 读不出来 ⇒ Legacy（`build_one` 会给出与今天同一条错误 ✓）
        };
        if !sokonanoda_front::project::is_project_source(&src) {
            continue;
        }
        let options = CompileOptions {
            prelude: prelude_mode_from_source(&src),
        };
        let root_override = if no_project {
            file.parent().map(Path::to_path_buf)
        } else {
            root.map(PathBuf::from)
        };
        let plan =
            sokonanoda_front::project::plan_project(file, Some(&src), root_override.as_deref());
        let digest = plan.digest(&options);
        let artifacts_root = plan.root.clone();
        if let Some(entry) =
            sokonanoda_front::project::cache::load_at(&artifacts_root, &digest, &options)
        {
            if entry.output.is_some() {
                prep[index] = Prep::Hit;
                continue;
            }
        }
        pending.push(Pending {
            index,
            plan,
            options,
            digest,
            root: artifacts_root,
        });
    }
    if pending.is_empty() {
        return prep;
    }
    // ③ 分组会话（库层只编一次）。拆成平行向量是为了把 `plans` 交给 `&mut [ProjectPlan]` ✓。
    let mut plans: Vec<sokonanoda_front::project::ProjectPlan> = Vec::with_capacity(pending.len());
    let mut options_all: Vec<CompileOptions> = Vec::with_capacity(pending.len());
    let mut meta: Vec<(usize, String, PathBuf)> = Vec::with_capacity(pending.len());
    for p in pending {
        plans.push(p.plan);
        options_all.push(p.options);
        meta.push((p.index, p.digest, p.root));
    }
    let reports = sokonanoda_front::project::compile_entries_shared(
        &mut plans,
        &options_all,
        jobs,
        on_entry_done,
    );
    for (((plan, options), (index, digest, root)), report) in
        plans.into_iter().zip(options_all).zip(meta).zip(reports)
    {
        match report {
            Some(report) => {
                // tick 计划要在 `plan` 被丢掉**之前**取 ✓（它借用 plan ✓）。
                let ticks = expand_ticks(&sokonanoda_front::project::closure_tick_plan(&plan));
                prep[index] = Prep::Shared {
                    report,
                    options,
                    digest,
                    root,
                    ticks,
                };
            }
            None => {
                prep[index] = Prep::Planned {
                    plan,
                    options,
                    digest,
                    root,
                };
            }
        }
    }
    prep
}

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

/// **心跳要不要发、按什么周期**（`None` = 不发）。
///
/// 用户 2026-09-29 实测原话：终端每秒刷 `{"elapsed_ms":1001,"file":"","type":"build.tick"}`。
/// 根因不是周期，是**发错了地方**：心跳写的是 stdout，而**人看的终端与管道消费者
/// 共用同一个 stdout** ✗（工作单给的三个修法里，「周期 5s」只让它慢一点、
/// 「`file` 空不发」实测会把心跳**整个删掉**（159/159 空））。
///
/// ⇒ 口径（**C1，2026-10-08 起按通道分开**）：
///
/// * **机器通道**（`--json` 的 stdout 心跳，契约**一字不变**）：`stdout` **是管道**
///   （扩展 spawn 的子进程、`| jq`、CI 的重定向）⇒ **发**，周期 [`DEFAULT_TICK_MS`]；
///   **是终端** ⇒ **不发** ✓ —— 这正是"人看的终端不再刷 + 机器消费者仍能拿到心跳"
///   两句话的合取（2026-09-29 用户口径）；
/// * **人看的通道**（**stderr**，C1 新增）：`--json` **没开**时默认**发** ——
///   一行人类可读的 `… still building (12s) · <相对路径>`，周期同样是
///   [`DEFAULT_TICK_MS`]，且**真事件会顶掉它**（`note()` ⇒ 距上一条输出 ≥1s ✓）。
///   为什么不是"恢复 JSON 心跳"：2026-09-29 投诉的是**终端刷 JSON**，
///   不是"动起来" ✗；这条走 stderr、是人话、且只在非 `--json` 时出现 ✓
///   ⇒ 扩展（`build --json`）**拿不到它**，它的输出面板照旧干净 ✓。
///
/// 开关（**三档**）：
///
/// 1. **默认**：见上（两条通道各自的默认）；
/// 2. `SOKO_BUILD_TICK_MS=<毫秒>` ⇒ **强制发**，用这个周期（终端里想看心跳的人/脚本）；
///    **空串** = 要心跳但用默认周期；
/// 3. `SOKO_BUILD_TICK_MS=0` 或 `SOKO_BUILD_NO_TICK=1` ⇒ **强制不发**（逃生门，两条通道都关）。
///
/// ⚠ **第一版把默认写成"一律不发"是错的** ✗（CI 当场判红）：那样**机器消费者也拿不到
/// 心跳**，`scripts/check-progress-gap.py` 的「最长无输出间隔 ≤ 2.5s」立刻假红
/// （实测 `4.99s > 2.5s`）—— 而那条判据要的正是"**能力上限**：两条通道都在时能做到
/// 多好"。用户抱怨的是**终端**刷屏，不是管道里有心跳 ✓。
fn tick_period_ms(json: bool) -> Option<u64> {
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
        // 没显式设置 ⇒ 按**通道**给默认：
        // * 机器通道（`--json`）：**非终端**才发 JSON 心跳（终端刷 JSON 正是
        //   2026-09-29 的投诉 ✗ ⇒ 契约不变）；
        // * 人看的通道（非 `--json`）：**默认发**，走 stderr 的人话
        //   （C1；见本函数文档的"人看的通道"那一段）。
        Err(_) => {
            use std::io::IsTerminal;
            if json {
                (!std::io::stdout().is_terminal()).then_some(DEFAULT_TICK_MS)
            } else {
                Some(DEFAULT_TICK_MS)
            }
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
    /// **人看的那一半**（用户 2026-10-08：「build 没有反应」✗）：非 `--json` 时
    /// **每个文件一条**（带相对路径）往 **stderr** 打 —— 861 个文件的预热以前是
    /// **几分钟零输出**（事件只在 `--json` 下发）✗。
    ///
    /// ⚠ **C1（2026-10-08）把粒度从"每 10%"改成"每文件"**：原先是
    /// `step = total/10`、`done % step == 0` 才打 ⇒ 真课程 ~240 文件时**每 24 个
    /// 文件才一行**（用户报的"约 24 个文件才一条"就是这个数 ✗）。判据见
    /// `scripts/check-progress-cadence.py`（N=120 ⇒ 人看的行数 ≥ N）。
    /// stderr 是给人看的，机器契约（stdout 的 `build.progress`）一个字没动 ✓。
    human: bool,
}

impl ProgressCounter {
    fn new(total: usize, json: bool) -> Self {
        Self {
            done: std::sync::atomic::AtomicUsize::new(0),
            total,
            enabled: json,
            human: !json,
        }
    }

    /// 一个文件编完了 ⇒ 报一条。**只报"已完成几个"**（不报是谁：结果按顺序重放）。
    fn note_file(&self, file: &Path) {
        use std::sync::atomic::Ordering;
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        if self.enabled {
            emit_json(serde_json::json!({
                "type": "build.progress",
                "done": done,
                "total": self.total,
                "file": file.display().to_string(),
            }));
            return;
        }
        if self.human {
            eprintln!(
                "… {done}/{} · {}",
                self.total,
                shorten_path(&file.display().to_string())
            );
        }
    }
}

/// **C1**：人看的那条上，两条 tick 之间的**最小间隔**（毫秒）。
///
/// 为什么节流：模块级 tick 是**每个模块**一条（真课程一个入口的闭包可能几十个模块），
/// 861 个文件的预热会变成刷屏 ✗。150ms 的依据 = 比人的阅读速度快一档、又远密于
/// 心跳的 1s（判据只认**行数**与**最长空档**，不认这个阈值 —— 见
/// `PLAN-cli-editor-perf.md` §6.5）。
const HUMAN_TICK_MIN_MS: u128 = 150;

/// **C1**：模块级 tick 的**人看**那一半（非 `--json` 时接到 stderr）。
///
/// 为什么需要：模块级 tick 以前**只喂 `--json`**（三处 `if json { Some(&mut sink) }
/// else { None }`）⇒ 一个入口要编 40 个闭包模块时，终端**整个入口期间零输出** ✗
/// （真课程冷编的首条 `build.decl` 在 160.8s 之后）。
struct HumanTicks {
    last: std::time::Instant,
}

impl HumanTicks {
    fn new() -> Self {
        Self {
            // 起手**允许第一条**立刻打（否则"第一个模块编完"要等到 150ms 后才有声音）。
            last: std::time::Instant::now()
                .checked_sub(std::time::Duration::from_millis(
                    HUMAN_TICK_MIN_MS as u64 + 1,
                ))
                .unwrap_or_else(std::time::Instant::now),
        }
    }
}

impl sokonanoda_front::compile::ProgressSink for HumanTicks {
    fn tick(&mut self, tick: sokonanoda_front::compile::ProgressTick<'_>) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last).as_millis() < HUMAN_TICK_MIN_MS {
            return;
        }
        self.last = now;
        eprintln!("… module {}/{} · {}", tick.index + 1, tick.total, tick.module);
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
    ///
    /// `human = true`（非 `--json`）⇒ 心跳走 **stderr 的人话**；`false` ⇒ 照旧发
    /// `build.tick` JSON（机器契约一字不变 ✓）。见 [`tick_period_ms`]。
    fn start(period_ms: Option<u64>, human: bool) -> Self {
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
                        if human {
                            // **C1**：人看的通道 ⇒ **人话**走 stderr（不假装百分比 ✓、
                            // 不往 stdout 塞 JSON ✓）。`file` 为空时只说用时。
                            let where_ = if file.is_empty() {
                                String::new()
                            } else {
                                format!(" · {}", shorten_path(&file))
                            };
                            eprintln!("… still building ({}s){where_}", now / 1000);
                        } else {
                            emit_json(serde_json::json!({
                                "type": "build.tick",
                                "elapsed_ms": now,
                                "file": file,
                            }));
                        }
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
        return clean_stores(args, json);
    }
    warm(args, json, root, no_project)
}

/// `sokonanoda clean …` ≡ `build --clean …`（用户 2026-10-08：「clean 没有实现」）。
///
/// 与扩展的 `Sokonanoda: Clean Cache`（E31）**同一条语义**：**只清不编** ✓。
pub(crate) fn clean(args: &[String], json: bool) -> ExitCode {
    clean_stores(args, json)
}

/// `sokonanoda rebuild …` = **先清（两处）再预热**（用户 2026-10-08：「rebuild 没有实现」）。
///
/// 与扩展的 `Sokonanoda: Rebuild (清空编译缓存后重编译)`（`alt+shift+b`）**同一条语义** ✓；
/// 扩展此前只能**两次调用**（`build --clean` + `build`）把它拼出来 ⇒ 现在 CLI 一条命令、
/// **一个进程** ✓（`--json` 的事件流仍是 `build.clean` 之后接 `build.begin`/`file`/`summary` ✓
/// —— additive，老消费者照旧 ✓）。
pub(crate) fn rebuild(
    args: &[String],
    json: bool,
    root: Option<&str>,
    no_project: bool,
) -> ExitCode {
    let cleaned = clean_stores(args, json);
    if cleaned != ExitCode::SUCCESS {
        return cleaned;
    }
    warm(args, json, root, no_project)
}

/// 清**两处**存储（全局 + 位置参数能解出的模块根）。
fn clean_stores(args: &[String], json: bool) -> ExitCode {
    // **不给路径时按"当前目录"解模块根**（**C2**，2026-10-08）—— 与 `warm` **同一个默认** ✓。
    //
    // 以前这里解不出任何模块根（`project_roots(&[])` ⇒ 空集）⇒ 项目产物
    // （`<模块根>/.sokonanoda/compiled/`）**一个都没清** ⇒ 紧接着的预热**全是 `hit`**
    // （实测：`build .` 后 `rebuild` 报 `1 hit, 1 compiled` —— "rebuild 什么都没重编"
    // 这个**假动作** ✗）。设计 §3.7 说"解不出就不清、也不假装"——**有参数**时那条仍然成立；
    // 无参数时**能**解出来（`warm` 的默认就是 `.`），只是以前没去解 ✗。
    // `clean` 的"只清不编"语义**一字不变** ✓（这里只清，预热在 `warm` 里）。
    let default_scan = args.is_empty();
    if default_scan {
        eprintln!(
            "note: 没有给路径 ⇒ 按**当前目录**解模块根（与 `build` 同一个默认 ✓；\
             只清全局的话给一个没有项目的路径即可）"
        );
    }
    // R-3（T-B5）：项目条目现在落在**模块根**的 `.sokonanoda/compiled/`，所以
    // `--clean` 必须**两处都清** —— 只清全局的话 `rebuild`（= `build --clean`）
    // 会命中项目条目 ⇒ 表面"清空了"，实际什么都没重编（用户可见的假动作 ✗）。
    let global = cache::clean();
    let mut project = 0usize;
    let roots = project_roots(args);
    let roots_found = roots.len();
    for root in roots {
        project += sokonanoda_front::project::cache::clean_at(&root);
    }
    if default_scan {
        eprintln!(
            "… 当前目录下扫到 {roots_found} 个模块根，项目产物清了 {project} 条（只清不编 ✓）"
        );
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
    ExitCode::SUCCESS
}

/// 预热：位置参数（**默认当前目录**）→ 递归收集 `*.sokonanoda` → 逐文件编译 → 写缓存。
fn warm(args: &[String], json: bool, root: Option<&str>, no_project: bool) -> ExitCode {
    let roots: Vec<PathBuf> = if args.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    // **默认扫描要说话**（用户 2026-10-08：「build 没有反应」✗）：不给路径时根是**当前
    // 目录**，扫描期间除了 1s 一次的心跳**没有任何输出** ⇒ 大目录（语言仓根 269k 文件）
    // 看起来就是"卡住了" ✓。⇒ 起止各一行 **stderr**（人看的；`--json` 的机器契约在
    // stdout 上、由 `build.begin` 报总数 ✓，一个字都没动 ✓）。
    let default_scan = args.is_empty();
    if default_scan {
        eprintln!(
            "scanning the current directory for *.sokonanoda …（跳过 {}）",
            SKIPPED_DIRS.join(" / ")
        );
    }
    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }
    files.sort();
    files.dedup();
    if default_scan {
        eprintln!("found {} .sokonanoda file(s)", files.len());
    }
    if files.is_empty() {
        // 2026-10-08：这里以前只打一行 `usage:` ✗ —— 用户给了**存在的**目录却"什么都没发生"
        // 时，看到 usage 会以为自己写错了命令 ✓⇒ 现在**先说清"没找到文件"**、再给用法 ✓。
        let where_ = if default_scan {
            "当前目录".to_string()
        } else {
            args.join("、")
        };
        eprintln!("error: {where_} 下没有 .sokonanoda 文件");
        eprintln!(
            "用法：sokonanoda build [--json] [--clean] [<file.sokonanoda> | <dir> ...]（不给路径 = 当前目录；跳过 {} 等目录）",
            SKIPPED_DIRS.join(" / ")
        );
        return ExitCode::FAILURE;
    }

    let mut hit = 0usize;
    let mut compiled = 0usize;
    let mut failed = 0usize;
    // 失败明细（文件 → 原因）：汇总行只报数字不够用（2026-10-08 用户报障 ✓）。
    let mut failures: Vec<(String, String)> = Vec::new();
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
    let mut heartbeat = Heartbeat::start(tick_period_ms(json), !json);

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
    // **② 文件级进度**：每编完一个就报一条（additive，见 [`ProgressCounter`]）。
    // ⚠ **必须建在预跑之前**（G-68）：会话那条路要在**会话内部**逐入口报 ✓
    // （否则进度条在整段会话期间**不动** ✗ —— 真课程那段是几十秒）。
    let progress_counter = ProgressCounter::new(files.len(), json);
    let mut per_file: Vec<Option<Result<&'static str, String>>> =
        (0..files.len()).map(|_| None).collect();
    let mut per_file_ticks: Vec<Vec<(String, usize, usize)>> =
        (0..files.len()).map(|_| Vec::new()).collect();

    // ═══ **G-68 预跑**（2026-10-06）：项目入口的共享库层**只编一次** ═══
    //
    // 预跑把每个文件分成四类（见 [`Prep`]）：`Hit`（缓存命中）/ `Shared`（会话已编好）/
    // `Planned`（计划已算好，自己编）/ `Legacy`（非项目源，走今天那条路 ✓）。
    // **只有 `Legacy` 进并行循环** —— 它正是"每文件独立"的那一半（单文件语料 ✓），
    // 与今天**逐字节相同** ✓；前三类都在主线程里处理（它们要么已经编完、要么只剩
    // 存缓存 + 组装，**不占 CPU** ✓）。
    //
    // ⚠ 预跑**先于**心跳/并行阶段：它自己就是"编译"那一段（组间并行在
    // `compile_entries_shared` 里 ✓）。
    let mut prep = prepare(&files, root, no_project, jobs, &|index| {
        progress_counter.note_file(&files[index])
    });
    let mut legacy: Vec<usize> = Vec::new();
    for (index, slot) in prep.iter_mut().enumerate() {
        let file = files[index].clone();
        match std::mem::replace(slot, Prep::Legacy) {
            Prep::Legacy => legacy.push(index),
            Prep::Hit => {
                per_file[index] = Some(Ok("hit"));
                progress_counter.note_file(&file);
            }
            Prep::Shared {
                report,
                options,
                digest,
                root: artifacts_root,
                ticks,
            } => {
                let outcome = finish_project(report, &options, &digest, &artifacts_root);
                per_file_ticks[index] = ticks;
                per_file[index] = Some(outcome);
                // ⚠ **不在这里 note_file**：会话已经逐入口报过 ✓（重复报 = 计数翻倍 ✗）。
            }
            Prep::Planned {
                plan,
                options,
                digest,
                root: artifacts_root,
            } => {
                let mut ticks: Vec<(String, usize, usize)> = Vec::new();
                let mut sink = |tick: sokonanoda_front::compile::ProgressTick<'_>| {
                    ticks.push((tick.module.to_string(), tick.index, tick.total));
                };
                // **C1**：模块级 tick **也喂人看的那条**（以前非 json 时是 `None`
                // ⇒ 一个入口编 40 个闭包模块期间终端零输出 ✗）。
                let mut human = HumanTicks::new();
                let progress: Option<&mut dyn sokonanoda_front::compile::ProgressSink> =
                    if json { Some(&mut sink) } else { Some(&mut human) };
                let report =
                    sokonanoda_front::project::compile_plan_prechecked(plan, &options, progress);
                let outcome = finish_project(report, &options, &digest, &artifacts_root);
                per_file_ticks[index] = ticks;
                per_file[index] = Some(outcome);
                progress_counter.note_file(&file);
            }
        }
    }

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
        let legacy = &legacy;
        let counter = &progress_counter;
        let collected: Vec<Vec<Slot>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..jobs.min(legacy.len().max(1)))
                .map(|_| {
                    // ⚠ **必须给大栈**（2026-10-06 · G-68 实测定位 ✓）：编译是**深度递归**的
                    // `elab_expr`，`scope.spawn` 的默认栈是 **2MB** ⇒ 真语料上
                    // `thread '<unknown>' has overflowed its stack` + `Abort trap: 6` ✗
                    // （实测：冷编 `units/I.3` · `SOKONANODA_BUILD_JOBS=4` ⇒ exit **134** ✗；
                    //  同一条命令 `JOBS=1`（主线程 8MB）**编得过** ✓ ⇒ 病根是**线程栈** ✓）。
                    // 32MB = LSP 侧同一条结论的同款药（`crates/lsp/src/lib.rs::run` ✓）。
                    std::thread::Builder::new()
                        .name("soko-build".to_string())
                        .stack_size(sokonanoda_front::project::COMPILE_STACK_BYTES)
                        .spawn_scoped(scope, move || {
                            let mut mine: Vec<Slot> = Vec::new();
                            loop {
                                let slot = next.fetch_add(1, Ordering::Relaxed);
                                if slot >= legacy.len() {
                                    break;
                                }
                                let index = legacy[slot];
                                let file = &files[index];
                                let mut ticks: Vec<(String, usize, usize)> = Vec::new();
                                let mut sink = |tick: sokonanoda_front::compile::ProgressTick<
                                    '_,
                                >| {
                                    ticks.push((tick.module.to_string(), tick.index, tick.total));
                                };
                                let mut human = HumanTicks::new();
                                let progress: Option<
                                    &mut dyn sokonanoda_front::compile::ProgressSink,
                                > = if json { Some(&mut sink) } else { Some(&mut human) };
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
                        .expect("spawn build thread")
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
        for &index in &legacy {
            let file = &files[index];
            let mut ticks: Vec<(String, usize, usize)> = Vec::new();
            let mut sink = |tick: sokonanoda_front::compile::ProgressTick<'_>| {
                ticks.push((tick.module.to_string(), tick.index, tick.total));
            };
            let mut human = HumanTicks::new();
            let progress: Option<&mut dyn sokonanoda_front::compile::ProgressSink> =
                if json { Some(&mut sink) } else { Some(&mut human) };
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
        // **失败原因要跟着走**（2026-10-08 用户：「`sokonanoda:rebuild` 会报失败 2，但是我又
        // 不知道哪里失败的」✗）：以前 `Some(Err(message))` 只把消息打到 stderr，而
        // `build.file` 事件里**只有** `status: "failed"` ✗ ⇒ 消费端（扩展通知）结构上就拿不到
        // 原因 ⇒ 用户只看到数字 ✗。
        let (status, failure) = match per_file[index].take() {
            Some(Ok(status)) => (status, None),
            Some(Err(message)) => {
                eprintln!("error: {}: {message}", file.display());
                ("failed", Some(message))
            }
            None => ("failed", None),
        };
        match status {
            "hit" => hit += 1,
            "compiled" => compiled += 1,
            _ => failed += 1,
        }
        if let Some(message) = &failure {
            failures.push((file_text.clone(), message.clone()));
        }
        if json {
            let mut event = serde_json::json!({
                "type": "build.file",
                "file": file_text,
                "status": status,
            });
            // additive：老消费者忽略未知字段 ✓（`docs/protocol.md` 已同步 ✓）。
            if let Some(message) = &failure {
                event["error"] = serde_json::Value::String(message.clone());
            }
            emit_json(event);
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
    // **失败明细**（用户 2026-10-08：「失败 2，但是我又不知道哪里失败的」✗）：原因排在汇总行
    // **之后**、成块给出 —— 不用在进度行里翻 ✗。stderr（人看），stdout 的机器契约不动 ✓。
    if !failures.is_empty() {
        eprintln!("失败明细（{} 个文件）：", failures.len());
        for (file, reason) in &failures {
            eprintln!("  ✗ {}: {reason}", shorten_path(file));
        }
        // 一句话出路 + 一句实话（**期望判红**的探针也会出现在这里 ⇒ 别把它当故障 ✗）。
        eprintln!(
            "提示：只想编某个子树就把目录当参数传（例如 `sokonanoda build units lib`）；\
             仓库/课程里**故意判红**的探针（常见于 `gaps/`，文件名常带 `reject`）本来就会出现在这里 ✓。"
        );
    }
    ExitCode::SUCCESS
}

/// 明细里用**相对当前目录**的路径（绝对路径太长 ⇒ 一行读不完 ✗）；不在当前目录下就原样 ✓。
fn shorten_path(file: &str) -> String {
    let path = Path::new(file);
    match std::env::current_dir().ok().and_then(|cwd| {
        path.strip_prefix(&cwd)
            .ok()
            .map(|rest| rest.display().to_string())
    }) {
        Some(short) => short,
        None => file.to_string(),
    }
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
/// **无参数 = 当前目录**（**C2**，2026-10-08）：与 [`warm`] **同一个默认** ✓ ——
/// 以前返回空集 ⇒ `clean`/`rebuild` 无参时项目产物一个都不清 ⇒ 紧接着的预热
/// 全是 `hit`（用户报的"rebuild 是假重编"✗）。有参数时"解不出就不清、也不假装"
/// （设计 §3.7）**一字不变** ✓；无参数时**能**解出来（`warm` 一直就是这么做的）。
/// 扫描跳过 `.git`/`node_modules`/`target`/`.sokonanoda` 等（与 `warm` 同一份
/// `SKIPPED_DIRS` ✓）。
fn project_roots(args: &[String]) -> Vec<PathBuf> {
    let default = [String::from(".")];
    let args: &[String] = if args.is_empty() { &default } else { args };
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

/// **G-68 预跑的收尾**：判定状态 + 干净即存缓存 —— 与 [`build_one`] 的项目分支**逐字同构** ✓
/// （`ok` 的口径、`store_at` 的条件都不许分叉 ✗：分叉 = "会话编的"与"自己编的"进不同的缓存 ✓）。
fn finish_project(
    report: sokonanoda_front::project::ProjectReport,
    options: &CompileOptions,
    digest: &str,
    artifacts_root: &Path,
) -> Result<&'static str, String> {
    let ok = report
        .entry_module()
        .is_none_or(|module| module.events.errors.is_empty())
        && !report.has_errors();
    if ok && report.is_clean() {
        sokonanoda_front::project::cache::store_at(artifacts_root, digest, options, &report);
    }
    if ok {
        Ok("compiled")
    } else {
        // **失败要带原因**（2026-10-08 用户：「`sokonanoda:rebuild` 会报失败 2，但是我又不知道
        // 哪里失败的」✗）：这里以前只返回 `"failed"`，report 里的诊断**原地丢掉** ✗ ⇒
        // 消费端（`build.file` 事件、扩展通知）只能报数字，用户无从下手 ✗。
        Err(failure_reason(&report))
    }
}

/// 一句话失败原因：**入口模块的第一条错误**（入口没错误就按拓扑序找第一条）⇒
/// `模块名: 行:列: code: 消息` ✓。
///
/// 坐标用**内核给的行列**（1-based，与 `query check` 的 `failed[].start_line/start_col`
/// 同一批数字 ✓）—— **不拿 offset 去切字符** ✗（中文/`α` 是多字节，G-15 的假象就是这么来的 ✓）。
fn failure_reason(report: &sokonanoda_front::project::ProjectReport) -> String {
    // `report.modules` 是**拓扑序、入口最后**（与 `query project` 同一条口径 ✓）⇒ 倒着走
    // = 先看入口（用户最关心自己的文件 ✓），再往依赖里找。
    for module in report.modules.iter().rev() {
        if let Some(error) = module
            .events
            .errors
            .first()
            .or_else(|| module.report.errors.first())
        {
            return format!(
                "{}: {}:{}: {}: {}",
                module.name,
                error.span.start.line,
                error.span.start.column,
                error.kind.code(),
                error.message
            );
        }
    }
    match report.diagnostics.iter().find(|diag| diag.kind.is_error()) {
        Some(diag) => format!("{}: {}: {}", diag.module, diag.code(), diag.message),
        // 兜底：状态说"失败"却一条诊断都没有 —— 说不出原因就**如实说**（不许编 ✗）。
        None => "编译失败，但报告里没有诊断（请把这一例开 issue）".to_string(),
    }
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
/// `build` 递归时**跳过的目录名**（2026-10-08 用户报「build 没有反应」✗）。
///
/// **为什么**：不给路径时根是**当前目录**，而语言仓根下有 **269,027 个文件 / 9,710 个目录**
/// （`target/` 24 万 + `editor/vscode/node_modules/` 1.5 万）⇒ 全递归要几分钟、期间只有
/// 1s 一次的心跳 ⇒ 用户看到的就是"没反应" ✓。跳过的全是**构建产物 / 依赖 / VCS 元数据**
/// ——里面不会有课程源 ✓（产物用别的扩展名，见 `docs/design/project-artifacts.md` §3.3 红线 ✓）。
const SKIPPED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    ".sokonanoda",
    ".vscode-test",
    "__pycache__",
    ".venv",
];

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
            // ⚠ **只跳"孩子"、不跳"根"** ✓：显式写 `sokonanoda build target/` 时照走
            // （用户点名了就算数 ✓）；只有**递归下去**遇到这些名字才跳过 ✓。
            if entry.is_dir()
                && entry
                    .file_name()
                    .is_some_and(|name| SKIPPED_DIRS.contains(&name.to_string_lossy().as_ref()))
            {
                continue;
            }
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
        let mut hb = Heartbeat::start(Some(1000), false);
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
        let mut hb = Heartbeat::start(None, false);
        let t = Instant::now();
        hb.stop();
        assert!(
            t.elapsed() < Duration::from_millis(100),
            "空心跳的 `stop()` 不该花时间"
        );
    }
}
