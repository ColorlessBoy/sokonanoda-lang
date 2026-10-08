# `import` 行的模块名可点（documentLink）—— 设计

> 用户 2026-10-08：「**import 这一行的代码增加跳转功能，打开对应的文件**」。
> 本文只写**契约**（过程进 commit message 与 `STATUS.md`）✓。

## 1 交付什么

编辑器里 `import lib.Set` 的**模块名**（`lib.Set`）变成一条 **LSP DocumentLink**：
悬停显示下划线 + tooltip，点开就是被导入的那个文件（`…/lib/Set.sokonanoda`）。

* 用 **`textDocument/documentLink`**（而不是 `definition`）—— 这是 LSP 为"点开文件"
  准备的那条路 ✓，VS Code 会**自动**把它渲染成可点链接（扩展侧**零改动**：
  `vscode-languageclient` 默认注册 `DocumentLinkFeature`，见
  `editor/vscode/node_modules/vscode-languageclient/lib/common/client.js` ✓）。
* **范围只到模块名**，不是整行 ✓（整行可点会让"点注释/点 `import` 关键字"也跳）。
* tooltip：`打开模块 <模块名>`（点下去会发生什么，先说清楚 ✓）。

## 2 真相层出口（唯一实现）

模块名 → 文件路径由**闭包模块表**回答（`QueryDoc::module_path`，它同时被跨文件
definition / references / rename 用着 ✓）；`import` 行的**位置**由

```rust
sokonanoda_front::query::import_lines(text) -> Vec<(String, Span)>   // span 覆盖模块名
```

给出 —— 它是**词法层** `token::scan_import_lines` 的薄出口（**闭包加载器用的就是它**，
`crates/front/src/project/graph.rs`）⇒ 不重算、不重跑内核、不产生第二份真相 ✓。

**为什么是词法而不是 AST** ✗✓：入口文件常用 import 带来的记法，**用户打字中**
`crate::parse` 会失败（`notation-unknown-symbol`）⇒ 走 AST 的话链接会**整片消失** ✗。
词法扫描对"打字中"稳健 ✓，而 `import` 的形状本来就是**行首关键字 + 点分名**（G-04
第二刀 §10.3 的判据与词法器 `on_import_line` 同款 ✓）。

**读哪份文本**：`Docs::latest_text()`（用户缓冲区），与语义 token 同一条纪律 ✓ ——
`text()` 是"上一次编译用的"那份，打字时会落后 ✗。

## 3 边界（都是**有意的**，不是漏做）

| 情形 | 行为 | 为什么 |
|---|---|---|
| 单文件 / 没有 `import` | `Some(vec![])` | 空数组合法；报错会让编辑器弹面板 ✗ |
| `import-not-found`（模块不在闭包里） | **不给链接** | 点开一个不存在的文件比"点不动"更糟；诊断已经在说原因 ✓ |
| 模块根 / 缺失模块的**推测路径** | **不拼** | 那会产生第二份真相（路径由模块表说了算 ✓）；要"能创建"的链接是另一个产品决定 |
| `definition`（F12）走 import 行 | **不做** | 用户要的是"点开文件" ⇒ DocumentLink 就是那条路 ✓；多接一条 definition 分支 = 第二条跳转路径 ✗ |

## 4 判据（`scripts/soko gate` / CI 里跑的那几条）

1. **LSP 单测**（`crates/lsp/src/tests/project.rs`）：真两文件项目 ⇒
   `textDocument/documentLink` 的 `links[0].target` == `lib/Set.sokonanoda` 的 URI、
   `range` **正好盖住 `lib.Set`**（起止列逐字断言 ✓）；
2. **能力断言**（`crates/lsp/src/testutil.rs`）：`initialize` 必须声明
   `documentLinkProvider`（没有它，客户端根本不会问 ⇒ 链接永远不出现 ✗）；
3. **反向判据**：单文件（无 `import`）⇒ `Some([])`（**不是** `None`、**不是**错误 ✓）；
   `import-not-found` ⇒ 该条**没有**链接（缺文件时不许给死链 ✗）；
4. **协议**：`docs/protocol.md` 的 LSP 一节登记这条方法（`docs/protocol.md` §Rename/
   references/inlay hints 那节旁 ✓）。

## 5 不做（明确记下，免得下一轮重新讨论）

* 不给 `import` 行做 **F12 / rename / references**；
* 不解析**缺失模块**的路径、不在链接里创建文件；
* 不动 `import` 的**语义**（记法继承、闭包加载顺序都不变 ✓ —— 本条只读不写）。
