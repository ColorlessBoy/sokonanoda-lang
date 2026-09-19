# 设计：编译结果落盘缓存（olean 式，2026-09-15）

> 触发（用户）：「想办法设计构建类似 lean4 的编译结果文件，要不然后面文件多了，
> 打开插件现场编译会很慢。」目标：已编译且内容未变的 `.sokonanoda` 文档再次打开
> 时**不重编译**，直接复用内核上次产出的报告。

## 1. 原则（不破坏硬规则）

- **内核仍是唯一判定者**：缓存条目只在「同一 (编译器版本, prelude 模式, 源文本)」
  下，由内核真实产出的 `DocumentReport` 写入；命中只是**省掉重复劳动**，不从缓存
  **推断**任何结论。
- **失效即重编**：key 里含 `CARGO_PKG_VERSION`（换二进制必 miss）与
  `CACHE_FORMAT`（报告 schema 变必 miss）；源文本变则 key 变。
- **best-effort**：读写失败绝不使请求失败（缓存是优化，不是依赖）。
- 零新增运行时依赖给用户：`serde`/`serde_json` 在 workspace 内已被使用；用户仍零 cargo。
- **归属（0.49.0）**：缓存模块在 **`crates/front/src/compile/cache.rs`**（不是 LSP 内部），
  LSP 与 CLI **共用**同一份；条目同时容纳 `report` 与 `output`。

## 2. 键与文件

- key = 稳定 FNV-1a 64（`format | version | bare? | text`），十六进制；
  不用 `DefaultHasher`（跨版本不稳定）。
- 文件：`<cache_root>/compiled/<key>.json`，内容 `{format, report}`。
- `cache_root`：`SOKONANODA_CACHE_DIR` 覆盖；否则 macOS
  `~/Library/Caches/sokonanoda`、Windows `%LOCALAPPDATA%\sokonanoda`、其它
  `$XDG_CACHE_HOME|~/.cache` + `/sokonanoda`。`SOKONANODA_NO_CACHE=1` 关闭。
- 写：先写 `<key>.json.tmp-<pid>` 再 `rename`（读者不会看到半截文件）。

## 3. 接入点

**LSP**：`crates/lsp/src/lib.rs::Backend::refresh`（didOpen/didChange 共用；`cfg!(test)` 时
不碰真实缓存）：

```
key = cache::key(VERSION, mode==Bare, text)
if let Some(report) = cache::load(&key):
    诊断 = report_diagnostics(&report)   # 与重编译同一构造
    doc.report = report; 跳过 session.update
else:
    update = session.update(text)        # 原路径
    成功 → cache::store(&key, &report)
```

诊断构造抽成 `report_diagnostics(&DocumentReport)`，缓存命中与重编译**产出完全一致**
（errors + 每个 `sorry` 一条 WARNING + 语法 warning）。

## 4. 语义注意

- 命中时**不**推进 `Session` 的增量快照：随后第一次编辑会退化为全量编译（正确、
  只是少了增量收益）。这是有意的：缓存服务于「打开」，增量服务于「编辑」。
- 同文件内容回退（撤销）也会命中——纯收益。
- 缓存只存报告（诊断/hover/目标态的数据源），不存内核环境；无跨文件依赖
  （教学文件自给自足，无 import），故 key 只需文件自身。

## 5. 测试

**CLI**（0.49.0）：`crates/cli/src/build.rs::build`、`check.rs::compile_cached`；
批量 `--json` 走 `compile_cached`。**无 `import` 的** `course` 单元同样走
`compile_cached`（键不变）；**有 `import` 的**单元（WO-007 起）走闭包分支——
`project_cache::plan/load` + `compile_plan`，与 `check`/`build`/`query` 共用
`ProjectPlan::digest` 摘要键（`project_cache.rs`），于是 `course` 冷跑一次之后
`build` 报 `hit`（`crates/cli/tests/course_project.rs` 钉住）。`sokonanoda --json` 冷/热
两次输出**逐字节一致**（有测试）。

```
sokonanoda build [--json] [--clean] [<file> | <dir> ...]
```
- 默认扫当前目录；目录递归收集 `*.sokonanoda`；
- 人类输出 `built K file(s) — H hit, M compiled, F failed`；`--clean` 打印删除数；
- `--json`：每文件 `{"type":"build.file",…}` + 末尾 `{"type":"build.summary",…}`；
  `build --clean --json` → `{"type":"build.clean","removed":N}`。

- 单测（`crates/front/src/compile/cache.rs`）：key 稳定/敏感；store→load 往返 + key 作用域；
  `format` 不匹配 miss；`clean` 删除；用 `_in(dir,…)` 目录参数版，避免 env 竞态。
- CLI 测试用**每进程临时 `SOKONANODA_CACHE_DIR`** 隔离：`cli_build_warms_and_reuses_cache`、
  `cli_build_clean_removes_entries`、`cli_course_is_stable_with_a_warm_cache`。
- 既有 LSP 测试全部经 `refresh`，即覆盖接线不崩。
- `front` 报告类型加 `serde` derive（owned，无生命周期）。

## 6. v1 边界

- 不做跨文件/增量持久化（不缓存 Session 快照，只缓存整份报告）。
- 不做 LRU/容量上限（教学文件报告为 KB 级；如需后续再加）。
- 不做与内核 `.olean` 等价的「已编译环境导入」。

---

## 7. 多文件闭包键（I16 P4，2026-09-17）

有 `import` 的文件，编译单元是**整个项目闭包**（`docs/design/imports-and-projects.md`），
所以键从"本文件源文本"升级为**闭包摘要**：

```
digest = FNV-1a64( "soko.project-iface/1",
                   prelude 模式,
                   for module in 拓扑序 { module 名 \0 module 源文本 \0 各 import 名 } )
cache::key(digest, options)   # 仍然复用同一个 format|version|build|bare 前缀
```

- **依赖变了必然 miss**：摘要按拓扑序含每个模块的源文本，改一行库文件 ⇒ 入口的
  摘要变 ⇒ 重编译（`crates/cli/tests/imports.rs` 有判别性测试：改依赖后 `#reduce`
  必须给出新值）。
- **单文件仍是原来的键**：没有 `import` 的文件走原路径，键与今天逐字节相同
  （既有 warm-cache 测试不受影响）。
- **只缓存"完全干净"的项目**（`ProjectReport::is_clean`）：条目里只有**入口**的
  报告与事件，带诊断的项目回放不出依赖模块的诊断，而冷跑/热跑必须逐字节一致。
- **不做**：deps 级 decl 产物（信任台账，见设计 §4.8 的可选项及其"先换强哈希"前置）。
