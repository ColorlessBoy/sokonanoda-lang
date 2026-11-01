//! **P2-12 S5 ②**：strict **两个过渡开关都开**时的**用户可见文字**判据
//! （设计稿 §Q1 的三个复现件 c1 / c5 / R1；权威实施单 `reports/2026-11-01-p212-s5-spec.md` §2②）。
//!
//! 为什么单独一个文件：`implicit_at_alignment.rs` / `protocol.rs` 钉的是**判红与 span**（t1/t2/c3 那一族），
//! 本文件钉的是**另一件事**——用户看到的**那一段文字**：码对不对 · 里面有没有内核内部名（`\0soko_m0` /
//! `?1073741824`）· span 是不是真的（不是 `0:0`）· **文案里给的出路能不能照抄就判绿**。
//!
//! ## 三个夹具（逐字取自设计稿 §Q1 的表）
//!
//! | tag | 写法 | 两开关开出什么 |
//! |---|---|---|
//! | `c1` | `axiom g : {A : Type} -> Nat -> Prop` + `theorem t1 : g 0` | `elab-implicit-argument-unsolved` · 点名 `A` · 出路 `@g <A> 0` |
//! | `c5` | `q : {A}{B} -> A -> Prop` + `q 0` | 同上，点名 `A`/`B` · 出路 `@q <A> <B> 0` |
//! | `R1` | P2-7 的 R1（`import lib.Image`，`''`/`∅`/`≠`） | 记法路由（`≠` ⇒ `Ne`）的隐式位，span = 整条 `≠` 应用 |
//! | `b2` | 用户 2026-10-16 的形状（`h Nat Nat (fun …) 5`） | `§Q3` 第一条规则（surplus）先手 ⇒ **`dead` 出口**（不是回落）—— **⑤** 的牙 |
//!
//! ## 四条断言（每条都断**用户可见的东西**，不是"字段存在"）
//!
//! 1. `code` 是那一个（可归属，不许"报了个别的错也算"）；
//! 2. **正文**（`message` 与 `hint`）不含内核内部名：`\0` · `soko_m` · `soko_p` · `?<数字>`
//!    —— 设计稿 §Q3 的最后一道守卫；
//! 3. `span` **不是 `0:0`**，且**逐字**落在源里那条应用上（`start`/`end` 的行列都钉死）；
//! 4. 正文给出的 **`@` 出路照抄就判绿**（能 mechanize 的那两个：c1/c5；R1 的出路在记法路由上
//!    是退化的 `@Ne <α> …`，见 §"R1 为什么不断言出路"）。
//!
//! ## 环境隔离（照 `strict_provenance.rs` / `strict_fallback_census.rs` 的写法）
//!
//! 每次 spawn 都先 `env_remove` 掉**所有**跟判定有关的开关再显式注入本用例那一档 —— `Command` 默认
//! 继承父环境，开发机/CI 只要外部设过任何一个，「两开关开」量到的就不是它 ✗（假红/假绿）。
//! 每次一个**新的** `SOKONANODA_CACHE_DIR`：命中的磁盘缓存会整份回放（`elab.rs` 那几行根本不跑）⇒
//! 判据会"绿得没道理" ✗。两份 `NO_*_ARTIFACTS` 关掉产物 ⇒ 不在 `courses/` 下落东西。
//!
//! ## R1 为什么不断言出路
//!
//! R1 走的是**记法路由**（`≠` 的头在源里不可见 ⇒ `kernel_app_fallback::facts_notation` 把
//! `head_text = whole_text = 规范名`）⇒ `unsolved_message` 的 `strip_prefix` 拿到**空** `rest`，出路印成
//! `` `@Ne <α> ` ``（**没有操作数**、末尾一个空格）—— 这条**不能**照抄（粘回去是语法错）。
//! S3 的实施单 §2.3 写的是"出路会退化成 `@<头> …`"，实测**不是**那个形状 ⇒ 这是本刀**如实记录、
//! 没有修**的既有文字缺陷（改它要动 S2/S4b 的文案规则与既有 golden，留给记法路由的下一刀）；
//! 所以这里**不**写"粘回去判绿"，也不把这个形状钉死（免得把缺陷固化成契约）。

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// c1：`{A}` 只由**后续显式实参**反解，而 `0 : Nat` 与 `Nat -> Prop` 都对不上 `A` ⇒ 补不出来。
const C1: &str = "\
axiom g : {A : Type} -> Nat -> Prop
theorem t1 : g 0 := sorry
";

/// c5：两个前导隐式；`A` 能从 `0` 反解出来，`B` 不能（loose 档接受，strict 判红 —— 设计稿 §Q5 的
/// 有意不同）。本用例只钉 strict 那一档的文字。
const C5: &str = "\
axiom q : {A : Type} -> {B : Type} -> A -> Prop
theorem t5 : q 0 := sorry
";

/// R1（P2-7 设计稿 §5 的复现件，逐字）：`''` / `∅` 来自 `lib.Set`，`≠` 是内建记法（目标 `Ne`）。
const R1: &str = "\
import lib.Image

theorem t : (fun (_ : Set Nat) => ∅) '' {(@Set.univ Nat)} ≠ (fun (_ : Set Nat) => ∅) '' {∅} := by sorry
";

/// c1/c5 的**来源**（axiom 那一行）：出路粘回去时要带着它一起判。
const C1_PREAMBLE: &str = "axiom g : {A : Type} -> Nat -> Prop\n";
const C5_PREAMBLE: &str = "axiom q : {A : Type} -> {B : Type} -> A -> Prop\n";

static SEQ: AtomicUsize = AtomicUsize::new(0);

fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&root).unwrap_or(root)
}

/// 一份只属于本用例的临时目录（cargo 的测试是**同进程多线程**跑的，共用路径会互相覆盖）。
fn scratch(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let dir =
        std::env::temp_dir().join(format!("s5-text-{}-{tag}-{nanos}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建临时目录");
    dir
}

/// **跟判定有关的环境变量一律先摘掉**（外部环境不许改读数）。
const JUDGE_ENV: [&str; 8] = [
    "SOKO_STRICT_AT",
    "SOKO_STRICT_KERNEL_APP",
    "SOKO_STRICT_NO_FALLBACK",
    "SOKO_KERNEL_ARG_PP",
    "SOKO_META_PATTERN",
    "SOKO_META_APPROX",
    "SOKO_META_INVENTORY",
    "SOKO_METAVAR",
];

/// 一次 `grade --dialect strict --json` 的结果。
struct Run {
    ok: bool,
    events: Vec<Value>,
}

impl Run {
    fn diagnostics(&self) -> Vec<&Value> {
        self.events
            .iter()
            .filter(|e| e.get("type").and_then(Value::as_str) == Some("diagnostic"))
            .collect()
    }

    fn checked(&self) -> Vec<String> {
        self.events
            .iter()
            .filter(|e| e.get("type").and_then(Value::as_str) == Some("decl.checked"))
            .filter_map(|e| e.get("name").and_then(Value::as_str).map(str::to_string))
            .collect()
    }
}

/// 跑一次用户那条命令（`grade --dialect strict --json`），**两个过渡开关都开**。
///
/// `root` = `--root`（R1 要 `import` 课程库 ⇒ 显式给模块根；c1/c5 不需要）。
fn grade(tag: &str, src: &str, root: Option<&Path>) -> Run {
    grade_logged(tag, src, root, None)
}

/// 同 [`grade`]，另带 `SOKO_KERNEL_APP_LOG`（内核路径每个出口记一行 `dead` / `kept`）。
///
/// 为什么需要这条通道：**判据 ⑤**（`§Q3` 第一条规则）在**用户可见文字**上**咬不住**（见下一条用例的
/// 注释：去掉它，b2 的文字逐字相同 ⇒ `implicit_at_alignment` 全绿）⇒ 只有日志通道能证明
/// "这个形状走的是**报错出口**，不是回落"。
fn grade_logged(tag: &str, src: &str, root: Option<&Path>, log: Option<&Path>) -> Run {
    let dir = scratch(tag);
    let path = dir.join("Probe.sokonanoda");
    std::fs::write(&path, src).expect("write fixture");
    let cache = dir.join("cache");
    std::fs::create_dir_all(&cache).expect("cache dir");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
    for key in JUDGE_ENV {
        cmd.env_remove(key);
    }
    cmd.env("SOKO_STRICT_KERNEL_APP", "1");
    cmd.env("SOKO_STRICT_NO_FALLBACK", "1");
    cmd.env("SOKONANODA_CACHE_DIR", &cache);
    cmd.env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1");
    cmd.env("SOKONANODA_NO_MODULE_ARTIFACTS", "1");
    if let Some(log) = log {
        cmd.env("SOKO_KERNEL_APP_LOG", log);
    }
    cmd.args(["grade", "--dialect", "strict"]);
    if let Some(root) = root {
        cmd.arg("--root").arg(root);
    }
    cmd.arg(&path).arg("--json");
    let out: Output = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run sokonanoda grade");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let events: Vec<Value> = text
        .lines()
        .filter(|line| line.trim_start().starts_with('{'))
        .map(|line| serde_json::from_str(line).expect("event JSON"))
        .collect();
    Run {
        ok: out.status.success(),
        events,
    }
}

/// 判据 ②：正文不含内核内部名（`\0` / `soko_m` / `soko_p` / `?<数字>`）。
///
/// 这一条是设计稿 §Q3 的**最后一道守卫**：内核给的 binder 名一律不能直接印（`Unsolved` 会带
/// `\0soko_mp{i}` 这种前端 fresh 名，外部元变量会带 `?1073741824`）⇒ 印出来就是"泄漏"。
fn assert_no_internal_names(what: &str, text: &str) {
    assert!(
        !text.contains('\0'),
        "{what} 里印了 NUL（内部名泄漏）：{text}"
    );
    for needle in ["soko_m", "soko_p"] {
        assert!(
            !text.contains(needle),
            "{what} 里印了内部 fresh 名 `{needle}`：{text}"
        );
    }
    // `?<数字>`：外部元变量的形状（`?1073741824`）。只查 `?` 紧跟 ASCII 数字的那一段。
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'?' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
            panic!("{what} 里印了外部元变量名（`?<数字>`）：{text}");
        }
    }
}

/// 取一行事件的 `span`，换算成 `(start_line, start_col, end_line, end_col)`（全 **1-based**）。
fn span_of(diag: &Value) -> (u64, u64, u64, u64) {
    let n = |section: &str, key: &str| -> u64 {
        diag["span"][section][key]
            .as_u64()
            .unwrap_or_else(|| panic!("诊断缺 span.{section}.{key}：{diag}"))
    };
    (
        n("start", "line"),
        n("start", "column"),
        n("end", "line"),
        n("end", "column"),
    )
}

/// 判据 ③：span **不是 `0:0`**，且**逐字**落在源里那条应用上。
///
/// `0:0` 是"位置丢了"的病征（设计稿 §Q1 的 c1/c5 两档开之前就是这个形状 ⇒ 用户定位不了 ✗）。
fn assert_real_span(tag: &str, diag: &Value, want: (u64, u64, u64, u64)) {
    let got = span_of(diag);
    assert_ne!(
        (got.0, got.1),
        (0, 0),
        "`{tag}` 的诊断位置丢了（`0:0` ⇒ 用户定位不了）✗：{diag}"
    );
    assert_eq!(
        got, want,
        "`{tag}` 的诊断必须逐字落在那条应用上（1-based 行列）✗：{diag}"
    );
}

/// 从文案里抠出 **`@` 出路**（`` `@g <A> 0` `` 那个反引号片段），把 `<…>` 占位符换成 `Nat`。
///
/// 判据 ④ 的原料：**文案说的那条形式必须真的能跑**（用户 2026-10-16 第 ③ 条「指令要明确」；
/// `implicit_at_alignment.rs` 的 `the_strict_error_names_…` 是同一把牙、钉应用路径那一支）。
fn escape_of(msg: &str) -> String {
    let raw = msg
        .split('`')
        .filter(|chunk| chunk.trim_start().starts_with('@'))
        .max_by_key(|chunk| chunk.len())
        .unwrap_or_else(|| panic!("文案里找不到可复制的 `@…` 出路：{msg}"));
    // `<A>` / `<B>` 是占位符（S2 的 `unsolved_message` 明说）⇒ 本用例把它填成 `Nat` 再判。
    let mut out = String::new();
    let mut rest = raw;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let close = rest[open..]
            .find('>')
            .unwrap_or_else(|| panic!("占位符 `<` 没有闭合：{raw}"));
        out.push_str("Nat");
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// 判据 ④：把文案给的出路粘回去 ⇒ **必须判绿**（且声明真的 checked）。
fn assert_escape_works(tag: &str, preamble: &str, escape: &str) {
    let probe = format!("{preamble}\ndef esc : Prop := {escape}\n");
    let run = grade(&format!("{tag}-escape"), &probe, None);
    let checked = run.checked();
    assert!(
        run.ok && checked.iter().any(|c| c == "esc"),
        "`{tag}` 文案给的出路 `{escape}` 必须**照抄就判绿** ✗（实测 ok={} checked={checked:?}，探针：{probe:?}）",
        run.ok
    );
    // 正对照的**反向**：探针里没有别的错（否则"绿"可能是别的原因）。
    assert!(
        run.diagnostics().is_empty(),
        "`{tag}` 的出路探针必须零诊断：{:?}",
        run.diagnostics()
    );
}

/// **c1**：`g 0` —— 隐式 `A` 补不出来。
///
/// 两档开关都开时它必须走**内核路径的报错出口**（不是老的 `unknown identifier \0soko_m0` · `0:0`）。
#[test]
fn c1_reports_the_unsolved_implicit_with_a_real_span_and_a_working_escape() {
    let run = grade("c1", C1, None);
    assert!(!run.ok, "c1 必须判红（两开关开）");
    let diags = run.diagnostics();
    assert_eq!(diags.len(), 1, "c1 恰好一条诊断：{diags:?}");
    let d = diags[0];

    // ① 码可归属。
    assert_eq!(
        d["code"], "elab-implicit-argument-unsolved",
        "c1 的码必须是 `elab-implicit-argument-unsolved`：{d}"
    );
    let msg = d["message"].as_str().expect("message");
    let hint = d["hint"].as_str().expect("hint");

    // ② 正文不许泄漏内核名（两处都查：`message` 是 S2 的文案，`hint` 是 `ErrorKind` 那张表）。
    assert_no_internal_names("c1.message", msg);
    assert_no_internal_names("c1.hint", hint);

    // ③ 真 span：`theorem t1 : g 0 := sorry` 的 `g 0`（1-based 2:14–2:17）。
    assert_real_span("c1", d, (2, 14, 2, 17));

    // 额外：点名**源级**形参（`A`，不是 `\0soko_p0`）。
    assert!(
        msg.contains("`A`"),
        "c1 的文案必须点名源级隐式形参 `A`：{msg}"
    );

    // ④ 出路照抄判绿。
    let escape = escape_of(msg);
    assert_eq!(escape, "@g Nat 0", "c1 的出路形状：{msg}");
    assert_escape_works("c1", C1_PREAMBLE, &escape);
}

/// **c5**：`q 0` —— 两个前导隐式，`B` 补不出来（loose 接受、strict 判红）。
#[test]
fn c5_names_every_leading_implicit_and_its_escape_works() {
    let run = grade("c5", C5, None);
    assert!(!run.ok, "c5 必须判红（strict 两开关开）");
    let diags = run.diagnostics();
    assert_eq!(diags.len(), 1, "c5 恰好一条诊断：{diags:?}");
    let d = diags[0];

    assert_eq!(
        d["code"], "elab-implicit-argument-unsolved",
        "c5 的码必须是 `elab-implicit-argument-unsolved`：{d}"
    );
    let msg = d["message"].as_str().expect("message");
    let hint = d["hint"].as_str().expect("hint");
    assert_no_internal_names("c5.message", msg);
    assert_no_internal_names("c5.hint", hint);

    // `theorem t5 : q 0 := sorry` 的 `q 0`（1-based 2:14–2:17）。
    assert_real_span("c5", d, (2, 14, 2, 17));

    // 两**个**前导隐式都要点名（只点一个 = 用户不知道该补几位 ✗）。
    for name in ["`A`", "`B`"] {
        assert!(msg.contains(name), "c5 的文案必须点名 `{name}`：{msg}");
    }

    let escape = escape_of(msg);
    assert_eq!(escape, "@q Nat Nat 0", "c5 的出路形状：{msg}");
    assert_escape_works("c5", C5_PREAMBLE, &escape);
}

/// **R1**：P2-7 的复现件（`import lib.Image` + `''`/`∅`/`≠`）。
///
/// ⚠ 本刀构建上它落在**记法路由**（`≠` ⇒ `Ne`）：两开关开出的是
/// `elab-implicit-argument-unsolved` · span = 整条 `≠` 应用（3:14–3:92），**不是**设计稿 §Q1 表里
/// 那个 `kernel-rejected`（S4b 把记法前导位在调内核前物化之后，这条路从"内核 def_eq 拒绝"变成了
/// "隐式位补不出来"）。判定仍是**拒绝**（`rc ≠ 0`）⇒ **没有** pass→reject。
///
/// 三条断言（第 4 条"出路照抄判绿"在这里**做不到**，见文件头：记法路由的出路是退化的
/// `` `@Ne <α> ` ``，没有操作数 —— 那是既有文字缺陷，如实记录、未修）。
#[test]
fn r1_notation_route_reports_a_readable_error_with_a_real_span() {
    let run = grade("r1", R1, Some(&repo_root().join("courses/set-theory")));
    assert!(!run.ok, "R1 在 strict 两开关开时必须判红");
    let diags = run.diagnostics();
    assert_eq!(diags.len(), 1, "R1 恰好一条诊断：{diags:?}");
    let d = diags[0];

    assert_eq!(
        d["code"], "elab-implicit-argument-unsolved",
        "R1（记法路由）的码：{d}"
    );
    let msg = d["message"].as_str().expect("message");
    let hint = d["hint"].as_str().expect("hint");
    assert_no_internal_names("R1.message", msg);
    assert_no_internal_names("R1.hint", hint);

    // 整条 `≠` 应用：源第 3 行、14 列起、92 列止（1-based；夹具 = import + 空行 + 定理）。
    assert_real_span("r1", d, (3, 14, 3, 92));

    // 点名的必须是**源级**形参（`α`）。记法路由的头在源里不可见 ⇒ 文案里的头是**规范名** `Ne`。
    assert!(
        msg.contains("`α`"),
        "R1 的文案必须点名源级隐式形参 `α`：{msg}"
    );
    assert!(
        msg.contains("`Ne`"),
        "R1 走记法路由，文案的头是规范名 `Ne`（头在源里不可见）：{msg}"
    );
}

/// **⑤ 的牙（替代品）**：`§Q3` 第一条规则（"实参比显式层多 ⇒ 先报 `strict_rejection_message`"）
/// 今天在**用户可见文字**上**咬不住** —— 实测（`reports/2026-11-01-p212-s5-readings.md` §6）：
///
/// * `b2` 形状的内核结局是 **`ArgMismatch`** ⇒ 去掉那条规则之后它进 [`Report::Fallback`]
///   （`argmismatch_text`，S3 写明的例外）⇒ **回落老路**，而老路（`elab.rs:4780`）出的是**同一句**
///   `strict_rejection_message` ⇒ `implicit_at_alignment` 的 b2 **照旧全绿**（7/7，实测）；
/// * 所以"去掉规则 ⇒ b2 判红"这个预期**不成立**（实施单 §2⑤ 的写法在本刀构建上量不到）。
///
/// 能咬住的判据在**出口通道**上：同一份 `--json` 文字 + `SOKO_KERNEL_APP_LOG` 里
/// **必须是 `dead`（内核路径的报错出口），一个 `kept` 都不许有**（`kept` = 回落老路）。
/// 反向验证（实测）：把那条规则改成 `if false && …` ⇒ 本用例拿到 `kept argmismatch`、**判红**。
#[test]
fn surplus_shape_reports_through_the_kernel_path_not_by_falling_back() {
    // b2（用户 2026-10-16 亲报的形状）：正常形式喂位置隐式实参 ⇒ `n_args(4) > explicit_layers(2)`。
    const B2: &str = "\
def h {α : Type} {β : Type} (f : α → β) (a : α) : β := f a

def b2 : Nat := h Nat Nat (fun (n : Nat) => n) 5
";
    let log = scratch("b2").join("kernel_app.log");
    let run = grade_logged("b2", B2, None, Some(&log));

    // ① 用户看见的那一条：码 + 正文（点名源级 α/β、给出能照抄的 `@` 出路）。
    assert!(!run.ok, "b2 必须判红");
    let diags = run.diagnostics();
    assert_eq!(diags.len(), 1, "b2 恰好一条诊断：{diags:?}");
    let d = diags[0];
    assert_eq!(d["code"], "elab-implicit-argument-unsolved", "{d}");
    let msg = d["message"].as_str().expect("message");
    assert_no_internal_names("b2.message", msg);
    for name in ["`α`", "`β`"] {
        assert!(msg.contains(name), "b2 的文案必须点名 `{name}`：{msg}");
    }
    let escape = escape_of(msg);
    assert!(
        escape.starts_with("@h ") && escape.contains("(fun (n : Nat) => n) 5"),
        "b2 的出路必须把**实参**也带上（`@h <α> <β> <f> <a>`）：{msg}"
    );

    // ② **这条腿才是 ⑤ 的判据**：走的是报错出口（`dead`），不是回落老路（`kept`）。
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    let dead: Vec<&str> = text.lines().filter(|l| l.starts_with("dead ")).collect();
    let kept: Vec<&str> = text.lines().filter(|l| l.starts_with("kept ")).collect();
    assert!(
        !dead.is_empty(),
        "b2 必须经**内核路径的报错出口**（`dead` 行）——日志是空的或只有 kept：{text:?}"
    );
    assert!(
        kept.is_empty(),
        "b2 不许**回落老路**（`kept` 行 = 写明的例外）——那说明 §Q3 第一条规则没生效：{kept:?}"
    );
    assert!(
        dead.iter().all(|l| l.contains("argmismatch")),
        "b2 的内核结局是 `ArgMismatch`（§Q3 第一行规则先手覆盖它）：{dead:?}"
    );
}
