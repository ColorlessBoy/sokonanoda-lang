# WO-013 LSP 单文件 parse 失败丢掉项目报告（G-20 / X15）

> 台账：`docs/gaps/ledger.jsonl` → `G-20`（`kind=tooling` · `severity=blocker` ·
> `status=fixed` · `fixed_in=0.61.0`）
> 复现件：`docs/gaps/repro/G20-lsp-drops-rescued-report.sh`（外壳）+
> `G20-lsp-drops-rescued-report.js`（真探针：node 写的 LSP over stdio JSON-RPC 客户端）
> 实测环境：本树 0.61.0（未发布）· `bash docs/gaps/repro/G20-lsp-drops-rescued-report.sh`

## 用户可见症状

课程单元（`import lib.Set` + 用库记法）在编辑器里：

- 收到**一条假诊断** `notation-unknown-symbol`（红波浪线），而同一份文本走 CLI
  `scripts/soko grade` 是 **exit 0**；
- `textDocument/hover`、`documentSymbol`、`codeAction`、`inlayHint` **全部回答 `null`**
  ——不是"降级"，是"消失"。

⇒ 用户要求的「hover 内容提示用户如何输入对应符号」（D5）**正好死在最需要它的那批
文件里**。

## 根因（两半互相矛盾）

| 层 | 代码 | 行为 |
|---|---|---|
| front | `project/mod.rs` 的 `is_project_source` | **专门**为这种情况留了退路：入口单独 parse 失败但写了 `import` ⇒ 走闭包编译 |
| front | `query/mod.rs::set_text_with_overlay` | 闭包报告**确实装好了**（`self.report = Some(report)`） |
| lsp | `crates/lsp/src/lib.rs::Doc::set_text` | 紧接着因 `parse_error.is_some()` 把 `report = None` **丢掉** |
| lsp | `Doc::diagnostics` | 又让 parse 错误**优先于**报告 |

记法随 `import` 传播（G-04 第二刀）之后，「入口单独 parse 失败」是**常态**而不是错误。

## 修法（已落，0.61.0）

1. `QueryDoc` 新增 `project_entry_compiled()`：入口模块状态 `== ModuleStatus::Compiled`
   才算"闭包救回来了"（入口真有语法错误时是 `LoadFailed` ⇒ `false`）。
2. `Doc::set_text`：只在 `project_entry_compiled() == false` 时维持老契约（`report = None`）。
3. `Doc::diagnostics`：闭包救回来时以**闭包报告**为准；否则维持 parse 错误优先。

## 回归测试（两层，都在 `crates/lsp/src/tests/project.rs`）

| 测试 | 钉住 |
|---|---|
| `imported_notation_keeps_the_report_and_the_diagnostics_honest` | 0 诊断 + hover 非 `null` + `documentSymbol` 非空 + hover 文案含 `\in`（D5 的输入提示对 **import 来的**符号也生效） |
| `a_genuinely_broken_entry_still_reports_the_parse_error` | 闭包**也**失败时仍发诊断（契约不许被放宽） |

复现脚本的退出码：**0 = 缺口仍在**（假诊断 / hover 为 `null`）· **1 = 已修** ·
**2 = 环境异常**。实测：修前 exit 0，修后 exit 1。

## 顺带修掉的测试基建 bug

`crates/lsp/src/testutil.rs` 的 `lsp_pos` 按**字节差**算 LSP `character`，而
`position_to_offset`（生产代码）按**字符数**解释它 ⇒ 任何含多字节符号的行都会偏
（实测：想 hover `⊗`，光标落到隔壁的 `b` 上）。ASCII 夹具上两种算法恒等，所以这个
bug 一直没显形。已改成按字符数计算，并把文档注释写清"`offset` 是字节、`character`
是字符"。

## 验收（同轮实测）

- `cargo test -p sokonanoda-lsp --locked` **145 passed / 0 failed**；
- `python3 scripts/gap.py check` **exit 0**（G-20 判"行为已变"= 已修）；
- 课程门禁 **36 目标 · 329 checked · 99 open · 0 判负**、`--selftest` exit 0；
- `git diff --stat -- crates/kernel/` **空**。
