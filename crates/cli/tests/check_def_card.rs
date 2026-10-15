//! **`#check <裸 def 名>` 的声明卡片**（2026-10-15 用户新需求，`REQUIREMENTS.md` §9）。
//!
//! 判据就是**用户那条命令**（原话 2026-10-15 15:22，`--root courses/set-theory`）：
//!
//! ```text
//! #check Set.subset
//! ⇒ def Set.subset : ∀ {α : Type}, Set α → Set α → Prop
//!   := fun {α : Type 0} (A B : Set α) => ∀ (x : α), x ∈ A → x ∈ B
//! ```
//!
//! **两件缺一不可**：① **带定义体**（`:= ` 后面有东西）＋ ② **记法渲染**
//! （`∀`/`→`/`∈`，**不是** `forall`/`->`/`Set.mem`）。只做一件 ⇒ 不算完成 ✗。
//!
//! ## 为什么钉在 CLI 层（而不是 front 单测）
//!
//! 这一条是**用户可见的结果**（终端印出来的那一行 / `--json` 的 `human`）——
//! front 单测只证明"事件里有卡片"，证明不了"用户看见了卡片"（`AGENTS.md`
//! 「验证设计纪律」：真相与显示是两条路，bug 活在接缝里）。
//!
//! ⚠ 本文件**只读** `courses/**`：两个产物逃生门都开
//! （`SOKONANODA_NO_PROJECT_ARTIFACTS` / `SOKONANODA_NO_MODULE_ARTIFACTS`）
//! ⇒ 不在课程目录里落 `.sokonanoda/`。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// **用户要的那一行**（逐字节）。`∀ {α : Type}`（**不是** `Type 0`）与
/// `{α : Type 0}`（**是**）的差别来自既有声明卡片的两半：
/// 类型那一半过 `lean4_universe_text`（`Type 0` → `Type`，Lean 4 的 `delabSort`），
/// 定义体那一半不过（与 `lsp::declaration_card` 的 `:=` 块同款）。
const EXPECTED: &str = "def Set.subset : ∀ {α : Type}, Set α → Set α → Prop := \
                        fun {α : Type 0} (A B : Set α) => ∀ (x : α), x ∈ A → x ∈ B";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}

/// 临时入口文件（**不写进 `courses/**`**）+ 隔离缓存目录。
struct Fixture {
    dir: PathBuf,
    entry: PathBuf,
}

impl Fixture {
    fn new(tag: &str, body: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-check-def-card-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let entry = dir.join("entry.sokonanoda");
        std::fs::write(&entry, body).expect("write entry");
        Self { dir, entry }
    }

    /// 模块根 = **真的** `courses/set-theory`（用户那条命令的 `--root`）。
    fn run(&self, json: bool) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sokonanoda"));
        if json {
            cmd.arg("--json");
        }
        cmd.arg("--root")
            .arg(repo_root().join("courses/set-theory"))
            .arg(&self.entry)
            .current_dir(&self.dir)
            .env("SOKONANODA_CACHE_DIR", self.dir.join(".cache"))
            // **只读课程目录**：两个产物逃生门都开。
            .env("SOKONANODA_NO_PROJECT_ARTIFACTS", "1")
            .env("SOKONANODA_NO_MODULE_ARTIFACTS", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.output().expect("run sokonanoda")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn json_events(out: &std::process::Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every stdout line is JSON"))
        .collect()
}

/// **判据 ①（用户那条命令）**：`--json` 的 `human` 就是用户要的那一行。
///
/// 同时钉住"判定路径一个字节都没动"：`inferred_type` 仍是**类型**
/// （`judge::judge_type_of` ⇒ 记法目标签名读的是它 ✓）。
#[test]
fn check_on_set_subset_prints_the_declaration_card() {
    let fx = Fixture::new("json", "import lib.Set\n\n#check Set.subset\n");
    let out = fx.run(true);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events = json_events(&out);
    let typed = events
        .iter()
        .find(|e| e["type"] == "expr.typed")
        .unwrap_or_else(|| panic!("no expr.typed in {events:?}"));
    assert_eq!(typed["human"], EXPECTED, "① 用户要的那一行（逐字节）");
    assert_eq!(typed["text"], "Set.subset", "源切片不变");
    // ⚠ **`inferred_type` 不许变成卡片** ✗ —— 它是**类型**，判定路径读它。
    assert_eq!(
        typed["inferred_type"], "{α : Type} -> (A : Set α) -> (B : Set α) -> Prop",
        "判定路径读的 `inferred_type` 必须仍是类型"
    );
    // ② **记法**：卡片里不许出现点名形式。
    let human = typed["human"].as_str().expect("human is a string");
    for pointed in ["forall", "->", "Set.mem"] {
        assert!(!human.contains(pointed), "② 不许出现 {pointed:?}：{human}");
    }
    assert!(
        human.contains('∀') && human.contains('→') && human.contains('∈'),
        "② 必须是记法：{human}"
    );
}

/// **同一条命令的终端输出**（非 `--json`）也必须是那一行 —— 用户看的是屏幕，
/// 不是 JSON（`AGENTS.md`「验收必须断言用户可见的结果」）。
#[test]
fn check_on_set_subset_prints_the_card_on_the_terminal_too() {
    let fx = Fixture::new("plain", "import lib.Set\n\n#check Set.subset\n");
    let out = fx.run(false);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(EXPECTED),
        "终端输出里必须有那一行：{stdout}"
    );
}

/// **反向验证（用户判据 ②）**：`#check <theorem>` / `<axiom>` / `<inductive>` /
/// **非裸标识符** 一律**只打签名**（逐字节回到今天）——
/// 卡片只给 `def` ✓。
#[test]
fn only_defs_get_the_card() {
    let fx = Fixture::new(
        "kinds",
        "import lib.Set\n\n\
         #check Set.subset_def\n\
         #check Set.ext\n\
         #check Set\n\
         #check (Set.subset)\n",
    );
    let out = fx.run(true);
    assert!(out.status.success());
    let humans: Vec<String> = json_events(&out)
        .iter()
        .filter(|e| e["type"] == "expr.typed")
        .map(|e| e["human"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(humans.len(), 4, "{humans:?}");
    // `Set.subset_def` 是 **theorem** ⇒ 只有类型（点形式，逐字节回到今天）。
    assert_eq!(
        humans[0],
        "Set.subset_def: {α : Type} -> (A : Set α) -> (B : Set α) -> \
         Iff (Set.subset A B) ((x : α) -> Set.mem x A -> Set.mem x B)"
    );
    // `Set.ext` 是 **axiom** ⇒ 同上。
    assert_eq!(
        humans[1],
        "Set.ext: {α : Type} -> (A : Set α) -> (B : Set α) -> \
         ((x : α) -> Iff (A x) (B x)) -> Eq A B"
    );
    // `Set` 是 **def** ⇒ 有卡片（它是"集合类型"本身，`fun (α : Type 0) => α → Prop`）。
    assert_eq!(
        humans[2],
        "def Set : Type → Type := fun (α : Type 0) => α → Prop"
    );
    // `(Set.subset)`：**括号在 AST 里不留节点** ⇒ 与裸写同一条路（同样给卡片 ✓，
    // 不是特例 —— 判据是"源表达式是不是一个标识符"，不是"用户打没打括号"）。
    assert_eq!(humans[3], EXPECTED);
}
