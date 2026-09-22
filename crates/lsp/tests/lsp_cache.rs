//! LSP 侧的项目缓存端到端（T-A11 / T-A12）。
//!
//! **为什么是集成测试而不是单元测试**：`crates/lsp/src/lib.rs` 里到处是
//! `cfg!(test)` 守卫（单元测试并行跑，共享真实缓存目录会互相污染）。
//! 集成测试链接的是**不带 `cfg(test)` 编译的库** ⇒ 缓存是活的，而且这里
//! 每次都把 `SOKONANODA_CACHE_DIR` 指到自己的临时目录，互不干扰。
//!
//! 跑的是**真的 `sokonanoda-lsp` 进程**（`env!("CARGO_BIN_EXE_sokonanoda-lsp")`），
//! 因为要验证的正是"跨进程复用缓存"——同进程内测不到。

mod common;

use common::Client;
use std::path::{Path, PathBuf};

const LIB: &str = "\
def Set (α : Type) : Type := α -> Prop\n\
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a\n\
infix:50 \" ∈ \" => Set.mem\n\
def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x\n\
infix:50 \" ⊆ \" => Set.subset\n";

const ENTRY: &str = "\
import SetLib\n\n\
theorem mem_self (α : Type) (a : α) (A : Set α) (h : a ∈ A) : a ∈ A := h\n\n\
theorem open_one (α : Type) (A B : Set α) : A ⊆ B := by\n  sorry\n\n\
-- 点名引用**依赖里**的声明：跨文件 definition 用它（T-A13）。\n\
theorem uses_lib (α : Type) (a : α) (A : Set α) : Set.mem α a A -> Set.mem α a A :=\n\
  fun (h : Set.mem α a A) => h\n";

struct Fixture {
    dir: PathBuf,
    cache: PathBuf,
    entry: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("soko-lsp-cache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("SetLib.sokonanoda"), LIB).expect("write lib");
        let entry = dir.join("Canvas.sokonanoda");
        std::fs::write(&entry, ENTRY).expect("write entry");
        Self {
            cache: dir.join("cache"),
            dir,
            entry,
        }
    }

    fn uri(&self) -> String {
        format!("file://{}", self.entry.display())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn cache_entries(cache: &Path) -> usize {
    std::fs::read_dir(cache.join("compiled"))
        .map(|dir| dir.filter_map(Result::ok).count())
        .unwrap_or(0)
}

/// **T-A11**：一次 LSP 打开之后，条目必须落在缓存里。
///
/// 不写的话，只有"用户先跑过 CLI `build`"才享受得到 T-A10 的命中——
/// 第一次打开仍然白编，而且那份成果没人存。
#[test]
fn opening_a_project_document_writes_a_cache_entry() {
    let fixture = Fixture::new("write");
    assert_eq!(cache_entries(&fixture.cache), 0, "夹具前提：缓存是空的");

    let mut client = Client::start(&fixture.cache);
    let diagnostics = client.open(&fixture.dir, &fixture.uri(), ENTRY);
    assert!(
        diagnostics.contains("sorry"),
        "夹具前提：诊断里应当有那条 sorry：{diagnostics}"
    );

    assert!(
        cache_entries(&fixture.cache) >= 1,
        "LSP 打开之后必须写出项目缓存条目（T-A11），实际 {}",
        cache_entries(&fixture.cache)
    );
}

/// **T-A12**：两个**独立**的 LSP 进程打开同一份文档，诊断**逐字节一致**。
///
/// 第二个进程会命中第一个写下的条目 ⇒ 这条同时钉住"回放出来的诊断与真编译
/// 的一模一样"。诊断是用户直接看到的东西，也是 agent 判卷的通道——回放少一条
/// 或改一个字都是"缓存让你看到另一个世界"。
#[test]
fn a_warm_process_publishes_byte_identical_diagnostics() {
    let fixture = Fixture::new("replay");

    // 冷：全新缓存，真编译，同时把条目写下（T-A11）。
    let cold = {
        let mut client = Client::start(&fixture.cache);
        client.open(&fixture.dir, &fixture.uri(), ENTRY)
    };
    assert!(cache_entries(&fixture.cache) >= 1, "冷跑必须写下条目");

    // 热：另一个进程，同一份缓存。
    let warm = {
        let mut client = Client::start(&fixture.cache);
        client.open(&fixture.dir, &fixture.uri(), ENTRY)
    };

    assert_eq!(
        cold, warm,
        "冷/热两次打开的诊断必须逐字节一致（缓存回放不许改变用户看到的东西）"
    );
    assert!(
        cold.contains("sorry"),
        "夹具前提：诊断里应当有那条 sorry：{cold}"
    );
}

/// **T-A13**：命中缓存之后，**跨文件能力仍然工作**。
///
/// 这条钉的是 T-A03 的"整份报告一起回放"：`textDocument/definition` 读的是
/// `project_modules()`（模块表）。只回放入口报告的话，冷跑能跳、**热跑跳不了**
/// ——"命中缓存的文档能显示、不能跳转"，而用户完全不知道为什么。
#[test]
fn cross_file_definition_still_works_after_a_cache_hit() {
    let fixture = Fixture::new("definition");

    // 冷：全新缓存，真编译，写下条目。
    let cold = {
        let mut client = Client::start(&fixture.cache);
        let diagnostics = client.open(&fixture.dir, &fixture.uri(), ENTRY);
        assert!(
            !diagnostics.contains("elab-unknown"),
            "夹具前提：这份入口必须编译得干净：{diagnostics}"
        );
        client.definition_at(&fixture.uri(), ENTRY, "Set.mem α a A ->")
    };
    assert_eq!(
        cold.len(),
        1,
        "冷跑的 definition 必须命中一个位置：{cold:?}"
    );
    assert!(
        cold[0].ends_with("SetLib.sokonanoda"),
        "definition 必须跳到**依赖文件**：{cold:?}"
    );

    // 热：另一个进程，同一份缓存。
    let warm = {
        let mut client = Client::start(&fixture.cache);
        client.open(&fixture.dir, &fixture.uri(), ENTRY);
        client.definition_at(&fixture.uri(), ENTRY, "Set.mem α a A ->")
    };
    assert_eq!(
        cold, warm,
        "命中缓存之后 definition 必须给出**同一个**答案（T-A03 的整份报告回放）"
    );
}
