//! **CLI 命令面**的判据（用户 2026-10-08：「cli 很多命令有问题，build 没有反应，
//! rebuild 和 clean 没有实现」）。
//!
//! 契约：`docs/protocol.md` §"Build cache: `sokonanoda build`"（子命令 + 退出码）；
//! 语义与扩展的三条命令一一对应（Build / Rebuild / Clean Cache ✓）。
//!
//! 判据全部走**真进程**（spawn CLI）+ 隔离的 `SOKONANODA_CACHE_DIR`（沿用
//! `artifacts.rs` 的纪律 ✓）；断言绑**用户动作的后果** ✓（退出码 + 事件流 + stderr），
//! 不是"函数返回了" ✗。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

const SOURCE: &str = "theorem t (a : Prop) : a -> a := fun (h : a) => h\n";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-cli-surface-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn cache(tag: &str) -> PathBuf {
    scratch(&format!("cache-{tag}"))
}

/// 跑真二进制：隔离缓存 + 可选工作目录 ⇒ `(exit, stdout, stderr)`。
fn run(cache: &Path, args: &[&str], cwd: Option<&Path>) -> (i32, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    cmd.args(args)
        .env("SOKONANODA_CACHE_DIR", cache)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd.output().expect("run sokonanoda");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn events(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

/// `build.summary` 事件（判据只认它 —— `build.tick`/`build.progress` 是心跳与进度 ✓）。
fn summary(stdout: &str) -> Value {
    events(stdout)
        .into_iter()
        .find(|event| event["type"] == "build.summary")
        .unwrap_or_else(|| panic!("没有 build.summary：{stdout}"))
}

fn types(stdout: &str) -> Vec<String> {
    events(stdout)
        .iter()
        .filter_map(|event| event["type"].as_str().map(str::to_string))
        .collect()
}

/// **`clean` ≡ `build --clean`**（用户：「clean 没有实现」✗）：输出与退出码**逐字节相同** ✓。
#[test]
fn clean_is_exactly_build_clean() {
    let dir = scratch("clean");
    let file = dir.join("a.sokonanoda");
    std::fs::write(&file, SOURCE).expect("write fixture");
    let path = file.to_str().expect("utf-8 path");
    let cache = cache("clean");

    // 先 warm 一次，制造"确实有东西可清"的状态 ✓（否则 removed=0 两边都一样、判据空转 ✗）。
    let (code, out, err) = run(&cache, &["build", "--json", path], None);
    assert_eq!(code, 0, "warm 必须成功：{err}");
    assert_eq!(summary(&out)["compiled"], 1, "{out}");

    let (code_clean, out_clean, err_clean) = run(&cache, &["clean", "--json", path], None);
    assert_eq!(code_clean, 0, "clean 必须成功：{err_clean}");
    // 清完再 warm 回来，让第二次比较面对**同一状态** ✓。
    let (code, _, err) = run(&cache, &["build", "--json", path], None);
    assert_eq!(code, 0, "重新 warm 必须成功：{err}");
    let (code_flag, out_flag, err_flag) = run(&cache, &["build", "--clean", "--json", path], None);
    assert_eq!(code_flag, 0, "build --clean 必须成功：{err_flag}");

    assert_eq!(
        out_clean, out_flag,
        "`clean` 与 `build --clean` 必须逐字节同一条输出（同一条实现 ✓）"
    );
    assert_eq!(
        types(&out_clean).first().map(String::as_str),
        Some("build.clean")
    );
}

/// **`rebuild` = 先清（两处）再预热** —— 与扩展的 `Rebuild (alt+shift+b)` 同一条语义 ✓。
///
/// 判据的核心是**"清完必须真重编"** ✓：先证明缓存生效（第二次 `build` 是 `hit` ✓），
/// 再证明 `rebuild` 之后**不是命中**（`compiled == 1`、`hit == 0` ✓）—— 那正是
/// "清空编译缓存后重编译"的用户语义 ✓。
#[test]
fn rebuild_clears_both_stores_then_recompiles() {
    let dir = scratch("rebuild");
    let file = dir.join("a.sokonanoda");
    std::fs::write(&file, SOURCE).expect("write fixture");
    let path = file.to_str().expect("utf-8 path");
    let cache = cache("rebuild");

    let (code, out, err) = run(&cache, &["build", "--json", path], None);
    assert_eq!(code, 0, "第一次 warm：{err}");
    assert_eq!(summary(&out)["compiled"], 1, "{out}");
    let (code, out, err) = run(&cache, &["build", "--json", path], None);
    assert_eq!(code, 0, "第二次 warm：{err}");
    assert_eq!(
        summary(&out)["hit"],
        1,
        "前提：同一份源第二次必须命中（否则后面的判据证明不了任何东西 ✗）：{out}"
    );

    let (code, out, err) = run(&cache, &["rebuild", "--json", path], None);
    assert_eq!(code, 0, "rebuild 必须成功：{err}");
    assert_eq!(
        types(&out).first().map(String::as_str),
        Some("build.clean"),
        "rebuild 必须**先清**：{out}"
    );
    assert_eq!(
        summary(&out)["compiled"],
        1,
        "清完必须**真重编**（不是命中）—— 这就是 Rebuild 的用户语义：{out}"
    );
    assert_eq!(summary(&out)["hit"], 0, "{out}");

    // 反向：`build`（不清）在这时**应当**命中 ⇒ 两条命令确实不同 ✓（咬得住"rebuild 没清" ✗）。
    let (code, out, err) = run(&cache, &["build", "--json", path], None);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        summary(&out)["hit"],
        1,
        "rebuild 之后的普通 build 应当命中：{out}"
    );
}

/// **未知子命令 = 用法错误（exit 2）**，不再是"当成文件路径"的裸 OS 错误 ✗✓。
#[test]
fn an_unknown_subcommand_is_a_usage_error_not_a_missing_file() {
    let (code, _out, err) = run(&cache("unknown"), &["rebuildd"], None);
    assert_eq!(
        code, 2,
        "未知子命令是**用法**错误（以前 exit 1 + 裸 OS 错误 ✗）：{err}"
    );
    assert!(
        err.contains("未知子命令") && err.contains("rebuildd"),
        "{err}"
    );
    assert!(
        err.contains("--help"),
        "要给指路（`sokonanoda --help`）：{err}"
    );
    assert!(
        !err.contains("os error"),
        "不许再出现裸 OS 错误（用户报的正是这个 ✗）：{err}"
    );
}

/// 缺文件时**指名文件**（以前是裸 `error: No such file or directory (os error 2)` ✗）。
#[test]
fn a_missing_file_error_names_the_file() {
    let missing = scratch("missing").join("nope.sokonanoda");
    let path = missing.to_str().expect("utf-8 path");
    let cache = cache("missing");

    let (code, _out, err) = run(&cache, &[path], None);
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains("nope.sokonanoda"),
        "check 路径必须指名文件：{err}"
    );

    let (code, _out, err) = run(&cache, &["grade", path], None);
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains("nope.sokonanoda"),
        "grade 路径必须指名文件：{err}"
    );
}

/// 把**目录**当文件检查 ⇒ 说清是目录 + 指路 `sokonanoda build <dir>` ✓。
#[test]
fn a_directory_argument_points_at_build() {
    let dir = scratch("dirarg");
    let (code, _out, err) = run(&cache("dirarg"), &[dir.to_str().expect("utf-8")], None);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("是目录"), "{err}");
    assert!(err.contains("sokonanoda build"), "要给一条出路：{err}");
}

/// `build` 在**没有源**的目录上：先说"没有 .sokonanoda 文件"，再给用法 ✓
///（以前**只**打一行 `usage:` ⇒ 用户以为命令写错了 ✗）。
#[test]
fn build_on_a_source_less_directory_says_so_instead_of_only_usage() {
    let dir = scratch("empty");
    let (code, _out, err) = run(
        &cache("empty"),
        &["build", dir.to_str().expect("utf-8")],
        None,
    );
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("没有 .sokonanoda 文件"), "{err}");
    assert!(err.contains("用法："), "用法仍要给：{err}");
}

/// **跳过构建产物/依赖目录**（用户：「build 没有反应」✗ —— 语言仓根 269k 文件全递归）。
///
/// 反向的一半同样要钉 ✓：**显式点名** `target/` 时**照走**（"只跳孩子、不跳根" ✓）。
#[test]
fn build_skips_build_and_dependency_directories() {
    let dir = scratch("skip");
    std::fs::write(dir.join("a.sokonanoda"), SOURCE).expect("write fixture");
    for junk in ["target", "node_modules", ".git", ".sokonanoda"] {
        let sub = dir.join(junk);
        std::fs::create_dir_all(&sub).expect("create junk dir");
        std::fs::write(sub.join("hidden.sokonanoda"), SOURCE).expect("write hidden fixture");
    }
    let cache = cache("skip");
    let (code, out, err) = run(
        &cache,
        &["build", "--json", dir.to_str().expect("utf-8")],
        None,
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        summary(&out)["files"],
        1,
        "只该看到 `a.sokonanoda`（target/node_modules/.git/.sokonanoda 全跳过）：{out}"
    );

    let target = dir.join("target");
    let (code, out, err) = run(
        &cache,
        &["build", "--json", target.to_str().expect("utf-8")],
        None,
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        summary(&out)["files"],
        1,
        "显式点名的根**不跳**（用户说了算 ✓）：{out}"
    );
}

/// `clean` 不给路径 ⇒ **只清全局**（设计 §3.7：解不出模块根就不清、也不假装清 ✓，语义不变）
/// ⇒ 但必须**说出来** ✗✓（以前是默不作声的假动作：紧接着的预热全 `hit` ⇒
/// "rebuild 什么都没重编" ✗ —— 正是用户报的那类"没反应"）。
#[test]
fn clean_without_a_path_says_project_stores_are_untouched() {
    let dir = scratch("clean-note");
    std::fs::write(dir.join("Lib.sokonanoda"), SOURCE).expect("write module");
    // ⚠ 入口的声明名**不能与模块重名**（`SOURCE` 里是 `t`）⇒ 否则撞名 ⇒ failed ✗。
    std::fs::write(
        dir.join("entry.sokonanoda"),
        "import Lib\n\ntheorem u (a : Prop) : a -> a := fun (h : a) => h\n",
    )
    .expect("write entry");
    let path = dir.to_str().expect("utf-8");
    let cache = cache("clean-note");

    // 前提：这一趟**真的写出项目产物**（否则下面的 project 断言两边都是 0 ⇒ 空转 ✗）。
    let (code, out, err) = run(&cache, &["build", "--json", path], None);
    assert_eq!(code, 0, "{err}");
    assert_eq!(summary(&out)["compiled"], 2, "两文件项目都要编：{out}");

    // ① 无路径：项目产物**一个都不清**（语义不变 ✓）+ **必须打印 note** ✓。
    let (code, out, err) = run(&cache, &["clean", "--json"], None);
    assert_eq!(code, 0, "{err}");
    let clean = events(&out)
        .into_iter()
        .find(|event| event["type"] == "build.clean")
        .expect("build.clean");
    assert_eq!(clean["project"], 0, "无路径时项目产物不动：{out}");
    assert!(
        err.contains("没有给路径"),
        "必须把「只清了全局」说出来（不许默不作声 ✗）：{err}"
    );

    // ② 带路径：项目产物**真的被清** ✓（反向 —— 否则"note 说了、功能没做"也看不出来 ✗）。
    let (code, out, err) = run(&cache, &["clean", "--json", path], None);
    assert_eq!(code, 0, "{err}");
    let clean = events(&out)
        .into_iter()
        .find(|event| event["type"] == "build.clean")
        .expect("build.clean");
    assert!(
        clean["project"].as_u64().unwrap_or(0) >= 1,
        "带路径必须清到项目条目：{out}"
    );
}

/// **失败的 build 必须说清「哪个文件、为什么」**（用户 2026-10-08：「我运行
/// `sokonanoda:rebuild` 会报 **失败 2**，但是我又不知道哪里失败的」✗）。
///
/// 两半都要钉 ✓：① `--json` 的 `build.file` 带 `error`（additive ⇒ 扩展/CI 拿得到原因 ✓）；
/// ② 人话模式给**失败明细块**（不用在进度行里翻 ✗）。夹具是**期望判红**的探针
///（`kernel-rejected` ✓ —— 与课程 `gaps/C-04-*-reject.sokonanoda` 同形 ✓）。
#[test]
fn a_failed_build_reports_the_file_and_the_reason() {
    let dir = scratch("failed");
    std::fs::write(dir.join("Lib.sokonanoda"), SOURCE).expect("write module");
    // 入口故意判红：`Prop -> Type` 的恒等函数（内核正确地拒绝它 ✓）。
    std::fs::write(
        dir.join("entry.sokonanoda"),
        "import Lib\n\ndef bad : Prop -> Type := fun (x : Prop) => x\n",
    )
    .expect("write entry");
    let path = dir.to_str().expect("utf-8");
    let cache = cache("failed");

    // ① `--json`：失败事件必须带 `error`（原因），summary 记 failed = 1 ✓。
    let (code, out, err) = run(&cache, &["build", "--json", path], None);
    assert_eq!(code, 0, "{err}");
    assert_eq!(summary(&out)["failed"], 1, "{out}");
    let failed_event = events(&out)
        .into_iter()
        .find(|event| event["type"] == "build.file" && event["status"] == "failed")
        .expect("必须有一条 build.file 失败事件");
    let reason = failed_event["error"].as_str().unwrap_or("");
    assert!(
        reason.contains("entry") && reason.contains("kernel-rejected"),
        "事件必须带原因（模块名 + code + 行列）：{failed_event}"
    );

    // ② 人话模式：stderr 要有**失败明细块**（点名 + 原因 ✓）。
    let (code, _out, err) = run(&cache, &["build", path], None);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("失败明细"), "要给明细块：{err}");
    assert!(err.contains("entry.sokonanoda"), "明细必须点名文件：{err}");
    assert!(err.contains("kernel-rejected"), "明细必须给原因：{err}");
}

/// `build` 不给路径 = **当前目录**，且**要说话**（用户报的就是"没反应" ✗）。
#[test]
fn build_without_a_path_scans_the_current_directory_and_says_so() {
    let dir = scratch("cwd");
    std::fs::write(dir.join("a.sokonanoda"), SOURCE).expect("write fixture");
    let (code, out, err) = run(&cache("cwd"), &["build", "--json"], Some(&dir));
    assert_eq!(code, 0, "{err}");
    assert_eq!(summary(&out)["files"], 1, "{out}");
    assert!(
        err.contains("scanning the current directory"),
        "默认扫描必须说话（用户报「build 没有反应」✗）：{err}"
    );
    assert!(err.contains("found 1 .sokonanoda file(s)"), "{err}");
}
