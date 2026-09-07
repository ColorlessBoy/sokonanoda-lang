# 设计：rename + find-references + inlay hints + `sokonanoda lsp`

> 状态：设计定稿（2026-09-07，本轮实施）。依据：`docs/gap-analysis.md` #6/#7
> 与附加小项；LSP 3.17 规范要点与 rust-analyzer/clangd/gleam 实践调研。

## 0. 头脑风暴与取舍

1. **防误改靠语义集**：rename 的改动集 = use→def 语义映射（`HoverType.
   resolution`）反向分组，**绝不扫文本**（rust-analyzer/clangd 共识；注释/
   字符串里的同名文本天然不受影响）。
2. **失败反馈用 ResponseError**：位置不可改名/新名非法 → JSON-RPC 错误
   （带中文 message），不返回空 edit（规范：null ≡ 「无事可做」）。
3. **inlay 只放「只读信息」**：洞的期望类型（InfoView 最小形态）。带
   textEdit 的可点击 inlay 性能昂贵且社区已放缓——不做。
4. **子命令分发**：`sokonanoda lsp` 单二进制形态（gleam 模式）：cli 依赖
   lsp lib；stdin 是 tty 时向 stderr 打人话提示；stdout 纪律 = LSP 协议
   帧，任何日志只进 stderr。

## 1. 前置：`ResolvedTarget` 携带名字子 span（主会话预接）

rename 必须编辑**名字 token**，而现有 target span 是整条命令/整个 binder：

```rust
pub enum ResolvedTarget {
    Binder { span: Span, name_span: Span },                       // span() 语义不变
    Declaration { name: String, span: Span, name_span: Span },
}
```

- `span()` 返回值不变（goto-def / highlight 现有断言不动）；
- 记录点：check.rs 记录 resolution 时一并记 name 子 span；声明名定位用
  `front::references::decl_name_span`（tokenize 命令切片找关键字后的 Ident，
  语义精确，不靠字符串扫描）。

## 2. find-references（front + LSP）

front 新模块 `crates/front/src/references.rs`：

```rust
pub fn decl_name_span(command_src: &str, name: &str) -> Option<Span>;
pub fn references_for(hovers: &[HoverType], target: &ResolvedTarget) -> Vec<Span>; // use 点，按 offset 升序
```

LSP（`crates/lsp/src/render.rs` 扩展 + main.rs 预接桩）：

- capability：`references_provider: Some(OneOf::Left(true))`；
- `textDocument/references`：光标处解析 target（use 点→其 def；def 点→自身，
  复用 highlight_uses 的双向逻辑）；`include_declaration` 时把**定义名
  Location**（name_span）插在最前；结果按 (offset) 排序、去重；
- 返回 `Some(Vec<Location>)`，无解析 → `Ok(None)`。

## 3. rename（LSP）

- capability：`rename_provider: Some(RenameOptions { prepare_provider:
  Some(true), .. })`；
- `textDocument/prepareRename`：解析 target →
  `Some(PrepareRenameResponse::Placeholder { range: name_span 的 0-based
  Range, placeholder: 原名 })`；prelude 名/未解析 → `Ok(None)`；
- `textDocument/rename`：
  1. **新名合法性**：`front::tokenize(new_name)` 必须产出恰好一个 Ident 且
     覆盖全串；否则 `Err(jsonrpc::Error { message: "「{new_name}」不是合法
     标识符：只允许字母/数字/_/非 ASCII，且不能以数字开头" })`；
  2. 改动集 = `references_for(target)` 全部 use span + 定义 name_span，
     每处 `TextEdit { range, new_text: new_name }`；
  3. `WorkspaceEdit.document_changes = [TextDocumentEdit { text_document:
     OptionalVersionedTextDocumentIdentifier { uri, version: doc.version },
     edits }]`（打开文档必带版本；主会话已给 `Doc` 加 `version` 字段）；
  4. 光标处无 target → ResponseError（「这里没有可以改名的名字」）。
- binder 与顶层声明同路径（target 统一抽象）；`example` 匿名声明不可 rename
  （无 name target，prepare 返回 None）。

## 4. inlay hints（LSP）

- capability：`inlay_hint_provider: Some(InlayHintOptions { resolve_provider:
  Some(false), .. })`；
- `textDocument/inlayHint`（`crates/lsp/src/inlay.rs` 新模块 + main.rs 预接桩）：
  对每个 `DeclStatus::Open` 声明的每个洞输出一条：
  - `position` = 洞 span 结束处（0-based，`???` 尾后）；`kind = TYPE`；
  - `label = ": <期望类型>"`——子洞取 `sub_goals` 中 span 相等项的 `ty`；
    单主洞（`holes.len() == 1` 且无匹配 sub_goal）取 `d.goal`；取不到类型
    不输出该条（不出现空 label）；
  - `tooltip` = markdown：剩余目标 + 已引入假设清单（与 hover 同数据）；
  - `padding_left: true`；
  - Checked/Failed 声明与 parse 失败 → 不输出（返回空数组）。
- 刷新：编辑后客户端按可视区重拉，无需服务器推送（不做 refresh 请求）。

## 5. `sokonanoda lsp` 子命令（单二进制，gleam 模式）

- `crates/lsp/Cargo.toml`：增加 `[lib] name = "sokonanoda_lsp"`（src/lib.rs
  承载全部现有 main.rs 内容：`pub mod actions/render/...`、`pub async fn
  run()` = 现 `main()` 体；`main.rs` 退化为 `fn main() { sokonanoda_lsp::run() }`；
  测试随 lib 迁移，`cargo test -p sokonanoda-lsp` 语义不变）；
- `crates/cli/Cargo.toml`：新增依赖 `sokonanoda-lsp = { path = "../lsp" }`；
- `crates/cli/src/main.rs`：`Some("lsp") => { tty 时 stderr 提示
  「lsp 由编辑器拉起：stdio 通道即将启动」; return sokonanoda_lsp::run(); }`
  （`--json` 与 lsp 互斥报错，同 repl 模式）；
- `help.rs` 用法文本补 `lsp` 行；`sokonanoda-lsp` 二进制保留（VS Code 端
  serverPath 不变）。

## 6. 验收

1. `cargo test -p sokonanoda-front`：references/decl_name_span 单测
   （use→def 反向、shadowing 内层胜出、跨声明）；
2. `cargo test -p sokonanoda-lsp`：references（含 include_declaration 两种）、
   prepare/rename（binder 与顶层、非法名 ResponseError、documentChanges 带
   version、注释里的同名文本不被改）、inlay（单洞/多洞/无洞）；
3. `cargo run -q -p sokonanoda-cli --bin sokonanoda -- lsp` 冒烟：stdin 非外
   层测试（不进 CI 的 e2e，只做 help 文本与编译）+ `sokonanoda --help` 含 lsp；
4. 全仓库门禁（fmt/clippy/test）绿。

## 7. 文件分工（互斥清单）

| owner | 允许修改 |
|---|---|
| 主会话（预接，先行完成） | `docs/protocol.md`、`crates/front/src/compile/{report,check}.rs`（ResolvedTarget 扩展 + name_span 记录）、`crates/front/src/lib.rs`、`crates/lsp/src/{lib.rs,main.rs}`（lib 化 + 能力 + 桩 + Doc.version）、两个 Cargo.toml、`crates/cli/src/{main.rs,help.rs}`、`crates/lsp/src/testutil.rs`（测试公共设施种子） |
| C（rename/references） | `crates/front/src/references.rs`（含测试）、`crates/lsp/src/render.rs`（references/prepare/rename 逻辑，含测试） |
| D（inlay hints） | `crates/lsp/src/inlay.rs`（新，含测试） |

冲突警戒：`crates/front/src/compile/check.rs` 的 resolution 记录点由主会话
一次性改完再冻结；subagent 阶段任何人不得再动它。
