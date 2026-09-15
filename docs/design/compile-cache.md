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
- 零新增运行时依赖给用户：`serde`/`serde_json` 早已在 CLI/LSP 里；用户仍零 cargo。

## 2. 键与文件

- key = 稳定 FNV-1a 64（`format | version | bare? | text`），十六进制；
  不用 `DefaultHasher`（跨版本不稳定）。
- 文件：`<cache_root>/compiled/<key>.json`，内容 `{format, report}`。
- `cache_root`：`SOKONANODA_CACHE_DIR` 覆盖；否则 macOS
  `~/Library/Caches/sokonanoda`、Windows `%LOCALAPPDATA%\sokonanoda`、其它
  `$XDG_CACHE_HOME|~/.cache` + `/sokonanoda`。`SOKONANODA_NO_CACHE=1` 关闭。
- 写：先写 `<key>.json.tmp-<pid>` 再 `rename`（读者不会看到半截文件）。

## 3. 接入点

`crates/lsp/src/lib.rs::Backend::refresh`（didOpen/didChange 共用）：

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

- `crates/lsp/src/cache.rs` 单测：key 稳定/敏感；store→load 往返 + key 作用域；
  `format` 不匹配 miss（`_in` 目录参数版，避免 env 竞态）。
- 既有 LSP 测试全部经 `refresh`，即覆盖接线不崩。
- `front` 报告类型加 `serde` derive（owned，无生命周期）。

## 6. v1 边界

- 不做跨文件/增量持久化（不缓存 Session 快照，只缓存整份报告）。
- 不做 LRU/容量上限（教学文件报告为 KB 级；如需后续再加）。
- 不做与内核 `.olean` 等价的「已编译环境导入」。
