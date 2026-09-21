# 缓存键里的 `build_stamp`（可执行文件 mtime）—— 实验记录

> 对应计划 **T-009**（`docs/design/vscode-editor-feedback-plan.md` §3），
> 缺口 **G-27**，修法在 **T-A02**。日期 2026-09-21，仓库 `0.63.0`，Apple Silicon。

## 结论（一句话）

**是**。编译缓存的键里折进了 `current_exe()` 的**秒级 mtime**，而 CLI
（`sokonanoda`）与 LSP（`sokonanoda-lsp`）是**两个不同的可执行文件** ⇒
`sokonanoda build` 预热出来的条目，**基本上不可能被编辑器里的 LSP 命中**。

本机四组实测里**三组不同秒**：

| 位置 | `sokonanoda` 的 mtime | `sokonanoda-lsp` 的 mtime | 同秒？ |
|---|---|---|---|
| `target/release/`（仓库构建） | 1789966247（12:50:47） | 1789966866（13:01:06） | **否**，差 619s |
| `target/debug/`（开发构建） | 1789966358（12:52:38） | 1789854199（09-20 05:43） | **否**，差一天 |
| `~/.local/share/sokonanoda/bin/`（启动器下载缓存） | 1789605306 | 1789605308 | **否**，差 2s |
| `editor/vscode/bin/darwin-arm64/`（扩展 staged） | 1789966866 | 1789966866 | **是**（同一次 `stage-lsp.js` 的巧合） |

最后一行是**唯一**同秒的一组，而它是"同一次 `cargo build` + 同一次 stage"的产物
——**不是设计保证**：release 走 upload/download-artifact + 两份独立 tarball，
VSIX 里的 LSP 是**解包时刻**的 mtime，CLI 若是启动器另外下载的又是另一个时刻。

## 机制

`crates/front/src/compile/cache.rs`：

```rust
pub const CACHE_FORMAT: u32 = 2;

/// Cheap build fingerprint: mtime (secs) of `std::env::current_exe()`, else 0.
pub fn build_stamp() -> u64 { /* current_exe() 的 modified() 取 as_secs() */ }

pub fn key(src: &str, options: &CompileOptions) -> String {
    key_with_build(src, options, build_stamp())      // ← 折进键
}

fn key_parts(format, version, build, prelude_bare, src) { /* FNV-1a 64 */ }
```

原意（`cache.rs:67-70` 的注释）是"开发期同一个版本号重建二进制时，不要复用陈旧报告"。
这个意图**是对的**，但用 mtime 实现它有两个问题：

1. **它不是"这份二进制是什么"，而是"这个文件是什么时候被写到磁盘上的"**——
   `cp`/`tar -x`/下载/`install` 都会改它，内容一个字节没变也照样变。
2. **CLI 与 LSP 是两个文件**，于是"谁预热的"决定了"谁命中不了"。

## 机械复现（缺口即测试）

```bash
bash docs/gaps/repro/G27-cache-key-folds-binary-mtime.sh ; echo "exit=$?"
# 期望（修前）：exit=0 —— 缺口仍在
```

脚本做的事：把**同一份二进制内容**拷成两个副本，只把 mtime 设成
2001 与 2023（`os.utime`），用**同一个** `SOKONANODA_CACHE_DIR` 跑两次
`sokonanoda build`：

```
mtime=2001 的副本（预热）= 0 hit
mtime=2023 的副本（再跑）= 0 hit, 1 compiled      ← 内容完全相同，仍然 miss
```

## 影响面

- **`sokonanoda build` / `alt+b` 预热对编辑器无效**（项目文件与单文件都一样：
  键的这一段与文件类型无关）。
- 反过来说，**编辑器自己跑热的条目，CLI 也命中不了**——`sokonanoda build` 报
  `hit` 与否，跟编辑器里快不快没有必然联系。
- 这条是 **G-25 的第二层**：G-25（LSP 对有 `import` 的文档根本不读缓存）修好之后，
  这一层才会暴露出来；不修它，G-25 的收益会被 mtime 吃掉。

## 修法（T-A02）

把 `build_stamp()` 换成**编译期常量**——它才真正表达"这份二进制是什么"：

- `env!("CARGO_PKG_VERSION")`（已有）
- `env!("PROFILE")` / `cfg!(debug_assertions)`（debug 与 release 不串台）
- 目标三元组（跨平台不串台）
- 可选：`option_env!("SOKO_BUILD_LABEL")`（开发期想区分同版本的不同构建时，
  由**显式环境变量**给标签，而不是靠文件系统的副作用）

`CACHE_FORMAT` 要 bump（键的构成变了，旧条目一律作废）。

**验收**：G-27 的复现转绿（同一份二进制内容、不同 mtime ⇒ 仍然 `1 hit`），
且 `cargo test -p sokonanoda-front cache::` 的既有断言全绿。

**注意**：改完之后，"同一版本号但代码变了"的开发期场景要靠 `SOKO_BUILD_LABEL`
或版本号来区分——**不能靠 mtime**。这一点要写进 `docs/design/compile-cache.md`。
