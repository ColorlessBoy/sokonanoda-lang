//! **按键墙钟探针（真 stdio LSP 子进程 · 真课程）** —— 北极星读数的那把尺。
//!
//! 规划 `docs/notes/PLAN-align-lean4.md` 的四个方向每个都要"**量一次按键墙钟**"，
//! 而在此之前仓库里只有两条路：
//!
//! * `crates/lsp/src/tests/perf_course.rs::perf_course_keystroke_is_recorded`
//!   —— **同进程** `LspService`（不经 stdio、不经真客户端的那条路）；
//! * `crates/front/tests/keystroke_structure.rs` —— 合成夹具的**结构计数**（无墙钟）。
//!
//! 本条补的是缺的那一格：**真的 `sokonanoda-lsp` 进程 + 真 stdio 客户端 + 真课程**
//! ⇒ 量的就是用户在编辑器里按键到 `publishDiagnostics` 落地的那段时间 ✓
//! （`Client::did_change` 返回 = 诊断已到 = 用户看到 solved 的那一刻 ✓）。
//!
//! ## 口径（`AGENTS.md` 判据纪律）
//!
//! **绝对毫秒不进判据** ⇒ 这里只做**量级哨兵**（30s），真正的比较留给
//! **同机、同二进制、同序列**的前后两次读数（本文件打印 PERFJSON + 构建身份 ✓）。
//! 每次读数**自带构建身份**（`sokonanoda-lsp` 二进制的 mtime）⇒ 跨轮比较前先看它 ✓。
//!
//! ## 两个场景（都是"做题"里真实发生的按键）
//!
//! | 场景 | 编辑 | 对应 §8.1 的那一列 |
//! |---|---|---|
//! | `proof` | 证明体里加一个空格（陈述不变） | 改证明 |
//! | `statement` | 把声明名 `demo_mem_image` 改名 | 改陈述（改名） |
//!
//! 两个场景各自先跑一刀**热身**（不计入），再量 5 刀取 **best/median/worst** ——
//! 单次采样会被邻居与本进程的调度放大（同 `perf.rs` 的口径 ✓）。
//!
//! ## 依赖
//!
//! * 课程仓与语言仓可以分开检出 ⇒ 找不到 `courses/set-theory/sokonanoda.toml` 就**跳过**
//!   （与 `lsp_keystroke_structure.rs` 同口径 ✓）；
//! * 产物：`courses/set-theory/.sokonanoda/` 有产物时开档走**产物命中**那条路
//!   （首次按键最贵，见 §8.2）—— 本文件把 `SOKONANODA_NO_PROJECT_ARTIFACTS`
//!   的状态**打进读数**，因为两条路的热按键读数不可比 ✗。

mod common;

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use common::Client;

/// 真课程根；找不到就跳过（课程仓可分开检出）。
fn course_root() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("courses")
        .join("set-theory");
    dir.join("sokonanoda.toml").is_file().then_some(dir)
}

/// **构建身份**：`sokonanoda-lsp` 二进制的 mtime（探针纪律：跨轮比较前先看它）。
///
/// ⚠ **认的是真正被 spawn 的那个二进制**（`SOKO_TEST_LSP_BIN` 可以把它换到
/// release 构建 ✓）—— 认错了就等于拿两个不同构建的读数并排比 ✗。
fn binary_identity() -> String {
    let overridden = std::env::var("SOKO_TEST_LSP_BIN").ok();
    let path = overridden
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new(env!("CARGO_BIN_EXE_sokonanoda-lsp")));
    let flavor = if overridden.is_some() {
        "override"
    } else {
        "cargo"
    };
    match std::fs::metadata(path).and_then(|meta| meta.modified()) {
        Ok(time) => {
            let secs = time
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("lsp-{flavor}-mtime={secs}")
        }
        Err(error) => format!("lsp-{flavor}-mtime=unknown({error})"),
    }
}

/// 一臂读数（全部是计数或毫秒；判据只用量级）。
#[derive(Debug, Clone)]
struct Arm {
    scenario: &'static str,
    samples: Vec<f64>,
    /// 服务端**自报的编译耗时**（`LSP_TRACE compile … <N>ms`）—— 与墙钟的差
    /// 就是"调度/防抖/管道"那一半（这是判"快在哪"的关键分解）。
    compile_ms: u64,
    modules: u64,
    prefix: u64,
    by: u64,
    /// **这一刀里 `TcCache` 构造了几次**（`LSP_TRACE … tc=`）—— §18/§19 要的"按一刀归属"✓。
    tc: u64,
    telescope: u64,
}

impl Arm {
    fn best(&self) -> f64 {
        self.samples.iter().copied().fold(f64::MAX, f64::min)
    }
    fn worst(&self) -> f64 {
        self.samples.iter().copied().fold(0.0f64, f64::max)
    }
    fn median(&self) -> f64 {
        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted[sorted.len() / 2]
    }
}

/// 跑一个场景：`warmup` 一刀 + `rounds` 刀，返回毫秒样本与**最后一刀**的结构计数。
fn run_scenario(
    client: &mut Client,
    uri: &str,
    scenario: &'static str,
    base: &str,
    flip: &dyn Fn(&str, bool) -> String,
    rounds: usize,
) -> Arm {
    let mut current = base.to_string();
    let mut samples = Vec::with_capacity(rounds);
    let mut version = 10i64;
    // 热身一刀（建立信任前缀/检查点，不计入 —— 同 `perf_course` 的"来回改 3 次取最小"口径）。
    for round in 0..=rounds {
        let next = flip(&current, round % 2 == 0);
        assert_ne!(next, current, "夹具前提：{scenario} 这一刀必须真的改变文本");
        version += 1;
        let started = Instant::now();
        let _ = client.did_change(uri, version, &next);
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        if round > 0 {
            samples.push(elapsed);
        }
        current = next;
    }
    // 结构计数取自**最后一刀**（stderr 读线程刚收进来的那一行）。
    let line = client.last_trace();
    Arm {
        scenario,
        samples,
        compile_ms: trace_compile_ms(&line),
        modules: Client::trace_field(&line, "modules"),
        prefix: Client::trace_field(&line, "prefix"),
        by: Client::trace_field(&line, "by"),
        // **这一刀里 `TcCache` 构造了几次**（§18/§19 的"按一刀归属"就在这里 ✓ ——
        // 不必再抓子进程的 `STAGE_STATS`：那是**退出时**才打的 ✗，而 `tc=` 是**每刀**都有 ✓）。
        tc: Client::trace_field(&line, "tc"),
        telescope: Client::trace_field(&line, "telescope"),
    }
}

/// 从 trace 行里取服务端自报的编译毫秒（`… v2 <N>ms publish=…`）。
fn trace_compile_ms(line: &str) -> u64 {
    line.split_whitespace()
        .find_map(|token| token.strip_suffix("ms")?.parse::<u64>().ok())
        .unwrap_or(0)
}

/// **连续键入**的一刀：在 `Set.mem_image α β f A<k 个空格>y` 里**再加一个空格**。
///
/// 为什么用它：每一刀都产生**从未编译过的新文本**（与"来回改同一份"不同 ✓），
/// 而编辑点固定在**同一条声明**里 ⇒ 量到的就是"用户一路敲下去"的成本 ✓。
fn add_one_space_before_y(current: &str) -> String {
    let marker = "Set.mem_image α β f A";
    let at = current.find(marker).expect("夹具前提：锚点必须在");
    let rest = &current[at + marker.len()..];
    let spaces = rest.chars().take_while(|c| *c == ' ').count();
    let mut next = String::with_capacity(current.len() + 1);
    next.push_str(&current[..at + marker.len()]);
    next.push_str(&" ".repeat(spaces + 1));
    next.push_str(&rest[spaces..]);
    next
}

#[test]
fn perf_course_keystroke_wallclock_is_recorded() {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml（课程仓可分开检出）");
        return;
    };
    let rel = "units/I.3/unit08-images-preimages.sokonanoda";
    let path = root.join(rel);
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {}", path.display());
        return;
    };
    // **夹具前提**：两个场景各自的编辑锚点必须在文件里，否则 `replace` 是空操作 ⇒
    // 量的是"同文本通知"（T-A21 短路 ⇒ ~0ms）✗（`perf_course.rs` 踩过）。
    assert!(
        text.contains("demo_mem_image") && text.contains("Set.mem_image α β f A y"),
        "夹具前提：unit08 的两个编辑锚点必须在文件里"
    );

    let artifacts_off = std::env::var_os("SOKONANODA_NO_PROJECT_ARTIFACTS").is_some();
    let cache = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-keystroke-wallclock-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&cache);
    // 产物开关决定开档走哪条路 —— 必须在**子进程环境**里钉死，读数才可归因。
    let mut client = Client::start_traced_with_env(
        &cache,
        if artifacts_off {
            &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")]
        } else {
            &[]
        },
    );
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text); // 开档（冷/产物命中都行，不判它）
    let _ = client.settled_compile_count();

    // 场景 1：**改证明体**（陈述不变）—— 做题时每一次键入就是它。
    let proof = run_scenario(
        &mut client,
        &uri,
        "proof",
        &text,
        &|current, even| {
            if even {
                current.replacen("Set.mem_image α β f A y", "Set.mem_image α β f A  y", 1)
            } else {
                current.replacen("Set.mem_image α β f A  y", "Set.mem_image α β f A y", 1)
            }
        },
        5,
    );
    // 场景 2：**改陈述**（改名）—— 改接口那一刀（本轮规划 §8.1 的"改陈述"列）。
    let statement = run_scenario(
        &mut client,
        &uri,
        "statement",
        &text,
        &|current, even| {
            if even {
                current.replace("demo_mem_image", "demo_mem_image_x")
            } else {
                current.replace("demo_mem_image_x", "demo_mem_image")
            }
        },
        5,
    );

    // 场景 3：**连续键入**（每一刀都是**新文本**）—— 真实做题的形状。
    //
    // ⚠ 场景 1/2 在**两份文本之间来回** ⇒ 第二刀起命中的是**第一次就喂热的缓存**
    // （实测 `by=9` · 78ms）⇒ 那**不是**用户连续敲键的成本 ✗。真实键入每刀都是
    // **没编过的文本**（实测 `by=81` · ~220ms）✓ —— 这一臂才是北极星的读数。
    let typing = run_scenario(
        &mut client,
        &uri,
        "typing",
        &text,
        &|current, _round| add_one_space_before_y(current),
        5,
    );

    // 场景 4：**等长改名**（每一刀都是新文本，但**字节偏移不变**）。
    //
    // 与场景 3 配对就是本轮的关键 A/B：两者都是"没编过的新文本"，**只差偏移是否平移**
    // ⇒ 若本臂便宜（`by=9`）而场景 3 贵（`by=81`），就证明**贵因是"插入字符把后缀的
    // 起点全推了位"**（`EntryCache::trusted_prefix` 要求起点逐条相同 ⇒ 后缀全部判脏 ✗）。
    let equal_length = run_scenario(
        &mut client,
        &uri,
        "typing_equal_length",
        &text,
        &|current, even| {
            let (from, to) = if even {
                ("demo_mem_image", "demo_mem_imagX")
            } else {
                ("demo_mem_imagX", "demo_mem_image")
            };
            current.replace(from, to)
        },
        5,
    );

    // 场景 5：**只追加尾部注释**（§16 的确认实验）—— **脏集为空**：追加在文件末尾不移动
    // 任何命令的起点 ⇒ 全部命令都可信任 ⇒ 这一刀量到的是"**一刀的固定底**"（
    // 服务端排空 + 报告装配 + 那一趟 pass 的固定成本）。它决定 T2-B 的天花板：
    // 若这个底已经 ≈70ms（= typing 的量级）⇒ T2-B **直接划掉** ✗。
    let trailing = run_scenario(
        &mut client,
        &uri,
        "trailing_comment",
        &text,
        &|current, even| {
            if even {
                format!("{current}-- \u{1F4CC}\n")
            } else {
                format!("{current}-- \u{1F4CD}\n")
            }
        },
        5,
    );

    let identity = binary_identity();
    for arm in [&proof, &statement, &typing, &equal_length, &trailing] {
        println!(
            "PERF keystroke-wallclock {} {}: best {:.1}ms · median {:.1}ms · worst {:.1}ms \
             (n={}, {}) · compile={}ms modules={} prefix={} by={} tc={} telescope={} · artifacts={}",
            rel,
            arm.scenario,
            arm.best(),
            arm.median(),
            arm.worst(),
            arm.samples.len(),
            identity,
            arm.compile_ms,
            arm.modules,
            arm.prefix,
            arm.by,
            arm.tc,
            arm.telescope,
            if artifacts_off { "off" } else { "on" },
        );
        perf_json(serde_json::json!({
            "schema": "soko.perf/1",
            "scope": "lsp-course",
            "case": format!("keystroke_wallclock_{}", arm.scenario),
            "entry": rel,
            "best_ms": (arm.best() * 10.0).round() / 10.0,
            "median_ms": (arm.median() * 10.0).round() / 10.0,
            "worst_ms": (arm.worst() * 10.0).round() / 10.0,
            "n": arm.samples.len(),
            "compile_ms": arm.compile_ms,
            "modules": arm.modules,
            "prefix": arm.prefix,
            "by": arm.by,
            "tc": arm.tc,
            "telescope": arm.telescope,
            "artifacts": if artifacts_off { "off" } else { "on" },
            "build": identity,
        }));
        // **量级哨兵**（30s，同 `perf_course.rs`）：抓的是"退化成分钟级"，
        // 不是"机器今天很吵"（绝对毫秒不进判据 ✓）。
        assert!(
            arm.worst() < 30_000.0,
            "{} 场景一次按键 {:.1}ms（量级哨兵 30s）",
            arm.scenario,
            arm.worst()
        );
    }
    let _ = client.request(99, "shutdown", serde_json::json!(null));
    // **不删 `courses/` 下的产物**：那是共享语料（别人也在用）⇒ 只清自己的 cache ✓。
    let _ = std::fs::remove_dir_all(&cache);
}

/// **产物命中后的第一刀**（§8.1 的"首次按键"）：开档**立刻**改一行 —— 不先等
/// A5/A5b 的后台预热。这是"打开就开敲"的真实形状，也是历史读数最贵的一刀
/// （§8.1：产物命中那臂 **1233ms**，`modules=5`）。
///
/// 与 [`perf_course_keystroke_wallclock_is_recorded`] **分开一条测试**：那条先
/// `settled_compile_count()`（等预热落定）⇒ 量的是**稳态**；这条刻意**不等**。
#[test]
fn perf_course_first_keystroke_after_open_is_recorded() {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml");
        return;
    };
    let rel = "units/I.3/unit08-images-preimages.sokonanoda";
    let path = root.join(rel);
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：读不到 {}", path.display());
        return;
    };
    let artifacts_off = std::env::var_os("SOKONANODA_NO_PROJECT_ARTIFACTS").is_some();
    let cache = std::env::temp_dir().join(format!(
        "sokonanoda-lsp-first-keystroke-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&cache);
    let mut client = Client::start_traced_with_env(
        &cache,
        if artifacts_off {
            &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")]
        } else {
            &[]
        },
    );
    let uri = Client::file_uri(&path);
    let _ = client.open(&root, &uri, &text); // 只等诊断，**不等预热**
    let edited = text.replacen("Set.mem_image α β f A y", "Set.mem_image α β f A  y", 1);
    assert_ne!(edited, text, "夹具前提：这一刀必须真的改变文本");
    let started = Instant::now();
    let _ = client.did_change(&uri, 2, &edited);
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    // 等 trace 行落定（stderr 读线程与 stdout 无顺序保证）—— 结构计数是判据，
    // 墙钟只是同机前后比较用的读数。
    let _ = client.wait_for_trace_after(0);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let line = client.last_trace();
    let identity = binary_identity();
    // **第一刀的墙钟被谁挡住？**（2026-10-09 第 13 轮）`compile=` 只是服务端自报的编译时长，
    // 差额去哪了要先看见：产物命中后紧跟着的是 **A5 的后台库层预热**（同一个编译 worker）
    // ⇒ 用户立刻敲的那一刀可能**排在它后面** ✗。
    let warm_lines = client.warm_trace_len();
    println!(
        "PERF first-keystroke-warmup: warm_lines={warm_lines} last={:?}",
        // `last_warm_trace()` 在"一行都还没有"时**会 panic**（它有自己的前提 ✓）——这里
        // 恰恰要问"预热到底跑完没有"，所以自己先判空 ✓。
        (warm_lines > 0).then(|| client.last_warm_trace())
    );
    println!(
        "PERF first-keystroke-after-open {rel}: {ms:.1}ms ({identity}) · compile={}ms modules={} \
         prefix={} by={} · artifacts={}\n  {line}",
        trace_compile_ms(&line),
        Client::trace_field(&line, "modules"),
        Client::trace_field(&line, "prefix"),
        Client::trace_field(&line, "by"),
        if artifacts_off { "off" } else { "on" },
    );
    perf_json(serde_json::json!({
        "schema": "soko.perf/1",
        "scope": "lsp-course",
        "case": "first_keystroke_after_open",
        "entry": rel,
        "ms": (ms * 10.0).round() / 10.0,
        "compile_ms": trace_compile_ms(&line),
        "modules": Client::trace_field(&line, "modules"),
        "prefix": Client::trace_field(&line, "prefix"),
        "by": Client::trace_field(&line, "by"),
        "artifacts": if artifacts_off { "off" } else { "on" },
        "build": identity,
    }));
    assert!(ms < 30_000.0, "开档后第一刀 {ms:.1}ms（量级哨兵 30s）");
    let _ = client.request(99, "shutdown", serde_json::json!(null));
    let _ = std::fs::remove_dir_all(&cache);
}

/// **跨入口切换**（方向①"产物化"的**用户可见**读数）：开 `unit08` 之后再开 `unit09`。
///
/// ## 为什么这一条才是方向①的尺
///
/// unit08 的库闭包 = `lib.{Logic,Exists,Set,Image}` + 传递依赖；unit09 换成
/// `lib.Equiv` ⇒ 两条闭包**共享一整段前缀**（`Logic`/`Exists`/`Set` …）。
/// **T1-A 之前**：库层键是**整条闭包**的摘要 ⇒ 换一个单元 = 整条库层重编 ✗；
/// **之后**：模块级前缀检查点命中 ⇒ 只编**分叉之后那几个模块** ✓。
///
/// 读数（结构计数优先）：`modules=` = 这一次 `didOpen` 真的编了几个模块；
/// 墙钟只作**同机前后**比较（`AGENTS.md` 判据纪律 ②）。
#[test]
fn perf_course_cross_entry_switch_is_recorded() {
    let Some(root) = course_root() else {
        eprintln!("跳过：找不到 courses/set-theory/sokonanoda.toml");
        return;
    };
    let first_rel = "units/I.3/unit08-images-preimages.sokonanoda";
    let second_rel = "units/I.3/unit09-equinumerosity.sokonanoda";
    let first = root.join(first_rel);
    let second = root.join(second_rel);
    let (Ok(text_a), Ok(text_b)) = (
        std::fs::read_to_string(&first),
        std::fs::read_to_string(&second),
    ) else {
        eprintln!("跳过：读不到 {first_rel} 或 {second_rel}");
        return;
    };
    let cache =
        std::env::temp_dir().join(format!("sokonanoda-lsp-cross-entry-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let artifacts_off = std::env::var_os("SOKONANODA_NO_PROJECT_ARTIFACTS").is_some();
    let mut client = Client::start_traced_with_env(
        &cache,
        if artifacts_off {
            &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")]
        } else {
            &[]
        },
    );
    let uri_a = Client::file_uri(&first);
    let uri_b = Client::file_uri(&second);
    // ① 开 unit08（冷：整条闭包 —— 不判它 ✓），并等 A5 的库层预热落定。
    let _ = client.open(&root, &uri_a, &text_a);
    let _ = client.settled_compile_count();
    let _ = client.warm_trace_len();
    std::thread::sleep(std::time::Duration::from_millis(50));
    // ② **被量的那一刀**：开 unit09（不同闭包、共享前缀）。
    let before = client.trace_len();
    let started = Instant::now();
    client.did_open(&uri_b, &text_b);
    let _ = client.diagnostics_for(&uri_b);
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    if client.trace_len() > before {
        let line = client.last_trace();
        let identity = binary_identity();
        // **同机参照**：另起一个进程**冷开** unit09（`NO_PROJECT_ARTIFACTS=1` 逼它真编）
        // ⇒ 拿到"整条闭包几个模块"的对照 ⇒ 读数**自足**（不依赖别处的历史数字 ✓）。
        let reference_modules = {
            let ref_cache = std::env::temp_dir().join(format!(
                "sokonanoda-lsp-cross-entry-ref-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&ref_cache);
            let mut ref_client = Client::start_traced_with_env(
                &ref_cache,
                &[("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")],
            );
            let ref_uri = Client::file_uri(&second);
            let _ = ref_client.open(&root, &ref_uri, &text_b);
            let _ = ref_client.wait_for_trace_after(0);
            let value = Client::trace_field(&ref_client.last_trace(), "modules");
            let _ = ref_client.request(99, "shutdown", serde_json::json!(null));
            let _ = std::fs::remove_dir_all(&ref_cache);
            value
        };
        println!(
            "PERF cross-entry-switch {first_rel} → {second_rel}: {ms:.1}ms ({identity}) · \
             compile={}ms modules={}（冷开同一入口 = {reference_modules} ⇒ 复用 \
             {} 个）· by={} · artifacts={}",
            trace_compile_ms(&line),
            Client::trace_field(&line, "modules"),
            reference_modules.saturating_sub(Client::trace_field(&line, "modules")),
            Client::trace_field(&line, "by"),
            if artifacts_off { "off" } else { "on" },
        );
        perf_json(serde_json::json!({
            "schema": "soko.perf/1",
            "scope": "lsp-course",
            "case": "cross_entry_switch",
            "from": first_rel,
            "to": second_rel,
            "ms": (ms * 10.0).round() / 10.0,
            "compile_ms": trace_compile_ms(&line),
            "modules": Client::trace_field(&line, "modules"),
            "cold_modules": reference_modules,
            "reused_modules": reference_modules.saturating_sub(Client::trace_field(&line, "modules")),
            "by": Client::trace_field(&line, "by"),
            "artifacts": if artifacts_off { "off" } else { "on" },
            "build": identity,
        }));
    } else {
        // **产物命中**那条路：`didOpen` 不编译 ⇒ 没有 trace 行（读数缺席，不是失败 ✓）。
        println!("PERF cross-entry-switch {first_rel} → {second_rel}: 未编译（产物命中）");
    }
    assert!(ms < 30_000.0, "跨入口切换 {ms:.1}ms（量级哨兵 30s）");
    let _ = client.request(99, "shutdown", serde_json::json!(null));
    let _ = std::fs::remove_dir_all(&cache);
}

/// `PERFJSON …` 一行（与 `crates/lsp/src/tests/perf.rs` 同格式，台账脚本按这个抓）。
fn perf_json(value: serde_json::Value) {
    println!(
        "PERFJSON {}",
        serde_json::to_string(&value).expect("serialize")
    );
}
