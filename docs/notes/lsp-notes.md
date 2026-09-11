# LSP Server（Rust）实践调研笔记（2025–2026）

> 调研日期：2026-09-06。目的：为 `crates/lsp`（tower-lsp）与 I8（真增量）/ I9（goal 视图）
> 路线图提供外部参照。本文先给 TL;DR 建议，再分主题展开，末尾附可直接抄的代码模式。
> 阅读顺序建议：`STATUS.md` → 本文件 → `docs/design/infrastructure.md` F1–F8。

---

## 0. TL;DR：对我们项目的建议

### 现在就改（crates/lsp，小改动）

1. **diagnostics 带版本号**：`publish_diagnostics(uri, diags, Some(version))`，
   version 取自 `didOpen`/`didChange` 参数里的 `text_document.version`（现在是 `None`，
   见 `main.rs:72`）。这是防"旧诊断盖住新文本"的第一道闸，3.15 起就是标准做法。
2. **didChange 取最后一段全文**：FULL 同步时用 `content_changes.last()` 而不是
   `into_iter().next()`（现在是取第一个，`main.rs:202`）。规范允许多个 change，
   最后一项才是最终文本。
3. **版本守卫（last-write-wins）**：`Doc` 存 `version: i32`，`refresh` 前先比版本，
   旧的通知直接丢弃。tower-lsp 对 notification 的并发处理有已知乱序争议
   （[tower-lsp#284]，async-lsp README 引用），版本守卫是最便宜的保险。
4. **didSave 去掉"要文本"**：FULL 同步下 `didChange` 已带来最新文本，`did_save`
   不必再触发重编译（或只做 no-op）。省一次重复编译。
5. **编译防抖 + 只认最新版**：把 `refresh` 里的同步编译改成"spawn 任务 + 发布前
   校验版本仍是最新"，连打字时不会排队堆积整文件重编译（模式见附录 A.2）。
6. **同步策略维持 FULL**：`.sokonanoda` 是小课程文件，FULL 简单且无 UTF-16 偏移坑；
   增量同步的复杂度先不买（见 §3）。

### 架构不变，为 I8/I9 留缝

- **框架留在 tower-lsp**：上游已不活跃（2024-08 后无更新），但 v0.20 稳定、API 与
  社区分叉 tower-lsp-server 几乎完全兼容，真要迁是"改一行依赖"级别的事
  （见 §1）。不必现在迁。
- **文档服务继续与框架解耦**：目前 `sokonanoda_front::compile::check_document` 已经是
  纯库 API，LSP 只是壳——这个分层是对的，I8 的"声明粒度 check-then-add"也应落在
  front 层，LSP 层只传版本号与增量范围。
- **I8 增量**：对齐 Lean 4 的"每条顶层命令一个 snapshot"模型（§5）：按声明边界
  切快照，编辑只重算受影响后缀，诊断随处理增量发布；先做"后缀重查"（design doc
  §5.3 已规划），依赖图裁剪后置。**不要引入 salsa**（见 §2.3，当前规模是杀鸡用牛刀）。
- **I9 goal 视图**：近期用 hover+code action 承载（已有 v1）；若要做到 Lean InfoView
  那种交互面板，参照 Lean 的做法加一条自定义 RPC 通道（tower-lsp 的
  `LspService::build().custom_method()` 就够），而不是塞进 LSP 标准能力（§5.3）。

---

## 1. 框架选型：tower-lsp / tower-lsp-server / async-lsp / lsp-server

| 框架 | 状态（2025–2026） | 并发模型 | 适合谁 |
|---|---|---|---|
| tower-lsp | 上游 2024-08 后无 push，最后 release v0.20.0（2023-08） | Service + 异步 handler，`&self` 共享状态 | 大量存量项目（Deno、Turborepo 等） |
| tower-lsp-server | tower-lsp 社区分叉，活跃（2025-12 仍在更新，0.23），Biome/Oxc/Harper/Polarity 在用 | 同 tower-lsp | 想要 tower-lsp API 但要维护性的新项目 |
| async-lsp | 活跃（oxalica） | tower `Layer` 组合中间件；notification 同步顺序执行；notification 用 `&mut self`，request 用 `&self` 并发 | 想精细控制并发/中间件、也要写 client 的项目 |
| lsp-server | rust-lang 官方小 crate，rust-analyzer 在用 | 同步主循环 + 自己管理线程池/任务，无 async 框架 | 想完全掌控事件循环的项目（rust-analyzer 级别） |

要点（引用见各链接）：

- **tower-lsp 不再维护**是社区共识，活跃分叉是
  [tower-lsp-server](https://github.com/tower-lsp-community/tower-lsp-server)（2025 年初
  从 0.23 起步，70+ dependents）与 [async-lsp](https://github.com/oxalica/async-lsp/)；
  2026 年的教程也直接建议用分叉（[codeinput.com 博客](https://codeinput.com/blog/lsp-server)）。
- **tower-lsp 的已知语义问题**：notification 被异步并发处理，可能乱序/交叠
  （[tower-lsp#284]）；async-lsp 用"notification 同步顺序执行 + request 并发"来规避
  （[async-lsp README 的对比](https://github.com/oxalica/async-lsp/)）。
  我们的对策不是换框架，而是：状态变更（didChange）带版本号做 last-write-wins，
  读请求（hover 等）永远读"最新一份 report"。这把乱序问题从"正确性"降级为"性能"。
- **lsp-server**（[crates.io](https://crates.io/crates/lsp-server)）只是
  `Connection`（stdio/crossbeam channel）+ 手写 dispatch。rust-analyzer 的主循环
  （[main_loop.rs](https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/main_loop.rs)）
  全部自己写：单线程主循环拥有全部可变状态（`GlobalState`），只读请求拿 snapshot 丢到
  线程池，响应走 channel 回主循环。**对我们不划算**：控制力用不上，代码量翻几倍。
- **请求乱序与取消是 LSP 的一等公民**：慢的 hover 不能阻塞诊断，客户端随时可能
  `$/cancelRequest`。tower-lsp 对 pending request 的取消是内建的
  （[LspService 文档](https://docs.rs/tower-lsp/latest/tower_lsp/struct.LspService.html)），
  handler 侧会收到 `RequestCancelled` 风格的中止（futures 被直接 drop）。

**结论**：教学型小服务器留在 tower-lsp（或直接换 tower-lsp-server）最划算；
把"文档服务"放框架外，框架随时可换。

---

## 2. 并发架构与状态共享

### 2.1 为什么必须乱序、并发地处理请求

LSP 客户端（VS Code）在用户打字时会连发 `didChange`，同时 UI 又会发 hover/symbol
等请求；`$/cancelRequest` 允许客户端在文档状态变化后放弃过时的请求
（[3.17 规范·Cancellation Support](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)）。
被取消的请求**仍要回一个响应**（不能挂着），取消时建议返回 `RequestCancelled` 错误码。
若服务器串行处理，一次慢检查（我们将来 kernel 变慢、多文件时必然发生）会冻结整个
编辑器反馈。rust-analyzer 在 issue #7444 里把并发活动列成清单（LSP 消息、后台任务、
VFS、cargo check…）并权衡了三种模型：actor 邮箱（现用）、闭包消息、全局 Mutex
（[rust-analyzer#7444]）。

### 2.2 常见状态共享模式（document store）

绝大多数 Rust 服务器是这两种之一：

- **单锁文档表**：`RwLock<HashMap<Url, DocumentState>>`，写通知（didOpen/didChange）
  拿写锁替换整份状态；读请求拿读锁 snapshot 后立刻释放，慢活（类型检查）在快照上做。
  我们现在的单文档 `Mutex<Doc>` 就是它的退化版，方向正确。
- **主循环拥有状态 + 任务池**（rust-analyzer 模式）：主线程是唯一写者，读请求
  clone 出不可变 `GlobalStateSnapshot` 丢线程池
  （[官方 architecture.md](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/architecture.md)：
  "the server is stateless, a-la HTTP"，需要跨请求状态时让第二个请求带足参数重建上下文）。

注意共同纪律：**锁/快照绝不跨 `.await` 持有**；编译产物（report）整体不可变，
替换即发布。我们现有代码遵守了这条（先算完再一次性换 `Doc`），保持即可。

### 2.3 rust-analyzer 的 salsa：两段话 + 适用性判断

salsa 是"需求驱动的增量计算框架"：你把程序写成一组查询（`K -> V`），分 inputs
（可随时改）与 tracked functions（纯函数，结果被记忆化）；输入变更后框架用 red-green
算法决定哪些查询要重跑，能通过"输出相等"把脏传播截止在更早一步（backdating），
还能给输入标 durability（stdlib=HIGH，工作区=LOW）跳过不必要的脏检查
（[How Salsa works](https://salsa-rs.github.io/salsa/how_salsa_works.html)、
[Algorithm](https://salsa-rs.github.io/salsa/reference/algorithm.html)）。
rust-analyzer 用它把分析管线分层成 `SourceDatabase → DefDatabase → HirDatabase`，
并把 `ItemTree`（签名结构）设计成脏传播的屏障：函数体内编辑只会重算该函数的
`body`/`infer` 查询，签名不变则名字解析全部缓存命中（[deepwiki 概览]）。

**对小教学编译器是否过度？** 现在过度。salsa 的成本在于：所有中间值要放进数据库
（tracked/interned 结构 + 生命周期），等于把编译器数据结构重写一遍。我们 I8 的
"声明粒度 check-then-add + 后缀重查"（design doc §5.3）是同一思想的手工实现，
粒度恰好（声明 = 我们的最小编译单元），代码量小一个数量级。触发引入 salsa 的条件
是：多文件项目、需要交叉文件失效、或 agent 高频连续分析——到那时再评估。

---

## 3. 文本同步：FULL vs INCREMENTAL

- `TextDocumentSyncKind::FULL`：每次 `didChange` 带全文；`INCREMENTAL`：带 range + 新文本。
  [3.17 规范](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)。
- **FULL 什么时候可接受**：文件小（< 几百 KB）、无逐字符增量解析需求时，FULL 完全够，
  很多生产服务器就这么干（对比见
  [EdgeCrab ADR-003](https://github.com/raphaelmansuy/edgecrab/blob/main/specs/lsp-and-code/ADR-003-document-sync-strategy.md)）。
  课程文件是几十行的小文件，**FULL 是正确选择**；"省流量"收益远小于引入的坑。
- **INCREMENTAL 的坑**：
  - 位置默认按 **UTF-16 code unit** 计数，不是字节也不是 char；emoji/生僻字（> U+FFFF）
    占 2 个 unit，算错会导致"编辑后的所有诊断位置整体漂移"。3.17 起可用
    `general.positionEncodings` 协商 UTF-8/UTF-32，但 VS Code 实际只认 UTF-16
    （[规范·positionEncoding](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)；
    [michaelpj 吐槽文](https://www.michaelpj.com/blog/2024/09/03/lsp-good-bad-ugly.html)）。
  - 多个 change 要**按序**应用，每个都基于前一个的结果。
  - `rangeLength` 不可信（历史上单位含糊），`range` 才是权威；
    客户端版本号递增方式各客户端不一，服务端很难做强校验
    （[LSP#1706](https://github.com/microsoft/language-server-protocol/issues/1706)）。
  - 实现参考：把 range 换算成 UTF-16 偏移、在缓冲区里 splice（附录 A.3）。
- **didSave**：`TextDocumentSyncOptions.save` 为 `true` 时保存事件会带全文、
  为 `"includeText": false`（或直接 sync kind 缺省）时不带。FULL 同步下我们已有最新
  文本，save 事件无需再触发编译。
- **防抖**：客户端本身会按击键批次合并 didChange，服务端要防的是"慢编译排队"：
  收到新版本时，若旧编译还在跑，标记作废、只启动最新版本的任务（附录 A.2）。
- **取消**：`$/cancelRequest` 取消的是"请求"（hover 等），不是 notification；
  编译任务的作废要靠我们自己的版本守卫（见 §0 第 3、5 条）。

---

## 4. publishDiagnostics 最佳实践

- **version 字段**（3.15+）：`PublishDiagnosticsParams.version` 应填对应文档版本，
  客户端据此丢弃过期诊断（[规范 3.17]；typescript-language-server 直到 2025 年还在
  补这个字段，见其 [issue #983]）。我们当前传 `None`，是首要修复项。
- **发布空集也是发布**：错误修掉后必须发 `diagnostics: []` 清除波浪线
  （我们每次全量发布，已满足）。
- **每条 Diagnostic**：始终填 `severity`（规范强烈建议）、`source: "sokonanoda"`、
  稳定 `code`（我们已做）；`data` 字段可在 publish 与后续 `codeAction` 间携带
  结构化上下文——**I9 的 goal 视图可以把它当免费通道**（把"声明 id + 洞位置"塞进
  code action 相关诊断的 `data`，客户端回来请求 action 时还原上下文）。
- **push vs pull（3.17 `textDocument/diagnostic`）**：pull 让客户端（它知道哪个文件
  可见/在编辑）主动拉诊断，服务端还能用 `previousResultId` 做"未变化就别重算"。
  但规范明确：**一旦声明了 pull，就不要对同一批资源再 push 同一套诊断**，两套是
  独立管理、会互相覆盖的流（[LSP#1743 讨论](https://github.com/microsoft/language-server-protocol/issues/1743)）。
  我们是单一诊断源、客户端少（自家 VS Code 壳 + agent），push 足够；除非将来要
  多面板/多客户端，才考虑 pull。
- **Lean 的扩展教训**：Lean 用 `isIncremental` 让"处理到一半"的诊断也能增量追加
  （append 语义），配合 `$/lean/fileProgress` 让用户看到实时进度（§5）。对我们的
  启示：I8 做"后缀重查"时，可以逐声明完成就先发布部分诊断（版本号不变，集合替换）
  ——不必等整文件检查完。

---

## 5. Lean 4 自己的 LSP（src/Lean/Server）：可借鉴的点

Lean 4 的 LSP 是 Rust 写的（在 lean4 仓库，但主体是 Lean 代码 + watchdog/worker 结构）：

- **watchdog + 每文件 worker 进程**：watchdog 管生命周期/协调与客户端通信，每个打开
  的文件一个 worker（[Server/README.md](https://github.com/leanprover/lean4/blob/master/src/Lean/Server/README.md)）。
  对我们现阶段是过度设计（单文件课程），但"**每文件独立状态**"的边界值得记下。
- **快照树（SnapshotTree）= 增量的核心**：语言处理器把文件处理成"按顶层命令组织的
  异步快照树"，每次编辑后从上一个版本的树**复用未受影响的快照**、重算受影响前缀/
  后缀；worker 异步遍历树、**边处理边增量发布诊断与进度**
  （README "Worker architecture" 节；
  [Language/Basic.lean](https://github.com/leanprover/lean4/blob/master/src/Lean/Language/Basic.lean)）。
  这就是我们 I8 目标形态的官方实现：**声明 = 快照 = 增量与诊断的最小单位**。
- **InfoTree = 类型/目标图**：elaboration 期间把"目标、子项类型、局部上下文"挂进
  InfoTree；请求处理用 `withWaitFindSnap` 找到光标所在快照（必要时等 elaboration
  推进），再用 `smallestInfo?`/`goalsAt?` 查位置。我们 I3 的"类型图 + hover"就是
  它的简化版；I9 的 goal 视图（悬停洞 → 目标 + 上下文）对应的正是
  `Lean.Widget.getInteractiveGoals` / `getInteractiveTermGoal`。
- **InfoView/widgets 走自定义 RPC，不塞标准 LSP**：Lean 在 LSP 之上加了
  `$/lean/rpc/connect|call|release` 通道，InfoView 面板通过
  `Lean.Widget.getInteractiveGoals` 等 RPC 拿结构化数据、客户端渲染交互组件
  （[ProtocolOverview.lean](https://github.com/leanprover/lean4/blob/master/src/Lean/Server/ProtocolOverview.lean)、
  [WidgetRequests.lean](https://github.com/leanprover/lean4/blob/master/src/Lean/Server/FileWorker/WidgetRequests.lean)）。
  **对我们的 I9**：第一版 goal 视图继续用 hover（零新协议）；如果要做成 Lean 那样的
  面板（可点击、可展开），学它：加一条 `$/soko/rpc/call` 自定义方法
  （tower-lsp `custom_method`），VS Code 壳负责渲染。
- **交互诊断的身份保持**：Lean 特意复用未变化的诊断对象，否则客户端会重载视图、
  丢掉用户展开的 trace 树（README "Communication" 节）。我们暂时没有可交互诊断，
  但做 hint 面板/code action 时记住：**内容没变的诊断别重新生成新对象**。
- **输出单线程化**：所有 worker 的 stdout 写入经单一专用线程，避免多版本文档的
  消息交错（README）。tower-lsp 的 `Client` 已为我们做了序列化。

---

## 6. 测试

- **tower-lsp 内存内直测（推荐给我们的单测）**：`LspService::new` 得到
  `(service, socket)`；用 tower 的 `ServiceExt::ready().call(request)` 发请求，
  `socket` 作为 `Stream/Sink` 收发 server→client 消息（可断言 publishDiagnostics
  的内容与 version）。这是 tower-lsp 自己测试用例的写法
  （[tower-lsp service.rs 内置测试](https://docs.rs/tower-lsp/latest/src/tower_lsp/service.rs.html)；
  官方讨论 [issue #355](https://github.com/ebkalderon/tower-lsp/issues/355)）。
- **typed TestServer 封装**：veryl 语言服务器把上述模式包成 `TestServer`
  （[veryl tests.rs](https://github.com/veryl-lang/veryl/blob/master/crates/languageserver/src/tests.rs)），
  earthfilels 用 typed request helper（[discussions #418](https://github.com/ebkalderon/tower-lsp/discussions/418)）。
  我们可以抄一个 ~60 行的 `TestServer`（附录 A.4）。
- **stdio 端到端**：`tokio::io::duplex` 喂 LSP 帧，真跑 `Server::serve`（typos-lsp 的
  做法，见 #355 里引用）。适合最后兜底，不必每个 case 都走。
- **rust-analyzer 的思路**：把绝大多数测试放在 LSP 之下的 ide/分析层（直接调查询），
  LSP 层只留少量冒烟。对我们对应："check_document 的黄金测试在 front 层做
  （已有语料测试），LSP 层只测 handler 接线 + version 传递"。
- **Lean 的交互测试**：客户端能发 `textDocument/waitForDiagnostics` 等测试专用方法
  等待诊断到位（[ProtocolOverview.lean]）。我们可加 `$/soko/waitForCheck` 之类的
  测试钩子（custom_method），让"编辑后悬停"类 e2e 测试可确定性等待。

---

## 7. 性能：增量、缓存与防抖

- **声明粒度增量**（I8）：check-then-add——失败的声明不进环境，重查从首个受影响声明
  起的后缀；前面声明的检查结果（含类型图/hover 数据）整体复用。对照 Lean：命令
  快照链复用（§5）；对照 salsa：这是把"脏传播边界"手工放在声明边界（§2.3）。
- **解析缓存**：`Doc` 里存 `(version, parse_result)`；只有文本变了才重 parse。
  后续 I8 再把"parse 结果 → 声明边界列表"缓存下来，用于计算受影响后缀。
- **防抖与 only-latest-wins**：连打字时不要每个 didChange 都排队全量重编译——
  spawn 前先看"是否已有更新的版本在等"，发布前再校验版本（附录 A.2）。
  rust-analyzer 把诊断任务分成 syntax（快）与 semantic（慢、可取消）两档，只挑
  latency-sensitive 线程跑（[main_loop.rs]）——我们的对应做法：parse 错误先即时发布，
  kernel 检查异步化。
- **取消过时分析**：发布前比对版本；版本不匹配就丢弃结果。这比真的中断编译简单
  （浪费一点 CPU，换取极简实现——教学文件编译在毫秒级，可接受）。
- **不要让 hover 等 kernel**：hover/symbol 永远读上一份完整 report（哪怕旧一版）；
  空报告时返回 None。Lean 用 `withWaitFindSnap` 选择性等待光标处快照，我们 v1 直接
  用 last-good report 即可。

---

## 附录 A：代码模式（可直接抄）

### A.1 多文档 store + 版本（把单 `Mutex<Doc>` 一般化）

```rust
struct Document {
    version: i32,
    text: String,
    report: Option<DocumentReport>, // 上一次完整检查结果（不可变）
}

#[derive(Default)]
struct DocStore {
    open: HashMap<Uri, Document>,
}

struct Backend {
    client: Client,
    docs: Mutex<DocStore>,
    latest: AtomicI64, // 全局单调计数，用于 only-latest-wins
}
```

### A.2 版本守卫 + only-latest-wins 的 refresh

```rust
async fn refresh(&self, uri: Url, version: i32, text: String) {
    let ticket = self.latest.fetch_add(1, Ordering::SeqCst) + 1;
    // 1) 先换文本（快，拿短锁）
    {
        let mut docs = self.docs.lock().unwrap();
        docs.open.insert(uri.clone(), Document { version, text: text.clone(), report: None });
    }
    // 2) 编译放 spawn（慢活不占 handler）
    let this = self.clone(); // Arc<Backend> 或拆出所需字段
    tokio::spawn(async move {
        let compiled = sokonanoda_front::compile::check_text(&text); // 纯函数
        // 3) 发布前校验：期间出现过更新的 ticket / 更新的版本 → 丢弃
        if this.latest.load(Ordering::SeqCst) != ticket { return; }
        let mut docs = this.docs.lock().unwrap();
        let Some(doc) = docs.open.get_mut(&uri) else { return };
        if doc.version != version { return; }
        doc.report = compiled.report.clone();
        drop(docs);
        let _ = this.client.publish_diagnostics(uri, compiled.diagnostics, Some(version)).await;
    });
}

async fn did_change(&self, params: DidChangeTextDocumentParams) {
    let v = params.text_document.version;
    if let Some(change) = params.content_changes.last() { // FULL 同步取最后一份全文
        self.refresh(params.text_document.uri, v, change.text.clone()).await;
    }
}
```

### A.3 INCREMENTAL 同步的 change 应用（I8 之后需要时再加）

```rust
// LSP position 按 UTF-16 code unit 计数；对 ASCII 文件与 char 索引一致。
// 一般做法：维护 line → (byte_start, utf16_len) 索引，把 Range 换算成字节偏移再 splice。
fn apply_change(text: &str, line_index: &LineIndex, range: Range, new_text: &str) -> String {
    let s = line_index.offset_utf16(text, range.start); // → byte offset
    let e = line_index.offset_utf16(text, range.end);
    let mut out = String::with_capacity(text.len() + new_text.len());
    out.push_str(&text[..s]);
    out.push_str(new_text);
    out.push_str(&text[e..]);
    out
}
// 多个 change 按序应用；无 range 的 change = 整文替换。
```

> 实现参考可看 rust-analyzer 的 `line_index.rs`（UTF-16/UTF-8 换算 + 缓存行索引）。

### A.4 LSP 单测骨架（内存内，不走 stdio）

```rust
use tower::ServiceExt;
use tower_lsp::jsonrpc::{Request as RpcRequest, Response as RpcResponse};
use tower_lsp::lsp_types::*;
use tower_lsp::LspService;
use futures::{SinkExt, StreamExt};

#[tokio::test]
async fn publish_diagnostics_carries_version() {
    let (mut service, mut socket) = LspService::new(Backend::new);
    // initialize
    let init = RpcRequest::build("initialize").params(json!({"capabilities": {}})).id(1).finish();
    let _ = service.ready().await.unwrap().call(init).await;
    // didOpen → 等待 server→client 的 publishDiagnostics
    let open = json!({"textDocument": {"uri": "file:///t.sokonanoda", "languageId": "sokonanoda", "version": 1, "text": "def id (n : Nat) : Nat := n\n"}});
    let did_open = RpcRequest::build("textDocument/didOpen").params(open).finish(); // notification
    service.ready().await.unwrap().call(did_open).await;
    let notif = socket.next().await.unwrap();
    // 断言 notif 是 textDocument/publishDiagnostics 且 params.version == 1
}
```

（`socket` 同时是 `Stream<Item=Request>` 与 `Sink<Response>`：客户端→服务器的请求
也能 mock 回复，见 [tower-lsp#355] 官方写法。）

### A.5 I9 自定义 RPC 通道（tower-lsp 写法）

```rust
let (service, socket) = LspService::build(Backend::new)
    .custom_method("$/soko/goal", Backend::goal_rpc) // params: {uri, version, line, character}
    .finish();
// handler 里读当前 report 的 goal/上下文，返回结构化 JSON（对标 Lean 的
// getInteractiveGoals）；VS Code 壳用 sendRequest 拉取并渲染面板。
```

---

## 参考链接汇总

- 框架对比与状态：[tower-lsp](https://github.com/ebkalderon/tower-lsp) ·
  [tower-lsp-server](https://github.com/tower-lsp-community/tower-lsp-server) ·
  [async-lsp](https://github.com/oxalica/async-lsp/) ·
  [lsp-server](https://crates.io/crates/lsp-server) ·
  [tower-lsp#284 乱序问题](https://github.com/ebkalderon/tower-lsp/issues/284) ·
  [2026 教程（建议用分叉）](https://codeinput.com/blog/lsp-server)
- rust-analyzer 架构：[architecture.md](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/architecture.md) ·
  [main_loop.rs](https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/main_loop.rs) ·
  [并发模型 issue #7444](https://github.com/rust-analyzer/rust-analyzer/issues/7444)
- salsa：[How Salsa works](https://salsa-rs.github.io/salsa/how_salsa_works.html) ·
  [Algorithm](https://salsa-rs.github.io/salsa/reference/algorithm.html) ·
  [tutorial/parser](https://salsa-rs.github.io/salsa/tutorial/parser.html)
- LSP 规范：[3.17 specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/) ·
  [版本校验 issue #1706](https://github.com/microsoft/language-server-protocol/issues/1706) ·
  [push vs pull issue #1743](https://github.com/microsoft/language-server-protocol/issues/1743) ·
  [LSP 好坏丑（michaelpj）](https://www.michaelpj.com/blog/2024/09/03/lsp-good-bad-ugly.html)
- Lean 4 server：[Server/README.md](https://github.com/leanprover/lean4/blob/master/src/Lean/Server/README.md) ·
  [ProtocolOverview.lean](https://github.com/leanprover/lean4/blob/master/src/Lean/Server/ProtocolOverview.lean) ·
  [WidgetRequests.lean](https://github.com/leanprover/lean4/blob/master/src/Lean/Server/FileWorker/WidgetRequests.lean) ·
  [Language/Basic.lean](https://github.com/leanprover/lean4/blob/master/src/Lean/Language/Basic.lean)
- 测试：[tower-lsp#355](https://github.com/ebkalderon/tower-lsp/issues/355) ·
  [discussions #418](https://github.com/ebkalderon/tower-lsp/discussions/418) ·
  [veryl TestServer](https://github.com/veryl-lang/veryl/blob/master/crates/languageserver/src/tests.rs)
